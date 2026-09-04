//! Child side of construction: the session init (PID 1 in the private PID
//! namespace). Runs steps 2, 4, 5, 6, 7 in order, reporting each over the
//! status pipe as `step N ok|fail <errno/detail>`; blocks on the barrier
//! pipe before step 9 (exec). After exec it is the workload's parent and
//! subreaper; it forwards SIGTERM and reaps until the workload exits.

use crate::sys::*;
use std::os::fd::RawFd;

pub struct ChildPlan {
    pub rootfs_fd: RawFd,               // open_tree clone of the runtime image (read-only, nosuid, nodev)
    pub mounts: Vec<(RawFd, String, bool)>, // (open_tree clone, target path inside root, read_only)
    pub uid: u32, pub gids: Vec<u32>,
    pub argv: Vec<String>, pub env: Vec<String>,
    pub status_w: RawFd, pub barrier_r: RawFd,
    pub keep_fds: Vec<RawFd>,           // stdin/stdout/stderr (0,1,2) per descriptor allowlist
    pub tmpfs_size: String, pub workspace_uid_chown: bool,
    pub nproc_limit: Option<u64>, pub nofile_limit: Option<u64>,
    pub stdio: (RawFd, RawFd),                // (stdin source, console sink) dup'd onto 0 and 1/2 so the harness pipe is never inherited
}

fn report(w: RawFd, step: u32, ok: bool, detail: &str) { write_all_fd(w, format!("step {step} {} {detail}\n", if ok { "ok" } else { "fail" }).as_bytes()); }
macro_rules! step { ($w:expr, $n:expr, $e:expr) => { match $e { Ok(v) => { report($w, $n, true, ""); v } Err(e) => { report($w, $n, false, &format!("errno={e}")); unsafe { libc::_exit(100 + $n as i32) } } } } }

/// Never returns.
pub fn run(p: ChildPlan) -> ! {
    let w = p.status_w;
    // no PDEATHSIG: the constructor is a transient parent that exits after activation; supervision is the lifecycle pidfd + scope
    unsafe { libc::dup2(p.stdio.0, 0); libc::dup2(p.stdio.1, 1); libc::dup2(p.stdio.1, 2); libc::close(p.stdio.0); libc::close(p.stdio.1); }
    // 2 — no propagation back to the host
    step!(w, 2, if unsafe { libc::mount(c("none").as_ptr(), c("/").as_ptr(), std::ptr::null(), libc::MS_REC | libc::MS_PRIVATE, std::ptr::null()) } == 0 { Ok(()) } else { Err(errno()) });
    // 4 — tmpfs root; image and intents attached by mount fd; pivot; detach old root
    step!(w, 4, (|| -> Result<(), i32> {
        let root = fsmount("tmpfs", &[("size", &p.tmpfs_size), ("mode", "0755")], &[], MOUNT_ATTR_NOSUID | MOUNT_ATTR_NODEV)?;
        let stage = "/tmp"; // staging directory exists on any Debian host; becomes invisible after pivot
        move_mount(root, libc::AT_FDCWD, stage)?; unsafe { libc::close(root) };
        let sd = unsafe { libc::open(c(stage).as_ptr(), libc::O_PATH | libc::O_DIRECTORY | libc::O_CLOEXEC) }; if sd < 0 { return Err(errno()); }
        for d in ["image", "workspace", "proc", "dev", "tmp", "oldroot", "etc"] { if unsafe { libc::mkdirat(sd, c(d).as_ptr(), 0o755) } != 0 && errno() != libc::EEXIST { return Err(errno()); } }
        move_mount(p.rootfs_fd, sd, "image")?;
        for (fd, target, _ro) in &p.mounts { let t = target.trim_start_matches('/'); let _ = unsafe { libc::mkdirat(sd, c(t).as_ptr(), 0o755) }; move_mount(*fd, sd, t)?; }
        // minimal /dev: devtmpfs is host-wide; use a tmpfs with the four unprivileged nodes bind-mounted from the host
        let dev = fsmount("tmpfs", &[("size", "64k"), ("mode", "0755")], &[], MOUNT_ATTR_NOSUID | MOUNT_ATTR_NOEXEC)?; move_mount(dev, sd, "dev")?; unsafe { libc::close(dev) };
        for n in ["null", "zero", "urandom", "random"] {
            let hfd = unsafe { libc::open(c(&format!("/dev/{n}")).as_ptr(), libc::O_PATH | libc::O_CLOEXEC) }; if hfd < 0 { return Err(errno()); }
            let tfd = unsafe { libc::openat(sd, c(&format!("dev/{n}")).as_ptr(), libc::O_CREAT | libc::O_WRONLY | libc::O_CLOEXEC, 0o666) }; if tfd < 0 { return Err(errno()); } unsafe { libc::close(tfd) };
            let t = open_tree_clone(hfd)?; unsafe { libc::close(hfd) }; move_mount(t, sd, &format!("dev/{n}"))?; unsafe { libc::close(t) };
        }
        let tmp = fsmount("tmpfs", &[("size", "64m"), ("mode", "1777")], &[], MOUNT_ATTR_NOSUID | MOUNT_ATTR_NODEV)?; move_mount(tmp, sd, "tmp")?; unsafe { libc::close(tmp) };
        // /bin,/lib,... resolve through the image: symlinks in the tmpfs root pointing into /image
        for (link, target) in [("bin", "image/bin"), ("usr", "image/usr"), ("lib", "image/lib"), ("lib64", "image/lib64"), ("sbin", "image/sbin")] { let _ = unsafe { libc::symlinkat(c(target).as_ptr(), sd, c(link).as_ptr()) }; }
        if unsafe { libc::chdir(c(stage).as_ptr()) } != 0 { return Err(errno()); }
        pivot_root(".", "oldroot")?;
        if unsafe { libc::chdir(c("/").as_ptr()) } != 0 { return Err(errno()); }
        if unsafe { libc::umount2(c("/oldroot").as_ptr(), libc::MNT_DETACH) } != 0 { return Err(errno()); }
        let _ = unsafe { libc::rmdir(c("/oldroot").as_ptr()) };
        unsafe { libc::close(sd) }; Ok(())
    })());
    // 5 — fresh procfs after pidns (nosuid,nodev,noexec); no sysfs at 1A (no netns content to expose)
    step!(w, 5, if unsafe { libc::mount(c("proc").as_ptr(), c("/proc").as_ptr(), c("proc").as_ptr(), libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC, std::ptr::null()) } == 0 { Ok(()) } else { Err(errno()) });
    // 6 — close everything not on the allowlist; verify through the fresh /proc before privilege drop
    step!(w, 6, (|| -> Result<(), i32> {
        let keep: Vec<RawFd> = p.keep_fds.iter().copied().chain([p.status_w, p.barrier_r]).collect();
        for (fd, _) in open_fds() { if !keep.contains(&fd) { unsafe { libc::close(fd) }; } }
        let now = open_fds(); let extra: Vec<String> = now.iter().filter(|(fd, _)| !keep.contains(fd)).map(|(fd, t)| format!("{fd}:{t}")).collect();
        if !extra.is_empty() { report(w, 6, false, &format!("leaked {}", extra.join(","))); unsafe { libc::_exit(106) } }
        // the status/barrier pipes are CLOEXEC and die at exec; record what survives
        write_all_fd(w, format!("fds {}\n", now.iter().map(|(fd, t)| format!("{fd}={t}")).collect::<Vec<_>>().join(" ")).as_bytes()); Ok(())
    })());
    // 7 — credentials, then irreversibility
    step!(w, 7, (|| -> Result<(), i32> {
        let e = |tag: &str| -> i32 { write_all_fd(w, format!("sub {tag}\n").as_bytes()); errno() };
        if let Some(n) = p.nproc_limit { let l = libc::rlimit { rlim_cur: n, rlim_max: n }; if unsafe { libc::setrlimit(libc::RLIMIT_NPROC, &l) } != 0 { return Err(e("rlimit_nproc")); } }
        if let Some(n) = p.nofile_limit { let l = libc::rlimit { rlim_cur: n, rlim_max: n }; if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &l) } != 0 { return Err(e("rlimit_nofile")); } }
        // bounding and ambient sets need CAP_SETPCAP: drop them while still root; the UID change then clears the rest
        drop_caps().map_err(|_| e("drop_caps"))?;
        let gids: Vec<libc::gid_t> = p.gids.iter().map(|g| *g as libc::gid_t).collect();
        if unsafe { libc::setgroups(gids.len(), gids.as_ptr()) } != 0 { return Err(e("setgroups")); }
        if unsafe { libc::setresgid(gids[0], gids[0], gids[0]) } != 0 { return Err(e("setresgid")); }
        if unsafe { libc::setresuid(p.uid, p.uid, p.uid) } != 0 { return Err(e("setresuid")); }
        let (mut ru, mut eu, mut su) = (0, 0, 0); unsafe { libc::getresuid(&mut ru, &mut eu, &mut su) }; if (ru, eu, su) != (p.uid, p.uid, p.uid) { return Err(e("verify_uid")); }
        let mut got = vec![0 as libc::gid_t; 64]; let n = unsafe { libc::getgroups(64, got.as_mut_ptr()) }; if n < 0 { return Err(e("getgroups")); } got.truncate(n as usize); got.sort(); let mut want = gids.clone(); want.sort(); if got != want { return Err(e("verify_groups")); }
        if unsafe { libc::setuid(0) } == 0 { return Err(e("setuid0_succeeded")); }
        if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 { return Err(e("no_new_privs")); }
        let st = std::fs::read_to_string("/proc/self/status").unwrap_or_default(); let z = |k: &str| st.lines().find(|l| l.starts_with(k)).map(|l| l[k.len()..].trim() == "0000000000000000").unwrap_or(false);
        if !(z("CapEff:") && z("CapPrm:") && z("CapBnd:") && z("CapAmb:") && z("CapInh:")) { return Err(e("verify_caps")); }
        if unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) } != 0 { return Err(e("subreaper")); }
        if unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) } != 0 { return Err(e("dumpable")); }
        seccomp_af_unix_only().map_err(|_| e("seccomp"))?;
        let s = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) }; if s >= 0 || errno() != libc::EPERM { return Err(e("seccomp_proof")); }
        Ok(())
    })());
    // wait for the parent's commit (step 8) — barrier release
    let mut b = [0u8; 1]; let n = unsafe { libc::read(p.barrier_r, b.as_mut_ptr() as *mut libc::c_void, 1) };
    if n != 1 || b[0] != b'g' { unsafe { libc::_exit(109) } }
    // 9 — become init: fork the workload, forward SIGTERM, reap
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        unsafe { libc::close(p.status_w); libc::close(p.barrier_r); }
        let argv: Vec<std::ffi::CString> = p.argv.iter().map(|a| c(a)).collect(); let mut av: Vec<*const libc::c_char> = argv.iter().map(|a| a.as_ptr()).collect(); av.push(std::ptr::null());
        let env: Vec<std::ffi::CString> = p.env.iter().map(|a| c(a)).collect(); let mut ev: Vec<*const libc::c_char> = env.iter().map(|a| a.as_ptr()).collect(); ev.push(std::ptr::null());
        let _ = unsafe { libc::chdir(c("/workspace").as_ptr()) };
        unsafe { libc::execve(av[0], av.as_ptr(), ev.as_ptr()) }; unsafe { libc::_exit(127) }
    }
    if pid < 0 { report(w, 9, false, &format!("fork errno={}", errno())); unsafe { libc::_exit(110) } }
    report(w, 9, true, &format!("workload_pid={pid}")); unsafe { libc::close(p.status_w); libc::close(p.barrier_r); }
    // init loop: SIGTERM → forward to workload; reap all; exit with the workload's status when it is gone
    static TERM: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    extern "C" fn on_term(_: libc::c_int) { TERM.store(true, std::sync::atomic::Ordering::SeqCst); }
    unsafe { let h = on_term as extern "C" fn(libc::c_int) as usize; libc::signal(libc::SIGTERM, h); libc::signal(libc::SIGINT, h); }
    let mut code = 0;
    loop {
        if TERM.swap(false, std::sync::atomic::Ordering::SeqCst) { unsafe { libc::kill(-1, libc::SIGTERM) }; }
        let mut st = 0; let r = unsafe { libc::wait(&mut st) };
        if r == pid { code = if libc::WIFEXITED(st) { libc::WEXITSTATUS(st) } else { 128 + libc::WTERMSIG(st) }; write_all_fd(2, format!("agentbound-init: workload exited status={code} raw={st}\n").as_bytes()); unsafe { libc::kill(-1, libc::SIGKILL) }; }
        if r < 0 && errno() == libc::ECHILD { break; }
        if r < 0 && errno() != libc::EINTR { break; }
    }
    unsafe { libc::_exit(code) }
}
