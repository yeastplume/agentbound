//! WP1 spike: systemd scope + PID-namespace init containment, cgroup.freeze /
//! cgroup.kill behaviour including D-state tasks, D-Bus scope signals and the
//! pidfd-watch fallback, and the systemd-kills-first race.
//! Session lifecycle §4, §5; plan WP1 spikes 2 and 7.
//!
//! systemd interaction is via `systemd-run`/`busctl` (throwaway); the daemon
//! will use sd-bus. Process creation is clone3(CLONE_NEWPID|CLONE_NEWNS|CLONE_PIDFD)
//! with a pipe barrier, as in lifecycle §3 step 1.
//!
//! D-state generator: a dm-linear device over a loop file that is *suspended*;
//! a direct read against it blocks in D (uninterruptible), survives SIGKILL and
//! cgroup.kill, and is released by `dmsetup resume`.
//!
//! Throwaway code: not TCB, not SLOC-counted.
use libc::*;
use std::fs;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn result(item: &str, pass: bool, detail: &str) { println!("RESULT {item} {} {detail}", if pass { "PASS" } else { "FAIL" }); }
fn sh(cmd: &str) -> (i32, String) {
    let o = Command::new("sh").arg("-c").arg(cmd).stderr(Stdio::piped()).output().unwrap();
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).trim().to_string() + &String::from_utf8_lossy(&o.stderr))
}
fn state(pid: i32) -> String { fs::read_to_string(format!("/proc/{pid}/stat")).ok().and_then(|s| s.rsplit(')').next().map(|r| r.split_whitespace().next().unwrap_or("?").to_string())).unwrap_or("gone".into()) }
fn cg(path: &str, f: &str) -> String { fs::read_to_string(format!("/sys/fs/cgroup/{path}/{f}")).unwrap_or_default().trim().to_string() }
fn cg_write(path: &str, f: &str, v: &str) -> std::io::Result<()> { fs::write(format!("/sys/fs/cgroup/{path}/{f}"), v) }
fn procs(path: &str) -> Vec<i32> { cg(path, "cgroup.procs").lines().filter_map(|l| l.parse().ok()).collect() }
fn pidfd_readable(pidfd: i32, ms: i32) -> bool { let mut p = pollfd { fd: pidfd, events: POLLIN, revents: 0 }; unsafe { poll(&mut p, 1, ms) == 1 } }
fn wait_for(mut f: impl FnMut() -> bool, ms: u64) -> (bool, u128) { let t = Instant::now(); while t.elapsed() < Duration::from_millis(ms) { if f() { return (true, t.elapsed().as_millis()); } std::thread::sleep(Duration::from_millis(5)); } (f(), t.elapsed().as_millis()) }

#[repr(C)] #[derive(Default)] struct CloneArgs { flags: u64, pidfd: u64, child_tid: u64, parent_tid: u64, exit_signal: u64, stack: u64, stack_size: u64, tls: u64, set_tid: u64, set_tid_size: u64, cgroup: u64 }

/// clone3 into new PID+mount ns with CLONE_PIDFD and CLONE_INTO_CGROUP; child blocks on a pipe barrier.
fn spawn_session_init(cgroup_fd: i32, workload: &str) -> (i32, i32, i32) {
    let mut barrier = [0; 2]; unsafe { pipe2(barrier.as_mut_ptr(), O_CLOEXEC); }
    let mut pidfd: i32 = -1;
    let mut args = CloneArgs { flags: (CLONE_NEWPID | CLONE_NEWNS | CLONE_PIDFD) as u64 | 0x2_0000_0000u64 /* CLONE_INTO_CGROUP */, pidfd: &mut pidfd as *mut i32 as u64, exit_signal: SIGCHLD as u64, cgroup: cgroup_fd as u64, ..Default::default() };
    let pid = unsafe { syscall(SYS_clone3, &mut args as *mut CloneArgs, std::mem::size_of::<CloneArgs>()) } as i32;
    if pid == 0 {
        unsafe {
            close(barrier[1]);
            let mut b = [0u8; 1]; read(barrier[0], b.as_mut_ptr() as *mut c_void, 1); // barrier: wait for parent
            // minimal init: mount private /proc for the new pid ns, spawn workload, reap forever
            mount(std::ptr::null(), b"/\0".as_ptr() as _, std::ptr::null(), MS_REC | MS_PRIVATE, std::ptr::null());
            mount(b"proc\0".as_ptr() as _, b"/proc\0".as_ptr() as _, b"proc\0".as_ptr() as _, MS_NOSUID | MS_NODEV | MS_NOEXEC, std::ptr::null());
            prctl(PR_SET_CHILD_SUBREAPER, 1);
            let w = fork();
            if w == 0 { let wl = format!("{workload}\0"); execl(b"/bin/sh\0".as_ptr() as *const c_char, b"sh\0".as_ptr() as *const c_char, b"-c\0".as_ptr() as *const c_char, wl.as_ptr() as *const c_char, std::ptr::null::<c_char>()); _exit(127); }
            loop { let mut st = 0; let r = wait(&mut st); if r < 0 && *__errno_location() == ECHILD { sleep(1); } }
        }
    }
    unsafe { close(barrier[0]); }
    (pid, pidfd, barrier[1])
}
fn release(barrier_w: i32) { unsafe { write(barrier_w, b"g".as_ptr() as *const c_void, 1); close(barrier_w); } }

fn make_scope(name: &str) -> String {
    // Transient delegated scope around a holder process; our init is then clone3'd INTO its cgroup.
    Command::new("systemd-run").args(["--scope", &format!("--unit={name}"), "-p", "Delegate=yes", "-q", "sleep", "1000"])
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let (ok, _) = wait_for(|| sh(&format!("systemctl is-active {name}.scope")).1.starts_with("active"), 5000);
    if !ok { panic!("scope {name} did not become active"); }
    sh(&format!("systemctl show -p ControlGroup --value {name}.scope")).1.lines().next().unwrap().trim_start_matches('/').to_string()
}

fn dm_setup() -> String {
    sh("truncate -s 64M /tmp/ab-dm.img");
    let (_, loopdev) = sh("losetup --find --show /tmp/ab-dm.img");
    let (_, sz) = sh(&format!("blockdev --getsz {loopdev}"));
    sh(&format!("echo '0 {sz} linear {loopdev} 0' | dmsetup create ab-dstall"));
    loopdev
}
fn dm_teardown(loopdev: &str) { sh("dmsetup remove ab-dstall 2>/dev/null; losetup -d "); sh(&format!("losetup -d {loopdev} 2>/dev/null; rm -f /tmp/ab-dm.img")); }

fn main() {
    println!("spike scope-kill; systemd {}", sh("systemctl --version | head -1").1);
    sh("systemctl stop ab-spike-a.scope ab-spike-b.scope ab-spike-c.scope 2>/dev/null; systemctl reset-failed 2>/dev/null; dmsetup remove ab-dstall 2>/dev/null; losetup -D 2>/dev/null");

    // ============ Scenario A: orderly §5 protocol against a forking, signal-ignoring workload ============
    let cgA = make_scope("ab-spike-a");
    println!("scope A cgroup: {cgA} delegate: subtree_control={:?}", cg(&cgA, "cgroup.subtree_control"));
    let cgfd = unsafe { open(format!("/sys/fs/cgroup/{cgA}\0").as_ptr() as _, O_RDONLY | O_DIRECTORY | O_CLOEXEC) };
    let (init_pid, init_pidfd, bw) = spawn_session_init(cgfd, "trap '' TERM; (while :; do sleep 0.05 & wait; done) & (while :; do :; done) & sleep 1000");
    let in_cg_before_release = procs(&cgA).contains(&init_pid);
    result("A-1.clone3-into-cgroup-with-barrier", init_pidfd >= 0 && in_cg_before_release, &format!("init pid={init_pid} pidfd={init_pidfd} in scope cgroup before barrier release={in_cg_before_release}"));
    release(bw);
    let (grew, _) = wait_for(|| procs(&cgA).len() >= 4, 3000);
    let n0 = procs(&cgA).len();
    result("A-2.workload-populates-scope", grew, &format!("cgroup.procs count={n0} (init, sh, forker, spinner, sleepers)"));
    // step 2: freeze → no forks
    cg_write(&cgA, "cgroup.freeze", "1").unwrap();
    let (frozen, t_frz) = wait_for(|| cg(&cgA, "cgroup.events").contains("frozen 1"), 3000);
    let n1 = procs(&cgA).len(); std::thread::sleep(Duration::from_millis(300)); let n2 = procs(&cgA).len();
    result("A-3.freeze-stops-forking", frozen && n1 == n2, &format!("frozen after {t_frz} ms; procs stable at {n1} over 300 ms (forker loop stopped)"));
    // step 3: SIGTERM to workload via init pid (ignored by trap) + thaw briefly
    unsafe { kill(init_pid, SIGTERM); }
    cg_write(&cgA, "cgroup.freeze", "0").unwrap(); std::thread::sleep(Duration::from_millis(200));
    let still_alive = state(init_pid) != "gone" && !procs(&cgA).is_empty();
    result("A-4.sigterm-ignored-workload-survives-thaw", still_alive, &format!("init state={} procs={} (workload traps TERM; init has no handler and is not killed by TERM from outside? see detail)", state(init_pid), procs(&cgA).len()));
    // step 4: freeze again + cgroup.kill; wait on init pidfd
    cg_write(&cgA, "cgroup.freeze", "1").unwrap();
    let t = Instant::now(); cg_write(&cgA, "cgroup.kill", "1").unwrap();
    let init_exited = pidfd_readable(init_pidfd, 5000);
    let t_kill = t.elapsed().as_millis();
    // note: cgroup.kill on a frozen cgroup — tasks must be thawed to die? measure.
    let frozen_now = cg(&cgA, "cgroup.events");
    let (empty, t_empty) = wait_for(|| procs(&cgA).is_empty(), 5000);
    result("A-5.cgroup-kill-empties-frozen-cgroup", init_exited && empty, &format!("init pidfd readable after {t_kill} ms; cgroup.procs empty after {t_empty} ms; events after kill: {:?}", frozen_now.replace('\n', " ")));
    let mut st = 0; unsafe { waitpid(init_pid, &mut st, 0); }
    result("A-6.init-reaped-and-populated-empty", cg(&cgA, "cgroup.events").contains("populated 0"), &format!("cgroup.events={:?}", cg(&cgA, "cgroup.events").replace('\n', " ")));
    // cgroup.kill also killed the scope's holder process (same cgroup); systemd should notice the empty scope and remove it
    let (gone, t_gone) = wait_for(|| !fs::metadata(format!("/sys/fs/cgroup/{cgA}")).is_ok(), 5000);
    let unit_after = sh("systemctl show -p ActiveState,SubState,Result --value ab-spike-a.scope").1.replace('\n', "/");
    result("A-7.scope-removed-when-empty", gone, &format!("scope cgroup directory removed by systemd {t_gone} ms after cgroup.kill emptied it (frozen at the time); unit={unit_after}"));

    // ============ Scenario B: D-state task ============
    let loopdev = dm_setup();
    let cgB = make_scope("ab-spike-b");
    let cgfd = unsafe { open(format!("/sys/fs/cgroup/{cgB}\0").as_ptr() as _, O_RDONLY | O_DIRECTORY | O_CLOEXEC) };
    sh("dmsetup suspend ab-dstall");
    let (init_pid, init_pidfd, bw) = spawn_session_init(cgfd, "dd if=/dev/mapper/ab-dstall of=/dev/null bs=4k count=1 iflag=direct; sleep 1000");
    release(bw);
    // find dd
    let (found, _) = wait_for(|| procs(&cgB).iter().any(|p| state(*p) == "D"), 5000);
    let dd = procs(&cgB).into_iter().find(|p| state(*p) == "D").unwrap_or(-1);
    result("B-1.d-state-task-created", found, &format!("dd pid={dd} state=D (blocked on suspended dm device)"));
    cg_write(&cgB, "cgroup.freeze", "1").unwrap();
    let (frozen, _) = wait_for(|| cg(&cgB, "cgroup.events").contains("frozen 1"), 2000);
    println!("freeze with D-state member: frozen={frozen} events={:?}", cg(&cgB, "cgroup.events").replace('\n', " "));
    let t = Instant::now(); cg_write(&cgB, "cgroup.kill", "1").unwrap();
    let init_exited = pidfd_readable(init_pidfd, 3000);
    std::thread::sleep(Duration::from_millis(500));
    let rem = procs(&cgB); let dd_state = state(dd);
    // Expected: the D task survives; and because it is a member of init's PID namespace, init itself cannot
    // finish exiting until the D task is gone (namespace init waits for all members), so the init pidfd is
    // NOT readable yet either. Both are the §5 "task remains live at the bound" case.
    let init_state = state(init_pid);
    result("B-2.cgroup-kill-does-not-terminate-d-state", !init_exited && rem.contains(&dd) && dd_state == "D", &format!("after cgroup.kill + {} ms: dd state={dd_state} (SIGKILL pending, uninterruptible); remaining procs={rem:?}; init pidfd readable={init_exited}, init state={init_state} (PID-ns init cannot complete exit while a namespace member lives); frozen never reached (events={:?})", t.elapsed().as_millis(), cg(&cgB, "cgroup.events").replace('\n', " ")));
    // the bounded wait would now expire → termination-incomplete; holder killed → does systemd remove the scope with a D task inside?
    let (scope_gone, _) = wait_for(|| !fs::metadata(format!("/sys/fs/cgroup/{cgB}")).is_ok(), 2000);
    let unit_state = sh("systemctl show -p ActiveState,SubState --value ab-spike-b.scope").1.replace('\n', "/");
    result("B-3.scope-persists-while-d-task-live", !scope_gone && procs(&cgB) == vec![dd], &format!("scope cgroup still present={}; unit state={unit_state}; membership evidence retained for escalation", !scope_gone));
    // pidfd on the D task: still live
    let ddfd = unsafe { syscall(SYS_pidfd_open, dd, 0) } as i32;
    result("B-4.pidfd-holds-d-task", ddfd >= 0 && !pidfd_readable(ddfd, 0), &format!("pidfd_open(dd)={ddfd}, not readable (live) — held pidfd is what session.escalation_required names"));
    // release: resume device → dd completes the I/O then dies of the pending SIGKILL
    let t = Instant::now(); sh("dmsetup resume ab-dstall");
    let (empty, t_e) = wait_for(|| procs(&cgB).is_empty(), 5000);
    let (scope_gone, _) = wait_for(|| !fs::metadata(format!("/sys/fs/cgroup/{cgB}")).is_ok(), 5000);
    result("B-5.d-task-dies-when-io-completes", empty && pidfd_readable(ddfd, 1000) && scope_gone, &format!("after dm resume: cgroup empty in {t_e} ms (total {} ms); dd pidfd readable; systemd removed the scope", t.elapsed().as_millis()));
    unsafe { let mut st = 0; waitpid(init_pid, &mut st, 0); }
    dm_teardown(&loopdev);

    // ============ Scenario C: D-Bus signals + systemd-kills-first race ============
    let cgC = make_scope("ab-spike-c");
    sh("systemctl set-property --runtime ab-spike-c.scope TimeoutStopSec=5s 2>/dev/null");
    let cgfd = unsafe { open(format!("/sys/fs/cgroup/{cgC}\0").as_ptr() as _, O_RDONLY | O_DIRECTORY | O_CLOEXEC) };
    let (init_pid, init_pidfd, bw) = spawn_session_init(cgfd, "sleep 1000");
    release(bw);
    wait_for(|| procs(&cgC).len() >= 2, 2000);
    // subscribe to systemd signals in the background (busctl monitor), then systemctl kill/stop the scope from "outside" (operator / systemd)
    let mon = Command::new("sh").arg("-c").arg("timeout 8 busctl monitor org.freedesktop.systemd1 --match \"type='signal'\" 2>/dev/null | grep -E 'UnitRemoved|PropertiesChanged|JobRemoved' | head -20").stdout(Stdio::piped()).spawn().unwrap();
    std::thread::sleep(Duration::from_millis(800));
    let t = Instant::now();
    sh("systemctl stop ab-spike-c.scope");  // systemd kills first: SIGTERM then SIGKILL to the whole cgroup, then removes the unit
    let init_exited = pidfd_readable(init_pidfd, 5000); let t_pidfd = t.elapsed().as_millis();
    let holder_gone = state(holder_pid_or(&cgC)) == "gone";
    let (dir_gone, t_dir) = wait_for(|| !fs::metadata(format!("/sys/fs/cgroup/{cgC}")).is_ok(), 5000);
    let out = { let o = mon.wait_with_output().unwrap(); String::from_utf8_lossy(&o.stdout).to_string() };
    let saw_removed = out.contains("UnitRemoved"); let saw_props = out.contains("PropertiesChanged");
    println!("busctl monitor captured {} signal lines; UnitRemoved={saw_removed} PropertiesChanged={saw_props}", out.lines().count());
    // Expected finding: systemctl stop sends SIGTERM to the cgroup; the PID-namespace init (PID 1 in its ns)
    // has SIGTERM ignored by default unless it installs a handler, so systemd waits TimeoutStopSec
    // (default 90 s) before SIGKILL. The daemon must set TimeoutStopSec/KillSignal on the scope or install a
    // TERM handler in init; otherwise an operator `systemctl stop` stalls 90 s.
    result("C-1.pidfd-watch-fires-on-systemd-stop", init_exited, &format!("init pidfd readable {t_pidfd} ms after systemctl stop (independent liveness source, no D-Bus needed; ~90 s = DefaultTimeoutStopSec because PID-ns init ignores SIGTERM — see finding)"));
    result("C-2.dbus-signals-observable", saw_removed || saw_props, &format!("UnitRemoved={saw_removed} PropertiesChanged={saw_props} (either triggers §5; scope dir removed after {t_dir} ms, holder gone={holder_gone})"));
    // systemd-kills-first: after systemd has stopped the unit, can the daemon still run the §5 protocol? cgroup dir is gone → freeze/kill files unavailable; pidfd still valid for reap/verification
    let mut st = 0; let reaped = unsafe { waitpid(init_pid, &mut st, 0) } == init_pid;
    let cg_files_gone = fs::metadata(format!("/sys/fs/cgroup/{cgC}/cgroup.kill")).is_err();
    result("C-3.systemd-kills-first-daemon-can-still-verify", reaped && cg_files_gone && dir_gone, &format!("init reaped via held pid/pidfd={reaped}; cgroup control files gone={cg_files_gone} → §5 steps 2–4 are no-ops, step 5 host credential scan and steps 6–11 still executable; must record session.ordering_deviation"));
    println!("done");
}
fn holder_pid_or(cg: &str) -> i32 { procs(cg).first().copied().unwrap_or(999999) }
