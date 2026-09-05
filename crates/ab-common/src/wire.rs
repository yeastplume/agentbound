//! `AF_UNIX SOCK_SEQPACKET` component transport (component interfaces §2) and
//! the message envelope defined in the WP2 wire-format document.
//!
//! One message per packet; canonical JSON; ≤ 64 KiB; the receiving side reads
//! `SO_PEERCRED` at accept and enforces the expected service UID before parsing.

use crate::json::{self, canonical, Value, MANIFEST_LIMITS};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

pub const MAX_MSG: usize = 64 * 1024;
pub const PROTOCOL_VERSION: &str = "agentbound.wire.v0.1";

#[derive(Clone, Debug)]
pub struct Peer { pub pid: i32, pub uid: u32, pub gid: u32 }

pub struct Conn { pub fd: OwnedFd, pub peer: Peer }

fn cstr(p: &str) -> Vec<u8> { let mut v = p.as_bytes().to_vec(); v.push(0); v }
fn sockaddr(path: &str) -> (libc::sockaddr_un, u32) {
    let mut a: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    a.sun_family = libc::AF_UNIX as u16;
    let b = path.as_bytes(); assert!(b.len() < a.sun_path.len());
    for (i, c) in b.iter().enumerate() { a.sun_path[i] = *c as libc::c_char; }
    (a, (std::mem::size_of::<libc::sa_family_t>() + b.len() + 1) as u32)
}
fn os(r: libc::c_int) -> io::Result<libc::c_int> { if r < 0 { Err(io::Error::last_os_error()) } else { Ok(r) } }

/// Bind a listener; mode applied via umask + fchmod-equivalent chmod on the path (§2: administrator-provisioned path/owner/mode).
pub fn listen(path: &str, mode: u32) -> io::Result<OwnedFd> {
    let _ = std::fs::remove_file(path);
    let fd = os(unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC, 0) })?;
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    let old = unsafe { libc::umask(0o777) };
    let (a, l) = sockaddr(path);
    let r = unsafe { libc::bind(fd.as_raw_fd(), &a as *const _ as *const libc::sockaddr, l) };
    unsafe { libc::umask(old) };
    os(r)?;
    os(unsafe { libc::chmod(cstr(path).as_ptr() as *const libc::c_char, mode) })?;
    os(unsafe { libc::listen(fd.as_raw_fd(), 16) })?;
    Ok(fd)
}

/// Accept and read `SO_PEERCRED` immediately (§2).
pub fn accept(listener: &OwnedFd) -> io::Result<Conn> {
    let fd = os(unsafe { libc::accept4(listener.as_raw_fd(), std::ptr::null_mut(), std::ptr::null_mut(), libc::SOCK_CLOEXEC) })?;
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    let peer = peercred(fd.as_raw_fd())?;
    Ok(Conn { fd, peer })
}
pub fn peercred(fd: RawFd) -> io::Result<Peer> {
    let mut uc: libc::ucred = unsafe { std::mem::zeroed() };
    let mut l = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    os(unsafe { libc::getsockopt(fd, libc::SOL_SOCKET, libc::SO_PEERCRED, &mut uc as *mut _ as *mut libc::c_void, &mut l) })?;
    Ok(Peer { pid: uc.pid, uid: uc.uid, gid: uc.gid })
}

pub fn connect(path: &str) -> io::Result<Conn> {
    let fd = os(unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC, 0) })?;
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    let (a, l) = sockaddr(path);
    os(unsafe { libc::connect(fd.as_raw_fd(), &a as *const _ as *const libc::sockaddr, l) })?;
    let peer = peercred(fd.as_raw_fd())?;
    Ok(Conn { fd, peer })
}

impl Conn {
    pub fn send(&self, v: &Value) -> io::Result<()> {
        let b = canonical(v);
        if b.len() > MAX_MSG { return Err(io::Error::new(io::ErrorKind::InvalidInput, "message too large")); }
        let n = os(unsafe { libc::send(self.fd.as_raw_fd(), b.as_ptr() as *const libc::c_void, b.len(), libc::MSG_NOSIGNAL) } as libc::c_int)?;
        if n as usize != b.len() { return Err(io::Error::new(io::ErrorKind::WriteZero, "short seqpacket send")); }
        Ok(())
    }
    /// Receive one packet; `None` on orderly close. Truncated packets are rejected (MSG_TRUNC).
    pub fn recv(&self) -> io::Result<Option<Value>> {
        let mut buf = vec![0u8; MAX_MSG + 1];
        let n = unsafe { libc::recv(self.fd.as_raw_fd(), buf.as_mut_ptr() as *mut libc::c_void, buf.len(), libc::MSG_TRUNC) };
        if n < 0 { return Err(io::Error::last_os_error()); }
        if n == 0 { return Ok(None); }
        if n as usize > MAX_MSG { return Err(io::Error::new(io::ErrorKind::InvalidData, "oversize packet")); }
        let v = json::parse_canonical(&buf[..n as usize], &MANIFEST_LIMITS).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(Some(v))
    }
    /// Send a request and wait for exactly one reply.
    pub fn call(&self, v: &Value) -> io::Result<Value> {
        self.send(v)?;
        self.recv()?.ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "peer closed"))
    }
    /// Send one packet carrying file descriptors via SCM_RIGHTS (launch→lifecycle pidfd handover, §3.3).
    pub fn send_with_fds(&self, v: &Value, fds: &[RawFd]) -> io::Result<()> {
        let b = canonical(v);
        let mut iov = libc::iovec { iov_base: b.as_ptr() as *mut libc::c_void, iov_len: b.len() };
        let space = unsafe { libc::CMSG_SPACE((fds.len() * 4) as u32) } as usize;
        let mut cbuf = vec![0u8; space];
        let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
        msg.msg_iov = &mut iov; msg.msg_iovlen = 1; msg.msg_control = cbuf.as_mut_ptr() as *mut libc::c_void; msg.msg_controllen = space;
        unsafe {
            let c = libc::CMSG_FIRSTHDR(&msg);
            (*c).cmsg_level = libc::SOL_SOCKET; (*c).cmsg_type = libc::SCM_RIGHTS; (*c).cmsg_len = libc::CMSG_LEN((fds.len() * 4) as u32) as usize;
            std::ptr::copy_nonoverlapping(fds.as_ptr(), libc::CMSG_DATA(c) as *mut RawFd, fds.len());
        }
        let n = unsafe { libc::sendmsg(self.fd.as_raw_fd(), &msg, libc::MSG_NOSIGNAL) };
        if n < 0 { return Err(io::Error::last_os_error()); }
        Ok(())
    }
    pub fn recv_with_fds(&self, max_fds: usize) -> io::Result<Option<(Value, Vec<OwnedFd>)>> {
        let mut buf = vec![0u8; MAX_MSG + 1];
        let mut iov = libc::iovec { iov_base: buf.as_mut_ptr() as *mut libc::c_void, iov_len: buf.len() };
        let space = unsafe { libc::CMSG_SPACE((max_fds * 4) as u32) } as usize;
        let mut cbuf = vec![0u8; space];
        let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
        msg.msg_iov = &mut iov; msg.msg_iovlen = 1; msg.msg_control = cbuf.as_mut_ptr() as *mut libc::c_void; msg.msg_controllen = space;
        let n = unsafe { libc::recvmsg(self.fd.as_raw_fd(), &mut msg, libc::MSG_CMSG_CLOEXEC) };
        if n < 0 { return Err(io::Error::last_os_error()); }
        if n == 0 { return Ok(None); }
        let mut fds = Vec::new();
        unsafe {
            let mut c = libc::CMSG_FIRSTHDR(&msg);
            while !c.is_null() {
                if (*c).cmsg_level == libc::SOL_SOCKET && (*c).cmsg_type == libc::SCM_RIGHTS {
                    let cnt = ((*c).cmsg_len - libc::CMSG_LEN(0) as usize) / 4;
                    let p = libc::CMSG_DATA(c) as *const RawFd;
                    for i in 0..cnt { fds.push(OwnedFd::from_raw_fd(*p.add(i))); }
                }
                c = libc::CMSG_NXTHDR(&msg, c);
            }
        }
        let v = json::parse_canonical(&buf[..n as usize], &MANIFEST_LIMITS).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(Some((v, fds)))
    }
}

/// Result of one gateway-side packet receive (ADR-0002 Decision 2): raw bytes plus ancillary accounting.
pub struct Packet { pub bytes: Vec<u8>, pub creds: Vec<Peer>, pub rights_fds: usize, pub truncated: bool }

/// Gateway packet receive: `SO_PASSCRED` must already be enabled. Every control message is counted;
/// any `SCM_RIGHTS` descriptors are closed immediately. The caller enforces "exactly one credential".
pub fn recv_packet(fd: RawFd, max_len: usize) -> io::Result<Option<Packet>> {
    let mut buf = vec![0u8; max_len + 1];
    let mut iov = libc::iovec { iov_base: buf.as_mut_ptr() as *mut libc::c_void, iov_len: buf.len() };
    let space = unsafe { libc::CMSG_SPACE(std::mem::size_of::<libc::ucred>() as u32) * 4 + libc::CMSG_SPACE(64 * 4) } as usize;
    let mut cbuf = vec![0u8; space];
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov; msg.msg_iovlen = 1; msg.msg_control = cbuf.as_mut_ptr() as *mut libc::c_void; msg.msg_controllen = space;
    let n = unsafe { libc::recvmsg(fd, &mut msg, libc::MSG_CMSG_CLOEXEC | libc::MSG_TRUNC) };
    if n < 0 { return Err(io::Error::last_os_error()); }
    if n == 0 { return Ok(None); }
    let (mut creds, mut rights) = (Vec::new(), 0usize);
    unsafe {
        let mut c = libc::CMSG_FIRSTHDR(&msg);
        while !c.is_null() {
            if (*c).cmsg_level == libc::SOL_SOCKET && (*c).cmsg_type == libc::SCM_CREDENTIALS {
                let u = &*(libc::CMSG_DATA(c) as *const libc::ucred); creds.push(Peer { pid: u.pid, uid: u.uid, gid: u.gid });
            } else if (*c).cmsg_level == libc::SOL_SOCKET && (*c).cmsg_type == libc::SCM_RIGHTS {
                let cnt = ((*c).cmsg_len - libc::CMSG_LEN(0) as usize) / 4; let p = libc::CMSG_DATA(c) as *const RawFd;
                for i in 0..cnt { libc::close(*p.add(i)); rights += 1; }
            } else { creds.push(Peer { pid: -1, uid: u32::MAX, gid: u32::MAX }); } // unknown control message counts as a defect
            c = libc::CMSG_NXTHDR(&msg, c);
        }
    }
    let truncated = (n as usize) > max_len || (msg.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC)) != 0;
    buf.truncate((n as usize).min(max_len)); Ok(Some(Packet { bytes: buf, creds, rights_fds: rights, truncated }))
}
pub fn set_passcred(fd: RawFd) -> io::Result<()> { let one: libc::c_int = 1; os(unsafe { libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_PASSCRED, &one as *const _ as *const libc::c_void, 4) }).map(|_| ()) }
pub fn send_raw(fd: RawFd, bytes: &[u8]) -> io::Result<()> { os(unsafe { libc::send(fd, bytes.as_ptr() as *const libc::c_void, bytes.len(), libc::MSG_NOSIGNAL) } as i32).map(|_| ()) }

/// Process-instance identity from a pidfd (ADR-0002 D2, WP1 F-1): the pidfs inode is the key; start time corroborates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcInstance { pub pid: i32, pub pidfs_ino: u64, pub start_time: u64, pub pidns: u64, pub cgroup: String }
pub fn proc_instance(pid: i32) -> io::Result<(OwnedFd, ProcInstance)> {
    let pfd = os(unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) } as i32)?; let pfd = unsafe { OwnedFd::from_raw_fd(pfd) };
    let mut st: libc::stat = unsafe { std::mem::zeroed() }; os(unsafe { libc::fstat(pfd.as_raw_fd(), &mut st) })?;
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))?; let after = stat.rsplit(") ").next().unwrap_or(""); let start_time: u64 = after.split(' ').nth(19).and_then(|x| x.parse().ok()).unwrap_or(0);
    let pidns = std::fs::metadata(format!("/proc/{pid}/ns/pid")).map(|m| { use std::os::unix::fs::MetadataExt; m.ino() }).unwrap_or(0);
    let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).unwrap_or_default().trim().rsplit(':').next().unwrap_or("").to_string();
    // the process may have exited and been replaced between pidfd_open and the /proc reads: re-check liveness of the same instance
    let mut st2: libc::stat = unsafe { std::mem::zeroed() }; os(unsafe { libc::fstat(pfd.as_raw_fd(), &mut st2) })?;
    if st2.st_ino != st.st_ino || start_time == 0 { return Err(io::Error::new(io::ErrorKind::NotFound, "process instance changed")); }
    Ok((pfd, ProcInstance { pid, pidfs_ino: st.st_ino, start_time, pidns, cgroup }))
}

// ---- message envelope (wire-format document §2) ----
/// Request: {"v":PROTOCOL_VERSION,"op":..., "idempotency_key":..., "body":{...}}
pub fn request(op: &str, idem: &str, body: Value) -> Value {
    Value::obj(vec![("body", body), ("idempotency_key", Value::s(idem)), ("op", Value::s(op)), ("v", Value::s(PROTOCOL_VERSION))])
}
/// Reply: {"v":..., "ok":bool, "class":<error class or "ok">, "body":{...}}
pub fn reply_ok(body: Value) -> Value { Value::obj(vec![("body", body), ("class", Value::s("ok")), ("ok", Value::Bool(true)), ("v", Value::s(PROTOCOL_VERSION))]) }
pub fn reply_err(class: &str, rule: &str, detail: &str) -> Value {
    Value::obj(vec![("body", Value::obj(vec![("detail", Value::s(detail)), ("rule", Value::s(rule))])), ("class", Value::s(class)), ("ok", Value::Bool(false)), ("v", Value::s(PROTOCOL_VERSION))])
}
/// Error classes (component interfaces §7).
pub const CLASS_INVALID: &str = "invalid";
pub const CLASS_UNAUTHENTICATED: &str = "unauthenticated";
pub const CLASS_UNAUTHORIZED: &str = "unauthorized";
pub const CLASS_CONFLICT: &str = "conflict";
pub const CLASS_UNAVAILABLE: &str = "unavailable";
pub const CLASS_INTERNAL: &str = "internal";

pub struct Req<'a> { pub op: &'a str, pub idem: &'a str, pub body: &'a Value }
pub fn parse_request(v: &Value) -> Result<Req<'_>, &'static str> {
    if v.get("v").and_then(|x| x.as_str()) != Some(PROTOCOL_VERSION) { return Err("unsupported protocol version"); }
    let m = v.as_obj().ok_or("object")?; if m.len() != 4 { return Err("envelope must have exactly v, op, idempotency_key, body"); }
    let op = v.get("op").and_then(|x| x.as_str()).ok_or("op")?;
    let idem = v.get("idempotency_key").and_then(|x| x.as_str()).filter(|s| !s.is_empty() && s.len() <= 128).ok_or("idempotency_key")?;
    let body = v.get("body").filter(|b| b.as_obj().is_some()).ok_or("body")?;
    Ok(Req { op, idem, body })
}

/// systemd fd store (sd_notify FDSTORE=1 with a name) — a daemon that is restarted receives them back via LISTEN_FDS/LISTEN_FDNAMES.
pub fn fdstore_push(name: &str, fd: RawFd) -> io::Result<()> {
    let Some(ns) = std::env::var_os("NOTIFY_SOCKET") else { return Err(io::Error::new(io::ErrorKind::NotFound, "NOTIFY_SOCKET")) };
    let ns = ns.to_string_lossy().to_string();
    let sock = os(unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) })?; let sock = unsafe { OwnedFd::from_raw_fd(sock) };
    let (a, l) = sockaddr(&ns);
    os(unsafe { libc::connect(sock.as_raw_fd(), &a as *const _ as *const libc::sockaddr, l) })?;
    let text = format!("FDSTORE=1\nFDNAME={name}\n");
    let mut iov = libc::iovec { iov_base: text.as_ptr() as *mut libc::c_void, iov_len: text.len() };
    let space = unsafe { libc::CMSG_SPACE(4) } as usize; let mut cbuf = vec![0u8; space];
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov; msg.msg_iovlen = 1; msg.msg_control = cbuf.as_mut_ptr() as *mut libc::c_void; msg.msg_controllen = space;
    unsafe { let h = libc::CMSG_FIRSTHDR(&msg); (*h).cmsg_level = libc::SOL_SOCKET; (*h).cmsg_type = libc::SCM_RIGHTS; (*h).cmsg_len = libc::CMSG_LEN(4) as usize; *(libc::CMSG_DATA(h) as *mut RawFd) = fd; }
    os(unsafe { libc::sendmsg(sock.as_raw_fd(), &msg, libc::MSG_NOSIGNAL) } as i32)?; Ok(())
}
pub fn sd_notify(text: &str) {
    let Some(ns) = std::env::var_os("NOTIFY_SOCKET") else { return };
    let ns = ns.to_string_lossy().to_string();
    let Ok(sock) = os(unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) }) else { return }; let sock = unsafe { OwnedFd::from_raw_fd(sock) };
    let (a, l) = sockaddr(&ns); if unsafe { libc::connect(sock.as_raw_fd(), &a as *const _ as *const libc::sockaddr, l) } != 0 { return; }
    unsafe { libc::send(sock.as_raw_fd(), text.as_ptr() as *const libc::c_void, text.len(), libc::MSG_NOSIGNAL) };
}
pub fn fdstore_remove(name: &str) {
    let Some(ns) = std::env::var_os("NOTIFY_SOCKET") else { return };
    let ns = ns.to_string_lossy().to_string();
    let Ok(sock) = os(unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) }) else { return }; let sock = unsafe { OwnedFd::from_raw_fd(sock) };
    let (a, l) = sockaddr(&ns); if unsafe { libc::connect(sock.as_raw_fd(), &a as *const _ as *const libc::sockaddr, l) } != 0 { return; }
    let text = format!("FDSTOREREMOVE=1\nFDNAME={name}\n");
    unsafe { libc::send(sock.as_raw_fd(), text.as_ptr() as *const libc::c_void, text.len(), libc::MSG_NOSIGNAL) };
}
/// Descriptors handed back by systemd on restart: (name, fd).
pub fn listen_fds() -> Vec<(String, OwnedFd)> {
    let Ok(pid) = std::env::var("LISTEN_PID") else { return vec![] };
    if pid.parse::<i32>().ok() != Some(unsafe { libc::getpid() }) { return vec![]; }
    let n: i32 = std::env::var("LISTEN_FDS").ok().and_then(|x| x.parse().ok()).unwrap_or(0);
    let names: Vec<String> = std::env::var("LISTEN_FDNAMES").map(|x| x.split(':').map(str::to_string).collect()).unwrap_or_default();
    (0..n).map(|i| (names.get(i as usize).cloned().unwrap_or_default(), unsafe { OwnedFd::from_raw_fd(3 + i) })).collect()
}
