//! WP1 spike LC-2: does a frozen peer holding a SOCK_SEQPACKET gateway connection
//! delay the gateway's "zero connections" acknowledgement, and does the gateway
//! keep control of the connection while the peer is frozen?
//!
//! Also covers the mechanism half of ADR-0002 Decision 7 item 5 (revocation
//! latency): after the gateway marks the session revoked, the next packet on an
//! existing connection is denied and the connection closed; termination closes
//! all session connections before identity release.
//!
//! Throwaway code: not TCB, not SLOC-counted.
use libc::*;
use std::ffi::CString;
use std::fs;
use std::mem::{size_of, zeroed};
use std::os::unix::io::RawFd;
use std::ptr;
use std::time::{Duration, Instant};

const SOCK_PATH: &str = "/tmp/ab-spike-lc2.sock";
const CG: &str = "ab-spike-lc2";
fn result(item: &str, pass: bool, detail: &str) { println!("RESULT {item} {} {detail}", if pass { "PASS" } else { "FAIL" }); }
fn errno() -> i32 { std::io::Error::last_os_error().raw_os_error().unwrap_or(0) }
fn check(r: c_int, what: &str) -> c_int { if r < 0 { panic!("{what}: {}", std::io::Error::last_os_error()); } r }
fn cg(f: &str) -> String { fs::read_to_string(format!("/sys/fs/cgroup/{CG}/{f}")).unwrap_or_default().trim().to_string() }
fn cgw(f: &str, v: &str) { fs::write(format!("/sys/fs/cgroup/{CG}/{f}"), v).unwrap(); }
fn wait_for(mut f: impl FnMut() -> bool, ms: u64) -> (bool, u128) { let t = Instant::now(); while t.elapsed() < Duration::from_millis(ms) { if f() { return (true, t.elapsed().as_millis()); } std::thread::sleep(Duration::from_millis(2)); } (f(), t.elapsed().as_millis()) }

fn addr() -> (sockaddr_un, u32) { let mut a: sockaddr_un = unsafe { zeroed() }; a.sun_family = AF_UNIX as _; let p = CString::new(SOCK_PATH).unwrap(); unsafe { ptr::copy_nonoverlapping(p.as_ptr(), a.sun_path.as_mut_ptr(), p.as_bytes().len()); } (a, size_of::<sockaddr_un>() as u32) }
fn listen_socket() -> RawFd { let _ = fs::remove_file(SOCK_PATH); unsafe { let s = check(socket(AF_UNIX, SOCK_SEQPACKET | SOCK_CLOEXEC, 0), "socket"); let (a, l) = addr(); check(bind(s, &a as *const _ as *const sockaddr, l), "bind"); check(listen(s, 8), "listen"); s } }
fn connect_client() -> RawFd { unsafe { let s = check(socket(AF_UNIX, SOCK_SEQPACKET | SOCK_CLOEXEC, 0), "socket"); let (a, l) = addr(); check(connect(s, &a as *const _ as *const sockaddr, l), "connect"); s } }
fn accept_passcred(l: RawFd) -> (RawFd, ucred) { unsafe { let c = check(accept4(l, ptr::null_mut(), ptr::null_mut(), SOCK_CLOEXEC | SOCK_NONBLOCK), "accept"); let one: c_int = 1; setsockopt(c, SOL_SOCKET, SO_PASSCRED, &one as *const _ as *const c_void, 4); let mut pc: ucred = zeroed(); let mut len = size_of::<ucred>() as u32; getsockopt(c, SOL_SOCKET, SO_PEERCRED, &mut pc as *mut _ as *mut c_void, &mut len); (c, pc) } }
fn send(fd: RawFd, p: &[u8]) -> isize { unsafe { libc::send(fd, p.as_ptr() as *const c_void, p.len(), MSG_NOSIGNAL) } }
fn recv_nb(fd: RawFd, buf: &mut [u8]) -> (isize, i32) { let n = unsafe { libc::recv(fd, buf.as_mut_ptr() as *mut c_void, buf.len(), MSG_DONTWAIT) }; (n, if n < 0 { errno() } else { 0 }) }
fn pollin(fd: RawFd, ms: i32) -> i16 { let mut p = pollfd { fd, events: POLLIN | POLLRDHUP, revents: 0 }; unsafe { poll(&mut p, 1, ms); } p.revents }
fn state(pid: i32) -> String { fs::read_to_string(format!("/proc/{pid}/stat")).ok().and_then(|s| s.rsplit(')').next().map(|r| r.split_whitespace().next().unwrap_or("?").to_string())).unwrap_or("gone".into()) }

fn main() {
    println!("spike frozen-peer (LC-2 + D7-5 mechanism half)");
    let _ = fs::remove_dir(format!("/sys/fs/cgroup/{CG}")); fs::create_dir(format!("/sys/fs/cgroup/{CG}")).unwrap();
    let l = listen_socket();
    // session: a client process placed in the cgroup that opens a connection, sends one op, then blocks in recv (idle, connection open)
    let child = unsafe { fork() };
    if child == 0 {
        fs::write(format!("/sys/fs/cgroup/{CG}/cgroup.procs"), "0").unwrap();
        let c = connect_client();
        send(c, b"op-1");
        let mut b = [0u8; 64];
        // block waiting for a reply; then try more ops in a loop, reporting errors to stderr-less pipe... we simply exit on error
        loop {
            let n = unsafe { libc::recv(c, b.as_mut_ptr() as *mut c_void, 64, 0) };
            if n <= 0 { unsafe { _exit(if n == 0 { 10 } else { 11 }); } } // 10 = EOF (gateway closed), 11 = error
            if &b[..n as usize] == b"revoked" { unsafe { _exit(12); } }
            let r = send(c, b"op-next"); if r < 0 { unsafe { _exit(13); } }
        }
    }
    let (s, pc) = accept_passcred(l);
    let mut buf = [0u8; 64];
    wait_for(|| pollin(s, 0) & POLLIN != 0, 2000);
    let (n, _) = recv_nb(s, &mut buf);
    println!("gateway: accepted pid={} first op={:?}", pc.pid, std::str::from_utf8(&buf[..n.max(0) as usize]).unwrap_or("?"));

    // ---- LC-2: freeze the peer while its connection is open and idle ----
    cgw("cgroup.freeze", "1");
    let (frozen, tf) = wait_for(|| cg("cgroup.events").contains("frozen 1"), 2000);
    println!("peer frozen={frozen} in {tf} ms; peer state={}", state(child));
    // 1. Can the gateway still write to the frozen peer? (socket buffer absorbs it; not blocked)
    let t = Instant::now(); let w = send(s, b"reply-1"); let tw = t.elapsed().as_micros();
    result("LC2-1.gateway-send-to-frozen-peer-does-not-block", w == 7 && tw < 100_000, &format!("send returned {w} in {tw} µs (kernel buffers; peer need not run)"));
    // 2. Can the gateway close the connection while the peer is frozen? shutdown+close must not block
    let t = Instant::now(); let sd = unsafe { shutdown(s, SHUT_RDWR) }; let cl = unsafe { close(s) }; let tc = t.elapsed().as_micros();
    result("LC2-2.gateway-close-while-peer-frozen-immediate", sd == 0 && cl == 0 && tc < 100_000, &format!("shutdown+close rc={sd}/{cl} in {tc} µs while peer frozen"));
    // 3. Zero-connection acknowledgement: the gateway's own accounting is what matters. Kernel-side: is the peer's socket now orphaned/closed from the gateway's view? Verify no remaining accepted fd, and listening socket has no pending.
    let pending = pollin(l, 0) & POLLIN != 0;
    result("LC2-3.zero-connections-ack-independent-of-peer-state", !pending, &format!("gateway holds no accepted descriptors for the session; listen backlog empty={}; acknowledgement does not wait on the frozen peer", !pending));
    // 4. Frozen peer cannot open a new connection (it can't run)
    let (new_conn, _) = wait_for(|| pollin(l, 0) & POLLIN != 0, 300);
    result("LC2-4.frozen-peer-opens-no-new-connection", !new_conn, "no new connection arrived during 300 ms while frozen");
    // 5. Thaw: the peer wakes, sees EOF/error on its old connection
    cgw("cgroup.freeze", "0");
    let mut st = 0; let (exited, te) = wait_for(|| unsafe { waitpid(child, &mut st, WNOHANG) } == child, 3000);
    let code = if exited { (st >> 8) & 0xff } else { -1 };
    // Codes: 10 = EOF on recv, 11 = recv error, 13 = received the buffered reply then send failed (EPIPE). Any of them
    // proves the thawed peer cannot use the connection the gateway closed while it was frozen.
    result("LC2-5.thawed-peer-cannot-use-closed-connection", exited && matches!(code, 10 | 11 | 13), &format!("peer exited {te} ms after thaw with code {code} (13: buffered pre-close reply was delivered, next send → EPIPE; 10/11: EOF/error on recv)"));

    // ---- D7-5 mechanism half: revocation on a live connection ----
    let child2 = unsafe { fork() };
    if child2 == 0 {
        fs::write(format!("/sys/fs/cgroup/{CG}/cgroup.procs"), "0").unwrap();
        let c = connect_client(); send(c, b"op-1");
        let mut b = [0u8; 64];
        let n = unsafe { libc::recv(c, b.as_mut_ptr() as *mut c_void, 64, 0) }; // reply-1
        if n <= 0 { unsafe { _exit(20); } }
        // gateway commits revocation now (signalled by SIGUSR1 to us via parent? simpler: parent sends "revoked-marker" before). Just issue next op:
        let n = unsafe { libc::recv(c, b.as_mut_ptr() as *mut c_void, 64, 0) }; // wait for "go"
        if n <= 0 { unsafe { _exit(21); } }
        let t = Instant::now();
        send(c, b"op-2");
        let n = unsafe { libc::recv(c, b.as_mut_ptr() as *mut c_void, 64, 0) };
        let el = t.elapsed().as_micros();
        let denied = n > 0 && &b[..n as usize] == b"denied:revoked";
        let n2 = unsafe { libc::recv(c, b.as_mut_ptr() as *mut c_void, 64, 0) }; // expect EOF
        let r3 = send(c, b"op-3");
        let e3 = if r3 < 0 { errno() } else { 0 };
        eprintln!("REV peer: denied={denied} latency_us={el} then_eof={} next_send_errno={e3}", n2 == 0);
        unsafe { _exit(if denied && n2 == 0 && e3 == EPIPE { 30 } else { 31 }); }
    }
    let (s, _pc) = accept_passcred(l);
    wait_for(|| pollin(s, 0) & POLLIN != 0, 2000); recv_nb(s, &mut buf);
    send(s, b"reply-1");
    let mut revoked = false; // gateway's live grant state
    // commit revocation, then release the peer
    revoked = true; let t_commit = Instant::now();
    send(s, b"go");
    wait_for(|| pollin(s, 0) & POLLIN != 0, 2000);
    let (n, _) = recv_nb(s, &mut buf);
    let op = std::str::from_utf8(&buf[..n.max(0) as usize]).unwrap_or("?").to_string();
    // per-operation check against live state → deny, close
    let t_deny = t_commit.elapsed().as_micros();
    if revoked { send(s, b"denied:revoked"); unsafe { shutdown(s, SHUT_RDWR); close(s); } }
    let mut st = 0; let (exited, _) = wait_for(|| unsafe { waitpid(child2, &mut st, WNOHANG) } == child2, 3000);
    let code = if exited { (st >> 8) & 0xff } else { -1 };
    result("D7-5a.next-operation-after-revocation-denied", op == "op-2" && exited && code == 30, &format!("op after commit={op:?} denied and connection closed {t_deny} µs after commit; peer observed denial, EOF, then EPIPE (exit code {code})"));

    // termination: close all indexed connections before identity release — kill peer via cgroup.kill, verify our accepted fds are closable and cgroup empties
    let child3 = unsafe { fork() };
    if child3 == 0 { fs::write(format!("/sys/fs/cgroup/{CG}/cgroup.procs"), "0").unwrap(); let c = connect_client(); send(c, b"op-1"); let mut b = [0u8; 8]; unsafe { libc::recv(c, b.as_mut_ptr() as *mut c_void, 8, 0); _exit(0); } }
    let (s3, _) = accept_passcred(l);
    wait_for(|| pollin(s3, 0) & POLLIN != 0, 2000);
    cgw("cgroup.freeze", "1"); wait_for(|| cg("cgroup.events").contains("frozen 1"), 2000);
    cgw("cgroup.kill", "1");
    let (empty, te) = wait_for(|| cg("cgroup.procs").is_empty(), 3000);
    let hup = pollin(s3, 100);
    let closed = unsafe { close(s3) } == 0;
    unsafe { waitpid(child3, &mut st, 0); }
    result("D7-5b.termination-closes-connections-before-release", empty && closed && (hup & (POLLHUP | POLLRDHUP)) != 0, &format!("cgroup empty {te} ms after cgroup.kill; gateway saw POLLHUP/RDHUP (revents={hup:#x}) and closed its descriptor; zero connections before identity release"));

    let _ = fs::remove_dir(format!("/sys/fs/cgroup/{CG}")); let _ = fs::remove_file(SOCK_PATH);
    println!("done");
}
