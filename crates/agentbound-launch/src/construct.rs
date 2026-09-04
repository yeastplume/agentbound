//! Parent side of construction (session lifecycle §3 steps 0–9) with the
//! rollback ledger. The constructor holds root for the duration of one
//! launch and exits after handover; it mutates no store (lifecycle does).

use crate::child::ChildPlan;
use crate::sys::*;
use ab_common::json::{self, Value, MANIFEST_LIMITS};
use ab_common::schema::{self, Manifest};
use ab_common::sig::{now_unix, object_digest, Keyring, Signer_};
use ab_common::{audit, envelope, wire};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

pub struct Config {
    pub spool: String, pub lease_dir: String, pub session_root: String, pub lifecycle_sock: String, pub keyring: Keyring, pub signer: Signer_,
    pub catalogue: Value, pub image_base: String, pub host_id: String, pub boot_id: String, pub self_digest: String, pub audit: audit::Sink, pub policy_uid: u32, pub fault: Option<String>,
}

#[derive(Default)]
pub struct Ledger { pub entries: Vec<Value>, pub allocation_id: Option<String>, pub state_seq: i64, pub scope: Option<String>, pub child_pidfd: Option<OwnedFd>, pub child_pid: i32, pub cgroup_dir: Option<OwnedFd>, pub lease: Option<String>, pub lrd: Option<String>, pub fds: Vec<RawFd>, pub session_dir: Option<String> }
impl Ledger { fn note(&mut self, step: u32, what: &str, detail: &str) { self.entries.push(Value::obj(vec![("detail", Value::s(detail)), ("step", Value::Int(step as i64)), ("what", Value::s(what))])); } }

pub struct Fail { pub step: u32, pub rule: &'static str, pub detail: String }
fn fail<T>(step: u32, rule: &'static str, d: impl Into<String>) -> Result<T, Fail> { Err(Fail { step, rule, detail: d.into() }) }
type R<T> = Result<T, Fail>;

fn call(sock: &str, op: &str, idem: &str, body: Value, fds: &[RawFd], step: u32) -> R<Value> {
    let c = wire::connect(sock).map_err(|e| Fail { step, rule: "lifecycle_unavailable", detail: e.to_string() })?;
    let req = wire::request(op, idem, body);
    if fds.is_empty() { c.send(&req) } else { c.send_with_fds(&req, fds) }.map_err(|e| Fail { step, rule: "lifecycle_unavailable", detail: e.to_string() })?;
    let r = c.recv().map_err(|e| Fail { step, rule: "lifecycle_unavailable", detail: e.to_string() })?.ok_or(Fail { step, rule: "lifecycle_unavailable", detail: "closed".into() })?;
    if r.get("ok").and_then(|x| x.as_bool()) == Some(true) { Ok(r.get("body").cloned().unwrap_or(Value::Null)) }
    else { fail(step, "lifecycle_rejected", format!("{}:{}:{}", r.get("class").and_then(|x| x.as_str()).unwrap_or(""), r.get("rule").and_then(|x| x.as_str()).unwrap_or(""), r.get("detail").and_then(|x| x.as_str()).unwrap_or(""))) }
}

/// D-Bus `StartTransientUnit` for a delegated scope around a placeholder, via busctl (libsystemd binding is a WP3 item).
/// `TimeoutStopUSec` is only settable here (WP1 finding); PIDs/Memory/CPU limits are installed as scope properties.
fn start_scope(name: &str, pids: Option<i64>, mem: Option<i64>, cpu_milli: Option<i64>) -> Result<(String, i32), String> {
    // a holder process keeps the scope alive until our init is cloned into it; it is killed after clone3
    let holder = unsafe { libc::fork() };
    if holder == 0 { unsafe { libc::setsid(); loop { libc::pause(); } } }
    if holder < 0 { return Err("fork holder".into()); }
    let mut props = vec![format!("Delegate b true"), format!("TimeoutStopUSec t 10000000"), format!("PIDs au 1 {holder}"), format!("CollectMode s inactive-or-failed")];
    if let Some(p) = pids { props.push(format!("TasksMax t {p}")); } if let Some(m) = mem { props.push(format!("MemoryMax t {m}")); } if let Some(c) = cpu_milli { props.push(format!("CPUQuotaPerSecUSec t {}", c * 1000)); }
    // busctl signature: ssa(sv)a(sa(sv))
    let mut args = vec!["call".into(), "org.freedesktop.systemd1".into(), "/org/freedesktop/systemd1".into(), "org.freedesktop.systemd1.Manager".into(), "StartTransientUnit".into(), "ssa(sv)a(sa(sv))".into(), format!("{name}.scope"), "fail".into(), props.len().to_string()];
    for p in &props { let mut it = p.splitn(3, ' '); args.push(it.next().unwrap().into()); args.push(it.next().unwrap().into()); for v in it.next().unwrap().split(' ') { args.push(v.into()); } }
    args.push("0".into());
    let out = std::process::Command::new("busctl").args(&args).output().map_err(|e| e.to_string())?;
    if !out.status.success() { unsafe { libc::kill(holder, libc::SIGKILL); libc::waitpid(holder, std::ptr::null_mut(), 0); } return Err(String::from_utf8_lossy(&out.stderr).trim().to_string()); }
    for _ in 0..200 { if let Ok(s) = std::fs::read_to_string(format!("/proc/{holder}/cgroup")) { if let Some(p) = s.trim().strip_prefix("0::/") { if p.ends_with(&format!("{name}.scope")) { return Ok((p.to_string(), holder)); } } } std::thread::sleep(std::time::Duration::from_millis(10)); }
    Err("holder never landed in scope".into())
}

pub fn construct(cfg: &mut Config, authorization_id: &str, led: &mut Ledger) -> R<Value> {
    let now = || now_unix().unwrap_or(0);
    let fault = |s: &str| cfg.fault.as_deref() == Some(s);
    // ---- step 0: verify the handoff, lease, reserve, scope ----
    let path = format!("{}/{authorization_id}.manifest.json", cfg.spool);
    let f = std::fs::File::open(&path).map_err(|e| Fail { step: 0, rule: "handoff_missing", detail: e.to_string() })?;
    { use std::os::unix::fs::MetadataExt; let m = f.metadata().map_err(|e| Fail { step: 0, rule: "handoff_missing", detail: e.to_string() })?;
      if m.uid() != cfg.policy_uid || m.mode() & 0o022 != 0 { return fail(0, "handoff_owner", format!("uid {} mode {:o}", m.uid(), m.mode() & 0o777)); } }
    let bytes = std::fs::read(&path).map_err(|e| Fail { step: 0, rule: "handoff_missing", detail: e.to_string() })?;
    let pair = json::parse_canonical(&bytes, &MANIFEST_LIMITS).map_err(|e| Fail { step: 0, rule: "manifest_noncanonical", detail: e.to_string() })?;
    let (mv, env) = (pair.get("authorization_manifest").ok_or(Fail { step: 0, rule: "handoff_shape", detail: String::new() })?.clone(), pair.get("envelope").ok_or(Fail { step: 0, rule: "handoff_shape", detail: String::new() })?.clone());
    let m: Manifest = schema::validate_manifest(&mv).map_err(|e| Fail { step: 0, rule: "manifest_schema", detail: e.to_string() })?;
    if m.authorization_id != authorization_id { return fail(0, "authorization_id_mismatch", ""); }
    let ver = envelope::verify_policy(&cfg.keyring, &mv, &env, authorization_id, now()).map_err(|e| Fail { step: 0, rule: "manifest_envelope", detail: e.to_string() })?;
    let mdigest = ver.digest.clone();
    if m.topology != "none" { return fail(0, "topology_unsupported_1a", m.topology); }
    let corr = audit::Correlation { authorization_id: Some(authorization_id.into()), session_id: Some(m.session_id.into()), trace_id: Some(m.trace_id.into()), ..Default::default() };
    cfg.audit.emit(&audit::event("session.manifest_verified", "agentbound-launch", "ok", &corr, Value::obj(vec![("key_id", Value::s(&ver.key_id)), ("manifest_digest", Value::s(&mdigest))])));
    // ownership lease: O_EXCL file named by authorization id (one constructor per authorization)
    let lease = format!("{}/{authorization_id}.lease", cfg.lease_dir);
    match std::fs::OpenOptions::new().create_new(true).write(true).open(&lease) { Ok(_) => led.lease = Some(lease.clone()), Err(_) => return fail(0, "lease_held", "another constructor owns this authorization") }
    led.note(0, "lease", &lease);
    // resolve catalogue objects the constructor needs (identifiers → host objects happens only here)
    let profile = cfg.catalogue.get("invocation_profiles").and_then(|p| p.get(m.invocation_profile)).cloned().ok_or(Fail { step: 0, rule: "profile_unresolvable", detail: m.invocation_profile.into() })?;
    let profile_digest = object_digest(&profile);
    let argv: Vec<String> = profile.get("argv").and_then(|a| a.as_arr()).map(|a| a.iter().filter_map(|x| x.as_str()).map(str::to_string).collect()).unwrap_or_default();
    let mut envv: Vec<String> = Vec::new();
    for e in profile.get("env").and_then(|a| a.as_arr()).map(|a| a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>()).unwrap_or_default() { if e.contains('=') { envv.push(e.into()); } else if let Ok(v) = std::env::var(e) { envv.push(format!("{e}={v}")); } }
    if argv.is_empty() { return fail(0, "profile_unresolvable", "empty argv"); }
    let lim = schema::limits(&m); let lv = |cls: &str| lim.iter().find(|l| l.class == cls && l.enforced).map(|l| l.limit);
    // reserve identity (lifecycle owns the allocator)
    let rb = Value::obj(vec![("agent_global_id", Value::s(m.agent_global_id)), ("authorization_id", Value::s(authorization_id)), ("authorization_manifest_digest", Value::s(&mdigest)), ("reclamation_domain_id", Value::s(m.reclamation_domain_id)), ("session_id", Value::s(m.session_id)), ("trace_id", Value::s(m.trace_id))]);
    let alloc = call(&cfg.lifecycle_sock, "reserve_identity", &format!("{authorization_id}/reserve"), rb, &[], 0)?;
    let (aid, uid) = (alloc.get("allocation_id").and_then(|x| x.as_str()).unwrap_or("").to_string(), alloc.get("uid").and_then(|x| x.as_int()).unwrap_or(0) as u32);
    let gids: Vec<u32> = alloc.get("gids").and_then(|x| x.as_arr()).map(|a| a.iter().filter_map(|x| x.as_int()).map(|g| g as u32).collect()).unwrap_or_default();
    led.allocation_id = Some(aid.clone()); led.state_seq = alloc.get("state_seq").and_then(|x| x.as_int()).unwrap_or(1); led.note(0, "reserve_identity", &format!("{aid} uid={uid}"));
    // transient delegated scope
    let scope_name = format!("agentbound-{}", aid.trim_start_matches("allocation:"));
    let (cgpath, holder) = start_scope(&scope_name, lv("pids"), lv("memory_bytes"), lv("cpu")).map_err(|e| Fail { step: 0, rule: "scope_start", detail: e })?;
    led.scope = Some(format!("{scope_name}.scope")); led.note(0, "scope", &cgpath);
    let cgfd = unsafe { libc::open(c(&format!("/sys/fs/cgroup/{cgpath}")).as_ptr(), libc::O_PATH | libc::O_DIRECTORY | libc::O_CLOEXEC) };
    if cgfd < 0 { return fail(0, "scope_cgroup_open", errno().to_string()); }
    led.cgroup_dir = Some(unsafe { OwnedFd::from_raw_fd(cgfd) });
    // ---- step 3 (parent, before clone so fds are inherited): confined resolution of image and intents ----
    let image_dir = unsafe { libc::open(c(&cfg.image_base).as_ptr(), libc::O_PATH | libc::O_DIRECTORY | libc::O_CLOEXEC) }; if image_dir < 0 { return fail(3, "image_base", errno().to_string()); }
    let rt = cfg.catalogue.get("runtimes").and_then(|r| r.get(m.runtime_catalogue_id)).cloned().ok_or(Fail { step: 3, rule: "runtime_unresolvable", detail: m.runtime_catalogue_id.into() })?;
    let img_rel = rt.get("image").and_then(|x| x.as_str()).unwrap_or("rootfs");
    let img = openat2(image_dir, img_rel, (libc::O_PATH | libc::O_DIRECTORY) as u64).map_err(|e| Fail { step: 3, rule: "image_resolve", detail: format!("errno={e}") })?;
    let rootfs = open_tree_clone(img).map_err(|e| Fail { step: 3, rule: "image_open_tree", detail: format!("errno={e}") })?; unsafe { libc::close(img); libc::close(image_dir) };
    mount_setattr(rootfs, MOUNT_ATTR_RDONLY | MOUNT_ATTR_NOSUID | MOUNT_ATTR_NODEV).map_err(|e| Fail { step: 3, rule: "image_setattr", detail: format!("errno={e}") })?;
    led.fds.push(rootfs);
    let mut mounts: Vec<(RawFd, String, bool)> = Vec::new(); let mut projections = Vec::new();
    for mi in &m.mount_intents {
        let src = cfg.catalogue.get("mount_sources").and_then(|s| s.get(mi.catalogue_id)).cloned().ok_or(Fail { step: 3, rule: "mount_source_unresolvable", detail: mi.catalogue_id.into() })?;
        let target = cfg.catalogue.get("mount_targets").and_then(|t| t.get(mi.target_template_id)).and_then(|x| x.as_str()).ok_or(Fail { step: 3, rule: "mount_target_unresolvable", detail: mi.target_template_id.into() })?.to_string();
        let (base, rel) = (src.get("base").and_then(|x| x.as_str()).unwrap_or(""), src.get("relative").and_then(|x| x.as_str()).unwrap_or(""));
        let bfd = unsafe { libc::open(c(base).as_ptr(), libc::O_PATH | libc::O_DIRECTORY | libc::O_CLOEXEC) }; if bfd < 0 { return fail(3, "mount_base", format!("{base} errno={}", errno())); }
        if fault("mount-symlink") { let _ = std::os::unix::fs::symlink("/etc", format!("{base}/{rel}-evil")); }
        let d = openat2(bfd, rel, (libc::O_PATH | libc::O_DIRECTORY) as u64).map_err(|e| Fail { step: 3, rule: if e == libc::EXDEV || e == libc::ELOOP { "mount_source_escape" } else { "mount_source_resolve" }, detail: format!("{rel} errno={e}") })?;
        unsafe { libc::close(bfd) };
        let t = open_tree_clone(d).map_err(|e| Fail { step: 3, rule: "mount_open_tree", detail: format!("errno={e}") })?; unsafe { libc::close(d) };
        let ro = mi.access == "read-only";
        mount_setattr(t, MOUNT_ATTR_NOSUID | MOUNT_ATTR_NODEV | if ro { MOUNT_ATTR_RDONLY } else { 0 }).map_err(|e| Fail { step: 3, rule: "mount_setattr", detail: format!("errno={e}") })?;
        // read-write workspaces are owned by the execution identity for the session (durable projection is a carry-in)
        if !ro { let p = format!("{base}/{rel}"); let _ = std::os::unix::fs::chown(&p, Some(uid), Some(gids[0])); }
        led.fds.push(t); mounts.push((t, target.clone(), ro));
        projections.push(Value::obj(vec![("access", Value::s(mi.access)), ("catalogue_version", Value::s(cfg.catalogue.get("catalogue_version").and_then(|x| x.as_str()).unwrap_or(""))), ("mount_id", Value::s(mi.mount_id)), ("target_template_projection", Value::s(mi.target_template_id))]));
    }
    led.note(3, "mounts", &format!("{} intents resolved", mounts.len()));
    let session_dir = format!("{}/{}", cfg.session_root, aid.trim_start_matches("allocation:")); std::fs::create_dir_all(&session_dir).map_err(|e| Fail { step: 3, rule: "session_dir", detail: e.to_string() })?; led.session_dir = Some(session_dir.clone());
    // ---- step 1: clone3 into the scope's cgroup with new namespaces; child blocks on the barrier ----
    let mut sp = [0i32; 2]; let mut bp = [0i32; 2];
    unsafe { libc::pipe2(sp.as_mut_ptr(), libc::O_CLOEXEC); libc::pipe2(bp.as_mut_ptr(), libc::O_CLOEXEC); }
    let mut pidfd: i32 = -1;
    let mut ca = CloneArgs { flags: (libc::CLONE_NEWNS | libc::CLONE_NEWPID | libc::CLONE_NEWIPC | libc::CLONE_NEWUTS | libc::CLONE_NEWNET) as u64 | CLONE_PIDFD | CLONE_INTO_CGROUP, pidfd: &mut pidfd as *mut i32 as u64, exit_signal: libc::SIGCHLD as u64, cgroup: cgfd as u64, ..Default::default() };
    let pid = clone3(&mut ca).map_err(|e| Fail { step: 1, rule: "clone3", detail: format!("errno={e}") })?;
    if pid == 0 {
        unsafe { libc::close(sp[0]); libc::close(bp[1]); }
        crate::child::run(ChildPlan { rootfs_fd: rootfs, mounts, uid, gids: gids.clone(), argv, env: envv, status_w: sp[1], barrier_r: bp[0], keep_fds: vec![0, 1, 2], tmpfs_size: "16m".into(), workspace_uid_chown: true,
            nproc_limit: lv("pids").map(|n| n as u64), nofile_limit: lv("file_descriptors").map(|n| n as u64) });
    }
    unsafe { libc::close(sp[1]); libc::close(bp[0]); libc::kill(holder, libc::SIGKILL); libc::waitpid(holder, std::ptr::null_mut(), 0); }
    led.child_pid = pid; led.child_pidfd = Some(unsafe { OwnedFd::from_raw_fd(pidfd) }); led.note(1, "clone3", &format!("pid={pid}"));
    let pidns = std::fs::read_link(format!("/proc/{pid}/ns/pid")).map(|p| p.to_string_lossy().trim_start_matches("pid:[").trim_end_matches(']').to_string()).unwrap_or_default();
    // ---- steps 2, 4, 5, 6, 7 reported by the child; the parent verifies each before proceeding ----
    let mut fdlist = String::new();
    for expect in [2u32, 4, 5, 6, 7] {
        loop {
            let line = read_line_fd(sp[0], 15_000).ok_or(Fail { step: expect, rule: "child_silent", detail: "no status within bound".into() })?;
            if let Some(rest) = line.strip_prefix("fds ") { fdlist = rest.to_string(); continue; }
            if let Some(rest) = line.strip_prefix("sub ") { led.note(expect, "sub", rest); continue; }
            let parts: Vec<&str> = line.splitn(4, ' ').collect();
            if parts.len() < 3 || parts[0] != "step" || parts[1].parse::<u32>().ok() != Some(expect) { return fail(expect, "child_protocol", line); }
            if parts[2] != "ok" { return fail(expect, "child_step_failed", parts.get(3).copied().unwrap_or("")); }
            led.note(expect, "child", "ok"); break;
        }
    }
    // step 7 external verification: the child's credentials from the host side
    let st = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap_or_default();
    let line = |k: &str| st.lines().find(|l| l.starts_with(k)).map(|l| l[k.len()..].trim().to_string()).unwrap_or_default();
    if line("Uid:") != format!("{uid}\t{uid}\t{uid}\t{uid}") || line("CapEff:") != "0000000000000000" || line("NoNewPrivs:") != "1" || line("Seccomp:") != "2" { return fail(7, "child_credentials_unverified", format!("uid={} cap={} nnp={} seccomp={}", line("Uid:"), line("CapEff:"), line("NoNewPrivs:"), line("Seccomp:"))); }
    // loginuid for kernel audit correlation (writable on this baseline per WP1)
    let _ = std::fs::write(format!("/proc/{pid}/loginuid"), uid.to_string());
    if fault("pre-commit-crash") { return fail(7, "fault_injected", "pre-commit-crash"); }
    // ---- step 8: assemble, sign, and commit the binding ----
    let rp = Value::obj(schema::RESOURCE_CLASSES.iter().map(|cls| { let l = lim.iter().find(|l| l.class == *cls).unwrap();
        (*cls, if l.enforced { Value::obj(vec![("enforcement_owner", Value::s(l.owner)), ("installed_value", Value::Int(l.limit)), ("unit", Value::s(l.unit))]) } else { Value::obj(vec![("enforcement_owner", Value::s("none")), ("status", Value::s("absent"))]) }) }).collect());
    let binding = Value::obj(vec![
        ("authorization_id", Value::s(authorization_id)), ("authorization_manifest_digest", Value::s(&mdigest)),
        ("constructor", Value::obj(vec![("agentbound_launch_version_digest", Value::s(&cfg.self_digest)), ("invocation_profile_digest", Value::s(&profile_digest)), ("key_id", Value::s(&cfg.signer.key_id))])),
        ("credential_grants", Value::Arr(vec![])),
        ("descriptor_allowlist", Value::Arr(["stdin", "stdout", "stderr"].iter().map(|k| Value::obj(vec![("descriptor_id", Value::s(&format!("fd:{k}"))), ("kind", Value::s(k)), ("purpose", Value::s("harness"))])).collect())),
        ("execution_identity", Value::obj(vec![("allocation_id", Value::s(&aid)), ("gids", Value::Arr(gids.iter().map(|g| Value::Int(*g as i64)).collect())), ("mac_context", Value::Null), ("uid", Value::Int(uid as i64))])),
        ("gateway_projection", Value::Null),
        ("host_binding", Value::obj(vec![("boot_id", Value::s(&cfg.boot_id)), ("host_id", Value::s(&cfg.host_id)), ("pid_namespace_id", Value::s(&format!("pidns:{pidns}"))), ("scope_id", Value::s(&format!("{scope_name}.scope")))])),
        ("launch_binding_version", Value::s(schema::BINDING_VERSION)), ("mount_projections", Value::Arr(projections)),
        ("namespaces", Value::obj(vec![("ipc", Value::s("private")), ("mount", Value::s("private")), ("pid", Value::s("private")), ("user", Value::s("inherited")), ("uts", Value::s("private"))])),
        ("resource_projection", rp),
    ]);
    let cenv = envelope::constructor_envelope(&cfg.signer, &binding, &mdigest, authorization_id, &aid, &cfg.host_id, &cfg.boot_id, now());
    let cb = Value::obj(vec![("allocation_id", Value::s(&aid)), ("authorization_manifest", mv.clone()), ("envelope", cenv), ("launch_binding", binding), ("manifest_envelope", env.clone())]);
    let committed = call(&cfg.lifecycle_sock, "commit_binding", &format!("{authorization_id}/commit"), cb, &[], 8)?;
    let lrd = committed.get("launch_record_digest").and_then(|x| x.as_str()).unwrap_or("").to_string(); led.lrd = Some(lrd.clone()); led.note(8, "commit_binding", &lrd);
    if fault("post-commit-crash") { return fail(8, "fault_injected", "post-commit-crash"); }
    // ---- step 9: hand the live evidence to lifecycle, release the barrier, report activation ----
    let ds = Value::Arr(vec![Value::obj(vec![("index", Value::Int(0)), ("kind", Value::s("init_pidfd"))]), Value::obj(vec![("index", Value::Int(1)), ("kind", Value::s("cgroup_dir"))])]);
    let reg = Value::obj(vec![("allocation_id", Value::s(&aid)), ("descriptors", ds), ("init_pid", Value::Int(pid as i64)), ("launch_record_digest", Value::s(&lrd)), ("pid_namespace_id", Value::s(&format!("pidns:{pidns}"))), ("scope_id", Value::s(&format!("{scope_name}.scope"))), ("session_dir", Value::s(&session_dir))]);
    call(&cfg.lifecycle_sock, "register_session", &format!("{authorization_id}/register"), reg, &[pidfd, cgfd], 9)?;
    if !write_all_fd(bp[1], b"g") { return fail(9, "barrier_release", errno().to_string()); }
    unsafe { libc::close(bp[1]) };
    let line = read_line_fd(sp[0], 15_000).ok_or(Fail { step: 9, rule: "child_silent", detail: "no exec report".into() })?;
    if !line.starts_with("step 9 ok") { return fail(9, "child_step_failed", line); }
    let act = Value::obj(vec![("allocation_id", Value::s(&aid)), ("launch_record_digest", Value::s(&lrd)),
        ("privilege_disposal", Value::obj(vec![("capabilities", Value::s("bounding+ambient+permitted cleared")), ("fds_at_exec", Value::s(&fdlist)), ("no_new_privs", Value::Bool(true)), ("seccomp", Value::s("af_unix_only")), ("uid_verified_host_side", Value::Bool(true))])), ("runtime_artifact_digest", Value::s(m.runtime_artifact_digest))]);
    call(&cfg.lifecycle_sock, "report_activation", &format!("{authorization_id}/activate"), act, &[], 9)?;
    led.note(9, "activated", &line);
    let _ = std::fs::remove_file(&path); let _ = std::fs::remove_file(&lease); led.lease = None;
    Ok(Value::obj(vec![("allocation_id", Value::s(&aid)), ("init_pid", Value::Int(pid as i64)), ("launch_record_digest", Value::s(&lrd)), ("scope_id", Value::s(&format!("{scope_name}.scope"))), ("uid", Value::Int(uid as i64))]))
}

/// Reverse rollback (§3): kill and reap the child, stop the scope, release the allocation via lifecycle, drop the lease.
pub fn rollback(cfg: &mut Config, authorization_id: &str, led: &mut Ledger, f: &Fail) -> Value {
    let mut steps = Vec::new();
    if let Some(pf) = &led.child_pidfd { pidfd_send_signal(pf.as_raw_fd(), libc::SIGKILL); let mut st = 0; unsafe { libc::waitpid(led.child_pid, &mut st, 0) }; steps.push(Value::s("child killed and reaped")); }
    if let Some(cg) = &led.cgroup_dir { let _ = std::fs::write(format!("/proc/self/fd/{}/cgroup.kill", cg.as_raw_fd()), "1"); steps.push(Value::s("cgroup.kill")); }
    for fd in led.fds.drain(..) { unsafe { libc::close(fd) }; }
    if let Some(s) = &led.scope { let _ = std::process::Command::new("busctl").args(["call", "org.freedesktop.systemd1", "/org/freedesktop/systemd1", "org.freedesktop.systemd1.Manager", "StopUnit", "ss", s, "replace"]).output(); steps.push(Value::s("scope stopped")); }
    if let Some(d) = &led.session_dir { let _ = std::fs::remove_dir_all(d); }
    if let Some(aid) = &led.allocation_id {
        let body = Value::obj(vec![("allocation_id", Value::s(aid)), ("failed_step", Value::Int(f.step as i64)), ("launch_record_digest", led.lrd.as_deref().map(Value::s).unwrap_or(Value::Null)), ("ledger", Value::Arr(led.entries.clone())), ("rule", Value::s(f.rule))]);
        match call(&cfg.lifecycle_sock, "report_construction_failed", &format!("{authorization_id}/failed/{}", f.step), body, &[], f.step) { Ok(_) => steps.push(Value::s("identity → reclaiming")), Err(e) => steps.push(Value::s(&format!("report failed: {}", e.detail))) }
    }
    if let Some(l) = &led.lease { let _ = std::fs::remove_file(l); }
    let c = audit::Correlation { authorization_id: Some(authorization_id.into()), allocation_id: led.allocation_id.clone(), launch_record_digest: led.lrd.clone(), ..Default::default() };
    let detail = Value::obj(vec![("detail", Value::s(&f.detail)), ("failed_step", Value::Int(f.step as i64)), ("ledger", Value::Arr(led.entries.clone())), ("rollback", Value::Arr(steps)), ("rule", Value::s(f.rule))]);
    cfg.audit.emit(&audit::event("session.construction_failed", "agentbound-launch", "construction-failed", &c, detail.clone()));
    detail
}
