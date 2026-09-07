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
pub struct Projection { pub authorization_id: String, pub allocation_id: String, pub uid: u32, pub gid: u32, pub path: String, pub listener: OwnedFd, pub lrd: Option<String>, pub admission: bool, pub record: Option<Value>, pub ops: Vec<Value>, pub bytes_used: u64, pub op_count: u64,
    /// per-operation-id consumption (operations admitted, payload bytes accepted): the budget state that R-GW-7 enforces. It is
    /// persisted to the lifecycle record store after every admitted operation and restored on reconstruct, so a gateway restart
    /// can never reset a session's budget (WP3.1 item 3).
    pub used: std::collections::BTreeMap<String, (u64, u64)>,
    /// completed-operation state per (operation_id, idempotency_key): (operation name, operation_seq, input digest, original reply).
    /// A repeated key with the same authenticated input returns the original reply; with a different input it is a conflict
    /// (component-interfaces §5). PERSISTED: written to the lifecycle record store in the same `record_budget` call that makes the
    /// consumption durable, and restored on activate/reconstruct. The earlier in-memory-only version was a real defect (independent
    /// WP3.1 validation, finding 6): a session survives a gateway restart, opens a new connection and retries — and a
    /// non-idempotent Git push would have executed twice.
    pub idem: std::collections::BTreeMap<(String, String), (String, i64, String, Value)> }

pub struct Gateway { pub cfg: Config, pub by_alloc: HashMap<String, Projection>, pub conns: Vec<session::Conn>, pub inherited: Vec<(String, OwnedFd)> }

/// Requirement named in a denial (D7 item 9). Rules map to the R-GW / R-ISO requirement whose check produced them.
pub fn requirement_for(rule: &str) -> &'static str {
    match rule {
        // authentication of the peer process instance and of every packet's credential (D2/D3)
        "process_mismatch" | "scope_mismatch" | "uid_mismatch" | "credential_count" | "peer_gone" | "one_connection" | "descriptor_transfer" => "R-GW-3",
        // authorization of the named operation and its arguments against the record's grants (D3)
        "operation_not_granted" | "scope_repository" | "args_schema" | "ref_tail_grammar" | "ref_tail_marker" | "ref_tail_charset" | "ref_tail_empty_or_long" | "ref_tail_names_ref" | "tip_grammar" => "R-GW-4",
        // admission state of the launch record (D4: revocation, deny_admission)
        "admission_closed" | "unknown_record" => "R-GW-2",
        // resource bounds (D1/R-GW-7)
        "budget_bytes" | "budget_operations" | "budget_persist" | "connection_limit" | "oversize_packet" | "payload_overrun" => "R-GW-7",
        // upstream mediation: bundle import, object budget, push refusal (D3/R-GW-5)
        "bundle_invalid" | "bundle_fetch" | "fsck" | "tip_mismatch" | "budget_objects" | "upstream_rejected" | "payload_digest" | "payload_missing" => "R-GW-5",
        // typed-envelope validity: the session spoke something that is not the protocol (R-GW-1)
        "parse" | "envelope" | "version" | "send" | "unknown_op" => "R-GW-1",
        // control-socket rules (lifecycle-facing, never reachable by a session)
        "peer_not_permitted" | "not_projected" => "R-GW-6",
        // an unmapped rule is a defect, not an R-GW-1 catch-all: name it so it is visible in the denial and in tests.
        _ => "R-GW-0-unmapped",
    }
}

fn main() {
    // `--provenance`: print the source provenance embedded at build time and exit (independent WP3.1 validation, finding 4). The
    // conformance runner asks every installed binary, so a stale install is visible as a commit mismatch rather than hidden.
    if std::env::args().nth(1).as_deref() == Some("--provenance") { println!("commit={} dirty={} tree={}", ab_common::provenance::COMMIT, ab_common::provenance::DIRTY, ab_common::provenance::TREE); return; }
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str, d: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned().unwrap_or_else(|| d.to_string());
    let catalogue = ab_common::json::parse(&std::fs::read(arg("--catalogue", "/etc/agentbound/catalogue.json")).expect("catalogue"), &ab_common::json::MANIFEST_LIMITS).expect("catalogue parse");
    let cfg = Config { lifecycle_sock: arg("--lifecycle-socket", "/run/agentbound/lifecycle.sock"), socket_dir: arg("--socket-dir", "/run/agentbound/gw"), catalogue, git_root: arg("--git-root", "/var/lib/agentbound/git"), credential: arg("--credential", "/var/lib/agentbound/gateway/credential"), quarantine: arg("--quarantine", "/var/lib/agentbound/gateway/quarantine"), audit: ab_common::audit::Sink::open(&arg("--audit-spool", "/var/lib/agentbound/gateway/audit-gateway.jsonl")), max_conns_per_session: arg("--max-conns", "16").parse().unwrap_or(16) };
    let _ = std::fs::create_dir_all(&cfg.socket_dir); let _ = std::fs::create_dir_all(&cfg.quarantine);
    let mut gw = Gateway { cfg, by_alloc: HashMap::new(), conns: Vec::new(), inherited: wire::listen_fds() };
    gw.reconstruct(); wire::sd_notify("READY=1\n");
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
        if pfds[0].revents != 0 { if let Ok(c) = wire::accept(&control) { gw.control(c, &control); } }
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
    /// Bounded: lifecycle may itself be calling back into this process (`deny_admission`/`release`), and both daemons serve one
    /// request at a time. A timeout here surfaces as a fail-closed refusal, never as a wedged gateway.
    /// A call to `agentbound-lifecycle`. `record_budget` is bounded much more tightly than the rest: it is the call that can close a
    /// cycle with lifecycle's own gateway calls, and the gateway's fail-closed path for it is cheap (refuse the operation, close
    /// admission) whereas lifecycle's is not.
    fn lc(&self, op: &str, body: Value) -> Option<Value> { let ms = ab_common::wire::GATEWAY_TO_LIFECYCLE_MS.min(if op == "record_budget" { ab_common::wire::BUDGET_PERSIST_MS } else { ab_common::wire::CROSS_DAEMON_MS });
        wire::connect_bounded(&self.cfg.lifecycle_sock, ms).ok()?.call(&wire::request(op, &format!("gw-{}", ab_common::sig::monotonic_ns()), body)).ok().filter(|r| r.get("ok").and_then(|x| x.as_bool()) == Some(true)).and_then(|r| r.get("body").cloned()) }

    /// As `lc`, but keeps serving inbound control requests that need no downstream call while the reply is outstanding
    /// (component-interfaces §3.8, "Progress"). This is what actually breaks the gateway ↔ lifecycle cycle: bounding both sides
    /// short only converts the deadlock into a livelock, because each side keeps re-colliding with the other's retry. Serving
    /// `deny_admission`/`release` here lets lifecycle's in-flight call complete immediately, so its reply to us arrives on the
    /// first attempt. Only calls that cannot re-enter this path are served (`served_inline`); anything else is deferred to the
    /// main loop by leaving the connection unaccepted, exactly as before.
    fn lc_serving(&mut self, control: &std::os::fd::OwnedFd, op: &str, body: Value) -> Option<Value> {
        use std::os::fd::AsRawFd;
        let ms = ab_common::wire::GATEWAY_TO_LIFECYCLE_MS.min(if op == "record_budget" { ab_common::wire::BUDGET_PERSIST_MS } else { ab_common::wire::CROSS_DAEMON_MS });
        let c = wire::connect_bounded(&self.cfg.lifecycle_sock, ms).ok()?;
        c.send(&wire::request(op, &format!("gw-{}", ab_common::sig::monotonic_ns()), body)).ok()?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(ms as u64);
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now()).as_millis() as i32;
            let mut pfds = [libc::pollfd { fd: c.fd.as_raw_fd(), events: libc::POLLIN, revents: 0 },
                            libc::pollfd { fd: control.as_raw_fd(), events: libc::POLLIN, revents: 0 }];
            let n = unsafe { libc::poll(pfds.as_mut_ptr(), 2, left.max(0)) };
            if n <= 0 { return None; }
            if pfds[0].revents != 0 {
                let r = c.recv().ok()??;
                return r.get("ok").and_then(|x| x.as_bool()).filter(|b| *b).and_then(|_| r.get("body").cloned());
            }
            if pfds[1].revents != 0 { if let Ok(ic) = wire::accept(control) { self.control_inline(ic); } }
        }
    }

    /// Serve, while a bounded downstream call is outstanding, only those control operations that are pure local state changes and
    /// therefore cannot re-enter `lc`/`lc_serving`: `project`, `deny_admission`, `release`. Anything else is answered
    /// `unavailable`/`reentrant_call_in_flight` so the caller retries rather than being silently dropped. The permitted operations are
    /// dispatched by the SAME `control` code path as always — a second, hand-copied dispatcher would drift from it (it did, once:
    /// a copy replied `socket_digest` where the real handler replies `socket_path`).
    fn control_inline(&mut self, c: wire::Conn) {
        let Ok(Some(msg)) = c.recv() else { return };
        let reentrant_safe = wire::parse_request(&msg).map(|r| matches!(r.op, "project" | "deny_admission" | "release")).unwrap_or(true);
        if !reentrant_safe {
            let op = wire::parse_request(&msg).map(|r| r.op.to_string()).unwrap_or_default();
            let _ = c.send(&wire::reply_err(wire::CLASS_UNAVAILABLE, "reentrant_call_in_flight", &op));
            return;
        }
        self.control_dispatch(c, msg, None);
    }

    /// Control plane: root callers only (launch and lifecycle).
    fn control(&mut self, c: wire::Conn, control: &std::os::fd::OwnedFd) {
        let Ok(Some(msg)) = c.recv() else { return };
        self.control_dispatch(c, msg, Some(control));
    }

    /// `control` receives, this dispatches. `control_fd` is `None` when we are already inside a bounded downstream call, in which case
    /// no operation reaching here is allowed to start another one.
    fn control_dispatch(&mut self, c: wire::Conn, msg: Value, control_fd: Option<&std::os::fd::OwnedFd>) {
        let reply = match wire::parse_request(&msg) {
            Err(e) => wire::reply_err(wire::CLASS_INVALID, "envelope", e),
            Ok(_) if c.peer.uid != 0 => wire::reply_err(wire::CLASS_UNAUTHENTICATED, "peer_not_permitted", ""),
            Ok(r) => { let s = |k: &str| r.body.get(k).and_then(|x| x.as_str()).map(str::to_string); match r.op {
                "project" => match (s("authorization_id"), s("allocation_id"), r.body.get("uid").and_then(|x| x.as_int()), r.body.get("gid").and_then(|x| x.as_int())) {
                    (Some(az), Some(aid), Some(u), Some(g)) => match self.project(&az, &aid, u as u32, g as u32) { Ok(p) => { let pr = &self.by_alloc[&aid]; let cr = Self::corr(pr); self.emit("gateway.projected", "ok", &cr, Value::obj(vec![("socket_type", Value::s("AF_UNIX/SOCK_SEQPACKET")), ("topology", Value::s("local-socket"))])); wire::reply_ok(Value::obj(vec![("socket_path", Value::s(&p))])) }, Err(e) => wire::reply_err(wire::CLASS_UNAVAILABLE, "project", &e) },
                    _ => wire::reply_err(wire::CLASS_INVALID, "body", "authorization_id, allocation_id, uid, gid") },
                "activate" => match (s("launch_record_digest"), control_fd) { (Some(lrd), Some(cf)) => self.activate(&lrd, cf),
                    (Some(_), None) => wire::reply_err(wire::CLASS_UNAVAILABLE, "reentrant_call_in_flight", "activate"),
                    (None, _) => wire::reply_err(wire::CLASS_INVALID, "body", "launch_record_digest") },
                "deny_admission" => match s("launch_record_digest").and_then(|l| self.by_lrd_mut(&l)) { Some(p) => { p.admission = false; let cr = Self::corr(p); self.emit("gateway.admission_denied", "ok", &cr, Value::obj(vec![("reason", Value::s("lifecycle"))])); wire::reply_ok(Value::obj(vec![("admission", Value::Bool(false))])) }, None => wire::reply_err(wire::CLASS_INVALID, "unknown_record", "") },
                "release" => match s("launch_record_digest").or_else(|| s("allocation_id").and_then(|a| self.by_alloc.get(&a).and_then(|p| p.lrd.clone().or(Some(format!("alloc:{a}")))))) { Some(key) => { let keep = s("fault").as_deref() == Some("socket-unmount"); self.release_with(&key, keep) }, None => wire::reply_err(wire::CLASS_INVALID, "body", "launch_record_digest or allocation_id") },
                "status" => match s("launch_record_digest").and_then(|l| self.by_lrd_mut(&l).map(|p| (p.admission, p.allocation_id.clone(), p.op_count, p.bytes_used))) { Some((adm, aid, n, b)) => { let conns = self.conns.iter().filter(|c| c.allocation_id == aid).count(); wire::reply_ok(Value::obj(vec![("admission", Value::Bool(adm)), ("bytes_used", Value::Int(b as i64)), ("connections", Value::Int(conns as i64)), ("operations", Value::Int(n as i64))])) }, None => wire::reply_err(wire::CLASS_INVALID, "unknown_record", "") },
                other => wire::reply_err(wire::CLASS_INVALID, "unknown_op", other) } }
        };
        let _ = c.send(&reply);
    }
    /// The latest `budget` record for a session, as lifecycle returns it with the binding: {operation_id: {operations, bytes}}.
    /// The latest `outcomes` record: {"<operation_id> <key>": {input_digest, operation, operation_seq, reply}} → idem map.
    fn reconstruct(&mut self) {
        // boot ordering: lifecycle is Type=simple, so After= does not imply its socket is bound yet — retry for up to ~10 s
        let mut list = None;
        for i in 0..20 { list = self.lc("list", Value::obj(vec![])); if list.is_some() { break; } std::thread::sleep(std::time::Duration::from_millis(500)); if i == 19 { eprintln!("gateway: lifecycle unreachable after 10 s; starting with no projections"); } }
        let Some(list) = list else { return };
        for s in list.get("sessions").and_then(|x| x.as_arr()).cloned().unwrap_or_default() {
            let (Some(lrd), Some(st)) = (s.get("launch_record_digest").and_then(|x| x.as_str()), s.get("state").and_then(|x| x.as_str())) else { continue };
            if !matches!(st, "active" | "quiescing" | "degraded") { continue; }
            let Some(rec) = self.lc("record", Value::obj(vec![("launch_record_digest", Value::s(lrd))])) else { continue };
            let b = rec.get("binding").cloned().unwrap_or(Value::Null);
            let g = |p: &[&str]| -> String { let mut v = Some(&b); for k in p { v = v.and_then(|x| x.get(k)); } v.and_then(|x| x.as_str()).unwrap_or("").to_string() };
            if g(&["authorization_manifest", "gateway", "channel_topology"]) != "local-socket" { continue; }
            let (az, aid) = (g(&["authorization_manifest", "authorization_id"]), g(&["launch_binding", "execution_identity", "allocation_id"]));
            let uid = b.get("launch_binding").and_then(|x| x.get("execution_identity")).and_then(|x| x.get("uid")).and_then(|x| x.as_int()).unwrap_or(0) as u32;
            // D4.7 / WP3.1 finding 6: completed idempotency outcomes are restored here too, not only in `activate`. A gateway that
            // came back through reconstruct with an empty `idem` map would re-execute a retried non-idempotent adapter operation.
            if let Ok(p) = self.project(&az, &aid, uid, uid) { let used = Self::budget_from_record(&rec); let outcomes = Self::outcomes_from_record(&rec); let pr = self.by_alloc.get_mut(&aid).unwrap(); pr.lrd = Some(lrd.to_string()); pr.record = Some(b.clone()); pr.idem = outcomes; pr.used = used.clone(); pr.op_count = used.values().map(|u| u.0).sum(); pr.bytes_used = used.values().map(|u| u.1).sum(); pr.ops = b.get("authorization_manifest").and_then(|m| m.get("gateway")).and_then(|g| g.get("operations")).and_then(|o| o.as_arr()).cloned().unwrap_or_default(); pr.admission = st == "active" || st == "degraded"; let _ = p; }
        }
        let stale: Vec<String> = self.inherited.drain(..).map(|(n, _)| n).collect();
        for n in &stale { wire::fdstore_remove(n); let _ = std::fs::remove_file(format!("{}/{n}.sock", self.cfg.socket_dir)); }
        self.emit("gateway.reconstructed", "ok", &Default::default(), Value::obj(vec![("projections", Value::Int(self.by_alloc.len() as i64)), ("stale_descriptors_dropped", Value::Int(stale.len() as i64))]));
    }
    fn project(&mut self, az: &str, aid: &str, uid: u32, gid: u32) -> Result<String, String> {
        if let Some(p) = self.by_alloc.get(aid) { return Ok(p.path.clone()); }
        let suffix = aid.rsplit(':').next().unwrap_or(aid).to_string();
        let path = format!("{}/{suffix}.sock", self.cfg.socket_dir);
        // D4.7 restart: the bound listener the session's mount points at is recovered from the systemd fd store (named by
        // allocation suffix); projection state itself is rebuilt from the launch-record store, never from the fd.
        if let Some(pos) = self.inherited.iter().position(|(n, _)| *n == suffix) {
            let (_, listener) = self.inherited.remove(pos);
            self.by_alloc.insert(aid.to_string(), Projection { authorization_id: az.into(), allocation_id: aid.into(), uid, gid, path: path.clone(), listener, lrd: None, admission: false, record: None, idem: Default::default(), ops: vec![], bytes_used: 0, op_count: 0, used: Default::default() });
            return Ok(path);
        }
        let _ = std::fs::remove_file(&path);
        // The gateway is unprivileged and cannot chown to the session UID. Reachability is by mount namespace: the
        // node lives in a directory only the gateway traverses (0770 gateway:agentbound) and is bind-mounted into exactly
        // one session; the establishment check (auth.rs) refuses any peer UID other than the allocation's.
        let listener = wire::listen(&path, 0o666).map_err(|e| e.to_string())?;
        if let Err(e) = wire::fdstore_push(&suffix, listener.as_raw_fd()) { eprintln!("gateway: fd store unavailable ({e}); a restart will orphan this session's socket node"); }
        self.by_alloc.insert(aid.to_string(), Projection { authorization_id: az.into(), allocation_id: aid.into(), uid, gid, path: path.clone(), listener, lrd: None, admission: false, record: None, idem: Default::default(), ops: vec![], bytes_used: 0, op_count: 0, used: Default::default() });
        Ok(path)
    }
    /// D7 item 8: audit loss follows the manifest's `audit.loss_behaviour`. The gateway cannot stop a session itself; on `quarantine`
    /// or `stop` it closes admission for every projection whose events are being lost and asks lifecycle to act, then stops
    /// forwarding new operations (fail closed). `continue-degraded` keeps admitting and counts.
    pub fn emit(&mut self, kind: &str, outcome: &str, c: &ab_common::audit::Correlation, detail: Value) {
        let before = self.cfg.audit.lost;
        self.cfg.audit.emit(&ab_common::audit::event(kind, "agentbound-gateway", outcome, c, detail));
        if self.cfg.audit.lost > before { self.audit_lost(); }
    }
    fn audit_lost(&mut self) {
        let keys: Vec<String> = self.by_alloc.keys().cloned().collect();
        for aid in keys {
            let p = &self.by_alloc[&aid];
            let beh = p.record.as_ref().and_then(|r| r.get("authorization_manifest")).and_then(|m| m.get("audit")).and_then(|a| a.get("loss_behaviour")).and_then(|x| x.as_str()).unwrap_or("stop").to_string();
            if beh == "continue-degraded" { continue; }
            let lrd = p.lrd.clone();
            self.by_alloc.get_mut(&aid).unwrap().admission = false;
            if let Some(l) = lrd { let _ = self.lc("revocation_signal", Value::obj(vec![("launch_record_digest", Value::s(&l)), ("source", Value::s("agentbound-gateway")), ("trigger", Value::s("audit_pipeline_degraded_below_stop_threshold"))])); }
            eprintln!("gateway: audit loss with behaviour {beh}: admission closed for {aid}");
        }
    }
    pub fn corr(p: &Projection) -> ab_common::audit::Correlation { ab_common::audit::Correlation { authorization_id: Some(p.authorization_id.clone()), allocation_id: Some(p.allocation_id.clone()), launch_record_digest: p.lrd.clone(), execution_uid: Some(p.uid), trace_id: p.record.as_ref().and_then(|r| r.get("authorization_manifest")).and_then(|m| m.get("session_trace")).and_then(|t| t.get("trace_id")).and_then(|x| x.as_str()).map(str::to_string), session_id: p.record.as_ref().and_then(|r| r.get("authorization_manifest")).and_then(|m| m.get("session_trace")).and_then(|t| t.get("session_id")).and_then(|x| x.as_str()).map(str::to_string), ..Default::default() } }

    /// Control plane: root callers only (launch and lifecycle).
    fn outcomes_from_record(rec: &Value) -> std::collections::BTreeMap<(String, String), (String, i64, String, Value)> {
        rec.get("outcomes").and_then(|o| o.as_obj()).map(|m| m.iter().filter_map(|(k, v)| { let (opid, key) = k.0.split_once(' ')?;
            Some(((opid.to_string(), key.to_string()), (v.get("operation").and_then(|x| x.as_str()).unwrap_or("").to_string(), v.get("operation_seq").and_then(|x| x.as_int()).unwrap_or(0),
                v.get("input_digest").and_then(|x| x.as_str()).unwrap_or("").to_string(), v.get("reply").cloned().unwrap_or(Value::Null)))) }).collect()).unwrap_or_default()
    }
    fn budget_from_record(rec: &Value) -> std::collections::BTreeMap<String, (u64, u64)> {
        rec.get("budget").and_then(|b| b.as_obj()).map(|m| m.iter().map(|(k, v)| (k.0.clone(), (v.get("operations").and_then(|x| x.as_int()).unwrap_or(0) as u64, v.get("bytes").and_then(|x| x.as_int()).unwrap_or(0) as u64))).collect()).unwrap_or_default()
    }
    /// Persist a projection's budget consumption to the lifecycle record store (durable, hash-chained). Failure to persist closes
    /// admission: an operation whose consumption cannot be recorded must not be followed by another.
    pub fn persist_budget(&mut self, aid: &str) -> bool {
        let Some(p) = self.by_alloc.get(aid) else { return false };
        let Some(lrd) = p.lrd.clone() else { return true };
        let body = Value::obj(p.used.iter().map(|(k, (o, b))| (k.as_str(), Value::obj(vec![("bytes", Value::Int(*b as i64)), ("operations", Value::Int(*o as i64))]))).collect());
        let keys: Vec<(String, Value)> = p.idem.iter().map(|((opid, key), (name, seq, digest, reply))| (format!("{opid} {key}"),
            Value::obj(vec![("input_digest", Value::s(digest)), ("operation", Value::s(name)), ("operation_seq", Value::Int(*seq)), ("reply", reply.clone())]))).collect();
        let outcomes = Value::obj(keys.iter().map(|(k, v)| (k.as_str(), v.clone())).collect());
        let ok = self.lc("record_budget", Value::obj(vec![("launch_record_digest", Value::s(&lrd)), ("budget", body), ("outcomes", outcomes)])).is_some();
        if !ok { let cr = Self::corr(&self.by_alloc[aid]); self.emit("gateway.admission_denied", "ok", &cr, Value::obj(vec![("reason", Value::s("budget_persist_failed"))])); self.by_alloc.get_mut(aid).unwrap().admission = false; }
        ok
    }
    fn by_lrd_mut(&mut self, lrd: &str) -> Option<&mut Projection> { self.by_alloc.values_mut().find(|p| p.lrd.as_deref() == Some(lrd)) }
    /// Grants exist only as the committed record says (D4.7): fetch it from lifecycle, never from the caller.
    fn activate(&mut self, lrd: &str, control: &std::os::fd::OwnedFd) -> Value {
        // §3.8: keep serving lifecycle's own inbound calls while this one is outstanding, or the two daemons livelock on each other's retries
        let Some(rec) = self.lc_serving(control, "record", Value::obj(vec![("launch_record_digest", Value::s(lrd))])) else { return wire::reply_err(wire::CLASS_UNAVAILABLE, "lifecycle", "record unavailable") };
        let b = rec.get("binding").cloned().unwrap_or(Value::Null);
        let aid = b.get("launch_binding").and_then(|x| x.get("execution_identity")).and_then(|x| x.get("allocation_id")).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let Some(p) = self.by_alloc.get_mut(&aid) else { return wire::reply_err(wire::CLASS_INVALID, "not_projected", &aid) };
        p.lrd = Some(lrd.to_string()); p.record = Some(b.clone()); p.admission = true;
        p.used = Self::budget_from_record(&rec); p.op_count = p.used.values().map(|u| u.0).sum(); p.bytes_used = p.used.values().map(|u| u.1).sum();
        p.idem = Self::outcomes_from_record(&rec);
        p.ops = b.get("authorization_manifest").and_then(|m| m.get("gateway")).and_then(|g| g.get("operations")).and_then(|o| o.as_arr()).cloned().unwrap_or_default();
        let (n, cr) = (p.ops.len(), Self::corr(p));
        self.emit("gateway.grants_loaded", "ok", &cr, Value::obj(vec![("operations", Value::Int(n as i64)), ("source", Value::s("launch-record-store"))]));
        wire::reply_ok(Value::obj(vec![("admission", Value::Bool(true)), ("operations", Value::Int(n as i64))]))
    }
    /// Close every indexed connection and remove the projection; reply with the count lifecycle must see as zero.
    fn release(&mut self, key: &str) -> Value { self.release_with(key, false) }
    /// `keep_node` is the F-T-09 fault: step 9 (remove the mounted gateway socket) fails while the projection is still released.
    /// The node then exists with no listener behind it — the gateway is inaccessible and the launch record is retained.
    fn release_with(&mut self, key: &str, keep_node: bool) -> Value {
        let aid = if let Some(a) = key.strip_prefix("alloc:") { a.to_string() } else { match self.by_lrd_mut(key) { Some(p) => p.allocation_id.clone(), None => return wire::reply_ok(Value::obj(vec![("connections_closed", Value::Int(0)), ("remaining", Value::Int(0)), ("released", Value::Bool(false))])) } };
        let mut closed = 0; let mut i = 0;
        while i < self.conns.len() { if self.conns[i].allocation_id == aid { self.close_conn(i, "released"); self.conns.remove(i); closed += 1; } else { i += 1; } }
        if let Some(p) = self.by_alloc.remove(&aid) { if !keep_node { let _ = std::fs::remove_file(&p.path); } else { self.emit("gateway.socket_removal_failed", "hold", &Self::corr(&p), Value::obj(vec![("path_digest", Value::s(&ab_common::sig::sha256_hex(p.path.as_bytes())[..16]))])); } wire::fdstore_remove(aid.rsplit(':').next().unwrap_or(&aid)); let cr = Self::corr(&p); self.emit("gateway.released", "ok", &cr, Value::obj(vec![("connections_closed", Value::Int(closed))])); }
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
            self.emit(if rule == "process_mismatch" { "gateway.process_mismatch" } else if rule == "descriptor_transfer" { "gateway.descriptor_transfer_rejected" } else if class == wire::CLASS_INVALID || rule == "process_mismatch" { "gateway.packet_rejected" } else { "gateway.operation_denied" }, "deny", &cr, if class == wire::CLASS_INVALID || rule == "process_mismatch" { Value::obj(vec![("class", Value::s(class)), ("credential_pid", Value::Int(self.conns[i].last_cred_pid as i64)), ("detail", Value::s(&detail)), ("establishing_pid", Value::Int(self.conns[i].inst.pid as i64)), ("rule", Value::s(rule))]) } else { Value::obj(vec![("class", Value::s(class)), ("credential_pid", Value::Int(self.conns[i].last_cred_pid as i64)), ("detail", Value::s(&detail)), ("establishing_pid", Value::Int(self.conns[i].inst.pid as i64)), ("idempotency_key", Value::s(&self.conns[i].last_idem)), ("operation", Value::s(&detail.split(' ').nth(1).unwrap_or("").to_string())), ("operation_seq", Value::Int(0)), ("rule", Value::s(rule))]) });
            // D7 item 9: a denial names the requirement, the authorization, the launch record and the trace — of this session only
            let mut reply = wire::reply_err(class, rule, &detail);
            let mut b = reply.get("body").cloned().unwrap_or(Value::Null);
            let opt = |x: &Option<String>| x.as_ref().map(|x| Value::s(x)).unwrap_or(Value::Null);
            b.set("requirement_id", Value::s(requirement_for(rule))); b.set("authorization_id", opt(&cr.authorization_id)); b.set("launch_record_digest", opt(&cr.launch_record_digest)); b.set("trace_id", opt(&cr.trace_id));
            reply.set("body", b);
            let _ = wire::send_raw(fd, &ab_common::json::canonical(&reply));
            if close { self.close_conn(i, rule); return false }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    /// Every rule string the gateway can emit MUST map to a requirement; `R-GW-0-unmapped` is a defect marker.
    /// The list is maintained by hand and cross-checked by `grep -o 'CLASS_[A-Z]*, "[a-z_]*"' src/*.rs` when a rule is added.
    #[test]
    fn every_rule_maps() {
        for r in ["process_mismatch", "scope_mismatch", "uid_mismatch", "credential_count", "peer_gone", "one_connection", "descriptor_transfer",
                  "operation_not_granted", "scope_repository", "args_schema", "ref_tail_grammar", "ref_tail_marker", "ref_tail_charset",
                  "ref_tail_empty_or_long", "ref_tail_names_ref", "tip_grammar", "admission_closed", "unknown_record", "budget_bytes",
                  "budget_operations", "budget_persist", "connection_limit", "oversize_packet", "payload_overrun", "bundle_invalid", "bundle_fetch", "fsck",
                  "tip_mismatch", "budget_objects", "upstream_rejected", "payload_digest", "payload_missing", "parse", "envelope", "version", "send",
                  "unknown_op", "peer_not_permitted", "not_projected"] {
            assert_ne!(super::requirement_for(r), "R-GW-0-unmapped", "rule {r} is not mapped to a requirement");
        }
        assert_eq!(super::requirement_for("creds_count"), "R-GW-0-unmapped", "the pre-WP3.1 typo must no longer resolve");
    }
}
