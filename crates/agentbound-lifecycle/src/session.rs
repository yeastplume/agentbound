//! Termination (session lifecycle §5 with WP1 F-3/F-4/F-5), quiesce (§6),
//! revocation dispatch, the reclamation condition (identity lifecycle §4.1),
//! the periodic liveness poll, and restart reconciliation (§7, component
//! interfaces §8). Every state change is appended to the store before it
//! becomes externally visible.

use crate::service::{gs, Reply, Service};
use ab_common::json::Value;
use ab_common::sig::monotonic_ns;
use ab_common::wire;
use std::os::fd::{AsRawFd, RawFd};
use std::time::{Duration, Instant};

/// Gateway control call (session-lifecycle §5 steps 1 and 6). Returns None when the gateway is unreachable; callers record that.
fn gateway_call(sock: &str, op: &str, lrd: &str, idem: &str) -> Option<Value> {
    let c = wire::connect(sock).ok()?;
    let r = c.call(&wire::request(op, idem, Value::obj(vec![("launch_record_digest", Value::s(lrd))]))).ok()?;
    if r.get("ok").and_then(|x| x.as_bool()) == Some(true) { r.get("body").cloned() } else { None }
}

pub const DEFAULT_TERM_BOUND_S: i64 = 10;
pub const SIGTERM_GRACE_MS: u64 = 2000;

// ---- cgroup helpers on the held directory descriptor ----
fn cg_read(dir: RawFd, name: &str) -> String {
    let c = std::ffi::CString::new(name).unwrap();
    let fd = unsafe { libc::openat(dir, c.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
    if fd < 0 { return String::new(); }
    let mut f = unsafe { <std::fs::File as std::os::fd::FromRawFd>::from_raw_fd(fd) };
    let mut s = String::new(); let _ = std::io::Read::read_to_string(&mut f, &mut s); s
}
fn cg_write(dir: RawFd, name: &str, v: &str) -> bool {
    let c = std::ffi::CString::new(name).unwrap();
    let fd = unsafe { libc::openat(dir, c.as_ptr(), libc::O_WRONLY | libc::O_CLOEXEC) };
    if fd < 0 { return false; }
    let n = unsafe { libc::write(fd, v.as_ptr() as *const libc::c_void, v.len()) }; unsafe { libc::close(fd) }; n == v.len() as isize
}
pub fn cg_procs(dir: RawFd) -> Vec<i32> { cg_read(dir, "cgroup.procs").lines().filter_map(|l| l.trim().parse().ok()).collect() }
fn cg_frozen(dir: RawFd) -> bool { cg_read(dir, "cgroup.events").lines().any(|l| l.trim() == "frozen 1") }
pub fn pidfd_exited(pidfd: RawFd) -> bool {
    let mut p = libc::pollfd { fd: pidfd, events: libc::POLLIN, revents: 0 };
    (unsafe { libc::poll(&mut p, 1, 0) }) > 0
}
fn proc_state(pid: i32) -> String { std::fs::read_to_string(format!("/proc/{pid}/stat")).ok().and_then(|s| s.rsplit(") ").next().and_then(|r| r.split(' ').next()).map(str::to_string)).unwrap_or_default() }
fn pids(v: &[i32]) -> Value { Value::Arr(v.iter().map(|p| Value::Int(*p as i64)).collect()) }

/// §4.1: scan host process credentials for the UID/GID; return (inside scope, outside scope).
pub fn credential_scan(uid: u32, gid: u32, scope_id: &str) -> (Vec<i32>, Vec<i32>) {
    let (mut inside, mut outside) = (Vec::new(), Vec::new());
    for e in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Ok(pid) = e.file_name().to_string_lossy().parse::<i32>() else { continue };
        let Ok(st) = std::fs::read_to_string(format!("/proc/{pid}/status")) else { continue };
        let has = |k: &str, id: u32| st.lines().find(|l| l.starts_with(k)).map(|l| l.split_whitespace().skip(1).any(|f| f == id.to_string())).unwrap_or(false);
        if has("Uid:", uid) || has("Gid:", gid) {
            let cg = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).unwrap_or_default();
            if !scope_id.is_empty() && cg.contains(scope_id) { inside.push(pid) } else { outside.push(pid) }
        }
    }
    (inside, outside)
}

impl Service {
    pub fn lifecycle_action(&mut self, op: &str, b: &Value, _uid: u32) -> Reply {
        let lrd = gs(b, "launch_record_digest")?.to_string();
        let s = self.sessions.get(&lrd).ok_or((wire::CLASS_INVALID, "unknown_record", String::new()))?;
        if matches!(s.state.as_str(), "terminated" | "cleaned/sealed" | "construction-failed" | "aborted") { return Err((wire::CLASS_CONFLICT, "terminal_state", s.state.clone())); }
        let reason = gs(b, "reason").unwrap_or("client_request").to_string();
        let bound = b.get("bound_s").and_then(|x| x.as_int()).unwrap_or(DEFAULT_TERM_BOUND_S).clamp(1, 300);
        match op {
            "terminate" => self.terminate(&lrd, &reason, bound),
            "quiesce" => self.quiesce(&lrd, &reason, bound),
            "revocation_signal" => {
                let trigger = gs(b, "trigger")?.to_string(); let source = gs(b, "source")?.to_string();
                let behaviour = self.manifest_behaviour(&lrd, &trigger)?;
                self.append_event(&lrd, "session.revocation_received", "ok", Value::obj(vec![("behaviour", Value::s(&behaviour)), ("source", Value::s(&source)), ("trigger", Value::s(&trigger))]));
                let r = match behaviour.as_str() { "terminate" => self.terminate(&lrd, &format!("revocation:{trigger}"), bound)?, "quiesce" => self.quiesce(&lrd, &format!("revocation:{trigger}"), bound)?, _ => self.degrade(&lrd, &trigger)? };
                Ok(Value::obj(vec![("behaviour", Value::s(&behaviour)), ("state", r.get("state").cloned().unwrap_or(Value::Null))]))
            }
            _ => unreachable!(),
        }
    }

    /// Manifest-declared behaviour for a trigger, read from the committed record (never caller-supplied).
    fn manifest_behaviour(&self, lrd: &str, trigger: &str) -> Result<String, (&'static str, &'static str, String)> {
        let recs = self.store.records(lrd).map_err(|e| (wire::CLASS_UNAVAILABLE, "store", e.to_string()))?;
        let binding = recs.iter().find(|(k, _)| k == "binding").ok_or((wire::CLASS_CONFLICT, "binding_not_committed", String::new()))?;
        binding.1.get("authorization_manifest").and_then(|m| m.get("revocation")).and_then(|r| r.get(trigger)).and_then(|x| x.as_str()).map(str::to_string).ok_or((wire::CLASS_INVALID, "unknown_trigger", trigger.to_string()))
    }

    pub fn append_event(&mut self, lrd: &str, kind: &str, outcome: &str, detail: Value) {
        let (aid, az) = match self.sessions.get(lrd) { Some(s) => (s.allocation_id.clone(), s.authorization_id.clone()), None => return };
        let mut payload = detail.clone(); payload.set("event", Value::s(kind));
        let _ = self.store.append_record("event", &aid, lrd, &az, &payload);
        let c = self.sessions.correlation(lrd);
        let ev = ab_common::audit::event(kind, "agentbound-lifecycle", outcome, &c, detail); self.audit.emit(&ev);
    }

    /// §6 quiesce: deny admission (no gateway at 1A), freeze the cgroup, arm the bound; terminate at expiry.
    pub fn quiesce(&mut self, lrd: &str, reason: &str, bound_s: i64) -> Reply {
        let s = self.sessions.get_mut(lrd).ok_or((wire::CLASS_INVALID, "unknown_record", String::new()))?;
        if s.state == "quiescing" { return Ok(Value::obj(vec![("state", Value::s("quiescing"))])); }
        let cg = s.cgroup_dir.as_ref().ok_or((wire::CLASS_CONFLICT, "session_not_registered", String::new()))?.as_raw_fd();
        let gw_deny = gateway_call(&self.cfg.gateway_sock, "deny_admission", lrd, &format!("{lrd}/deny-q/{}", monotonic_ns()));
        let frozen = cg_write(cg, "cgroup.freeze", "1");
        s.deadline_mono_ns = Some(monotonic_ns() + bound_s * 1_000_000_000);
        self.sessions.set_state(lrd, "quiescing", Some(reason));
        self.append_event(lrd, "session.quiesce_started", if frozen { "ok" } else { "freeze-failed" }, Value::obj(vec![("admission", Value::s(if gw_deny.is_some() { "denied" } else { "denied-no-gateway" })), ("bound_s", Value::Int(bound_s)), ("freeze_requested", Value::Bool(frozen)), ("trigger", Value::s(reason))]));
        if !frozen { return self.terminate(lrd, &format!("{reason}:freeze-failed"), bound_s); }
        Ok(Value::obj(vec![("state", Value::s("quiescing"))]))
    }

    fn degrade(&mut self, lrd: &str, trigger: &str) -> Reply {
        self.append_event(lrd, "session.degraded", "ok", Value::obj(vec![("compensating_control", Value::s("no fresh-policy operation admitted; no new authority")), ("remaining_authority", Value::s("existing manifest grants only (none at 1A)")), ("trigger", Value::s(trigger))]));
        if let Some(s) = self.sessions.get_mut(lrd) { s.reason = Some(format!("degraded:{trigger}")); s.observation_seq += 1; }
        Ok(Value::obj(vec![("overlay", Value::s("degraded")), ("state", Value::s("active"))]))
    }

    /// §5 steps 1–5, then 8/10/11 through `cleanup_and_seal`. Returns `terminated` only with recorded proof.
    pub fn terminate(&mut self, lrd: &str, reason: &str, bound_s: i64) -> Reply {
        let (cg, pidfd, init_pid, uid, gid, scope) = { let s = self.sessions.get(lrd).ok_or((wire::CLASS_INVALID, "unknown_record", String::new()))?;
            (s.cgroup_dir.as_ref().ok_or((wire::CLASS_CONFLICT, "session_not_registered", String::new()))?.as_raw_fd(), s.init_pidfd.as_ref().map(|f| f.as_raw_fd()).unwrap_or(-1), s.init_pid, s.uid, s.gid, s.scope_id.clone()) };
        self.sessions.set_state(lrd, "quiescing", Some(reason));
        self.append_event(lrd, "session.termination_started", "ok", Value::obj(vec![("bound_s", Value::Int(bound_s)), ("ordering_deviation", Value::Null), ("reason", Value::s(reason)), ("scope_id", Value::s(&scope))]));
        let t0 = Instant::now();
        // 1 deny admission at the gateway (mandatory on entry, distinct from releasing grant records — §5). 2 freeze.
        let gw_deny = gateway_call(&self.cfg.gateway_sock, "deny_admission", lrd, &format!("{lrd}/deny/{}", monotonic_ns()));
        let step2 = cg_write(cg, "cgroup.freeze", "1");
        // 3 thaw and SIGTERM init via pidfd, bounded (F-4: a PID-ns init without a handler ignores it)
        cg_write(cg, "cgroup.freeze", "0");
        let step3 = pidfd >= 0 && unsafe { libc::syscall(libc::SYS_pidfd_send_signal, pidfd, libc::SIGTERM, 0usize, 0u32) } == 0;
        let grace = Instant::now();
        while grace.elapsed() < Duration::from_millis(SIGTERM_GRACE_MS) && !cg_procs(cg).is_empty() { std::thread::sleep(Duration::from_millis(20)); }
        // 4 refreeze and cgroup.kill without waiting for `frozen 1` (F-3: never reached with a D-state member)
        cg_write(cg, "cgroup.freeze", "1");
        let step4 = cg_write(cg, "cgroup.kill", "1");
        // 5 bounded wait for emptiness and init exit; then host credential scan
        let deadline = t0 + Duration::from_secs(bound_s as u64);
        let mut procs = cg_procs(cg);
        while Instant::now() < deadline && (!procs.is_empty() || (pidfd >= 0 && !pidfd_exited(pidfd))) { std::thread::sleep(Duration::from_millis(25)); procs = cg_procs(cg); }
        let init_exited = pidfd < 0 || pidfd_exited(pidfd);
        let (inside, outside) = credential_scan(uid, gid, &scope);
        let dstate: Vec<i32> = procs.iter().copied().filter(|p| proc_state(*p) == "D").collect();
        let complete = procs.is_empty() && init_exited && inside.is_empty() && outside.is_empty();
        let evidence = Value::obj(vec![("cgroup_kill_written", Value::Bool(step4)), ("cgroup_procs_remaining", pids(&procs)), ("credential_scan_inside_scope", pids(&inside)), ("credential_scan_outside_scope", pids(&outside)), ("d_state", pids(&dstate)),
            ("elapsed_ms", Value::Int(t0.elapsed().as_millis() as i64)), ("freeze_written", Value::Bool(step2)), ("gateway_admission_denied", Value::Bool(gw_deny.is_some())), ("frozen_observed", Value::Bool(cg_frozen(cg))), ("init_pid", Value::Int(init_pid as i64)), ("init_pidfd_exited", Value::Bool(init_exited)), ("sigterm_sent", Value::Bool(step3))]);
        if !outside.is_empty() { self.append_event(lrd, "identity.scope_escape_suspected", "hold", Value::obj(vec![("pids", pids(&outside)), ("uid", Value::Int(uid as i64))])); }
        if !complete {
            self.sessions.set_state(lrd, "termination-incomplete", Some(reason));
            self.append_event(lrd, "session.termination_incomplete", "termination-incomplete", evidence.clone());
            return Ok(Value::obj(vec![("evidence", evidence), ("state", Value::s("termination-incomplete"))]));
        }
        self.sessions.set_state(lrd, "terminated", Some(reason));
        self.append_event(lrd, "session.terminated", "ok", evidence.clone());
        self.cleanup_and_seal(lrd); // 6–7 are empty at 1A (no grants/brokers); 8, 10, 11 follow
        Ok(Value::obj(vec![("evidence", evidence), ("state", Value::s(&self.sessions.get(lrd).map(|s| s.state.clone()).unwrap_or_default()))]))
    }

    /// §5 steps 8, 10, 11 and identity lifecycle §4.1: unmount, scan the managed domain for UID/GID residue,
    /// `reclaiming` → `quarantined` only when the condition holds, then seal. Uncertainty holds the identity.
    pub fn cleanup_and_seal(&mut self, lrd: &str) {
        let (aid, uid, gid, scope, session_dir, pidfd) = match self.sessions.get(lrd) { Some(s) => (s.allocation_id.clone(), s.uid, s.gid, s.scope_id.clone(), s.session_dir.clone(), s.init_pidfd.as_ref().map(|f| f.as_raw_fd()).unwrap_or(-1)), None => return };
        let mut unmounts = Vec::new();
        if let Some(dir) = &session_dir {
            for sub in ["rootfs", ""] {
                let p = if sub.is_empty() { dir.clone() } else { format!("{dir}/{sub}") };
                let c = std::ffi::CString::new(p).unwrap();
                let r = unsafe { libc::umount2(c.as_ptr(), libc::MNT_DETACH) }; let e = if r == 0 { 0 } else { std::io::Error::last_os_error().raw_os_error().unwrap_or(-1) };
                // ENOENT / EINVAL (not a mount point) are the expected 1A results: the session's mounts live in its own namespace and died with it
                unmounts.push(Value::obj(vec![("errno", Value::Int(e as i64)), ("path_class", Value::s(if sub.is_empty() { "session-dir" } else { sub })), ("result", Value::s(match e { 0 => "unmounted", libc::ENOENT | libc::EINVAL => "not-a-host-mount", _ => "failed" }))]));
            }
        }
        // §4 durable ownership projection: what the ephemeral identity created under a workspace root is chowned to the manifest's
        // storage principal (catalogue `storage_principals` → host user) before the identity is released, so nothing durable is
        // left owned by a UID that will be reused. Unmapped principal → hold (not removed, not released).
        let storage_ref = self.sessions.get(lrd).map(|s| s.storage_ref.clone()).unwrap_or_default();
        let sp = self.cfg.storage_principals.iter().find(|(r, _, _)| *r == storage_ref).map(|(_, u, g)| (*u, *g));
        let mut ws_files = Vec::new(); for w in &self.cfg.workspace_roots { scan_owned_deep(w, uid, gid, &mut ws_files); }
        let ws_files: Vec<String> = ws_files.into_iter().filter(|p| !self.cfg.workspace_roots.contains(p)).collect();
        let (mut projected, mut bytes, mut failed) = (0i64, 0i64, 0i64);
        if let Some((su, sg)) = sp { for f in &ws_files { match std::fs::symlink_metadata(f) { Ok(m) => { if std::os::unix::fs::lchown(f, Some(su), Some(sg)).is_ok() { projected += 1; if m.is_file() { bytes += m.len() as i64; } } else { failed += 1; } } Err(_) => {} } } }
        let projection_ok = ws_files.is_empty() || (sp.is_some() && failed == 0);
        self.append_event(lrd, "session.ownership_projected", if projection_ok { "ok" } else if sp.is_none() { "unmapped-principal" } else { "partial" }, Value::obj(vec![("bytes", Value::Int(bytes)), ("failed", Value::Int(failed)), ("files", Value::Int(projected)), ("storage_principal", Value::s(&storage_ref))]));
        let mut paths: Vec<String> = self.cfg.managed_paths.clone(); if let Some(d) = &session_dir { paths.push(d.clone()); }
        let mut residue = Vec::new();
        for p in &paths { scan_owned(p, uid, gid, &mut residue); }
        residue.sort(); residue.dedup();
        // remove deepest first so a directory is not counted after its contents are already gone
        residue.sort_by(|a, b| b.len().cmp(&a.len()));
        // workspace roots (registered mount sources) are durable projections: reset their group to the projection owner's
        // group instead of removing; everything else the identity owns is session residue and is removed
        let removed: Vec<Value> = residue.iter().map(|r| {
            let is_ws = self.cfg.workspace_roots.iter().any(|w| r == w);
            if !is_ws && std::fs::symlink_metadata(r).is_err() { return Value::obj(vec![("path_class", Value::s("already-gone")), ("removed", Value::Bool(true))]); }
            let ok = if is_ws { std::fs::metadata(r).ok().map(|m| { use std::os::unix::fs::MetadataExt; std::os::unix::fs::chown(r, None, Some(m.uid())).is_ok() }).unwrap_or(false) } else { std::fs::remove_file(r).or_else(|_| std::fs::remove_dir_all(r)).is_ok() };
            Value::obj(vec![("path_class", Value::s(if is_ws { "workspace-root-group-reset" } else if session_dir.as_deref().map(|d| r.starts_with(d)).unwrap_or(false) { "session-dir" } else { "registered-path" })), ("removed", Value::Bool(ok))]) }).collect();
        if let Some(d) = &session_dir { let _ = std::fs::remove_dir_all(d); }
        let (inside, outside) = credential_scan(uid, gid, &scope);
        // §5 step 6: release gateway grant records and indexed connections; the gateway MUST acknowledge zero connections
        // before identity release. A projection that was never made (topology none) releases as `released:false, remaining:0`.
        let gw = gateway_call(&self.cfg.gateway_sock, "release", lrd, &format!("{lrd}/release/{}", monotonic_ns()));
        let gw_remaining = gw.as_ref().and_then(|b| b.get("remaining")).and_then(|x| x.as_int());
        let gw_ok = gw_remaining == Some(0) || (gw.is_none() && self.sessions.get(lrd).map(|s| s.topology != "local-socket").unwrap_or(true));
        let grants = match &gw { Some(b) => Value::obj(vec![("connections_closed", b.get("connections_closed").cloned().unwrap_or(Value::Int(0))), ("released", b.get("released").cloned().unwrap_or(Value::Bool(false))), ("remaining", b.get("remaining").cloned().unwrap_or(Value::Null))]), None => Value::s("gateway unreachable") };
        let cond = inside.is_empty() && outside.is_empty() && (pidfd < 0 || pidfd_exited(pidfd)) && removed.iter().all(|r| r.get("removed").and_then(|x| x.as_str()).is_none() && r.get("removed").and_then(|x| x.as_bool()) == Some(true)) && gw_ok && projection_ok;
        self.append_event(lrd, "session.cleanup_completed", if cond { "ok" } else { "hold" }, Value::obj(vec![("acl_entries_removed", Value::Int(0)), ("grants", grants), ("ipc_namespace", Value::s("destroyed with last process")), ("residue", Value::Arr(removed)), ("unmounts", Value::Arr(unmounts))]));
        if let Ok(Some(a)) = self.store.latest(&aid) {
            let a = if a.state == "in-use" || a.state == "allocated" { self.store.transition(&aid, a.state_seq, "reclaiming", "termination complete", None, None, "agentbound-lifecycle").ok() } else { Some(a) };
            if let Some(a) = a { if a.state == "reclaiming" && cond {
                if let Ok(q) = self.store.transition(&aid, a.state_seq, "quarantined", &format!("reclamation condition met; lrd {lrd}"), None, None, "agentbound-lifecycle") {
                    self.append_event(lrd, "session.identity_released", "quarantined", Value::obj(vec![("allocation_id", Value::s(&aid)), ("quarantine_state_seq", Value::Int(q.state_seq)), ("reclamation_proof", Value::s(&q.hash))]));
                } } }
        }
        if !cond { if let Some(s) = self.sessions.get_mut(lrd) { s.reason = Some("cleanup-hold".into()); } return; }
        let (az, reason) = self.sessions.get(lrd).map(|s| (s.authorization_id.clone(), s.reason.clone().unwrap_or_default())).unwrap_or_default();
        if let Ok(seq) = self.store.append_record("seal", &aid, lrd, &az, &Value::obj(vec![("final_state", Value::s("cleaned/sealed")), ("termination_reason", Value::s(&reason))])) {
            self.sessions.set_state(lrd, "cleaned/sealed", None);
            let c = self.sessions.correlation(lrd);
            let ev = ab_common::audit::event("session.sealed", "agentbound-lifecycle", "ok", &c, Value::obj(vec![("seal_seq", Value::Int(seq)), ("termination_reason", Value::s(&reason))])); self.audit.emit(&ev);
        }
        if let Some(s) = self.sessions.get_mut(lrd) { s.init_pidfd = None; s.cgroup_dir = None; }
    }

    /// Periodic: init pidfd liveness (prompt trigger alongside D-Bus), quiesce deadlines, retries, reclamation.
    pub fn poll_sessions(&mut self) {
        let now = monotonic_ns();
        let due: Vec<(String, &'static str)> = self.sessions.all().into_iter().filter_map(|s| {
            if s.state == "quiescing" && s.deadline_mono_ns.map(|d| now >= d).unwrap_or(false) { return Some((s.lrd.clone(), "quiesce_bound_expired")); }
            if s.state == "active" && s.init_pidfd.as_ref().map(|f| pidfd_exited(f.as_raw_fd())).unwrap_or(false) { return Some((s.lrd.clone(), "init_exited")); }
            if s.state == "termination-incomplete" { return Some((s.lrd.clone(), "retry")); }
            None }).collect();
        for (lrd, why) in due {
            // a recovered session has no cgroup fd: retry means re-evaluating the containment evidence by path and sealing once it is clean
            let recovered = self.sessions.get(&lrd).map(|s| s.cgroup_dir.is_none() && s.state == "termination-incomplete").unwrap_or(false);
            if recovered { self.retry_recovered(&lrd); continue; }
            let _ = self.terminate(&lrd, why, DEFAULT_TERM_BOUND_S);
        }
        let pending = std::mem::take(&mut self.sessions.pending_reclaim);
        for (aid, lrd) in pending {
            if let Some(l) = lrd { self.cleanup_and_seal(&l); continue; }
            if let Ok(Some(a)) = self.store.latest(&aid) {
                let (i, o) = credential_scan(a.uid, a.gid, "");
                if i.is_empty() && o.is_empty() && a.state == "reclaiming" { let _ = self.store.transition(&aid, a.state_seq, "quarantined", "pre-binding rollback; scan clean", None, None, "agentbound-lifecycle"); } else { self.sessions.pending_reclaim.push((aid, None)); }
            }
        }
        if let Ok(v) = self.store.nonfree() { for a in v { if a.state == "quarantined" { let _ = self.store.transition(&a.allocation_id, a.state_seq, "free", "quarantine floor elapsed", None, None, "agentbound-lifecycle"); } } }
    }

    /// §7 / component interfaces §8.1: reconcile every unsealed record before serving. Live evidence without a
    /// held pidfd is an orphan: contain (`cgroup.kill` the scope), hold the identity, record.
    pub fn reconcile_on_start(&mut self) {
        for (aid, lrd, az) in self.store.bindings().unwrap_or_default() {
            let recs = self.store.records(&lrd).unwrap_or_default();
            if recs.iter().any(|(k, _)| k == "seal") { continue; }
            let Ok(Some(a)) = self.store.latest(&aid) else { continue };
            let g = |p: &Value, path: &[&str]| -> String { let mut v = Some(p); for k in path { v = v.and_then(|x| x.get(k)); } v.and_then(|x| x.as_str()).unwrap_or("").to_string() };
            let (scope, sid, tid, domain) = recs.iter().find(|(k, _)| k == "binding").map(|(_, p)| (g(p, &["launch_binding", "host_binding", "scope_id"]), g(p, &["authorization_manifest", "session_trace", "session_id"]), g(p, &["authorization_manifest", "session_trace", "trace_id"]), g(p, &["authorization_manifest", "termination_retention", "reclamation_domain_id"]))).unwrap_or_default();
            let topo = recs.iter().find(|(k, _)| k == "binding").map(|(_, p)| g(p, &["authorization_manifest", "gateway", "channel_topology"])).unwrap_or_default();
            let sref = recs.iter().find(|(k, _)| k == "binding").map(|(_, p)| g(p, &["authorization_manifest", "agent", "durable_ownership_projection", "reference"])).unwrap_or_default();
            self.sessions.bind(&aid, &lrd, &az, &scope, &sid, &tid, a.uid, a.gid, &domain, &topo, &sref);
            let (inside, outside) = credential_scan(a.uid, a.gid, &scope);
            let cgpath = format!("/sys/fs/cgroup/system.slice/{scope}");
            let live_cg = std::fs::read_to_string(format!("{cgpath}/cgroup.procs")).map(|s| !s.trim().is_empty()).unwrap_or(false);
            let action = if live_cg || !inside.is_empty() || !outside.is_empty() {
                let _ = std::fs::write(format!("{cgpath}/cgroup.kill"), "1");
                self.sessions.set_state(&lrd, "termination-incomplete", Some("recovery: live evidence without held pidfd")); "contained-and-held"
            } else { self.sessions.set_state(&lrd, "terminated", Some("recovery: no live evidence")); "cleanup-and-seal" };
            self.append_event(&lrd, "session.recovery_reconciled", action, Value::obj(vec![("cgroup_live", Value::Bool(live_cg)), ("credential_scan_inside", Value::Int(inside.len() as i64)), ("credential_scan_outside", Value::Int(outside.len() as i64)), ("identity_state", Value::s(&a.state)), ("scope_id", Value::s(&scope))]));
            if action == "cleanup-and-seal" { self.cleanup_and_seal(&lrd); }
        }
        if let Ok(v) = self.store.nonfree() { for a in v { if a.state == "allocated" && !self.store.record_exists("binding", "allocation_id", &a.allocation_id).unwrap_or(true) {
            let _ = self.store.transition(&a.allocation_id, a.state_seq, "reclaiming", "recovery: no binding committed", None, None, "agentbound-lifecycle");
            self.sessions.pending_reclaim.push((a.allocation_id.clone(), None)); } } }
    }
}

impl Service {
    /// Recovery retry (§5 restart case): kill by path again, re-scan; when the scope is gone and no credential-holding
    /// process remains, the session is terminated and cleanup/seal runs. Otherwise it stays termination-incomplete.
    fn retry_recovered(&mut self, lrd: &str) {
        let Some(s) = self.sessions.get(lrd) else { return }; let (uid, gid, scope) = (s.uid, s.gid, s.scope_id.clone());
        let cgpath = format!("/sys/fs/cgroup/system.slice/{scope}");
        let _ = std::fs::write(format!("{cgpath}/cgroup.kill"), "1");
        let live_cg = std::fs::read_to_string(format!("{cgpath}/cgroup.procs")).map(|s| !s.trim().is_empty()).unwrap_or(false);
        let (inside, outside) = credential_scan(uid, gid, &scope);
        if live_cg || !inside.is_empty() || !outside.is_empty() { return; }
        self.sessions.set_state(lrd, "terminated", Some("recovery retry: no live evidence"));
        self.append_event(lrd, "session.recovery_reconciled", "cleanup-and-seal", Value::obj(vec![("cgroup_live", Value::Bool(false)), ("credential_scan_inside", Value::Int(0)), ("credential_scan_outside", Value::Int(0)), ("identity_state", Value::s("in-use")), ("scope_id", Value::s(&scope))]));
        self.cleanup_and_seal(lrd);
    }
}

/// Like `scan_owned` but descends into owned directories too: ownership projection must reach every file, not just the top of each tree.
fn scan_owned_deep(root: &str, uid: u32, gid: u32, out: &mut Vec<String>) {
    use std::os::unix::fs::MetadataExt;
    let Ok(rd) = std::fs::read_dir(root) else { return };
    for e in rd.flatten() {
        let p = e.path(); let Ok(m) = std::fs::symlink_metadata(&p) else { continue };
        if m.uid() == uid || m.gid() == gid { out.push(p.to_string_lossy().into_owned()); }
        if m.is_dir() && !m.file_type().is_symlink() { scan_owned_deep(&p.to_string_lossy(), uid, gid, out); }
    }
}

fn scan_owned(root: &str, uid: u32, gid: u32, out: &mut Vec<String>) {
    use std::os::unix::fs::MetadataExt;
    let Ok(rd) = std::fs::read_dir(root) else { return };
    for e in rd.flatten() {
        let p = e.path(); let Ok(m) = std::fs::symlink_metadata(&p) else { continue };
        if m.uid() == uid || m.gid() == gid { out.push(p.to_string_lossy().into_owned()); continue; }
        if m.is_dir() && !m.file_type().is_symlink() { scan_owned(&p.to_string_lossy(), uid, gid, out); }
    }
}
