//! Raw kernel interfaces used by the constructor (WP1 mount-construct,
//! scope-kill, netns-seccomp spikes; verified on 6.12). Every wrapper returns
//! the errno on failure so the rollback ledger can record it.

use std::ffi::CString;
use std::os::fd::RawFd;

pub const SYS_OPENAT2: libc::c_long = 437; pub const SYS_OPEN_TREE: libc::c_long = 428; pub const SYS_MOVE_MOUNT: libc::c_long = 429;
pub const SYS_CLOSE_RANGE: libc::c_long = 436; pub const SYS_FSOPEN: libc::c_long = 430; pub const SYS_FSCONFIG: libc::c_long = 431; pub const SYS_FSMOUNT: libc::c_long = 432;
pub const SYS_MOUNT_SETATTR: libc::c_long = 442; pub const SYS_CLONE3: libc::c_long = 435; pub const SYS_PIVOT_ROOT: libc::c_long = 155;
pub const RESOLVE_NO_MAGICLINKS: u64 = 0x02; pub const RESOLVE_NO_SYMLINKS: u64 = 0x04; pub const RESOLVE_BENEATH: u64 = 0x08;
pub const OPEN_TREE_CLONE: libc::c_uint = 1; pub const AT_RECURSIVE: libc::c_int = 0x8000;
pub const MOVE_MOUNT_F_EMPTY_PATH: libc::c_uint = 0x4;
pub const FSCONFIG_SET_STRING: libc::c_uint = 1; pub const FSCONFIG_SET_FLAG: libc::c_uint = 0; pub const FSCONFIG_CMD_CREATE: libc::c_uint = 6;
pub const MOUNT_ATTR_RDONLY: u64 = 0x1; pub const MOUNT_ATTR_NOSUID: u64 = 0x2; pub const MOUNT_ATTR_NODEV: u64 = 0x4; pub const MOUNT_ATTR_NOEXEC: u64 = 0x8;
pub const CLONE_INTO_CGROUP: u64 = 0x2_0000_0000; pub const CLONE_PIDFD: u64 = 0x1000;

#[repr(C)] #[derive(Default)] struct OpenHow { flags: u64, mode: u64, resolve: u64 }
#[repr(C)] struct MountAttr { attr_set: u64, attr_clr: u64, propagation: u64, userns_fd: u64 }
#[repr(C)] #[derive(Default)] pub struct CloneArgs { pub flags: u64, pub pidfd: u64, pub child_tid: u64, pub parent_tid: u64, pub exit_signal: u64, pub stack: u64, pub stack_size: u64, pub tls: u64, pub set_tid: u64, pub set_tid_size: u64, pub cgroup: u64 }

pub fn errno() -> i32 { std::io::Error::last_os_error().raw_os_error().unwrap_or(0) }
pub fn c(s: &str) -> CString { CString::new(s).unwrap() }
fn r(v: libc::c_long) -> Result<i32, i32> { if v < 0 { Err(errno()) } else { Ok(v as i32) } }

/// Confined open: no symlinks, no magic links, never escapes `dir`.
pub fn openat2(dir: RawFd, path: &str, flags: u64) -> Result<RawFd, i32> {
    let how = OpenHow { flags: flags | libc::O_CLOEXEC as u64, mode: 0, resolve: RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS };
    r(unsafe { libc::syscall(SYS_OPENAT2, dir, c(path).as_ptr(), &how as *const OpenHow, std::mem::size_of::<OpenHow>()) })
}
pub fn open_tree_clone(fd: RawFd) -> Result<RawFd, i32> { r(unsafe { libc::syscall(SYS_OPEN_TREE, fd, c("").as_ptr(), OPEN_TREE_CLONE | libc::AT_EMPTY_PATH as libc::c_uint | AT_RECURSIVE as libc::c_uint | libc::O_CLOEXEC as libc::c_uint) }) }
pub fn mount_setattr(fd: RawFd, set: u64) -> Result<(), i32> {
    let a = MountAttr { attr_set: set, attr_clr: 0, propagation: libc::MS_PRIVATE as u64, userns_fd: 0 };
    r(unsafe { libc::syscall(SYS_MOUNT_SETATTR, fd, c("").as_ptr(), libc::AT_EMPTY_PATH | AT_RECURSIVE, &a as *const MountAttr, std::mem::size_of::<MountAttr>()) }).map(|_| ())
}
pub fn move_mount(from: RawFd, to_dir: RawFd, to_path: &str) -> Result<(), i32> { r(unsafe { libc::syscall(SYS_MOVE_MOUNT, from, c("").as_ptr(), to_dir, c(to_path).as_ptr(), MOVE_MOUNT_F_EMPTY_PATH) }).map(|_| ()) }
/// New detached filesystem instance (`fstype`, string options, flags) → mount fd.
pub fn fsmount(fstype: &str, opts: &[(&str, &str)], flags: &[&str], attr: u64) -> Result<RawFd, i32> {
    let fs = r(unsafe { libc::syscall(SYS_FSOPEN, c(fstype).as_ptr(), 1u32 /*FSOPEN_CLOEXEC*/) })?;
    for (k, v) in opts { r(unsafe { libc::syscall(SYS_FSCONFIG, fs, FSCONFIG_SET_STRING, c(k).as_ptr(), c(v).as_ptr(), 0) })?; }
    for f in flags { r(unsafe { libc::syscall(SYS_FSCONFIG, fs, FSCONFIG_SET_FLAG, c(f).as_ptr(), 0usize, 0) })?; }
    r(unsafe { libc::syscall(SYS_FSCONFIG, fs, FSCONFIG_CMD_CREATE, 0usize, 0usize, 0) })?;
    let m = r(unsafe { libc::syscall(SYS_FSMOUNT, fs, 1u32 /*FSMOUNT_CLOEXEC*/, attr) }); unsafe { libc::close(fs) }; m
}
pub fn pivot_root(new: &str, old: &str) -> Result<(), i32> { r(unsafe { libc::syscall(SYS_PIVOT_ROOT, c(new).as_ptr(), c(old).as_ptr()) }).map(|_| ()) }
pub fn close_range_from(first: u32) -> Result<(), i32> { r(unsafe { libc::syscall(SYS_CLOSE_RANGE, first, u32::MAX, 0u32) }).map(|_| ()) }
pub fn clone3(a: &mut CloneArgs) -> Result<i32, i32> { r(unsafe { libc::syscall(SYS_CLONE3, a as *mut CloneArgs, std::mem::size_of::<CloneArgs>()) }) }
pub fn pidfd_send_signal(pidfd: RawFd, sig: i32) -> bool { unsafe { libc::syscall(libc::SYS_pidfd_send_signal, pidfd, sig, 0usize, 0u32) == 0 } }
pub fn open_fds() -> Vec<(i32, String)> {
    std::fs::read_dir("/proc/self/fd").map(|d| d.filter_map(|e| { let e = e.ok()?; let fd: i32 = e.file_name().to_str()?.parse().ok()?; Some((fd, std::fs::read_link(e.path()).map(|p| p.to_string_lossy().into_owned()).unwrap_or_default())) }).collect()).unwrap_or_default()
}

// ---- seccomp: kill on foreign arch; socket(2) with family != AF_UNIX → EPERM (netns-seccomp spike SC-1..5) ----
const SECCOMP_SET_MODE_FILTER: libc::c_uint = 1; const SECCOMP_FILTER_FLAG_TSYNC: libc::c_ulong = 1;
const SECCOMP_RET_ERRNO: u32 = 0x0005_0000; const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000; const SECCOMP_RET_KILL_PROCESS: u32 = 0x8000_0000;
const AUDIT_ARCH_X86_64: u32 = 0xC000_003E;
#[repr(C)] struct Filter { code: u16, jt: u8, jf: u8, k: u32 }
#[repr(C)] struct Prog { len: u16, filter: *const Filter }
pub fn seccomp_af_unix_only() -> Result<(), i32> {
    let st = |code: u16, k: u32| Filter { code, jt: 0, jf: 0, k }; let jmp = |k: u32, jt: u8, jf: u8| Filter { code: 0x15, jt, jf, k };
    let f = [st(0x20, 4), jmp(AUDIT_ARCH_X86_64, 1, 0), st(0x06, SECCOMP_RET_KILL_PROCESS),
        st(0x20, 0), jmp(libc::SYS_socket as u32, 0, 3), st(0x20, 16), jmp(libc::AF_UNIX as u32, 1, 0), st(0x06, SECCOMP_RET_ERRNO | libc::EPERM as u32), st(0x06, SECCOMP_RET_ALLOW)];
    let p = Prog { len: f.len() as u16, filter: f.as_ptr() };
    r(unsafe { libc::syscall(libc::SYS_seccomp, SECCOMP_SET_MODE_FILTER, SECCOMP_FILTER_FLAG_TSYNC, &p as *const Prog) }).map(|_| ())
}
/// Drop the capability bounding set and ambient set entirely.
pub fn drop_caps() -> Result<(), i32> {
    for cap in 0..=40 { let v = unsafe { libc::prctl(libc::PR_CAPBSET_DROP, cap as libc::c_ulong, 0, 0, 0) }; if v < 0 && errno() != libc::EINVAL { return Err(errno()); } }
    if unsafe { libc::prctl(libc::PR_CAP_AMBIENT, libc::PR_CAP_AMBIENT_CLEAR_ALL as libc::c_ulong, 0, 0, 0) } < 0 { return Err(errno()); }
    // clear permitted/effective/inheritable
    #[repr(C)] struct Hdr { version: u32, pid: i32 } #[repr(C)] #[derive(Default, Clone, Copy)] struct Data { e: u32, p: u32, i: u32 }
    let h = Hdr { version: 0x2008_0522, pid: 0 }; let d = [Data::default(); 2];
    r(unsafe { libc::syscall(libc::SYS_capset, &h as *const Hdr, d.as_ptr()) }).map(|_| ())
}
pub fn write_all_fd(fd: RawFd, b: &[u8]) -> bool { let mut off = 0; while off < b.len() { let n = unsafe { libc::write(fd, b[off..].as_ptr() as *const libc::c_void, b.len() - off) }; if n <= 0 { return false; } off += n as usize; } true }
pub fn read_line_fd(fd: RawFd, timeout_ms: i32) -> Option<String> {
    let mut out = Vec::new(); let mut p = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
    loop { if unsafe { libc::poll(&mut p, 1, timeout_ms) } <= 0 { return None; } let mut b = [0u8; 1]; let n = unsafe { libc::read(fd, b.as_mut_ptr() as *mut libc::c_void, 1) }; if n <= 0 { return None; } if b[0] == b'\n' { return String::from_utf8(out).ok(); } out.push(b[0]); if out.len() > 4096 { return None; } }
}
