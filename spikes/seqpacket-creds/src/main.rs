//! WP1 spike: ADR-0002 Decision 7 items 1–3 on the pinned baseline.
//!
//!  1. `SOCK_SEQPACKET` + `SO_PASSCRED`: exactly one `SCM_CREDENTIALS` per
//!     `recvmsg` for one `sendmsg`; oversize packets truncate, never split.
//!  2. pidfd from credential PID: `pidfd_open` on the live peer; start time and
//!     PID namespace readable via the pidfd; recycled PID detected.
//!  3. Descriptor transfer: `SCM_RIGHTS` detected and rejected; a connection
//!     used by a forked child (inherited descriptor) fails the establishing-PID
//!     check on its first packet and the connection is closed.
//!
//! Extra: an unprivileged sender cannot forge the PID in `SCM_CREDENTIALS`.
//!
//! Throwaway code: not TCB, not SLOC-counted.
use libc::*;
use std::ffi::CString;
use std::fs;
use std::io::Write;
use std::mem::{size_of, zeroed};
use std::os::unix::io::RawFd;
use std::ptr;

const SOCK_PATH: &str = "/tmp/ab-spike-gw.sock";
const PIDFD_GET_PID_NAMESPACE: c_ulong = 0xFF05; // _IO(PIDFS_IOCTL_MAGIC=0xFF, 5), Linux ≥ 6.11

fn result(item: &str, pass: bool, detail: &str) {
    println!("RESULT {item} {} {detail}", if pass { "PASS" } else { "FAIL" });
}
fn errno() -> i32 { std::io::Error::last_os_error().raw_os_error().unwrap_or(0) }
fn check(r: c_int, what: &str) -> c_int { if r < 0 { panic!("{what}: {}", std::io::Error::last_os_error()); } r }

#[derive(Debug, Clone)]
struct Cmsg { level: c_int, ty: c_int, data: Vec<u8> }
struct Rx { n: isize, flags: c_int, cmsgs: Vec<Cmsg> }

fn recvmsg_all(fd: RawFd, buf: &mut [u8]) -> Rx {
    unsafe {
        let mut iov = iovec { iov_base: buf.as_mut_ptr() as *mut c_void, iov_len: buf.len() };
        let mut ctl = [0u8; 512];
        let mut msg: msghdr = zeroed();
        msg.msg_iov = &mut iov; msg.msg_iovlen = 1;
        msg.msg_control = ctl.as_mut_ptr() as *mut c_void; msg.msg_controllen = ctl.len();
        let n = recvmsg(fd, &mut msg, 0);
        let mut cmsgs = vec![];
        if n >= 0 {
            let mut c = CMSG_FIRSTHDR(&msg);
            while !c.is_null() {
                let h = &*c;
                let dlen = h.cmsg_len as usize - CMSG_LEN(0) as usize;
                let data = std::slice::from_raw_parts(CMSG_DATA(c), dlen).to_vec();
                cmsgs.push(Cmsg { level: h.cmsg_level, ty: h.cmsg_type, data });
                c = CMSG_NXTHDR(&msg, c);
            }
        }
        Rx { n, flags: msg.msg_flags, cmsgs }
    }
}
fn creds_of(c: &Cmsg) -> ucred { unsafe { ptr::read_unaligned(c.data.as_ptr() as *const ucred) } }

fn sendmsg_plain(fd: RawFd, payload: &[u8]) -> isize {
    unsafe {
        let mut iov = iovec { iov_base: payload.as_ptr() as *mut c_void, iov_len: payload.len() };
        let mut msg: msghdr = zeroed(); msg.msg_iov = &mut iov; msg.msg_iovlen = 1;
        sendmsg(fd, &msg, MSG_NOSIGNAL)
    }
}
fn sendmsg_cmsg(fd: RawFd, payload: &[u8], level: c_int, ty: c_int, data: &[u8]) -> isize {
    unsafe {
        let mut iov = iovec { iov_base: payload.as_ptr() as *mut c_void, iov_len: payload.len() };
        let space = CMSG_SPACE(data.len() as u32) as usize;
        let mut ctl = vec![0u8; space];
        let mut msg: msghdr = zeroed(); msg.msg_iov = &mut iov; msg.msg_iovlen = 1;
        msg.msg_control = ctl.as_mut_ptr() as *mut c_void; msg.msg_controllen = space;
        let c = CMSG_FIRSTHDR(&msg);
        (*c).cmsg_level = level; (*c).cmsg_type = ty; (*c).cmsg_len = CMSG_LEN(data.len() as u32) as _;
        ptr::copy_nonoverlapping(data.as_ptr(), CMSG_DATA(c), data.len());
        sendmsg(fd, &msg, MSG_NOSIGNAL)
    }
}

fn listen_socket() -> RawFd {
    let _ = fs::remove_file(SOCK_PATH);
    unsafe {
        let s = check(socket(AF_UNIX, SOCK_SEQPACKET | SOCK_CLOEXEC, 0), "socket");
        let mut addr: sockaddr_un = zeroed(); addr.sun_family = AF_UNIX as _;
        let p = CString::new(SOCK_PATH).unwrap();
        ptr::copy_nonoverlapping(p.as_ptr(), addr.sun_path.as_mut_ptr(), p.as_bytes().len());
        check(bind(s, &addr as *const _ as *const sockaddr, size_of::<sockaddr_un>() as u32), "bind");
        check(listen(s, 8), "listen");
        s
    }
}
fn accept_with_passcred(l: RawFd) -> (RawFd, ucred) {
    unsafe {
        let c = check(accept4(l, ptr::null_mut(), ptr::null_mut(), SOCK_CLOEXEC), "accept");
        let one: c_int = 1;
        check(setsockopt(c, SOL_SOCKET, SO_PASSCRED, &one as *const _ as *const c_void, 4), "SO_PASSCRED");
        let mut pc: ucred = zeroed(); let mut len = size_of::<ucred>() as u32;
        check(getsockopt(c, SOL_SOCKET, SO_PEERCRED, &mut pc as *mut _ as *mut c_void, &mut len), "SO_PEERCRED");
        (c, pc)
    }
}
fn connect_client() -> RawFd {
    unsafe {
        let s = check(socket(AF_UNIX, SOCK_SEQPACKET | SOCK_CLOEXEC, 0), "socket");
        let mut addr: sockaddr_un = zeroed(); addr.sun_family = AF_UNIX as _;
        let p = CString::new(SOCK_PATH).unwrap();
        ptr::copy_nonoverlapping(p.as_ptr(), addr.sun_path.as_mut_ptr(), p.as_bytes().len());
        check(connect(s, &addr as *const _ as *const sockaddr, size_of::<sockaddr_un>() as u32), "connect");
        s
    }
}

fn pidfd_open(pid: pid_t) -> c_int { unsafe { syscall(SYS_pidfd_open, pid, 0) as c_int } }
fn pidfd_alive(pidfd: RawFd) -> bool {
    // POLLIN on a pidfd means the process has exited.
    let mut p = pollfd { fd: pidfd, events: POLLIN, revents: 0 };
    unsafe { poll(&mut p, 1, 0) == 0 }
}
fn pidfd_ino(pidfd: RawFd) -> u64 { unsafe { let mut st: stat = zeroed(); check(fstat(pidfd, &mut st), "fstat pidfd"); st.st_ino } }
fn proc_starttime(pid: pid_t) -> Option<u64> {
    let s = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after = &s[s.rfind(')')? + 2..];
    after.split_whitespace().nth(19)?.parse().ok() // field 22 overall
}
fn pidfd_pidns_ino(pidfd: RawFd) -> Option<u64> {
    unsafe {
        let nsfd = ioctl(pidfd, PIDFD_GET_PID_NAMESPACE as _, 0usize); // pidfs requires arg == 0
        if nsfd < 0 { return None; }
        let mut st: stat = zeroed(); let r = fstat(nsfd, &mut st); close(nsfd);
        if r < 0 { None } else { Some(st.st_ino) }
    }
}
fn proc_pidns_ino(pid: pid_t) -> Option<u64> { fs::metadata(format!("/proc/{pid}/ns/pid")).ok().map(|m| std::os::unix::fs::MetadataExt::ino(&m)) }

/// Race-free peer binding: pidfd first, read /proc state, then confirm the pidfd is still live.
struct Peer { pid: pid_t, pidfd: RawFd, ino: u64, starttime: u64, pidns: u64 }
fn bind_peer(pid: pid_t) -> Result<Peer, String> {
    let pidfd = pidfd_open(pid);
    if pidfd < 0 { return Err(format!("pidfd_open errno {}", errno())); }
    let starttime = proc_starttime(pid).ok_or("no /proc stat")?;
    let pidns = pidfd_pidns_ino(pidfd).ok_or_else(|| format!("PIDFD_GET_PID_NAMESPACE errno {}", errno()))?;
    if !pidfd_alive(pidfd) { return Err("peer exited during binding".into()); }
    Ok(Peer { pid, pidfd, ino: pidfd_ino(pidfd), starttime, pidns })
}

fn wait_child(pid: pid_t) -> i32 { let mut st = 0; unsafe { waitpid(pid, &mut st, 0) }; st }

fn main() {
    println!("spike seqpacket-creds; uid={} pid={}", unsafe { getuid() }, unsafe { getpid() });
    let l = listen_socket();

    // ---------- Item 1: one SCM_CREDENTIALS per packet; truncation ----------
    let client_pid = unsafe { fork() };
    if client_pid == 0 {
        let c = connect_client();
        let _ = sendmsg_plain(c, b"packet-1");
        let _ = sendmsg_plain(c, b"packet-2");
        let big = vec![0xABu8; 8192];
        let _ = sendmsg_plain(c, &big);           // oversize relative to the server's 1024 buffer
        let _ = sendmsg_plain(c, b"after-big");
        // hold the connection until told to exit
        let mut b = [0u8; 8]; unsafe { read(c, b.as_mut_ptr() as *mut c_void, 8); _exit(0); }
    }
    let (s, pc) = accept_with_passcred(l);
    println!("SO_PEERCRED at accept: pid={} uid={} gid={}", pc.pid, pc.uid, pc.gid);
    let mut buf = [0u8; 1024];
    let r1 = recvmsg_all(s, &mut buf);
    let creds1: Vec<_> = r1.cmsgs.iter().filter(|c| c.level == SOL_SOCKET && c.ty == SCM_CREDENTIALS).collect();
    println!("recv#1 n={} cmsgs={} scm_credentials={} payload={:?}", r1.n, r1.cmsgs.len(), creds1.len(), std::str::from_utf8(&buf[..r1.n.max(0) as usize]).unwrap_or("?"));
    let c1 = creds1.first().map(|c| creds_of(c));
    result("D7-1a.one-credential-per-packet", r1.n == 8 && creds1.len() == 1 && r1.cmsgs.len() == 1,
        &format!("cmsg_count={} scm_credentials={} pid_matches_peercred={}", r1.cmsgs.len(), creds1.len(), c1.map(|c| c.pid == pc.pid).unwrap_or(false)));
    let r2 = recvmsg_all(s, &mut buf);
    let creds2 = r2.cmsgs.iter().filter(|c| c.level == SOL_SOCKET && c.ty == SCM_CREDENTIALS).count();
    result("D7-1b.second-packet-independent", r2.n == 8 && &buf[..8] == b"packet-2" && creds2 == 1,
        &format!("n={} payload={:?} scm_credentials={}", r2.n, std::str::from_utf8(&buf[..8]).unwrap_or("?"), creds2));
    let r3 = recvmsg_all(s, &mut buf);
    let trunc = r3.flags & MSG_TRUNC != 0;
    println!("recv#3 (oversize) n={} MSG_TRUNC={} all_AB={}", r3.n, trunc, buf.iter().all(|&b| b == 0xAB));
    let r4 = recvmsg_all(s, &mut buf);
    let r4_payload = std::str::from_utf8(&buf[..r4.n.max(0) as usize]).unwrap_or("?").to_string();
    println!("recv#4 n={} payload={:?}", r4.n, r4_payload);
    result("D7-1c.oversize-truncates-not-splits", trunc && r3.n == 1024 && r4_payload == "after-big",
        &format!("MSG_TRUNC={} delivered={} next_packet={:?} (remainder discarded, boundary preserved)", trunc, r3.n, r4_payload));

    // ---------- Item 2: pidfd from credential PID ----------
    let cred_pid = c1.map(|c| c.pid).unwrap_or(-1);
    match bind_peer(cred_pid) {
        Ok(p) => {
            let proc_ns = proc_pidns_ino(p.pid);
            println!("peer pid={} pidfd={} pidfs_ino={} starttime={} pidns_ino(pidfd)={} pidns_ino(proc)={:?}", p.pid, p.pidfd, p.ino, p.starttime, p.pidns, proc_ns);
            result("D7-2a.pidfd_open-live-peer", true, &format!("pidfd={} pidfs_inode={}", p.pidfd, p.ino));
            result("D7-2b.starttime-and-pidns-via-pidfd", proc_ns == Some(p.pidns),
                &format!("starttime={} pidns_via_ioctl={} pidns_via_proc={:?} (PIDFD_GET_PID_NAMESPACE 6.11+; start time via /proc then pidfd-liveness recheck)", p.starttime, p.pidns, proc_ns));

            // PID reuse: let the peer exit, then force the same PID onto a new process.
            unsafe { write(s, b"bye\n".as_ptr() as *const c_void, 4); }
            wait_child(client_pid);
            let exited = !pidfd_alive(p.pidfd);
            let mut reused = false; let mut new_ino = 0; let mut new_start = 0;
            if let Ok(mut f) = fs::OpenOptions::new().write(true).open("/proc/sys/kernel/ns_last_pid") {
                let _ = write!(f, "{}", p.pid - 1);
                drop(f);
                let np = unsafe { fork() };
                if np == 0 { unsafe { sleep(2); _exit(0); } }
                reused = np == p.pid;
                if reused {
                    let nf = pidfd_open(np); new_ino = pidfd_ino(nf); new_start = proc_starttime(np).unwrap_or(0);
                    unsafe { close(nf); }
                }
                unsafe { kill(np, SIGKILL); } wait_child(np);
                println!("reuse: forced ns_last_pid; new pid={} reused={} new_pidfs_ino={} new_starttime={}", np, reused, new_ino, new_start);
            }
            let sig = unsafe { syscall(SYS_pidfd_send_signal, p.pidfd, 0, 0usize, 0u32) };
            let sig_err = if sig < 0 { errno() } else { 0 };
            result("D7-2c.recycled-pid-detected-via-pidfd", exited && sig_err == ESRCH && (!reused || new_ino != p.ino),
                &format!("old_pidfd_exited={} pidfd_send_signal_errno={} pid_recycled={} pidfs_inode_old={} new={} (the held pidfd and its pidfs inode identify the process instance)", exited, sig_err, reused, p.ino, new_ino));
            // Finding: /proc start time has clock-tick granularity (CONFIG_HZ, 10 ms here); a PID recycled within
            // one tick has an identical start time, so start time alone cannot detect reuse.
            result("D7-2d.starttime-alone-detects-reuse", reused && new_start != p.starttime,
                &format!("pid_recycled={} old_starttime={} new_starttime={} CLK_TCK={} (FINDING: start time is not a sufficient reuse check; the pidfs inode is)", reused, p.starttime, new_start, unsafe { sysconf(_SC_CLK_TCK) }));
            unsafe { close(p.pidfd); }
        }
        Err(e) => { result("D7-2a.pidfd_open-live-peer", false, &e); result("D7-2b.starttime-and-pidns-via-pidfd", false, &e); result("D7-2c.recycled-pid-detected", false, &e); }
    }
    unsafe { close(s); }

    // ---------- Item 3a: SCM_RIGHTS rejected ----------
    let cp = unsafe { fork() };
    if cp == 0 {
        let c = connect_client();
        let fd_to_pass = unsafe { open(b"/etc/hostname\0".as_ptr() as *const c_char, O_RDONLY) };
        let _ = sendmsg_cmsg(c, b"with-rights", SOL_SOCKET, SCM_RIGHTS, &fd_to_pass.to_ne_bytes());
        let mut b = [0u8; 8]; unsafe { read(c, b.as_mut_ptr() as *mut c_void, 8); _exit(0); }
    }
    let (s, pc) = accept_with_passcred(l);
    let r = recvmsg_all(s, &mut buf);
    let rights: Vec<_> = r.cmsgs.iter().filter(|c| c.level == SOL_SOCKET && c.ty == SCM_RIGHTS).collect();
    let creds: Vec<_> = r.cmsgs.iter().filter(|c| c.level == SOL_SOCKET && c.ty == SCM_CREDENTIALS).collect();
    println!("SCM_RIGHTS packet: n={} cmsgs={} rights={} creds={}", r.n, r.cmsgs.len(), rights.len(), creds.len());
    // Gateway policy: any SCM_RIGHTS → close received fds, deny, close connection.
    for c in &rights { for chunk in c.data.chunks_exact(4) { unsafe { close(i32::from_ne_bytes(chunk.try_into().unwrap())); } } }
    result("D7-3a.scm_rights-detectable-and-rejected", rights.len() == 1 && creds.len() == 1,
        &format!("SCM_RIGHTS cmsgs={} (fds closed, connection closed) SCM_CREDENTIALS still present={}", rights.len(), creds.len() == 1));
    unsafe { close(s); } wait_child(cp);
    let _ = pc;

    // ---------- Item 3b: inherited descriptor fails establishing-PID check ----------
    let (rp, wp) = unsafe { let mut p = [0; 2]; pipe(p.as_mut_ptr()); (p[0], p[1]) };
    let cp = unsafe { fork() };
    if cp == 0 {
        unsafe { close(rp); }
        let c = connect_client();
        let _ = sendmsg_plain(c, b"from-parent");
        let gc = unsafe { fork() };
        if gc == 0 {
            // child inherits the connected descriptor and uses it
            let n = sendmsg_plain(c, b"from-child");
            let e1 = if n < 0 { errno() } else { 0 };
            unsafe { usleep(300_000); }
            let n2 = sendmsg_plain(c, b"from-child-2");
            let e2 = if n2 < 0 { errno() } else { 0 };
            let msg = format!("{} {} {} {}\n", n, e1, n2, e2);
            unsafe { write(wp, msg.as_ptr() as *const c_void, msg.len()); _exit(0); }
        }
        wait_child(gc);
        unsafe { _exit(0); }
    }
    unsafe { close(wp); }
    let (s, pc) = accept_with_passcred(l);
    let est_pid = pc.pid;
    let r = recvmsg_all(s, &mut buf);
    let p1 = r.cmsgs.iter().find(|c| c.ty == SCM_CREDENTIALS).map(|c| creds_of(c).pid).unwrap_or(-1);
    let r = recvmsg_all(s, &mut buf);
    let p2 = r.cmsgs.iter().find(|c| c.ty == SCM_CREDENTIALS).map(|c| creds_of(c).pid).unwrap_or(-1);
    let mismatch = p2 != est_pid;
    println!("establishing pid={} packet1 pid={} packet2 pid={} payload2={:?}", est_pid, p1, p2, std::str::from_utf8(&buf[..r.n.max(0) as usize]).unwrap_or("?"));
    if mismatch { unsafe { close(s); } }  // gateway.process_mismatch → close
    let mut rep = String::new();
    unsafe { let mut b = [0u8; 64]; let n = read(rp, b.as_mut_ptr() as *mut c_void, 64); if n > 0 { rep = String::from_utf8_lossy(&b[..n as usize]).trim().to_string(); } }
    wait_child(cp);
    let parts: Vec<i64> = rep.split_whitespace().filter_map(|x| x.parse().ok()).collect();
    let child_second_send_failed = parts.len() == 4 && parts[2] < 0 && (parts[3] == EPIPE as i64 || parts[3] == ECONNRESET as i64);
    result("D7-3b.inherited-descriptor-fails-pid-check", p1 == est_pid && mismatch && child_second_send_failed,
        &format!("establishing_pid={} inherited_packet_pid={} mismatch_detected={} child_next_send_errno={} (kernel-supplied PID; no protocol cooperation needed)", est_pid, p2, mismatch, parts.get(3).copied().unwrap_or(-1)));

    // ---------- Extra: unprivileged sender cannot forge SCM_CREDENTIALS pid ----------
    let cp = unsafe { fork() };
    if cp == 0 {
        let c = connect_client();
        unsafe { check(setgid(65534), "setgid"); check(setuid(65534), "setuid"); }
        let fake = ucred { pid: 1, uid: 0, gid: 0 };
        let data = unsafe { std::slice::from_raw_parts(&fake as *const _ as *const u8, size_of::<ucred>()) };
        let n = sendmsg_cmsg(c, b"forged", SOL_SOCKET, SCM_CREDENTIALS, data);
        let e = if n < 0 { errno() } else { 0 };
        let _ = sendmsg_plain(c, format!("forge:{n}:{e}").as_bytes());
        unsafe { _exit(0); }
    }
    let (s, pc) = accept_with_passcred(l);
    let r = recvmsg_all(s, &mut buf);
    let payload = std::str::from_utf8(&buf[..r.n.max(0) as usize]).unwrap_or("?").to_string();
    let cred = r.cmsgs.iter().find(|c| c.ty == SCM_CREDENTIALS).map(|c| creds_of(c));
    println!("forge attempt: first packet payload={:?} cred={:?}", payload, cred.map(|c| (c.pid, c.uid, c.gid)));
    let forged_rejected = payload.starts_with("forge:-1:") && payload.ends_with(&format!(":{}", EPERM));
    result("X-1.unprivileged-cannot-forge-credentials", forged_rejected && cred.map(|c| c.pid == pc.pid && c.uid == 65534).unwrap_or(false),
        &format!("forged sendmsg result={payload} (expect -1:EPERM={}), genuine cred pid={:?} uid={:?}", EPERM, cred.map(|c| c.pid), cred.map(|c| c.uid)));
    unsafe { close(s); } wait_child(cp);
    let _ = fs::remove_file(SOCK_PATH);
    println!("done");
}
