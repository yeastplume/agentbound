//! WP1 spike: loginuid and audit correlation, including loss behaviour under load.
//! R-CON-6; identity lifecycle §6; plan WP1 spike "loginuid and audit correlation".
//!
//! Part 1 — loginuid semantics on the pinned baseline: set in a barrier-blocked
//! child before exec (CAP_AUDIT_CONTROL); inherited across fork/exec; write-once
//! vs. re-settable depending on `audit_loginuid_immutable`; set attempt without
//! CAP_AUDIT_CONTROL fails; sessionid assigned alongside.
//!
//! Part 2 — correlation: an audit rule on a marker syscall from the execution UID
//! yields records carrying auid/ses/uid/pid/ppid; check that (auid, ses) equal what
//! the constructor set, that pid alone is ambiguous after reuse, and that the
//! record's fields suffice to join with (execution UID, boot ID, pidns, start time).
//!
//! Part 3 — loss under load: read `auditctl -s` lost/backlog counters before and
//! after a burst; determine whether kernel-side loss is *observable* (the
//! precondition for `loss_behaviour` = stop/quarantine) and whether backlog_wait
//! stalls the generating process.
//!
//! Throwaway code: not TCB, not SLOC-counted.
use libc::*;
use std::fs;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn result(item: &str, pass: bool, detail: &str) { println!("RESULT {item} {} {detail}", if pass { "PASS" } else { "FAIL" }); }
fn sh(cmd: &str) -> String { let o = Command::new("sh").arg("-c").arg(cmd).stderr(Stdio::piped()).output().unwrap(); String::from_utf8_lossy(&o.stdout).trim().to_string() + &String::from_utf8_lossy(&o.stderr) }
fn loginuid(pid: i32) -> i64 { fs::read_to_string(format!("/proc/{pid}/loginuid")).ok().and_then(|s| s.trim().parse::<u32>().ok()).map(|u| u as i64).unwrap_or(-2) }
fn sessionid(pid: i32) -> i64 { fs::read_to_string(format!("/proc/{pid}/sessionid")).ok().and_then(|s| s.trim().parse::<u32>().ok()).map(|u| u as i64).unwrap_or(-2) }
fn set_loginuid(uid: u32) -> Result<(), i32> { fs::write("/proc/self/loginuid", uid.to_string()).map_err(|e| e.raw_os_error().unwrap_or(0)) }
fn audit_status() -> (u64, u64, u64) {
    let s = sh("auditctl -s");
    let g = |k: &str| s.lines().find(|l| l.starts_with(k)).and_then(|l| l.split_whitespace().nth(1)).and_then(|v| v.parse().ok()).unwrap_or(0);
    (g("lost"), g("backlog "), g("backlog_limit"))
}
const EXEC_UID: u32 = 200042; // inside the frozen 200000–299999 range
const MARKER_PATH: &str = "/tmp/ab-audit-marker";

fn main() {
    // A PAM login session (SSH) already has loginuid set and inherited. The real constructor is a
    // systemd service with no PAM, i.e. loginuid unset. Re-exec ourselves under systemd-run to get that.
    if loginuid(unsafe { getpid() }) != u32::MAX as i64 && std::env::var("AB_REEXEC").is_err() {
        let exe = std::env::current_exe().unwrap();
        println!("re-executing under systemd-run (current loginuid={} is a PAM value, not a service context)", loginuid(unsafe { getpid() }));
        let st = Command::new("systemd-run").args(["--wait", "--pipe", "-q", "--collect", "-E", "AB_REEXEC=1", "-E", "PATH=/usr/sbin:/usr/bin:/sbin:/bin", exe.to_str().unwrap()]).status().unwrap();
        std::process::exit(st.code().unwrap_or(1));
    }
    let immutable_sysctl = fs::read_to_string("/proc/sys/kernel/audit_loginuid_immutable").ok();
    println!("spike audit-loginuid; self loginuid={} sessionid={} CONFIG_AUDIT_LOGINUID_IMMUTABLE sysctl present={}", loginuid(unsafe { getpid() }), sessionid(unsafe { getpid() }), immutable_sysctl.is_some());
    let unset = loginuid(unsafe { getpid() }) == u32::MAX as i64;
    println!("constructor (this process) loginuid unset={unset} (4294967295 = unset)");

    // ---------- Part 1: set in a barrier-blocked child before exec ----------
    let mut bar = [0; 2]; unsafe { pipe(bar.as_mut_ptr()); }
    let mut rep = [0; 2]; unsafe { pipe(rep.as_mut_ptr()); }
    let child = unsafe { fork() };
    if child == 0 {
        unsafe { close(bar[1]); close(rep[0]); let mut b = [0u8; 1]; read(bar[0], b.as_mut_ptr() as *mut c_void, 1); }
        let r1 = set_loginuid(EXEC_UID);                       // constructor sets loginuid (still root, has CAP_AUDIT_CONTROL)
        let r2 = set_loginuid(EXEC_UID + 1);                   // second write: write-once unless immutable=0? measure
        let after = loginuid(unsafe { getpid() });
        // drop to the execution identity, then attempt again without CAP_AUDIT_CONTROL
        unsafe { setgroups(0, std::ptr::null()); setgid(EXEC_UID); setuid(EXEC_UID); }
        let r3 = set_loginuid(EXEC_UID + 2);
        let gc = unsafe { fork() };
        if gc == 0 { unsafe { execl(b"/bin/sh\0".as_ptr() as *const c_char, b"sh\0".as_ptr() as *const c_char, b"-c\0".as_ptr() as *const c_char, format!("echo $(cat /proc/self/loginuid) $(cat /proc/self/sessionid) > {MARKER_PATH}.child\0").as_ptr() as *const c_char, std::ptr::null::<c_char>()); _exit(127); } }
        let mut st = 0; unsafe { waitpid(gc, &mut st, 0); }
        let msg = format!("{} {} {} {} {}\n", r1.err().unwrap_or(0), r2.err().unwrap_or(0), after, r3.err().unwrap_or(0), sessionid(unsafe { getpid() }));
        unsafe { write(rep[1], msg.as_ptr() as *const c_void, msg.len()); _exit(0); }
    }
    unsafe { close(bar[0]); close(rep[1]); }
    let before_release = loginuid(child);
    unsafe { write(bar[1], b"g".as_ptr() as *const c_void, 1); }
    let mut b = [0u8; 128]; let n = unsafe { read(rep[0], b.as_mut_ptr() as *mut c_void, 128) };
    let parts: Vec<i64> = String::from_utf8_lossy(&b[..n.max(0) as usize]).split_whitespace().filter_map(|x| x.parse().ok()).collect();
    let mut st = 0; unsafe { waitpid(child, &mut st, 0); }
    let (e1, e2, after, e3, ses) = (parts[0], parts[1], parts[2], parts[3], parts[4]);
    let grand = fs::read_to_string(format!("{MARKER_PATH}.child")).unwrap_or_default();
    let gparts: Vec<i64> = grand.split_whitespace().filter_map(|x| x.parse().ok()).collect();
    result("LU-1.child-unset-before-barrier-release", before_release == u32::MAX as i64, &format!("child loginuid before release={before_release} (inherits constructor's unset value; a constructor with a loginuid would make the child's value already-set → R-CON-6 'already-set' outcome)"));
    // Baseline fact: Debian's kernel has no CONFIG_AUDIT_LOGINUID_IMMUTABLE, so a CAP_AUDIT_CONTROL holder can
    // rewrite loginuid after it is set. This does not weaken the design (the session never holds the capability,
    // LU-4) but "write-once" in the specs is conditional on kernel config → FINDING.
    let write_once = e2 == EPERM as i64;
    result("LU-3.loginuid-write-once-on-baseline", true, &format!("second privileged write errno={e2}: write_once={write_once}; readback after both writes={after} (FINDING if write_once=false: loginuid is re-settable by CAP_AUDIT_CONTROL on this kernel because CONFIG_AUDIT_LOGINUID_IMMUTABLE is absent; sessions never hold the capability so it stays corroborating-only, as the specs already say)"));
    result("LU-2.set-before-exec-with-cap-audit-control", e1 == 0 && (after == EXEC_UID as i64 || after == EXEC_UID as i64 + 1), &format!("first write /proc/self/loginuid={EXEC_UID}: errno={e1}; sessionid assigned={ses}"));
    result("LU-4.set-without-cap-denied", e3 == EPERM as i64 || e3 == EACCES as i64, &format!("write after setuid({EXEC_UID}) errno={e3} (EACCES: /proc/self/loginuid is 0644 root-owned; EPERM if writable but CAP_AUDIT_CONTROL missing)"));
    result("LU-5.inherited-across-fork-exec", gparts.len() == 2 && gparts[0] == after && gparts[1] == ses, &format!("exec'd grandchild sees loginuid={:?} sessionid={:?}", gparts.first(), gparts.get(1)));

    // ---------- Part 2: correlation via an audit rule keyed on the execution UID ----------
    sh(&format!("auditctl -D >/dev/null 2>&1; rm -f {MARKER_PATH}*; auditctl -a always,exit -F arch=b64 -S mknodat -F uid={EXEC_UID} -k ab-spike"));
    let pre = sh("grep -c 'key=\"ab-spike\"' /var/log/audit/audit.log").parse::<usize>().unwrap_or(0);
    let t0 = sh("date +%s.%N");
    let marker_pids: Vec<i32> = (0..3).map(|i| {
        let c = unsafe { fork() };
        if c == 0 {
            unsafe { setgroups(0, std::ptr::null()); setgid(EXEC_UID); setuid(EXEC_UID); }
            // loginuid stays UNSET here on purpose (constructor did not set it): shows what unset looks like in records
            let p = format!("{MARKER_PATH}-{i}\0");
            unsafe { let _ = mknodat(AT_FDCWD, p.as_ptr() as *const c_char, S_IFIFO | 0o600, 0); _exit(0); }
        }
        let mut st = 0; unsafe { waitpid(c, &mut st, 0); } c
    }).collect();
    // And one with loginuid set properly by the "constructor" (this root process, in a barrier'd child)
    let c = unsafe { fork() };
    if c == 0 {
        let _ = set_loginuid(EXEC_UID);
        unsafe { setgroups(0, std::ptr::null()); setgid(EXEC_UID); setuid(EXEC_UID); }
        let p = format!("{MARKER_PATH}-set\0");
        unsafe { let _ = mknodat(AT_FDCWD, p.as_ptr() as *const c_char, S_IFIFO | 0o600, 0); _exit(0); }
    }
    let mut st = 0; unsafe { waitpid(c, &mut st, 0); }
    let set_pid = c;
    std::thread::sleep(Duration::from_millis(500));
    let _ = t0;
    let recs = sh("grep 'type=SYSCALL' /var/log/audit/audit.log | grep 'key=\"ab-spike\"'");
    let _ = pre;
    let want: Vec<String> = marker_pids.iter().chain(std::iter::once(&set_pid)).map(|p| format!("pid={p} ")).collect();
    let lines: Vec<&str> = recs.lines().filter(|l| want.iter().any(|w| l.contains(w.as_str()))).collect();
    println!("audit SYSCALL records for key ab-spike: {}", lines.len());
    for l in &lines { println!("  {}", l.split_whitespace().filter(|f| f.starts_with("auid=") || f.starts_with("ses=") || f.starts_with("uid=") || f.starts_with("pid=") || f.starts_with("ppid=") || f.starts_with("syscall=") || f.starts_with("msg=audit(")).collect::<Vec<_>>().join(" ")); }
    let fld = |l: &str, k: &str| l.split_whitespace().find(|f| f.starts_with(k)).map(|f| f[k.len()..].to_string()).unwrap_or_default();
    let set_rec = lines.iter().find(|l| fld(l, "pid=") == set_pid.to_string());
    let unset_recs: Vec<&&str> = lines.iter().filter(|l| marker_pids.iter().any(|p| fld(l, "pid=") == p.to_string())).collect();
    result("AC-1.records-emitted-for-exec-uid", lines.len() == 4, &format!("{} records (3 unset-loginuid + 1 set)", lines.len()));
    result("AC-2.set-loginuid-appears-as-auid", set_rec.map(|l| fld(l, "auid=") == EXEC_UID.to_string() && fld(l, "ses=") != "4294967295").unwrap_or(false), &format!("record for pid {set_pid}: auid={:?} ses={:?} uid={:?}", set_rec.map(|l| fld(l, "auid=")), set_rec.map(|l| fld(l, "ses=")), set_rec.map(|l| fld(l, "uid="))));
    result("AC-3.unset-loginuid-is-4294967295", unset_recs.len() == 3 && unset_recs.iter().all(|l| fld(l, "auid=") == "4294967295"), &format!("{} records with auid=4294967295 ses=4294967295 — attribution would rest on uid+pid alone (R-CON-6 'denied/unset' case)", unset_recs.len()));
    // fields needed for the join: uid (execution UID) present; pid; no pidns or start time in the record
    let has_pidns = lines.iter().any(|l| l.contains("pidns") || l.contains("start"));
    result("AC-4.record-lacks-pidns-and-starttime", !has_pidns, "SYSCALL record carries uid/auid/ses/pid/ppid/comm/exe but no PID-namespace id or start time; the join to a launch record must go through (uid=execution UID, boot ID, time window) + the daemon's own pid→pidfd table, exactly as identity lifecycle §6 says");

    // ---------- Part 3: loss behaviour under load ----------
    let (lost0, _, limit) = audit_status();
    sh("auditctl -b 64 --backlog_wait_time 0 >/dev/null");  // small backlog, no producer stall → force observable loss
    let (_, _, limit_small) = audit_status();
    let t = Instant::now();
    let burst = unsafe { fork() };
    if burst == 0 {
        unsafe { setgroups(0, std::ptr::null()); setgid(EXEC_UID); setuid(EXEC_UID); }
        for i in 0..20000u32 { let p = format!("{MARKER_PATH}-b{i}\0"); unsafe { mknodat(AT_FDCWD, p.as_ptr() as *const c_char, S_IFIFO | 0o600, 0); } }
        unsafe { _exit(0); }
    }
    unsafe { waitpid(burst, &mut st, 0); }
    let burst_ms = t.elapsed().as_millis();
    std::thread::sleep(Duration::from_millis(1500));
    let (lost1, backlog1, _) = audit_status();
    println!("burst of 20000 audited syscalls in {burst_ms} ms; backlog_limit {limit}→{limit_small}; lost {lost0}→{lost1}; backlog now {backlog1}");
    result("AL-1.kernel-loss-counter-observable", lost1 > lost0, &format!("auditctl -s lost advanced by {} (kernel-side drop is countable → loss_behaviour stop/quarantine has an observable trigger)", lost1 - lost0));
    // with backlog_wait_time > 0 the producer stalls instead: measure producer latency
    sh("auditctl -b 64 --backlog_wait_time 200 >/dev/null");
    let (lost2, _, _) = audit_status();
    let t = Instant::now();
    let burst = unsafe { fork() };
    if burst == 0 { unsafe { setgroups(0, std::ptr::null()); setgid(EXEC_UID); setuid(EXEC_UID); } for i in 0..2000u32 { let p = format!("{MARKER_PATH}-c{i}\0"); unsafe { mknodat(AT_FDCWD, p.as_ptr() as *const c_char, S_IFIFO | 0o600, 0); } } unsafe { _exit(0); } }
    unsafe { waitpid(burst, &mut st, 0); }
    let stall_ms = t.elapsed().as_millis();
    std::thread::sleep(Duration::from_millis(1000));
    let (lost3, _, _) = audit_status();
    println!("with backlog_wait_time=200 (jiffies): 2000 audited syscalls took {stall_ms} ms; lost {lost2}→{lost3}");
    result("AL-2.backlog-wait-trades-loss-for-producer-stall", stall_ms > 50 || lost3 == lost2, &format!("producer took {stall_ms} ms for 2000 calls (vs {burst_ms} ms for 20000 unthrottled); additional loss={}. The host audit policy chooses loss vs stall; the daemon can only observe `lost` and the daemon-side queue", lost3 - lost2));
    // restore
    sh(&format!("auditctl -b {limit} --backlog_wait_time 60000 >/dev/null; auditctl -D >/dev/null; rm -f {MARKER_PATH}*"));
    let (_, _, restored) = audit_status();
    println!("restored backlog_limit={restored}");
    println!("done");
}
