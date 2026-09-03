# WP1 evidence — `scope-kill`

**Covers:** plan WP1 spikes "systemd scope + PID-namespace init containment and `cgroup.kill` behaviour, including D-state tasks" and "`agentbound-lifecycle` D-Bus scope-signal subscription and pidfd-watch fallback, including the systemd-kills-first race"; session lifecycle §3 step 1 (`clone3` barrier), §4, §5 steps 2–5; requirements R-ISO-4 (mechanism projection); catalogue T-6.2 rows (mechanism projection).
**Baseline:** VM 110, Linux `6.12.107+deb13-cloud-amd64`, systemd `257.13-1~deb13u1`.
**Spike:** `spikes/scope-kill/`. **Raw transcript:** `raw/scope-kill.txt`. **Command:** `spikes/run.sh scope-kill` (≈ 100 s: scenario C deliberately waits out the systemd stop timeout).

Setup: a transient delegated scope is created (`systemd-run --scope -p Delegate=yes` around a holder). A session init is created with `clone3(CLONE_NEWPID | CLONE_NEWNS | CLONE_PIDFD | CLONE_INTO_CGROUP)` directly into the scope's cgroup and blocks on a pipe barrier before doing anything; once released it mounts a private `/proc`, becomes a subreaper, runs the workload, and reaps. D-state tasks are produced with a *suspended* dm-linear device over a loop file: a direct read blocks uninterruptibly, ignores `SIGKILL`, and completes when the device is resumed.

## Results

| ID | Required result | Observed | Result |
|---|---|---|---|
| A-1 | `clone3` with `CLONE_PIDFD` and `CLONE_INTO_CGROUP`; child is in the scope cgroup and blocked before any action | init in `cgroup.procs` before barrier release; pidfd returned | **PASS** |
| A-2 | Workload populates the scope | 7 processes (init, `sh`, fork loop, CPU spinner, sleepers) | **PASS** |
| A-3 | §5 step 2: freeze stops forking | `frozen 1` in 5 ms; process count constant over 300 ms | **PASS** |
| A-4 | §5 step 3: `SIGTERM`-ignoring workload survives the brief thaw | init and all 7 processes alive after thaw | **PASS** (expected; step 4 exists for this) |
| A-5 | §5 step 4: `cgroup.kill` on a *frozen* cgroup empties it; init pidfd fires | pidfd readable and `cgroup.procs` empty within the first poll (0 ms); events `populated 0 frozen 1` | **PASS** |
| A-6 | Init reaped; `populated 0` | as required | **PASS** |
| A-7 | systemd removes the empty scope | scope directory gone 5 ms after emptiness; unit `success/inactive/dead` | **PASS** |
| B-1 | D-state task exists inside the session | `dd` in state `D` on the suspended device | **PASS** |
| B-2 | `cgroup.kill` does not terminate a D-state task; init cannot complete exit | after `cgroup.kill` + 3.5 s: `dd` still `D` with `SIGKILL` pending; init still `S`, pidfd not readable; freeze never reached `frozen 1` | **PASS** (lifecycle §5 text is correct) |
| B-3 | Scope and membership evidence persist while the D task lives | scope cgroup present, unit `active/running`, `cgroup.procs = [dd]` | **PASS** |
| B-4 | A pidfd on the D task is holdable | `pidfd_open` succeeds, not readable (live) — the handle `session.escalation_required` names | **PASS** |
| B-5 | The D task dies when the I/O completes | `dmsetup resume` → cgroup empty within 8 ms, pidfd readable, scope removed by systemd | **PASS** |
| C-1 | Held init pidfd fires when systemd stops the scope (pidfd-watch fallback) | pidfd readable — but only **90 090 ms** after `systemctl stop` (see F-4) | **PASS** with finding |
| C-2 | D-Bus signals observable for the scope | `PropertiesChanged` observed on the unit; `UnitRemoved` emitted (separate probe: ≈1.5 s after the unit becomes inactive, i.e. at garbage collection) | **PASS** |
| C-3 | systemd-kills-first: the daemon can still complete the §5 protocol | init reaped via the held pid/pidfd; cgroup control files gone so steps 2–4 are no-ops; steps 5–11 executable; `session.ordering_deviation` required | **PASS** |

## Findings

### F-3 — `cgroup.freeze` never completes while a D-state member exists (session lifecycle §5 step 4)

With a D-state member, `cgroup.freeze = 1` was written but `cgroup.events` stayed `frozen 0` indefinitely (the freezer requires every task to reach the frozen state; an uninterruptible task never does). §5 step 4 says "freeze again and kill the cgroup". `cgroup.kill` on the not-yet-frozen cgroup still killed every killable task (B-2), so the protocol's outcome is unaffected, but **the daemon MUST NOT wait for `frozen 1` before writing `cgroup.kill`**, or it deadlocks against the very D-state case §5 is written for. Recommended wording: "request freeze; write `cgroup.kill` without waiting for the frozen state; use the bounded wait on emptiness and the init pidfd". Also note: `frozen 1` *was* reached in scenario A because there the freeze completed before the kill; the ordering is safe only if the wait is on emptiness, not on the freezer.

### F-4 — PID-namespace init ignores `SIGTERM`; `systemctl stop` stalls for `DefaultTimeoutStopSec` (90 s)

The session init is PID 1 in its namespace. The kernel discards signals to a namespace init that has no handler installed (only `SIGKILL`/`SIGSTOP` from the parent namespace get through unconditionally). `systemctl stop <scope>` therefore delivered `SIGTERM` to no effect and waited the unit's `TimeoutStopSec` (defaulting to 90 s) before `SIGKILL`; the unit finished `Failed with result 'timeout'`. Consequences for the design:

1. The daemon's §5 protocol never relies on `SIGTERM` reaching init from outside — step 3 delivers `SIGTERM` *via* init — so the ordered protocol is unaffected.
2. An operator or systemd-initiated `systemctl stop` (the systemd-kills-first race) takes 90 s by default. The constructor MUST set `TimeoutStopUSec` on the transient scope at creation (`StartTransientUnit` accepts it; verified 2 s takes effect and `systemctl stop` then completes in 7 ms). `systemctl set-property` cannot change it later on a scope (verified: "Cannot set property TimeoutStopUSec"). Alternatively or additionally the session init installs a `SIGTERM` handler that forwards to the workload. Owner: session lifecycle §3 (scope creation prerequisites) and §4.

### F-5 — `UnitRemoved` fires at unit garbage collection, not at process death

`UnitRemoved` for a stopped scope was emitted ≈1.5 s after the unit went inactive; `PropertiesChanged` (`ActiveState`) fires promptly. The daemon should treat `PropertiesChanged` with `ActiveState ∈ {inactive, failed}` as the D-Bus trigger and the held init pidfd as the authoritative one; `UnitRemoved` is confirmation only. Owner: session lifecycle §4 (the text already names both; ordering guidance would help the implementer). No obligation change.

## Implementation notes

- `CLONE_INTO_CGROUP` is `0x200000000` (bit 33); the `libc` crate's `c_int` constant is wrong and will change type. Placing init in the scope cgroup at `clone3` time removes the "process exists outside its cgroup" window entirely.
- `cgroup.kill` acts on a frozen cgroup with no thaw needed (A-5); frozen tasks die on `SIGKILL`.
- A namespace init cannot finish exiting while any namespace member is alive (B-2: init stayed `S` after `SIGKILL` with the D task present), so waiting on the init pidfd alone would also stall; the emptiness wait plus per-task pidfds is the right observation set, as §5 step 5 requires.
- Once systemd has removed the scope the cgroup control files are gone; the daemon must tolerate `ENOENT` on steps 2–4 and proceed to the host credential scan (step 5).
