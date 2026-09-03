//! WP1 spike: namespace, mount, and procfs construction in the §2.1 / lifecycle §3
//! ordering with descriptor-relative mount-source resolution; descriptor closure
//! and runtime launch ordering. R-REQ-6, R-CON-2, R-CON-3, R-CON-4;
//! fault points F-C-02..06; catalogue T-6.1-009 (mechanism projection).
//!
//! Sequence exercised (parent = constructor, child = session init):
//!   1. clone3(NEWNS|NEWPID|NEWUTS|NEWIPC|NEWNET|PIDFD) + barrier
//!   2. child: make / rprivate
//!   3. parent: resolve mount sources with openat2(RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS|
//!      RESOLVE_NO_MAGICLINKS) relative to a trusted base; open_tree(OPEN_TREE_CLONE)
//!      to detached mount FDs; a symlink race attempt fails
//!   4. child: build tmpfs root, move_mount() the detached trees in by FD, pivot_root
//!   5. child: mount proc AFTER pid ns exists; host /proc invisible; nosuid,nodev
//!      everywhere; no writable cgroupfs; no systemd socket
//!   6. child: close_range everything outside the allowlist; verify /proc/self/fd,
//!      memfd, and SCM_RIGHTS re-introduction are neutralised
//!   then exec.
//!
//! Throwaway code: not TCB, not SLOC-counted.
use libc::*;
use std::ffi::CString;
use std::fs;
use std::os::unix::io::RawFd;
use std::ptr;

fn result(item: &str, pass: bool, detail: &str) { println!("RESULT {item} {} {detail}", if pass { "PASS" } else { "FAIL" }); }
fn errno() -> i32 { std::io::Error::last_os_error().raw_os_error().unwrap_or(0) }
fn c(s: &str) -> CString { CString::new(s).unwrap() }

// ---- syscalls not in libc (or fragile there) ----
const SYS_OPENAT2: c_long = 437; const SYS_OPEN_TREE: c_long = 428; const SYS_MOVE_MOUNT: c_long = 429; const SYS_CLOSE_RANGE: c_long = 436; const SYS_FSOPEN: c_long = 430; const SYS_FSCONFIG: c_long = 431; const SYS_FSMOUNT: c_long = 432;
const RESOLVE_NO_MAGICLINKS: u64 = 0x02; const RESOLVE_NO_SYMLINKS: u64 = 0x04; const RESOLVE_BENEATH: u64 = 0x08;
const OPEN_TREE_CLONE: c_uint = 1; const OPEN_TREE_CLOEXEC: c_uint = O_CLOEXEC as c_uint; const AT_RECURSIVE: c_int = 0x8000;
const MOVE_MOUNT_F_EMPTY_PATH: c_uint = 0x4;
const FSCONFIG_SET_STRING: c_uint = 1; const FSCONFIG_CMD_CREATE: c_uint = 6; const FSMOUNT_CLOEXEC: c_uint = 1;
const MOUNT_ATTR_RDONLY: u64 = 0x1; const MOUNT_ATTR_NOSUID: u64 = 0x2; const MOUNT_ATTR_NODEV: u64 = 0x4; const MOUNT_ATTR_NOEXEC: u64 = 0x8;
const CLOSE_RANGE_CLOEXEC: c_uint = 4;
#[repr(C)] #[derive(Default)] struct OpenHow { flags: u64, mode: u64, resolve: u64 }
fn openat2(dir: RawFd, path: &str, flags: u64, resolve: u64) -> Result<RawFd, i32> {
    let how = OpenHow { flags, mode: 0, resolve };
    let r = unsafe { syscall(SYS_OPENAT2, dir, c(path).as_ptr(), &how as *const OpenHow, std::mem::size_of::<OpenHow>()) };
    if r < 0 { Err(errno()) } else { Ok(r as RawFd) }
}
fn open_tree(dir: RawFd, path: &str, flags: c_uint) -> Result<RawFd, i32> { let r = unsafe { syscall(SYS_OPEN_TREE, dir, c(path).as_ptr(), flags) }; if r < 0 { Err(errno()) } else { Ok(r as RawFd) } }
fn move_mount_fd(from: RawFd, to_dir: RawFd, to_path: &str) -> Result<(), i32> { let r = unsafe { syscall(SYS_MOVE_MOUNT, from, c("").as_ptr(), to_dir, c(to_path).as_ptr(), MOVE_MOUNT_F_EMPTY_PATH) }; if r < 0 { Err(errno()) } else { Ok(()) } }
fn fsmount_tmpfs(size: &str) -> Result<RawFd, i32> {
    unsafe {
        let fsfd = syscall(SYS_FSOPEN, c("tmpfs").as_ptr(), 1u32) as RawFd; if fsfd < 0 { return Err(errno()); }
        syscall(SYS_FSCONFIG, fsfd, FSCONFIG_SET_STRING, c("size").as_ptr(), c(size).as_ptr(), 0);
        syscall(SYS_FSCONFIG, fsfd, FSCONFIG_SET_STRING, c("mode").as_ptr(), c("0755").as_ptr(), 0);
        if syscall(SYS_FSCONFIG, fsfd, FSCONFIG_CMD_CREATE, 0usize, 0usize, 0) < 0 { return Err(errno()); }
        let m = syscall(SYS_FSMOUNT, fsfd, FSMOUNT_CLOEXEC, MOUNT_ATTR_NOSUID | MOUNT_ATTR_NODEV) as RawFd; close(fsfd);
        if m < 0 { Err(errno()) } else { Ok(m) }
    }
}
#[repr(C)] struct MountAttr { attr_set: u64, attr_clr: u64, propagation: u64, userns_fd: u64 }
fn mount_setattr(fd: RawFd, set: u64) -> Result<(), i32> { let a = MountAttr { attr_set: set, attr_clr: 0, propagation: 0, userns_fd: 0 }; let r = unsafe { syscall(442, fd, c("").as_ptr(), AT_EMPTY_PATH | AT_RECURSIVE, &a as *const MountAttr, std::mem::size_of::<MountAttr>()) }; if r < 0 { Err(errno()) } else { Ok(()) } }
#[repr(C)] #[derive(Default)] struct CloneArgs { flags: u64, pidfd: u64, child_tid: u64, parent_tid: u64, exit_signal: u64, stack: u64, stack_size: u64, tls: u64, set_tid: u64, set_tid_size: u64, cgroup: u64 }
fn mounts_of(pid: &str) -> String { fs::read_to_string(format!("/proc/{pid}/mountinfo")).unwrap_or_default() }
fn open_fds() -> Vec<i32> { fs::read_dir("/proc/self/fd").map(|d| d.filter_map(|e| { let e = e.ok()?; let fd: i32 = e.file_name().to_str()?.parse().ok()?; let tgt = fs::read_link(e.path()).ok()?; if tgt.to_str()?.ends_with("/fd") { None } else { Some(fd) } }).collect()).unwrap_or_default() }

fn main() {
    println!("spike mount-construct");
    // ---------- trusted base with a workspace, an attacker-controlled symlink, and a "secret" outside ----------
    let base = "/tmp/ab-mc"; let _ = fs::remove_dir_all(base);
    fs::create_dir_all(format!("{base}/workspaces/ws1/src")).unwrap();
    fs::write(format!("{base}/workspaces/ws1/src/hello.txt"), "workspace-file").unwrap();
    fs::create_dir_all(format!("{base}/secret")).unwrap(); fs::write(format!("{base}/secret/key"), "SECRET").unwrap();
    fs::create_dir_all(format!("{base}/runtime-image/bin")).unwrap(); fs::copy("/bin/busybox", format!("{base}/runtime-image/bin/busybox")).expect("busybox-static required");
    // attacker (session-controlled workspace content) plants symlinks that a string-path re-walk would follow
    std::os::unix::fs::symlink("../../secret", format!("{base}/workspaces/ws1/escape")).unwrap();
    std::os::unix::fs::symlink("/etc", format!("{base}/workspaces/ws1/abs")).unwrap();
    std::os::unix::fs::symlink("/proc/self/root/etc", format!("{base}/workspaces/ws1/magic")).unwrap();
    let basefd = unsafe { open(c(&format!("{base}/workspaces")).as_ptr(), O_PATH | O_DIRECTORY | O_CLOEXEC) };

    // ---------- step 3: descriptor-relative, symlink-safe resolution ----------
    let ok = openat2(basefd, "ws1/src", (O_PATH | O_DIRECTORY) as u64, RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS);
    let esc = openat2(basefd, "ws1/escape/key", O_RDONLY as u64, RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS);
    let abs = openat2(basefd, "ws1/abs/passwd", O_RDONLY as u64, RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS);
    let mag = openat2(basefd, "ws1/magic/passwd", O_RDONLY as u64, RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS);
    let dotdot = openat2(basefd, "ws1/../../secret/key", O_RDONLY as u64, RESOLVE_BENEATH, );
    result("R6-1.openat2-resolves-legitimate-path", ok.is_ok(), &format!("openat2(base, ws1/src) → fd {:?}", ok));
    result("R6-2.openat2-rejects-relative-symlink-escape", esc == Err(ELOOP), &format!("ws1/escape→../../secret: {:?} (ELOOP={ELOOP})", esc));
    result("R6-3.openat2-rejects-absolute-symlink", abs == Err(ELOOP), &format!("ws1/abs→/etc: {:?}", abs));
    result("R6-4.openat2-rejects-magic-link", mag == Err(ELOOP), &format!("ws1/magic→/proc/self/root/etc: {:?}", mag));
    result("R6-5.openat2-beneath-rejects-dotdot-escape", dotdot == Err(EXDEV), &format!("ws1/../../secret/key with RESOLVE_BENEATH: {:?} (EXDEV={EXDEV})", dotdot));
    // TOCTOU: resolve, then attacker swaps the directory for a symlink; the held FD is unaffected, a string re-walk would not be
    let wsfd = ok.unwrap();
    fs::rename(format!("{base}/workspaces/ws1/src"), format!("{base}/workspaces/ws1/src.real")).unwrap();
    std::os::unix::fs::symlink(&format!("{base}/secret"), format!("{base}/workspaces/ws1/src")).unwrap();
    let via_fd = unsafe { openat(wsfd, c("hello.txt").as_ptr(), O_RDONLY) }; let via_fd_ok = via_fd >= 0; unsafe { close(via_fd); }
    let via_path = fs::read_to_string(format!("{base}/workspaces/ws1/src/key")).unwrap_or_default();
    result("R6-6.held-fd-immune-to-swap-race", via_fd_ok && via_path == "SECRET", &format!("after swapping src→symlink(secret): openat(held_fd, hello.txt) ok={via_fd_ok}; a string re-walk of the same path now yields {via_path:?} — the race a re-walk would lose"));
    // detached mount tree from the held FD (open_tree CLONE), made read-only+nosuid+nodev via mount_setattr
    let tree = open_tree(wsfd, "", OPEN_TREE_CLONE | OPEN_TREE_CLOEXEC | AT_EMPTY_PATH as c_uint).expect("open_tree");
    mount_setattr(tree, MOUNT_ATTR_NOSUID | MOUNT_ATTR_NODEV).unwrap();
    let img_fd = openat2(unsafe { open(c(base).as_ptr(), O_PATH | O_CLOEXEC) }, "runtime-image", (O_PATH | O_DIRECTORY) as u64, RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS).unwrap();
    let img_tree = open_tree(img_fd, "", OPEN_TREE_CLONE | OPEN_TREE_CLOEXEC | AT_EMPTY_PATH as c_uint).unwrap();
    mount_setattr(img_tree, MOUNT_ATTR_RDONLY | MOUNT_ATTR_NOSUID | MOUNT_ATTR_NODEV).unwrap();
    result("R6-7.detached-mount-fds-from-held-fds", tree >= 0 && img_tree >= 0, &format!("open_tree(OPEN_TREE_CLONE) → workspace tree fd {tree}, image tree fd {img_tree} (ro,nosuid,nodev via mount_setattr); no string path used after resolution"));

    // leak candidates the constructor must not pass: a secret fd and a memfd
    let leak_fd = unsafe { open(c(&format!("{base}/secret/key")).as_ptr(), O_RDONLY) }; // deliberately NOT cloexec
    let memfd = unsafe { memfd_create(c("ab-leak").as_ptr(), 0) }; unsafe { write(memfd, b"M".as_ptr() as *const c_void, 1); }
    let mut sv = [0; 2]; unsafe { socketpair(AF_UNIX, SOCK_SEQPACKET, 0, sv.as_mut_ptr()); } // sv[1] would carry SCM_RIGHTS re-introduction

    // ---------- step 1: clone3 + barrier ----------
    let mut bar = [0; 2]; unsafe { pipe(bar.as_mut_ptr()); }
    let mut rep = [0; 2]; unsafe { pipe2(rep.as_mut_ptr(), O_CLOEXEC); }
    let mut pidfd: i32 = -1;
    let mut args = CloneArgs { flags: (CLONE_NEWNS | CLONE_NEWPID | CLONE_NEWUTS | CLONE_NEWIPC | CLONE_NEWNET | CLONE_PIDFD) as u64, pidfd: &mut pidfd as *mut i32 as u64, exit_signal: SIGCHLD as u64, ..Default::default() };
    let host_mounts_before = mounts_of("self").lines().count();
    let pid = unsafe { syscall(SYS_clone3, &mut args as *mut CloneArgs, std::mem::size_of::<CloneArgs>()) } as i32;
    if pid == 0 {
        unsafe {
            let mut b = [0u8; 1]; read(bar[0], b.as_mut_ptr() as *mut c_void, 1); close(bar[0]); close(bar[1]);
            let say = |s: String| { let s = s + "\n"; write(rep[1], s.as_ptr() as *const c_void, s.len()); };
            // step 2
            let r = mount(ptr::null(), c("/").as_ptr(), ptr::null(), MS_REC | MS_PRIVATE, ptr::null()); say(format!("rprivate {r}"));
            // step 4: tmpfs root by fd, move detached trees in, pivot_root
            let rootfs = fsmount_tmpfs("8m").unwrap();
            let r = move_mount_fd(rootfs, AT_FDCWD, "/mnt"); say(format!("root-tmpfs-moved {:?}", r));
            for d in ["/mnt/workspace", "/mnt/image", "/mnt/proc", "/mnt/oldroot", "/mnt/sys"] { fs::create_dir_all(d).unwrap(); }
            let r1 = move_mount_fd(tree, AT_FDCWD, "/mnt/workspace"); let r2 = move_mount_fd(img_tree, AT_FDCWD, "/mnt/image");
            say(format!("trees-moved {:?} {:?}", r1, r2));
            chdir(c("/mnt").as_ptr());
            let r = syscall(SYS_pivot_root, c(".").as_ptr(), c("oldroot").as_ptr()); say(format!("pivot_root {r}"));
            chdir(c("/").as_ptr());
            let r = umount2(c("/oldroot").as_ptr(), MNT_DETACH); say(format!("umount-oldroot {r}"));
            fs::remove_dir("/oldroot").ok();
            // step 5: proc after pid ns (we ARE pid 1 in a new pid ns); nosuid,nodev,noexec
            let r = mount(c("proc").as_ptr(), c("/proc").as_ptr(), c("proc").as_ptr(), MS_NOSUID | MS_NODEV | MS_NOEXEC, ptr::null()); say(format!("proc {r} pid1={} nproc={}", getpid(), fs::read_dir("/proc").map(|d| d.filter(|e| e.as_ref().ok().and_then(|e| e.file_name().to_str()?.parse::<i32>().ok()).is_some()).count()).unwrap_or(0)));
            let r = mount(c("sysfs").as_ptr(), c("/sys").as_ptr(), c("sysfs").as_ptr(), MS_NOSUID | MS_NODEV | MS_NOEXEC | MS_RDONLY, ptr::null()); say(format!("sysfs {r} netifs={}", fs::read_dir("/sys/class/net").map(|d| d.count()).unwrap_or(99)));
            // R-CON-4 checks
            let mi = mounts_of("self");
            let all_nosuid_nodev = mi.lines().all(|l| l.contains("nosuid") && l.contains("nodev"));
            let cgroup_visible = mi.contains("cgroup2"); let host_root_visible = mi.lines().any(|l| l.split_whitespace().nth(4) == Some("/") && l.contains("/dev/sda"));
            let systemd_sock = fs::metadata("/run/systemd/private").is_ok() || fs::metadata("/run/dbus/system_bus_socket").is_ok();
            say(format!("world mounts={} all_nosuid_nodev={all_nosuid_nodev} cgroupfs_visible={cgroup_visible} host_root_visible={host_root_visible} systemd_or_dbus_socket={systemd_sock} secret_readable={}", mi.lines().count(), fs::read_to_string(format!("{base}/secret/key")).is_ok() || fs::read_to_string("/workspace/../secret/key").is_ok()));
            say(format!("workspace-file {:?} image-exec-bit {:?}", fs::read_to_string("/workspace/hello.txt").ok(), fs::metadata("/image/bin/busybox").map(|m| std::os::unix::fs::PermissionsExt::mode(&m.permissions()) & 0o111 != 0).ok()));
            // step 6: descriptor closure — allowlist = {0,1,2, rep[1]}
            let before = open_fds();
            let keep = rep[1];
            // move keep to a high fixed number, then close everything else via close_range
            let hi = 1000; dup3(keep, hi, O_CLOEXEC); close(keep);
            syscall(SYS_CLOSE_RANGE, 3u32, (hi - 1) as u32, 0u32); syscall(SYS_CLOSE_RANGE, (hi + 1) as u32, u32::MAX, 0u32);
            let after = open_fds();
            let say2 = |s: String| { let s = s + "\n"; write(hi, s.as_ptr() as *const c_void, s.len()); };
            say2(format!("fds before={:?} after={:?}", before, after));
            // reintroduction paths: /proc/self/fd/N of a closed fd; memfd via /proc; SCM_RIGHTS via the socketpair (closed)
            let reopen = open(c(&format!("/proc/self/fd/{leak_fd}")).as_ptr(), O_RDONLY); let e1 = if reopen < 0 { errno() } else { 0 };
            let reopen_m = open(c(&format!("/proc/self/fd/{memfd}")).as_ptr(), O_RDONLY); let e2 = if reopen_m < 0 { errno() } else { 0 };
            let sock_alive = fcntl(sv[1], F_GETFD) >= 0;
            // parent's fd table via /proc/1/fd? we are pid 1 in this ns; host pids invisible
            let host_pid_visible = fs::metadata(format!("/proc/{}", 2)).is_ok(); // kthreadd on host
            say2(format!("reintro leak_fd={e1} memfd={e2} scm_socket_alive={sock_alive} host_pid2_visible={host_pid_visible}"));
            // step 7 (partial): no_new_privs + drop caps, then exec the image binary
            prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0);
            for cap in 0..=40 { prctl(PR_CAPBSET_DROP, cap, 0, 0, 0); }
            setgroups(0, ptr::null()); setgid(200042); setuid(200042);
            say2(format!("exec-ready uid={} nnp={} capbnd_cap_sys_admin={}", getuid(), prctl(PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0), prctl(PR_CAPBSET_READ, 21, 0, 0, 0)));
            close(hi);
            execl(c("/image/bin/busybox").as_ptr(), c("busybox").as_ptr(), c("true").as_ptr(), ptr::null::<c_char>());
            _exit(126);
        }
    }
    unsafe { close(bar[0]); close(rep[1]); }
    // parent: before releasing the barrier, the child exists in the new namespaces but has done nothing
    let child_mounts = mounts_of(&pid.to_string()).lines().count();
    result("C1-1.clone3-barrier-child-idle-in-new-namespaces", pidfd >= 0 && child_mounts == host_mounts_before, &format!("pidfd={pidfd}; child mountinfo lines={child_mounts} == host {host_mounts_before} (nothing changed yet; rollback here = kill via pidfd)"));
    unsafe { write(bar[1], b"g".as_ptr() as *const c_void, 1); close(bar[1]); }
    let mut report = String::new(); let mut buf = [0u8; 4096];
    loop { let n = unsafe { read(rep[0], buf.as_mut_ptr() as *mut c_void, 4096) }; if n <= 0 { break; } report.push_str(&String::from_utf8_lossy(&buf[..n as usize])); }
    let mut st = 0; unsafe { waitpid(pid, &mut st, 0); }
    let code = (st >> 8) & 0xff;
    for l in report.lines() { println!("  child: {l}"); }
    let g = |k: &str| report.lines().find(|l| l.starts_with(k)).unwrap_or("").to_string();
    result("C2-1.rprivate-before-any-bind", g("rprivate").ends_with(" 0"), &g("rprivate"));
    result("C4-1.root-by-fsmount-and-move_mount", g("root-tmpfs-moved").contains("Ok") && g("trees-moved").contains("Ok(()) Ok(())"), "tmpfs root via fsopen/fsconfig/fsmount; workspace and image trees attached by move_mount from detached FDs — no source path string crossed the namespace");
    result("C4-2.pivot_root-not-chroot", g("pivot_root").ends_with(" 0") && g("umount-oldroot").ends_with(" 0"), &format!("{} / {}", g("pivot_root"), g("umount-oldroot")));
    let proc_line = g("proc ");
    result("C5-1.proc-after-pidns-shows-only-session", proc_line.contains("pid1=1") && proc_line.contains("nproc=1"), &proc_line);
    result("C5-2.fresh-sysfs-shows-only-lo", g("sysfs").contains("netifs=1"), &format!("{} (F-2 follow-up: sysfs mounted after netns → lo only)", g("sysfs")));
    let world = g("world");
    result("C4-3.world-nosuid-nodev-no-cgroup-no-systemd", world.contains("all_nosuid_nodev=true") && world.contains("cgroupfs_visible=false") && world.contains("host_root_visible=false") && world.contains("systemd_or_dbus_socket=false") && world.contains("secret_readable=false"), &world);
    result("C4-4.workspace-and-image-content-correct", g("workspace-file").contains("Some(\"workspace-file\")") && g("workspace-file").contains("image-exec-bit Some(true)"), &g("workspace-file"));
    let fds = g("fds");
    result("C6-1.close_range-leaves-only-allowlist", fds.contains("after=[0, 1, 2, 1000]"), &fds);
    let re = g("reintro");
    result("C6-2.no-reintroduction-via-procfd-memfd-scm", re.contains(&format!("leak_fd={ENOENT}")) && re.contains(&format!("memfd={ENOENT}")) && re.contains("scm_socket_alive=false") && re.contains("host_pid2_visible=false"), &format!("{re} (ENOENT={ENOENT}: closed fds have no /proc/self/fd entry; socketpair end closed so SCM_RIGHTS has no carrier)"));
    result("C7-1.nnp-caps-uid-then-exec", g("exec-ready").contains("uid=200042 nnp=1 capbnd_cap_sys_admin=0") && code == 0, &format!("{}; exec of /image/bin/busybox true exit code={code}", g("exec-ready")));
    // host side after: nothing leaked into host mount table; leak_fd still ours (closure was in the child only)
    let host_mounts_after = mounts_of("self").lines().count();
    result("C2-2.no-propagation-to-host", host_mounts_after == host_mounts_before && fs::metadata("/mnt/workspace").map(|_| false).unwrap_or(true), &format!("host mountinfo lines {host_mounts_before}→{host_mounts_after}; /mnt/workspace absent on host"));
    let secret_intact = fs::read_to_string(format!("{base}/secret/key")).unwrap() == "SECRET";
    println!("secret intact on host={secret_intact}; parent still holds leak_fd={} memfd={}", leak_fd, memfd);
    unsafe { close(leak_fd); close(memfd); close(sv[0]); close(sv[1]); close(tree); close(img_tree); close(wsfd); close(pidfd); }
    let _ = fs::remove_dir_all(base);
    println!("done");
}
