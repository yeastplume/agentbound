//! agentbound-gateway: on-host mediation daemon (ADR-0002 Decisions 1–5).
//! One control socket (root callers: launch, lifecycle) and one listening
//! SEQPACKET socket per projected session. Every connection is bound to one
//! process instance and one active launch record; every packet carries a
//! kernel credential that must match that instance. Adapters are typed.
mod auth;
mod session;
mod adapters;
mod git;
use ab_common::json::Value;
use ab_common::wire;
use std::collections::HashMap;
use std::os::fd::{AsRawFd, OwnedFd, RawFd};

pub struct Config { pub lifecycle_sock: String, pub socket_dir: String, pub catalogue: Value, pub git_root: String, pub credential: String, pub quarantine: String, pub audit: ab_common::audit::Sink, pub max_conns_per_session: usize }

/// A projected session: its listener, admission flag and grants (loaded from the committed record).
pub struct Projection { pub authorization_id: String, pub allocation_id: String, pub uid: u32, pub gid: u32, pub path: String, pub listener: OwnedFd, pub lrd: Option<String>, pub admission: bool, pub record: Option<Value>, pub ops: Vec<Value>, pub bytes_used: u64, pub op_count: u64 }

pub struct Gateway { pub cfg: Config, pub by_alloc: HashMap<String, Projection>, pub conns: Vec<session::Conn> }

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str, d: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned().unwrap_or_else(|| d.to_string());
    let catalogue = ab_common::json::parse(&std::fs::read(arg("--catalogue", "/etc/agentbound/catalogue.json")).expect("catalogue"), &ab_common::json::MANIFEST_LIMITS).expect("catalogue parse");
    let cfg = Config { lifecycle_sock: arg("--lifecycle-socket", "/run/agentbound/lifecycle.sock"), socket_dir: arg("--socket-dir", "/run/agentbound/gw"), catalogue, git_root: arg("--git-root", "/var/lib/agentbound/git"), credential: arg("--credential", "/var/lib/agentbound/gateway/credential"), quarantine: arg("--quarantine", "/var/lib/agentbound/gateway/quarantine"), audit: ab_common::audit::Sink::open(&arg("--audit-spool", "/var/lib/agentbound/gateway/audit-gateway.jsonl")), max_conns_per_session: arg("--max-conns", "16").parse().unwrap_or(16) };
    let _ = std::fs::create_dir_all(&cfg.socket_dir); let _ = std::fs::create_dir_all(&cfg.quarantine);
    let mut gw = Gateway { cfg, by_alloc: HashMap::new(), conns: Vec::new() };
    gw.reconstruct();
    let control = wire::listen(&arg("--socket", "/run/agentbound/gateway.sock"), 0o660).expect("listen control");
    loop {
        // poll: control listener, every session listener, every connection (data + peer pidfd exit)
        let mut pfds: Vec<libc::pollfd> = vec![libc::pollfd { fd: control.as_raw_fd(), events: libc::POLLIN, revents: 0 }];
        let allocs: Vec<String> = gw.by_alloc.keys().cloned().collect();
        for a in &allocs { pfds.push(libc::pollfd { fd: gw.by_alloc[a].listener.as_raw_fd(), events: libc::POLLIN, revents: 0 }); }
        let nconn = gw.conns.len();
        for c in &gw.conns { pfds.push(libc::pollfd { fd: c.fd.as_raw_fd(), events: libc::POLLIN, revents: 0 }); pfds.push(libc::pollfd { fd: c.pidfd.as_raw_fd(), events: libc::POLLIN, revents: 0 }); }
        let n = unsafe { libc::poll(pfds.as_mut_ptr(), pfds.len() as libc::nfds_t, 500) };
        if n <= 0 { continue; }
        if pfds[0].revents != 0 { if let Ok(c) = wire::accept(&control) { gw.control(c); } }
        for (i, a) in allocs.iter().enumerate() { if pfds[1 + i].revents != 0 { gw.accept_session(a); } }
        let base = 1 + allocs.len();
        let mut drop_idx = Vec::new();
        for i in 0..nconn {
            let (data, exit) = (pfds[base + 2 * i].revents != 0, pfds[base + 2 * i + 1].revents != 0);
            if exit { gw.close_conn(i, "peer_exited"); drop_idx.push(i); continue; }
            if data && !gw.handle_packet(i) { drop_idx.push(i); }
        }
        for i in drop_idx.into_iter().rev() { gw.conns.remove(i); }
    }
}

impl Gateway {
    fn lc(&self, op: &str, body: Value) -> Option<Value> { wire::connect(&self.cfg.lifecycle_sock).ok()?.call(&wire::request(op, &format!("gw-{}", ab_common::sig::monotonic_ns()), body)).ok().filter(|r| r.get("ok").and_then(|x| x.as_bool()) == Some(true)).and_then(|r| r.get("body").cloned()) }
    /// D4.7: on start, rebuild projections only for records lifecycle still reports live; no connection survives.
    fn reconstruct(&mut self) {
        let Some(list) = self.lc("list", Value::obj(vec![])) else { eprintln!("gateway: lifecycle unreachable; starting with no projections"); return };
        for s in list.get("sessions").and_then(|x| x.as_arr()).cloned().unwrap_or_default() {
            let (Some(lrd), Some(st)) = (s.get("launch_record_digest").and_then(|x| x.as_str()), s.get("state").and_then(|x| x.as_str())) else { continue };
            if !matches!(st, "active" | "quiescing" | "degraded") { continue; }
            let Some(rec) = self.lc("record", Value::obj(vec![("launch_record_digest", Value::s(lrd))])) else { continue };
            let b = rec.get("binding").cloned().unwrap_or(Value::Null);
            let g = |p: &[&str]| -> String { let mut v = Some(&b); for k in p { v = v.and_then(|x| x.get(k)); } v.and_then(|x| x.as_str()).unwrap_or("").to_string() };
            if g(&["authorization_manifest", "gateway", "channel_topology"]) != "local-socket" { continue; }
            let (az, aid) = (g(&["authorization_manifest", "authorization_id"]), g(&["launch_binding", "execution_identity", "allocation_id"]));
            let uid = b.get("launch_binding").and_then(|x| x.get("execution_identity")).and_then(|x| x.get("uid")).and_then(|x| x.as_int()).unwrap_or(0) as u32;
            if let Ok(p) = self.project(&az, &aid, uid, uid) { let pr = self.by_alloc.get_mut(&aid).unwrap(); pr.lrd = Some(lrd.to_string()); pr.record = Some(b.clone()); pr.ops = b.get("authorization_manifest").and_then(|m| m.get("gateway")).and_then(|g| g.get("operations")).and_then(|o| o.as_arr()).cloned().unwrap_or_default(); pr.admission = st == "active" || st == "degraded"; let _ = p; }
        }
        self.emit("gateway.reconstructed", "ok", &Default::default(), Value::obj(vec![("projections", Value::Int(self.by_alloc.len() as i64))]));
    }
    fn project(&mut self, az: &str, aid: &str, uid: u32, gid: u32) -> Result<String, String> {
        if let Some(p) = self.by_alloc.get(aid) { return Ok(p.path.clone()); }
        let suffix = aid.rsplit(':').next().unwrap_or(aid).to_string();
        let path = format!("{}/{suffix}.sock", self.cfg.socket_dir);
        let _ = std::fs::remove_file(&path);
        // The gateway is unprivileged and cannot chown to the session UID. Reachability is by mount namespace: the
        // node lives in a directory only the gateway traverses (0770 gateway:agentbound) and is bind-mounted into exactly
        // one session; the establishment check (auth.rs) refuses any peer UID other than the allocation's.
        let listener = wire::listen(&path, 0o666).map_err(|e| e.to_string())?;
        self.by_alloc.insert(aid.to_string(), Projection { authorization_id: az.into(), allocation_id: aid.into(), uid, gid, path: path.clone(), listener, lrd: None, admission: false, record: None, ops: vec![], bytes_used: 0, op_count: 0 });
        Ok(path)
    }
    pub fn emit(&mut self, kind: &str, outcome: &str, c: &ab_common::audit::Correlation, detail: Value) { self.cfg.audit.emit(&ab_common::audit::event(kind, "agentbound-gateway", outcome, c, detail)); }
    pub fn corr(p: &Projection) -> ab_common::audit::Correlation { ab_common::audit::Correlation { authorization_id: Some(p.authorization_id.clone()), allocation_id: Some(p.allocation_id.clone()), launch_record_digest: p.lrd.clone(), execution_uid: Some(p.uid), ..Default::default() } }

    /// Control plane: root callers only (launch and lifecycle).
    fn control(&mut self, c: wire::Conn) {
        let Ok(Some(msg)) = c.recv() else { return };
        let reply = match wire::parse_request(&msg) {
            Err(e) => wire::reply_err(wire::CLASS_INVALID, "envelope", e),
            Ok(_) if c.peer.uid != 0 => wire::reply_err(wire::CLASS_UNAUTHENTICATED, "peer_not_permitted", ""),
            Ok(r) => { let s = |k: &str| r.body.get(k).and_then(|x| x.as_str()).map(str::to_string); match r.op {
                "project" => match (s("authorization_id"), s("allocation_id"), r.body.get("uid").and_then(|x| x.as_int()), r.body.get("gid").and_then(|x| x.as_int())) {
                    (Some(az), Some(aid), Some(u), Some(g)) => match self.project(&az, &aid, u as u32, g as u32) { Ok(p) => { let pr = &self.by_alloc[&aid]; let cr = Self::corr(pr); self.emit("gateway.projected", "ok", &cr, Value::obj(vec![("socket_type", Value::s("AF_UNIX/SOCK_SEQPACKET")), ("topology", Value::s("local-socket"))])); wire::reply_ok(Value::obj(vec![("socket_path", Value::s(&p))])) }, Err(e) => wire::reply_err(wire::CLASS_UNAVAILABLE, "project", &e) },
                    _ => wire::reply_err(wire::CLASS_INVALID, "body", "authorization_id, allocation_id, uid, gid") },
                "activate" => match s("launch_record_digest") { Some(lrd) => self.activate(&lrd), None => wire::reply_err(wire::CLASS_INVALID, "body", "launch_record_digest") },
                "deny_admission" => match s("launch_record_digest").and_then(|l| self.by_lrd_mut(&l)) { Some(p) => { p.admission = false; let cr = Self::corr(p); self.emit("gateway.admission_denied", "ok", &cr, Value::obj(vec![("reason", Value::s("lifecycle"))])); wire::reply_ok(Value::obj(vec![("admission", Value::Bool(false))])) }, None => wire::reply_err(wire::CLASS_INVALID, "unknown_record", "") },
                "release" => match s("launch_record_digest").or_else(|| s("allocation_id").and_then(|a| self.by_alloc.get(&a).and_then(|p| p.lrd.clone().or(Some(format!("alloc:{a}")))))) { Some(key) => self.release(&key), None => wire::reply_err(wire::CLASS_INVALID, "body", "launch_record_digest or allocation_id") },
                "status" => match s("launch_record_digest").and_then(|l| self.by_lrd_mut(&l).map(|p| (p.admission, p.allocation_id.clone(), p.op_count, p.bytes_used))) { Some((adm, aid, n, b)) => { let conns = self.conns.iter().filter(|c| c.allocation_id == aid).count(); wire::reply_ok(Value::obj(vec![("admission", Value::Bool(adm)), ("bytes_used", Value::Int(b as i64)), ("connections", Value::Int(conns as i64)), ("operations", Value::Int(n as i64))])) }, None => wire::reply_err(wire::CLASS_INVALID, "unknown_record", "") },
                other => wire::reply_err(wire::CLASS_INVALID, "unknown_op", other) } }
        };
        let _ = c.send(&reply);
    }
    fn by_lrd_mut(&mut self, lrd: &str) -> Option<&mut Projection> { self.by_alloc.values_mut().find(|p| p.lrd.as_deref() == Some(lrd)) }
    /// Grants exist only as the committed record says (D4.7): fetch it from lifecycle, never from the caller.
    fn activate(&mut self, lrd: &str) -> Value {
        let Some(rec) = self.lc("record", Value::obj(vec![("launch_record_digest", Value::s(lrd))])) else { return wire::reply_err(wire::CLASS_UNAVAILABLE, "lifecycle", "record unavailable") };
        let b = rec.get("binding").cloned().unwrap_or(Value::Null);
        let aid = b.get("launch_binding").and_then(|x| x.get("execution_identity")).and_then(|x| x.get("allocation_id")).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let Some(p) = self.by_alloc.get_mut(&aid) else { return wire::reply_err(wire::CLASS_INVALID, "not_projected", &aid) };
        p.lrd = Some(lrd.to_string()); p.record = Some(b.clone()); p.admission = true;
        p.ops = b.get("authorization_manifest").and_then(|m| m.get("gateway")).and_then(|g| g.get("operations")).and_then(|o| o.as_arr()).cloned().unwrap_or_default();
        let (n, cr) = (p.ops.len(), Self::corr(p));
        self.emit("gateway.grants_loaded", "ok", &cr, Value::obj(vec![("operations", Value::Int(n as i64)), ("source", Value::s("launch-record-store"))]));
        wire::reply_ok(Value::obj(vec![("admission", Value::Bool(true)), ("operations", Value::Int(n as i64))]))
    }
    /// Close every indexed connection and remove the projection; reply with the count lifecycle must see as zero.
    fn release(&mut self, key: &str) -> Value {
        let aid = if let Some(a) = key.strip_prefix("alloc:") { a.to_string() } else { match self.by_lrd_mut(key) { Some(p) => p.allocation_id.clone(), None => return wire::reply_ok(Value::obj(vec![("connections_closed", Value::Int(0)), ("remaining", Value::Int(0)), ("released", Value::Bool(false))])) } };
        let mut closed = 0; let mut i = 0;
        while i < self.conns.len() { if self.conns[i].allocation_id == aid { self.close_conn(i, "released"); self.conns.remove(i); closed += 1; } else { i += 1; } }
        if let Some(p) = self.by_alloc.remove(&aid) { let _ = std::fs::remove_file(&p.path); let cr = Self::corr(&p); self.emit("gateway.released", "ok", &cr, Value::obj(vec![("connections_closed", Value::Int(closed))])); }
        let remaining = self.conns.iter().filter(|c| c.allocation_id == aid).count();
        wire::reply_ok(Value::obj(vec![("connections_closed", Value::Int(closed)), ("remaining", Value::Int(remaining as i64)), ("released", Value::Bool(true))]))
    }
    fn accept_session(&mut self, aid: &str) {
        let Some(p) = self.by_alloc.get(aid) else { return };
        let Ok(c) = wire::accept(&p.listener) else { return };
        let cr = Self::corr(p);
        let existing = self.conns.iter().filter(|x| x.allocation_id == aid).count();
        match auth::establish(&c, p, existing, self.cfg.max_conns_per_session) {
            Ok(conn) => { let d = conn.describe(); self.emit("gateway.connection_established", "ok", &cr, d); self.conns.push(conn); }
            Err((rule, detail)) => { self.emit("gateway.connection_refused", "deny", &cr, Value::obj(vec![("detail", Value::s(&detail)), ("peer_pid", Value::Int(c.peer.pid as i64)), ("peer_uid", Value::Int(c.peer.uid as i64)), ("rule", Value::s(rule))])); }
        }
    }
    fn close_conn(&mut self, i: usize, reason: &str) {
        let c = &self.conns[i]; let cr = self.by_alloc.get(&c.allocation_id).map(Self::corr).unwrap_or_default(); let d = Value::obj(vec![("establishing_pid", Value::Int(c.inst.pid as i64)), ("operations", Value::Int(c.ops as i64)), ("reason", Value::s(reason))]);
        self.emit("gateway.connection_closed", "ok", &cr, d);
    }
    /// One packet = one message. Returns false when the connection must be dropped.
    fn handle_packet(&mut self, i: usize) -> bool {
        let fd: RawFd = self.conns[i].fd.as_raw_fd();
        let pk = match wire::recv_packet(fd, session::MAX_PACKET) { Ok(Some(p)) => p, Ok(None) => { self.close_conn(i, "peer_closed"); return false }, Err(_) => { self.close_conn(i, "recv_error"); return false } };
        let outcome = session::handle(self, i, pk);
        if let Err((class, rule, detail, close)) = outcome {
            let cr = self.by_alloc.get(&self.conns[i].allocation_id).map(Self::corr).unwrap_or_default();
            self.emit(if rule == "process_mismatch" { "gateway.process_mismatch" } else if rule == "descriptor_transfer" { "gateway.descriptor_transfer_rejected" } else if class == wire::CLASS_INVALID || rule == "process_mismatch" { "gateway.packet_rejected" } else { "gateway.operation_denied" }, "deny", &cr, if class == wire::CLASS_INVALID || rule == "process_mismatch" { Value::obj(vec![("class", Value::s(class)), ("credential_pid", Value::Int(self.conns[i].last_cred_pid as i64)), ("detail", Value::s(&detail)), ("establishing_pid", Value::Int(self.conns[i].inst.pid as i64)), ("rule", Value::s(rule))]) } else { Value::obj(vec![("class", Value::s(class)), ("credential_pid", Value::Int(self.conns[i].last_cred_pid as i64)), ("detail", Value::s(&detail)), ("establishing_pid", Value::Int(self.conns[i].inst.pid as i64)), ("operation", Value::s(&detail.split(' ').nth(1).unwrap_or("").to_string())), ("operation_seq", Value::Int(0)), ("rule", Value::s(rule))]) });
            let _ = wire::send_raw(fd, &ab_common::json::canonical(&wire::reply_err(class, rule, &detail)));
            if close { self.close_conn(i, rule); return false }
        }
        true
    }
}
