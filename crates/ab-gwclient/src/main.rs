//! Session-side gateway client (statically linked into the workload image). Not privileged code.
//! ab-gwclient <socket> <operation_id> <operation> <args-canonical-json> [payload-file] [--fork] [--scm-rights]
//! One packet = one message; payload follows in ≤128 KiB chunks; every packet carries the kernel credential.
use sha2::{Digest, Sha256};
use std::io::Write;
use std::os::fd::AsRawFd;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.get(1).map(String::as_str) == Some("--families") { // T-6.4-001: socket() for non-AF_UNIX families under the session seccomp filter
        for (name, fam) in [("inet", libc::AF_INET), ("inet6", libc::AF_INET6), ("packet", libc::AF_PACKET), ("netlink", libc::AF_NETLINK), ("vsock", libc::AF_VSOCK)] {
            let r = unsafe { libc::socket(fam, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) };
            let e = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            println!("{name} {}", if r >= 0 { "OPENED".to_string() } else { format!("errno={e}") }); if r >= 0 { unsafe { libc::close(r) }; }
        }
        return;
    }
    if a.get(1).map(String::as_str) == Some("--fdbound") { // T-6.9-002: RLIMIT_NOFILE read back from the kernel, then opened until EMFILE
        let mut rl = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut rl) } != 0 { println!("rlimit_error errno={}", std::io::Error::last_os_error().raw_os_error().unwrap_or(0)); std::process::exit(1); }
        let ceiling = 200_000usize; let mut opened = 0usize; let mut errno = 0;
        // hold the descriptors so the count is the real simultaneous bound, and report from this process (a shell subprocess at the
        // ceiling cannot write its own result anywhere — that is what made the previous measurement silently empty).
        let mut held: Vec<i32> = Vec::new();
        while opened < ceiling { let fd = unsafe { libc::open(b"/dev/null\0".as_ptr() as *const libc::c_char, libc::O_RDONLY) }; if fd < 0 { errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0); break; } held.push(fd); opened += 1; }
        for fd in held { unsafe { libc::close(fd) }; }
        println!("rlimit_cur={} rlimit_max={} opened={opened} errno={errno}", rl.rlim_cur, rl.rlim_max);
        return;
    }
    if a.get(1).map(String::as_str) == Some("--memhog") { // T-6.9-003: touch <MiB> of anonymous memory; the cgroup must refuse
        let mib: usize = a.get(2).and_then(|x| x.parse().ok()).unwrap_or(512);
        let mut held: Vec<Vec<u8>> = Vec::new();
        for i in 0..mib { let p = unsafe { libc::malloc(1 << 20) } as *mut u8; if p.is_null() { println!("touched_mib={i} errno=12"); std::process::exit(12); }
            unsafe { for off in (0..(1 << 20)).step_by(4096) { *p.add(off) = 1; } held.push(Vec::from_raw_parts(p, 1 << 20, 1 << 20)); } }
        println!("touched_mib={mib} errno=0"); return;
    }
    // T-6.1-010: pidfd_open on a host pid, then pidfd_send_signal through it. A private pid namespace means the host pid is not
    // addressable at all; if a pidfd could be obtained it must still not be usable to signal outside the namespace.
    if a.get(1).map(String::as_str) == Some("--pidfd") {
        let pid: i32 = a.get(2).and_then(|x| x.parse().ok()).unwrap_or(1);
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        let open_err = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        if fd < 0 { println!("pidfd_open={pid} rc=-1 errno={open_err}"); std::process::exit(1); }
        let sig = unsafe { libc::syscall(libc::SYS_pidfd_send_signal, fd as i32, libc::SIGTERM, std::ptr::null_mut::<libc::siginfo_t>(), 0) };
        let sig_err = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        println!("pidfd_open={pid} rc={fd} send_signal_rc={sig} send_signal_errno={sig_err}");
        if sig == 0 { std::process::exit(0); } else { std::process::exit(2); }
    }
    // T-6.1-011: process_vm_readv against a target outside this namespace — must be denied, never a partial read.
    if a.get(1).map(String::as_str) == Some("--vmread") {
        let pid: i32 = a.get(2).and_then(|x| x.parse().ok()).unwrap_or(1);
        let addr: usize = a.get(3).and_then(|x| usize::from_str_radix(x.trim_start_matches("0x"), 16).ok()).unwrap_or(0x400000);
        let mut buf = [0u8; 64];
        let local = libc::iovec { iov_base: buf.as_mut_ptr() as *mut libc::c_void, iov_len: buf.len() };
        let remote = libc::iovec { iov_base: addr as *mut libc::c_void, iov_len: buf.len() };
        let n = unsafe { libc::syscall(libc::SYS_process_vm_readv, pid, &local, 1usize, &remote, 1usize, 0usize) };
        let e = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        println!("process_vm_readv pid={pid} rc={n} errno={e}");
        if n > 0 { std::process::exit(0); } else { std::process::exit(3); }
    }
    // T-6.1-012: bind/connect an abstract AF_UNIX name. Abstract names are per network namespace, so a host name must be unreachable.
    if a.get(1).map(String::as_str) == Some("--abstract") {
        let (mode, name) = (a.get(2).cloned().unwrap_or_default(), a.get(3).cloned().unwrap_or_default());
        let mut sa: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        sa.sun_family = libc::AF_UNIX as u16;
        let b = name.as_bytes(); // sun_path[0] == 0 marks an abstract name
        for (i, c) in b.iter().enumerate() { sa.sun_path[i + 1] = *c as libc::c_char; }
        let len = (std::mem::size_of::<libc::sa_family_t>() + 1 + b.len()) as libc::socklen_t;
        let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0) };
        let rc = if mode == "connect" { unsafe { libc::connect(fd, &sa as *const _ as *const libc::sockaddr, len) } }
                 else { unsafe { libc::bind(fd, &sa as *const _ as *const libc::sockaddr, len) } };
        let e = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        println!("abstract {mode} name={name} rc={rc} errno={e}");
        if rc == 0 { std::process::exit(0); } else { std::process::exit(4); }
    }
    if a.get(1).map(String::as_str) == Some("--fds") { // T-6.3-003: enumerate inherited descriptors
        for e in std::fs::read_dir("/proc/self/fd").unwrap().flatten() { let n = e.file_name().to_string_lossy().to_string(); if let Ok(t) = std::fs::read_link(e.path()) { println!("{n} {}", t.display()); } }
        return;
    }
    if a.len() < 5 { eprintln!("usage"); std::process::exit(2); }
    let (fork, rights) = (a.iter().any(|x| x == "--fork"), a.iter().any(|x| x == "--scm-rights"));
    // T-6.4-008 credential cases, sent from *inside* the session (session uid, in-scope). `--creds <case>`:
    //   none    — send with no SCM_CREDENTIALS at all (SO_PASSCRED then makes the kernel synthesise the true one)
    //   two     — attach two SCM_CREDENTIALS cmsgs in one sendmsg
    //   forged  — attach one SCM_CREDENTIALS naming pid 1 / uid 0
    //   short   — attach a truncated ucred payload
    // Each case prints `case=<name> sendmsg_errno=<n>` so the driver can distinguish "the kernel refused the send" from
    // "the gateway refused the packet", and never has to infer one from the other.
    let creds_case = a.iter().position(|x| x == "--creds").and_then(|i| a.get(i + 1)).cloned();
    let hold = a.iter().any(|x| x == "--hold"); // keep the connection open after the first reply, send the next packet on stdin EOF (T-6.4-014)
    let sock_type = if a.iter().any(|x| x == "--stream") { libc::SOCK_STREAM } else if a.iter().any(|x| x == "--dgram") { libc::SOCK_DGRAM } else { libc::SOCK_SEQPACKET };
    let payload = a.get(5).filter(|p| !p.starts_with("--")).map(|p| std::fs::read(p).expect("payload")).unwrap_or_default();
    let sha = format!("sha256:{}", hex::encode(Sha256::digest(&payload)));
    let msg = format!("{{\"args\":{},\"operation\":\"{}\",\"operation_id\":\"{}\",\"payload_len\":{},\"payload_sha256\":\"{}\",\"v\":\"agentbound.gateway.v0.1\"}}", a[4], a[3], a[2], payload.len(), sha);
    let fd = unsafe { libc::socket(libc::AF_UNIX, sock_type | libc::SOCK_CLOEXEC, 0) };
    if fd < 0 { eprintln!("socket errno={}", std::io::Error::last_os_error()); std::process::exit(3); }
    let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() }; addr.sun_family = libc::AF_UNIX as u16;
    for (i, b) in a[1].bytes().enumerate() { addr.sun_path[i] = b as libc::c_char; }
    if unsafe { libc::connect(fd, &addr as *const _ as *const libc::sockaddr, std::mem::size_of::<libc::sockaddr_un>() as u32) } != 0 { eprintln!("connect errno={}", std::io::Error::last_os_error()); std::process::exit(4); }
    // T-6.4-009: the establishing process exits immediately, leaving a forked holder with the connected descriptor. The holder waits
    // for `--trigger <file>` to appear, then sends its packet. This lets the driver recycle the establishing PID (with privileges no
    // session has) and prove that a packet arriving on the connection afterwards is never accepted as the establishing instance.
    if let Some(i) = a.iter().position(|x| x == "--hold-fd") {
        let trigger = a.get(i + 1).cloned().unwrap_or_else(|| "/tmp/gw-trigger".into());
        let est_pid = unsafe { libc::getpid() };
        let pid = unsafe { libc::fork() };
        if pid > 0 { println!("establishing_pid={est_pid} holder_pid={pid}"); std::process::exit(0); } // establisher exits at once
        // holder: keep the inherited connected fd, wait for the trigger, then speak
        for _ in 0..600 { if std::path::Path::new(&trigger).exists() { break; } std::thread::sleep(std::time::Duration::from_millis(100)); }
    }
    if fork { // T-6.4-007: a child inherits the connected descriptor and speaks first
        let pid = unsafe { libc::fork() };
        if pid > 0 { let mut st = 0; unsafe { libc::waitpid(pid, &mut st, 0) }; std::process::exit(libc::WEXITSTATUS(st)); }
    }
    let send = |bytes: &[u8]| -> bool {
        if let Some(case) = creds_case.as_deref() {
            let me = unsafe { libc::ucred { pid: libc::getpid(), uid: libc::getuid(), gid: libc::getgid() } };
            let forged = libc::ucred { pid: 1, uid: 0, gid: 0 };
            let mut iov = libc::iovec { iov_base: bytes.as_ptr() as *mut _, iov_len: bytes.len() };
            let mut cbuf = [0u8; 128]; let mut m: libc::msghdr = unsafe { std::mem::zeroed() };
            m.msg_iov = &mut iov; m.msg_iovlen = 1;
            let ucred_len = std::mem::size_of::<libc::ucred>();
            let n_cmsg = match case { "none" => 0, "two" => 2, _ => 1 };
            if n_cmsg > 0 {
                m.msg_control = cbuf.as_mut_ptr() as *mut _;
                let each = unsafe { libc::CMSG_SPACE(ucred_len as u32) } as usize;
                m.msg_controllen = each * n_cmsg;
                unsafe {
                    let mut c = libc::CMSG_FIRSTHDR(&m);
                    for i in 0..n_cmsg {
                        let payload_len = if case == "short" { 3 } else { ucred_len };
                        (*c).cmsg_level = libc::SOL_SOCKET; (*c).cmsg_type = libc::SCM_CREDENTIALS; (*c).cmsg_len = libc::CMSG_LEN(payload_len as u32) as usize;
                        let src = if case == "forged" { &forged } else { &me };
                        std::ptr::copy_nonoverlapping(src as *const libc::ucred as *const u8, libc::CMSG_DATA(c), payload_len.min(ucred_len));
                        if i + 1 < n_cmsg { c = libc::CMSG_NXTHDR(&m, c); }
                    }
                }
            }
            let n = unsafe { libc::sendmsg(fd, &m, 0) };
            let e = if n < 0 { std::io::Error::last_os_error().raw_os_error().unwrap_or(0) } else { 0 };
            println!("case={case} cmsgs={n_cmsg} sendmsg_errno={e}");
            return n >= 0;
        }
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
    if hold { let mut sink = String::new(); let _ = std::io::Read::read_to_string(&mut std::io::stdin(), &mut sink); if !send(msg.as_bytes()) { eprintln!("send errno={}", std::io::Error::last_os_error()); std::process::exit(5); } let Some(r) = recv() else { eprintln!("closed by gateway"); std::process::exit(6) }; let _ = writeln!(out, "{r}"); std::process::exit(if r.contains("\"ok\":true") { 0 } else { 1 }); }
    let mut off = 0; let mut last = String::new();
    while off < payload.len() { let end = (off + (128 << 10)).min(payload.len()); if !send(&payload[off..end]) { std::process::exit(5); } let Some(r) = recv() else { eprintln!("closed by gateway"); std::process::exit(6) }; last = r; off = end; }
    if !last.is_empty() { let _ = writeln!(out, "{last}"); if !last.contains("\"ok\":true") { std::process::exit(1); } }
    let _ = fd.as_raw_fd();
}
