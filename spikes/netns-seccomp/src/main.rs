//! WP1 spike: empty network namespace, abstract-socket isolation, socket-family
//! seccomp. ADR-0002 Decision 7 item 4; plan WP1 spike "socket-family seccomp and
//! abstract-socket isolation in an empty network namespace"; R-GW-2.
//!
//! Layout: the parent (host side) owns an abstract socket `@ab-host` and a
//! pathname gateway socket under a private dir. Two "sessions" are spawned,
//! each in its own new network + mount namespace. Session A binds `@ab-sibling`.
//! Session B then verifies that: host abstract and sibling abstract sockets are
//! unreachable (ECONNREFUSED); the pathname gateway socket bind-mounted into its
//! tree IS reachable; after installing the seccomp filter (TSYNC), socket() for
//! every family except AF_UNIX fails with EPERM, including from a second thread;
//! the netns has no non-loopback interfaces and lo is down.
//!
//! Throwaway code: not TCB, not SLOC-counted.
use libc::*;
use std::ffi::CString;
use std::mem::{size_of, zeroed};
use std::os::unix::io::RawFd;
use std::ptr;
use std::thread;

fn result(item: &str, pass: bool, detail: &str) { println!("RESULT {item} {} {detail}", if pass { "PASS" } else { "FAIL" }); }
fn errno() -> i32 { std::io::Error::last_os_error().raw_os_error().unwrap_or(0) }
fn check(r: c_int, what: &str) -> c_int { if r < 0 { panic!("{what}: {}", std::io::Error::last_os_error()); } r }

fn unix_addr(name: &[u8], abstract_: bool) -> (sockaddr_un, u32) {
    let mut a: sockaddr_un = unsafe { zeroed() }; a.sun_family = AF_UNIX as _;
    let off = if abstract_ { 1 } else { 0 };
    for (i, b) in name.iter().enumerate() { a.sun_path[off + i] = *b as c_char; }
    (a, (size_of::<sa_family_t>() + off + name.len()) as u32)
}
fn bind_listen(name: &[u8], abstract_: bool) -> RawFd {
    unsafe {
        let s = check(socket(AF_UNIX, SOCK_SEQPACKET | SOCK_CLOEXEC, 0), "socket");
        let (a, l) = unix_addr(name, abstract_);
        check(bind(s, &a as *const _ as *const sockaddr, l), &format!("bind {}", String::from_utf8_lossy(name)));
        check(listen(s, 4), "listen"); s
    }
}
/// connect; returns 0 on success or errno
fn try_connect(name: &[u8], abstract_: bool) -> i32 {
    unsafe {
        let s = socket(AF_UNIX, SOCK_SEQPACKET | SOCK_CLOEXEC, 0);
        if s < 0 { return errno(); }
        let (a, l) = unix_addr(name, abstract_);
        let r = connect(s, &a as *const _ as *const sockaddr, l);
        let e = if r < 0 { errno() } else { 0 }; close(s); e
    }
}
fn c(s: &str) -> CString { CString::new(s).unwrap() }

// --- seccomp: allow everything except socket(2) with family != AF_UNIX (EPERM) ---
const SECCOMP_SET_MODE_FILTER: c_uint = 1;
const SECCOMP_FILTER_FLAG_TSYNC: c_ulong = 1;
const SECCOMP_RET_ERRNO: u32 = 0x0005_0000;
const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;
const SECCOMP_RET_KILL_PROCESS: u32 = 0x8000_0000;
const AUDIT_ARCH_X86_64: u32 = 0xC000_003E;
#[repr(C)] struct Filt { code: u16, jt: u8, jf: u8, k: u32 }
const fn st(code: u16, k: u32) -> Filt { Filt { code, jt: 0, jf: 0, k } }
const fn jmp(code: u16, k: u32, jt: u8, jf: u8) -> Filt { Filt { code, jt, jf, k } }
const BPF_LD_W_ABS: u16 = 0x20; const BPF_JMP_JEQ_K: u16 = 0x15; const BPF_RET_K: u16 = 0x06;
fn install_seccomp() -> i32 {
    // seccomp_data: nr@0, arch@4, ip@8, args[0]@16 (low 32 bits)
    let prog = [
        st(BPF_LD_W_ABS, 4), jmp(BPF_JMP_JEQ_K, AUDIT_ARCH_X86_64, 1, 0), st(BPF_RET_K, SECCOMP_RET_KILL_PROCESS),
        st(BPF_LD_W_ABS, 0), jmp(BPF_JMP_JEQ_K, SYS_socket as u32, 0, 3),
        st(BPF_LD_W_ABS, 16), jmp(BPF_JMP_JEQ_K, AF_UNIX as u32, 1, 0), st(BPF_RET_K, SECCOMP_RET_ERRNO | EPERM as u32),
        st(BPF_RET_K, SECCOMP_RET_ALLOW),
    ];
    #[repr(C)] struct Prog { len: u16, filter: *const Filt }
    let p = Prog { len: prog.len() as u16, filter: prog.as_ptr() };
    unsafe {
        if prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 { return errno(); }
        let r = syscall(SYS_seccomp, SECCOMP_SET_MODE_FILTER, SECCOMP_FILTER_FLAG_TSYNC, &p as *const Prog);
        if r < 0 { errno() } else { 0 }
    }
}
fn socket_errno(family: c_int, ty: c_int) -> i32 { unsafe { let s = socket(family, ty, 0); if s < 0 { errno() } else { close(s); 0 } } }

/// Interface count and lo state via the netns itself (RTM_GETLINK dump), not /sys —
/// a stale /sys still shows the *host's* interfaces until sysfs is remounted in the new netns.
fn netns_state() -> (usize, bool) {
    unsafe {
        let s = socket(AF_NETLINK, SOCK_RAW | SOCK_CLOEXEC, NETLINK_ROUTE);
        if s < 0 { return (99, true); }
        #[repr(C)] struct Req { nh: nlmsghdr, ifi: ifinfomsg }
        let mut req: Req = zeroed();
        req.nh.nlmsg_len = size_of::<Req>() as u32; req.nh.nlmsg_type = RTM_GETLINK; req.nh.nlmsg_flags = (NLM_F_REQUEST | NLM_F_DUMP) as u16; req.nh.nlmsg_seq = 1;
        req.ifi.ifi_family = AF_UNSPEC as u8;
        send(s, &req as *const _ as *const c_void, size_of::<Req>(), 0);
        let mut buf = vec![0u8; 65536]; let (mut count, mut lo_up) = (0usize, false);
        'outer: loop {
            let n = recv(s, buf.as_mut_ptr() as *mut c_void, buf.len(), 0);
            if n <= 0 { break; }
            let mut off = 0usize;
            while off + size_of::<nlmsghdr>() <= n as usize {
                let h = &*(buf.as_ptr().add(off) as *const nlmsghdr);
                if h.nlmsg_type == NLMSG_DONE as u16 { break 'outer; }
                if h.nlmsg_type == RTM_NEWLINK {
                    let ifi = &*(buf.as_ptr().add(off + size_of::<nlmsghdr>()) as *const ifinfomsg);
                    count += 1; if ifi.ifi_type == ARPHRD_LOOPBACK && ifi.ifi_flags & IFF_UP as u32 != 0 { lo_up = true; }
                }
                off += ((h.nlmsg_len + 3) & !3) as usize;
            }
        }
        close(s); (count, lo_up)
    }
}
fn sys_class_net_count() -> usize { std::fs::read_dir("/sys/class/net").map(|d| d.count()).unwrap_or(99) }

fn session(role: &str, gw_dir: &str, sync_r: RawFd, sync_w: RawFd) -> ! {
    unsafe {
        check(unshare(CLONE_NEWNET | CLONE_NEWNS | CLONE_NEWUTS), "unshare");
        check(mount(ptr::null(), c("/").as_ptr(), ptr::null(), MS_REC | MS_PRIVATE, ptr::null()), "make-rprivate");
        // Build a minimal root: tmpfs newroot containing only the gateway-socket projection, then pivot_root.
        let newroot = "/run/ab-newroot"; std::fs::create_dir_all(newroot).unwrap();
        check(mount(c("tmpfs").as_ptr(), c(newroot).as_ptr(), c("tmpfs").as_ptr(), MS_NOSUID | MS_NODEV, c("size=1m").as_ptr() as *const c_void), "tmpfs newroot");
        let gw_src = format!("{gw_dir}/gateway.sock");
        let gw_dst = format!("{newroot}/gateway.sock");
        std::fs::File::create(&gw_dst).unwrap();
        check(mount(c(&gw_src).as_ptr(), c(&gw_dst).as_ptr(), ptr::null(), MS_BIND, ptr::null()), "bind gateway.sock");
        check(mount(ptr::null(), c(&gw_dst).as_ptr(), ptr::null(), MS_BIND | MS_REMOUNT | MS_RDONLY | MS_NOSUID | MS_NODEV | MS_NOEXEC, ptr::null()), "remount ro");
        // stale /sys view from the old root, captured before pivot (illustrates the sysfs finding)
        let stale_sys = sys_class_net_count();
        std::fs::create_dir_all(format!("{newroot}/oldroot")).unwrap();
        check(chdir(c(newroot).as_ptr()), "chdir newroot");
        check(syscall(SYS_pivot_root, c(".").as_ptr(), c("oldroot").as_ptr()) as c_int, "pivot_root");
        check(chdir(c("/").as_ptr()), "chdir /");
        check(umount2(c("/oldroot").as_ptr(), MNT_DETACH), "umount oldroot");
        std::fs::remove_dir("/oldroot").ok();
        let gw_dst = "/gateway.sock".to_string();
        if role == "B" {
            result("NS-0.stale-sysfs-shows-host-interfaces", stale_sys == 2, &format!("/sys/class/net before remount listed {stale_sys} interfaces from the HOST netns (FINDING: constructor must mount a fresh sysfs or none; procfs/sysfs from the parent leak host network topology)"));
        }
        if role == "A" {
            let _l = bind_listen(b"ab-sibling", true);
            write(sync_w, b"A".as_ptr() as *const c_void, 1);
            let mut b = [0u8; 1]; read(sync_r, b.as_mut_ptr() as *mut c_void, 1); // wait for parent to finish
            _exit(0);
        }
        // role B: the session under test
        let (ifn, lo_up) = netns_state();
        result("NS-1.empty-netns", ifn == 1 && !lo_up, &format!("RTM_GETLINK: interfaces={ifn} (lo only) lo_up={lo_up}"));
        let e_host = try_connect(b"ab-host", true);
        let e_sib = try_connect(b"ab-sibling", true);
        result("D7-4a.host-abstract-unreachable", e_host == ECONNREFUSED, &format!("connect(@ab-host) errno={e_host} (ECONNREFUSED={ECONNREFUSED})"));
        result("D7-4b.sibling-abstract-unreachable", e_sib == ECONNREFUSED, &format!("connect(@ab-sibling) errno={e_sib}"));
        // bind our own abstract with the same name the host uses: succeeds → separate namespace
        let own = socket(AF_UNIX, SOCK_SEQPACKET, 0); let (a, l) = unix_addr(b"ab-host", true);
        let own_bind = bind(own, &a as *const _ as *const sockaddr, l);
        result("D7-4c.abstract-namespace-is-per-netns", own_bind == 0, &format!("bind(@ab-host) inside session rc={own_bind} (name free in this netns although host holds it)"));
        // host pathname socket via the original path is not visible; via the projection it is
        let e_gw_orig = try_connect(gw_src.as_bytes(), false);
        let e_gw_proj = try_connect(gw_dst.as_bytes(), false);
        result("NS-2.gateway-socket-only-via-projection", e_gw_proj == 0 && e_gw_orig == ENOENT, &format!("after pivot_root: connect(/gateway.sock)={e_gw_proj} connect(original host path)={e_gw_orig} (ENOENT={ENOENT})"));
        // pathname sockets travel with the filesystem, not the netns: a pathname socket connects across netns
        result("NS-3.pathname-crosses-netns-by-design", e_gw_proj == 0, "pathname Unix socket reachable across netns; isolation of pathname sockets is the mount namespace's job");

        // seccomp
        let se = install_seccomp();
        result("SC-1.seccomp-tsync-installed", se == 0, &format!("seccomp(SET_MODE_FILTER, TSYNC) errno={se}; no_new_privs=1"));
        let fams = [(AF_INET, "AF_INET"), (AF_INET6, "AF_INET6"), (AF_NETLINK, "AF_NETLINK"), (AF_PACKET, "AF_PACKET"), (AF_VSOCK, "AF_VSOCK"), (AF_BLUETOOTH, "AF_BLUETOOTH"), (AF_ALG, "AF_ALG")];
        let mut all = true; let mut det = String::new();
        for (f, n) in fams { let e = socket_errno(f, SOCK_DGRAM); if e != EPERM && !(f == AF_INET && false) { all = false; } det.push_str(&format!("{n}={e} ")); }
        result("SC-2.non-unix-families-EPERM", all, &det.trim());
        result("SC-3.af_unix-allowed", socket_errno(AF_UNIX, SOCK_SEQPACKET) == 0 && socket_errno(AF_UNIX, SOCK_STREAM) == 0, "AF_UNIX seqpacket+stream still permitted");
        // TSYNC: a thread created *before* the filter must also be filtered. Spawn thread first? We installed already; test a thread created before install by re-doing: spawn thread, then install second identical filter.
        let (tx, rx) = std::sync::mpsc::channel();
        let (go_tx, go_rx) = std::sync::mpsc::channel::<()>();
        let t = thread::spawn(move || { go_rx.recv().unwrap(); tx.send(socket_errno(AF_INET, SOCK_STREAM)).unwrap(); });
        // the thread already inherited the filter (installed before spawn); additionally install again with TSYNC to prove multi-thread install works
        let se2 = install_seccomp();
        go_tx.send(()).unwrap(); let te = rx.recv().unwrap(); t.join().unwrap();
        result("SC-4.filter-applies-across-threads", te == EPERM && se2 == 0, &format!("thread socket(AF_INET) errno={te}; second TSYNC install with a live thread rc={se2}"));
        // AF_INET via socketpair? socketpair is a different syscall — check the bypass
        let mut sv = [0; 2]; let sp = socketpair(AF_INET, SOCK_STREAM, 0, sv.as_mut_ptr()); let spe = if sp < 0 { errno() } else { 0 };
        result("SC-5.socketpair-not-a-bypass", spe != 0, &format!("socketpair(AF_INET) errno={spe} (kernel rejects: AF_INET has no socketpair; AF_UNIX socketpair remains available and is fine — it is netns-less and cannot reach anything)"));
        write(sync_w, b"B".as_ptr() as *const c_void, 1);
        _exit(0);
    }
}

fn main() {
    println!("spike netns-seccomp; uid={}", unsafe { getuid() });
    let gw_dir = "/tmp/ab-spike-gw"; let _ = std::fs::remove_dir_all(gw_dir); std::fs::create_dir_all(gw_dir).unwrap();
    let _host_abs = bind_listen(b"ab-host", true);
    let _gw = bind_listen(format!("{gw_dir}/gateway.sock").as_bytes(), false);
    let (host_iface, _) = netns_state();
    println!("host: interfaces={host_iface}, @ab-host bound, {gw_dir}/gateway.sock bound");
    unsafe {
        let mut p1 = [0; 2]; let mut p2 = [0; 2]; pipe(p1.as_mut_ptr()); pipe(p2.as_mut_ptr());
        let a = fork(); if a == 0 { session("A", gw_dir, p2[0], p1[1]); }
        let mut b = [0u8; 1]; read(p1[0], b.as_mut_ptr() as *mut c_void, 1); // A ready
        let bpid = fork(); if bpid == 0 { session("B", gw_dir, p2[0], p1[1]); }
        read(p1[0], b.as_mut_ptr() as *mut c_void, 1); // B done
        write(p2[1], b"x".as_ptr() as *const c_void, 1);
        let mut st = 0; waitpid(a, &mut st, 0); waitpid(bpid, &mut st, 0);
        // host still fine
        result("NS-4.host-unaffected", try_connect(b"ab-host", true) == 0 && std::fs::metadata("/run/ab-newroot/gateway.sock").is_err(), "host reaches @ab-host; session mounts did not propagate");
    }
    let _ = std::fs::remove_dir_all(gw_dir);
    println!("done");
}
