//! Session-side gateway client (statically linked into the workload image). Not privileged code.
//! ab-gwclient <socket> <operation_id> <operation> <args-canonical-json> [payload-file] [--fork] [--scm-rights]
//! One packet = one message; payload follows in ≤128 KiB chunks; every packet carries the kernel credential.
use sha2::{Digest, Sha256};
use std::io::Write;
use std::os::fd::AsRawFd;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 5 { eprintln!("usage"); std::process::exit(2); }
    let (fork, rights) = (a.iter().any(|x| x == "--fork"), a.iter().any(|x| x == "--scm-rights"));
    let payload = a.get(5).filter(|p| !p.starts_with("--")).map(|p| std::fs::read(p).expect("payload")).unwrap_or_default();
    let sha = format!("sha256:{}", hex::encode(Sha256::digest(&payload)));
    let msg = format!("{{\"args\":{},\"operation\":\"{}\",\"operation_id\":\"{}\",\"payload_len\":{},\"payload_sha256\":\"{}\",\"v\":\"agentbound.gateway.v0.1\"}}", a[4], a[3], a[2], payload.len(), sha);
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC, 0) };
    if fd < 0 { eprintln!("socket errno={}", std::io::Error::last_os_error()); std::process::exit(3); }
    let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() }; addr.sun_family = libc::AF_UNIX as u16;
    for (i, b) in a[1].bytes().enumerate() { addr.sun_path[i] = b as libc::c_char; }
    if unsafe { libc::connect(fd, &addr as *const _ as *const libc::sockaddr, std::mem::size_of::<libc::sockaddr_un>() as u32) } != 0 { eprintln!("connect errno={}", std::io::Error::last_os_error()); std::process::exit(4); }
    if fork { // T-6.4-007: a child inherits the connected descriptor and speaks first
        let pid = unsafe { libc::fork() };
        if pid > 0 { let mut st = 0; unsafe { libc::waitpid(pid, &mut st, 0) }; std::process::exit(libc::WEXITSTATUS(st)); }
    }
    let send = |bytes: &[u8]| -> bool {
        if rights { // T-6.4-006: attach a descriptor with SCM_RIGHTS
            let mut iov = libc::iovec { iov_base: bytes.as_ptr() as *mut _, iov_len: bytes.len() };
            let mut cbuf = [0u8; 24]; let mut m: libc::msghdr = unsafe { std::mem::zeroed() };
            m.msg_iov = &mut iov; m.msg_iovlen = 1; m.msg_control = cbuf.as_mut_ptr() as *mut _; m.msg_controllen = unsafe { libc::CMSG_SPACE(4) } as usize;
            unsafe { let c = libc::CMSG_FIRSTHDR(&m); (*c).cmsg_level = libc::SOL_SOCKET; (*c).cmsg_type = libc::SCM_RIGHTS; (*c).cmsg_len = libc::CMSG_LEN(4) as usize; *(libc::CMSG_DATA(c) as *mut i32) = 0; }
            let n = unsafe { libc::sendmsg(fd, &m, 0) }; return n >= 0;
        }
        let n = unsafe { libc::send(fd, bytes.as_ptr() as *const _, bytes.len(), libc::MSG_NOSIGNAL) }; n >= 0
    };
    let recv = || -> Option<String> { let mut b = vec![0u8; 1 << 17]; let n = unsafe { libc::recv(fd, b.as_mut_ptr() as *mut _, b.len(), 0) }; if n <= 0 { None } else { Some(String::from_utf8_lossy(&b[..n as usize]).into_owned()) } };
    let out = std::io::stdout(); let mut out = out.lock();
    if !send(msg.as_bytes()) { eprintln!("send errno={}", std::io::Error::last_os_error()); std::process::exit(5); }
    let Some(r) = recv() else { eprintln!("closed by gateway"); std::process::exit(6) }; let _ = writeln!(out, "{r}");
    if !r.contains("\"ok\":true") { std::process::exit(1); }
    let mut off = 0; let mut last = String::new();
    while off < payload.len() { let end = (off + (128 << 10)).min(payload.len()); if !send(&payload[off..end]) { std::process::exit(5); } let Some(r) = recv() else { eprintln!("closed by gateway"); std::process::exit(6) }; last = r; off = end; }
    if !last.is_empty() { let _ = writeln!(out, "{last}"); if !last.contains("\"ok\":true") { std::process::exit(1); } }
    let _ = fd.as_raw_fd();
}
