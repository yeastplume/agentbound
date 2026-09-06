# WP3.1 evidence register — conformance correction and independent test (in progress)

Hard gate before WP4 (plan 0.16 §WP3.1). This register is written incrementally; each round adds a raw run under `raw/` and updates the tables. Nothing here is a pass claim until the go/no-go section at the end is filled in.

## Round 1 — harness integrity

**Change.** `ab-conformance` now checks itself against the frozen catalogue: `crates/ab-conformance/expected-ids.txt` is generated from `docs/architecture/test-catalogue.md` 0.7 by `crates/ab-conformance/tools/gen-expected-ids.py` (every row whose Milestone column includes 1A or 1B — 118 ids — plus the three ADR-0002 rows D4, D7-8, D7-9 that the WP3 register relied on). Runner rows map to a catalogue id by prefix (`T-6.4-003.only` → `T-6.4-003`; unit-tested). The run **fails** (exit 1) if any catalogue id has no PASS row, if any row id is duplicated, if any non-fixture row maps to an id outside the catalogue, or if any assertion fails. Verdict classes are now separate: `PASS`, `WEAK` (assertion true but weaker than the row's intent), `RECORDED` (1A partial/N-A re-asserted under 1B), `FAIL`, `FIXTURE` (setup/marker — excluded from every count). Machine output records run id, repository commit, SHA-256 (16 hex) of every installed binary and in-image script, and the catalogue coverage table. WP1 `GS-*` ids used as row names were re-homed to their catalogue rows (T-6.3-002, T-6.4-011, T-6.4-012); the fixtures `PROBE-COMPLETE`, `GW-COMPLETE`, `GW-HELD`, `T-6.8-setup`, `T-6.2-005` (orphan spawn; asserted by D-07), `D-10.bundle` are now `FIXTURE`.

**Result** ([raw/run-01-harness-integrity.md](raw/run-01-harness-integrity.md), commit `207930e` binaries): 126 PASS, 3 WEAK, 4 RECORDED, 0 FAIL assertions; **catalogue coverage 85/121 PASS, 2 WEAK, 4 RECORDED, 30 NOT-EXECUTED; run verdict FAIL.** This is the honest baseline the WP3 register should have reported.

**Not executed (30), grouped by what is needed:**

| Group | Catalogue ids | Note |
|---|---|---|
| 1B rows named by the review | D-16, T-6.3-005, T-6.3-008, T-6.8-008, T-6.8-009, T-6.9-008, F-C-08, F-T-01, F-T-06, F-T-07, F-T-09 | WP3.1 item 4 |
| 1A rows that WP2 recorded from prose or never ran | D-03, D-05, T-6.1-006, T-6.1-008, T-6.1-010, T-6.1-011, T-6.1-012, T-6.5-005, T-6.5-008, T-6.6-007, T-6.7-001, T-6.9-003 | The WP2 register's "84/84" had the same defect as WP3's: rows written up without a machine row. To be implemented or recorded not-executed with reason; WP2 register to be annotated |
| Constructor fault points not injected | F-C-01, F-C-02, F-C-04, F-C-05, F-C-06 | WP2 injected F-C-03/07/09 only |
| Termination fault points not injected | F-T-05 | with F-T-06/07/09 above |

**Still to do under item 1:** per-run scoping of every assertion that reads cumulative VM state (audit greps not keyed on this run's launch records; `process_mismatch` corpus in T-6.4-009), exact `class`+`rule` on every denial, seed recording. These are folded into the false-positive repairs of item 2 because they are the same rows.

## Round 2 — false-positive repair (part 1 of 2)

Repaired the rows whose assertion did not test what the row names. T-6.4-008/009 (the in-scope malformed-credential peer) need a new session-side tool and are round 3.

| Row | Was | Now |
|---|---|---|
| T-6.3-004 | `r T-6.3-004 PASS` unconditionally | asserts four things and FAILs on a missing measurement: no credential-like variable in the child environment, no descriptor beyond 0/1/2, the child **cannot use the parent's inherited connected socket** (`--fork` → `process_mismatch`), and the child *can* establish its own authenticated connection |
| T-6.9-002 | shell subshell counted fds; an empty capture was read as 0 and passed | measured in-process by `ab-gwclient --fdbound`: `getrlimit(RLIMIT_NOFILE)` read back from the kernel, descriptors held open until `EMFILE`, and the row FAILs unless `opened < rlimit_cur` with `errno=24`. Root cause of the empty capture: the row ran *after* the T-6.9-001 fork bomb, so at `TasksMax` the shell could not fork and command substitution returned empty — the row is now ordered before it, with the reason in the file |
| T-6.4-004 | connected to an unbound abstract name (`ECONNREFUSED` for the wrong reason) | binds a real abstract socket in the host netns, proves it is **reachable from the host**, then proves the session netns gets `ECONNREFUSED (111)` |
| T-6.4-014 new-connection half | refused at the unrelated UID gate | check-order control: `establish` tests admission before peer identity, so the *same* peer is refused `uid_mismatch` while the session admits and `admission_closed` once quiesced; both rules are read from the gateway's own `gateway.connection_refused` event for this allocation, and the row requires both |
| D4.7-reconstruct | could pass with no reconstruction evidence (and read the component spool) | requires the **hash-chained receiver** to gain exactly one `gateway.reconstructed` for this restart, that event to report ≥ 1 projection, and an in-scope peer to complete an operation afterwards |
| D-12 | reported as a pass | reclassified **WEAK** with the evidence string stating it is a presence check and *not* the pre-registered metric (item 5) |
| `requirement_for` | mapped a non-existent rule `creds_count`; everything unknown fell through to `R-GW-1` | complete map over every rule the gateway emits (34 rules, unit-tested); unmapped rules now resolve to `R-GW-0-unmapped` (a visible defect marker) instead of a plausible-looking requirement; the typo is asserted *not* to resolve |

**Result** ([raw/run-02-false-positive-repair.md](raw/run-02-false-positive-repair.md)): 125 PASS, 4 WEAK, 4 RECORDED, 0 FAIL; catalogue 84/121 PASS, 30 NOT-EXECUTED; **run verdict FAIL** (as it must be until the missing rows exist).

Defects found by the repairs themselves: (i) the fd row had been silently measuring nothing since WP2 because of its position after the fork bomb; (ii) `gateway.reconstructed` reaches the receiver up to several seconds after restart, so the previous single `sleep 1` would have mis-scoped the evidence even if it had checked it; (iii) the rule→requirement map had one dead entry and a catch-all that made any future unmapped rule look like R-GW-1 in a denial — i.e. D7 item 9's diagnostics could have named the wrong requirement.

## Round 3 — false-positive repair (part 2): the credential and PID-reuse rows

These are the two rows the review identified as testing a different mechanism than their name. Both now run **from inside the session** (session uid, in the scope cgroup, in the session's namespaces) instead of from host root, and both are backed by measurements of what the kernel actually permits rather than by assumption.

### Kernel facts measured first (they change what the rows can honestly claim)

| Measured on the pinned kernel (6.12.107) | Consequence |
|---|---|
| With `SO_PASSCRED` set on the receiver, the kernel delivers **exactly one** `SCM_CREDENTIALS` per packet — even when the sender attaches none | The gateway's `credential_count != 1` branch is **unreachable from any peer**. It is a defensive assertion, not a tested path |
| Two `SCM_CREDENTIALS` cmsgs in one `sendmsg` are collapsed to one true credential (two ucreds inside one cmsg → `EINVAL`) | "multiple credentials" cannot be delivered at all |
| An unprivileged sender attaching a credential naming another pid or uid gets **`EPERM` at `sendmsg`** | A session peer cannot emit a false credential; forgery requires `CAP_SYS_ADMIN` in the namespace |
| A truncated ucred payload gets `EINVAL` at `sendmsg` | ditto |
| `/proc/sys/kernel/ns_last_pid` is not writable by the session uid (EIO/EACCES) even inside the session's own pid namespace | Driving PID reuse requires privileges the threat model's session does not have |

This is a finding about the mechanism, and it is the honest reading of ADR-0002 D2's "exactly one `SCM_CREDENTIALS` per packet": the kernel enforces it, and the gateway's check is defence in depth. The ADR should say so rather than implying the gateway is what makes it true.

### T-6.4-008 — four cases, from inside the session

`ab-gwclient --creds {none|two|forged|short}` attaches the cmsgs itself and prints `sendmsg_errno`, so the driver can tell a kernel refusal from a gateway refusal instead of inferring one from the other. Results: `none` and `two` are delivered and answered **as the true calling process** (no identity substitution); `forged` is refused by the kernel with `EPERM`; `short` with `EINVAL`. Four separate rows, all PASS, each stating which layer refused. The previous version ran as host root and was refused at the establishment UID check, never reaching the packet path at all.

### T-6.4-009 — a real PID-reuse construction, and what it revealed

Construction: an in-scope session-uid client establishes a connection and exits immediately, leaving a forked holder with the connected descriptor; the driver then recycles the establishing PID inside the session's pid namespace via `ns_last_pid` (privileges no session has — so the adversary is deliberately *stronger* than the threat model) and confirms `same_pid_as_establisher`. The recycled process shares pid, uid and scope cgroup with the establisher; only the process instance differs.

**Outcome:** the gateway polls each connection's peer pidfd, so the establisher's exit **closes the connection before the recycled process exists**. The holder never reaches the packet check. The row asserts that composite defence — connection closed on establisher exit, and no operation admitted for that pid afterwards — and says so in its evidence. The packet-level inode comparison is now a pure function `session::instance_mismatch` with unit tests, including `recycled_pid_rejected`: identical pid, uid, cgroup **and start time**, differing only in pidfs inode. That is the same-tick reuse case the catalogue asks for, covered deterministically rather than by a corpus grep. The row is no longer WEAK.

**Result** ([raw/run-03-credential-and-pid-reuse.md](raw/run-03-credential-and-pid-reuse.md)): 130 PASS, 3 WEAK, 4 RECORDED, 0 FAIL; catalogue 85/121 PASS, 30 NOT-EXECUTED; run verdict FAIL. Remaining WEAK: T-6.9-006 (cooperative fan-out), T-6.4-012 (no TLS upstream), D-12 (presence check pending item 5).

## Round 4 — implementation defects (item 3)

Three defects the review named, each now fixed **and** covered by a row that would have failed against the previous binaries.

### 1. `installed_value` was manifest intent, not installation

`agentbound-launch` wrote the manifest's `limit` straight into the binding's `resource_projection.installed_value` for every enforced class. Now:

| Class | Installed by | Read back from |
|---|---|---|
| `pids`, `memory_bytes`, `cpu` | scope properties (as before) | `pids.max`, `memory.max`, `cpu.max` in the scope cgroup, after the child is in it |
| `io_bandwidth` | **new**: `IOReadBandwidthMax`/`IOWriteBandwidthMax` on the device backing the mount-intent base | `io.max` (`wbps=`) |
| `disk_bytes`, `disk_inodes` | **new**: `size=` and `nr_inodes=` on the session's `/tmp` tmpfs (its bounded volatile storage) | `statfs("/tmp")` inside the child → `f_blocks × f_frsize`, `f_files` |
| `file_descriptors` | `setrlimit` (as before) | `getrlimit(RLIMIT_NOFILE)` inside the child |
| `audit_capacity` | **new**: the receiver reserves a per-session event budget (`reserve` op, root only; enforced per `authorization_id` in `append`; released by lifecycle at cleanup) | the figure the receiver reports it installed (may be lower than requested) |
| `delegation_fanout` | policy: no delegation operation exists in Phase 1 | `0` by construction; a non-zero manifest value is a construction failure (`unsupported_limit`) |

A constructor-owned class declared `enforced` with no kernel read-back is now a **construction failure** (`limit_not_observed`, step 7), and the schema refuses a constructor-owned class recorded as `declared_by_owner` — so the binding cannot attest an installation that was not observed. The first launch after this change failed exactly this way (the read-back path was wrong), which is the behaviour wanted.

Host-side cross-check `T-6.9-004.readback`: fetches the committed binding from the lifecycle store and compares all seven kernel-observed classes against the live cgroup files and, through `nsenter`, `statfs`/`ulimit` inside the session: `pids=64/64 memory_bytes=268435456/268435456 cpu=1000/1000 io_bandwidth=52428800/52428800 disk_bytes=268435456/268435456 disk_inodes=65536/65536 file_descriptors=1024/1024` (binding/kernel).

### 2. Gateway budgets reset on restart; total bytes never checked

`Projection.bytes_used` was accumulated after execution and compared with nothing; both counters were zeroed on reconstruct. Now the gateway keeps **per-operation-id** consumption `(operations, bytes)`, checks `operations`, `bytes_per_operation` and a new total `bytes` budget *before* counting, and persists the counters to the lifecycle record store (new gateway-only op `record_budget`, new record kind `budget`, hash-chained with the rest of the session's record, monotonic — a lower figure is refused `budget_regression`) **before the operation proceeds**. If persistence fails the operation is refused and admission closes. `record` now returns the latest `budget` alongside the binding, and both `activate` and `reconstruct` restore from it. The catalogue's two push operations gained `bytes` totals (64 MiB / 16 MiB).

`T-6.9-005.budget-persist`: 5 pings admitted (store: 43 → shows the counter that existed before the row began); **gateway restarted**; op_count restored to 52, not 0; 21 more admitted and 49 refused `budget_operations` (read from the gateway's own denial events for this allocation); stored `op:gateway-ping.operations = 64 = budget`, and 43 + 21 = 64. A reset would have admitted 64 more.

### 3. T-6.9-003 / T-6.9-004 did not exhaust anything

- `T-6.9-004.inodes`: creates files in `/tmp` until refused — `nr_inodes=65536; creation refused after 65529 files, IFree=0`.
- `T-6.9-004.bytes`: writes past capacity — stopped at 264 155 136 bytes (capacity 268 435 456) **by the memory cgroup, not by tmpfs**. **Finding:** tmpfs pages are charged to the writer's memory cgroup, so with `disk_bytes == memory_bytes` (both 256 MiB in this catalogue) the memory limit fires first. The write is still bounded at or below the installed capacity, and the row records *which* mechanism stopped it; it does not accept an unbounded write. A deployment that wants ENOSPC semantics must set `disk_bytes < memory_bytes`. This belongs in the manifest-schema guidance for §3.5.
- `T-6.9-003.memory`: `ab-gwclient --memhog 512` touching pages under `memory.max=256 MiB` is SIGKILLed (rc 137) — refused, never served.
- `T-6.9-003.cpu`: `cpu.max` read back as 1000 milli-cpu; `cpu.stat nr_throttled=76` during the probe's fan-out (accounted; throttling is contention-dependent and the row says so).
- `T-6.9-003.owners`: `audit_capacity` installed by the receiver (10 000), `delegation_fanout` 0.

Two probe-side facts learned the hard way and now written into `probe.sh`: a failed redirection on a special builtin aborts busybox `sh` (the inode loop must create in a subshell), and an OOM kill must land on a subshell, not the probe shell. Both had silently ended the probe on the first attempt — the runner now waits for `PROBE-END` rather than a fixed 8 s.

**Result** ([raw/run-04-implementation-defects.md](raw/run-04-implementation-defects.md)): 136 PASS, 3 WEAK, 4 RECORDED, 0 FAIL; catalogue 86/121 PASS, **29 NOT-EXECUTED**; run verdict FAIL. Unit tests: 25 (was 19).

**R-CON-8 watch:** direct privileged SLOC 2 417 (launch 494, lifecycle 826, ab-common 1 097; was 2 124 at WP3, +293 for read-back, audit reservation and budget records) — 40 % of the 6 000 ceiling. Gateway 424 (was 317+107).

## Round 5 — missing rows (item 4)

Item 4 of the plan is the largest single block of work in WP3.1: the rows the frozen catalogue names that WP3 never executed. This
round implemented the fault-injection families and the remaining 1B rows. The 1A prose-only rows are still outstanding.

### Fault injection: eleven rows, each proving its own step

The catalogue's F-C (construction) and F-T (termination) families require that each step of the two protocols be made to fail *with
its side effects already in place*, and that the recovery be observed rather than assumed. Faults are accepted from root only, apply
to exactly one action, and never relax a check — a fault makes a step fail; it does not make a test pass.

| Row | Step made to fail | What must then hold |
| --- | --- | --- |
| F-C-01 | 1 — `clone3` barrier never released | child reaped; it never reached `execve`; no session registered |
| F-C-02 | 2 — private mount namespace | construction fails at step 2; nothing mounted on the host |
| F-C-03 | 3 — mount intent (symlinked source) | refused at step 3 with a `mount_source` rule |
| F-C-04 | 4 — `pivot_root` (fails *after* the pivot) | failure inside the restricted tree still rolls back cleanly |
| F-C-05 | 5 — `proc` mount after the pid namespace | host mount table gains nothing (`findmnt` count 0) |
| F-C-06 | 6 — a descriptor opened *after* the closure pass | step 6's own verification through the fresh `/proc` catches it (`leaked 3:/image`) |
| F-C-07 | pre-commit crash | no launch record; identity reclaimed |
| F-C-08 | 8 — record committed and socket bound, activation never reached | socket node removed, gateway holds no projection (`unknown_record`), rollback names both, identity held |
| F-C-09 | post-commit crash | record retained, session never activated |
| F-T-01 | 1 — admission closure | the step-6 projection release still makes the node unusable: no new operation can be admitted |
| F-T-05 | 5 — no-live-process confirmation | `termination-incomplete`; identity **not** released; record **not** sealed |
| F-T-06 | 6 — gateway grant/connection release | cleanup holds with `released:false`; no identity release precedes the failure |
| F-T-07 | 7 — broker/credential closure | cleanup holds with `broker_closed:false`; identity retained |
| F-T-09 | 9 — gateway socket removal | the node survives with no listener behind it: connect+send is reset, ledger retained |

Each F-T row also asserts `<id>.resumable`: with the fault removed, a repeated `terminate` drives the session to a terminal state.
The protocol is resumable, and a failed step is a hold, not a lost session.

Two things about *how* these rows assert are worth recording, because both were initially wrong in ways that would have produced
false passes:

**No new audit event kinds.** The first implementation emitted `session.fault_injected`, `session.grant_release_failed` and
`session.credential_closure_failed`. The receiver rejected all three with `event_member_count` / `unknown_event_kind`: the audit
vocabulary in `agentbound-audit/src/events.rs` is **closed**, and every event's detail members are checked against a fixed set. The
events therefore reached only the component spool — the exact failure mode WP3 recorded, reproduced here by accident. A test
affordance must not extend the production event set, so steps 6 and 7 now report inside the existing `session.cleanup_completed`
record, whose `grants` object carries `released`, `remaining` and (new) `broker_closed`.

**Order, not a timing snapshot.** The rows first read session status *after* the faulted terminate and asserted it had not advanced.
That is unsound: lifecycle re-terminates a session whose init has exited, and that retry runs without the fault, so a later snapshot
legitimately reads `cleaned/sealed`. The rows now assert **ordering** in the hash-chained log — no `session.identity_released` and no
`session.sealed` may precede the record of the failing step — which is the property the protocol actually owes.

### A real deadlock between the two daemons (found by this round, fixed here)

The full run stopped making progress for 25 minutes. All three processes were blocked in `recvmsg`:

```
ab-conformance      __skb_wait_for_more_packets  (fd 3 → /run/agentbound/lifecycle.sock)
agentbound-lifecycle __skb_wait_for_more_packets  (fd 11 → gateway, 1280 bytes queued unread)
agentbound-gateway   __skb_wait_for_more_packets  (fd 9  → lifecycle, 1280 bytes queued unread)
```

Both daemons serve **one request at a time**, and each calls the other:

- gateway → lifecycle `record_budget`, on every admitted operation (R-GW-7 budget persistence, added in round 4);
- lifecycle → gateway `deny_admission` / `release`, during quiesce and termination (§5 steps 1 and 6), including from the
  unattended `poll_sessions` deadline path.

When those cross, each daemon is waiting for a reply from a peer that is blocked waiting for *it*. Neither can time out, because
`Conn::call` had no receive bound. Every live session is held with them, and no revocation or termination can be processed — a
containment failure, not merely a liveness bug. **Round 4's budget-persistence change is what made this reachable**; the cycle was
latent before it, since termination already called the gateway, but nothing on the gateway's request path called back.

Fix, in `ab-common/src/wire.rs`: `connect_bounded` sets `SO_RCVTIMEO`/`SO_SNDTIMEO`, and both cross-daemon call helpers use it with a
4 s bound. Each side already had a fail-closed path for "the other daemon did not answer" — the gateway closes admission and emits
`budget_persist_failed`; lifecycle records `released:false` and holds cleanup — so a bound converts an unbounded wedge into the
refusal the design already specified. A timeout is the correct remedy rather than a lock or a thread, because the requirement is that
*neither daemon may be made unavailable by the other*, and that must hold regardless of what the peer is doing.

`T-6.9-005.no-deadlock` is a permanent regression row for it: a session issues gateway operations in a tight loop (each persisting a
budget through lifecycle) while the driver terminates it from the other side, and the row requires that the terminate return, that
both daemons answer afterwards within a bound, and that the session reach a terminal state. This row FAILs on the round-4 build.

### Remaining 1B rows

- **T-6.3-005** — the socket is a broker capability, not a transferable object: copying the node out of the session yields nothing
  usable, handing the connected descriptor to another process is refused (`descriptor_transfer`), and root on the host connecting to
  the same node from outside the session's mount namespace is refused. Every use is authenticated per peer instance.
- **T-6.3-008** — replay from *another session*: two live sessions, B connects to A's socket node from inside B's namespaces with B's
  credentials. A's projection refuses, and the refusal is recorded against A. This is distinct from T-6.4-013 (caller-supplied
  identity arguments on the caller's *own* socket): here the path is right and the peer is wrong.
- **T-6.8-008** — Git grant withdrawn: a gateway ping that succeeded before the signal must not succeed after it, on the same
  in-scope peer. The declared behaviour comes from the manifest, and the effect is observed, not inferred.
- **T-6.8-009** — gateway unavailable (class RR): the gateway is genuinely stopped for this row, and the declared behaviour must be
  reached *without* it. `session.quiesce_started` records `admission:denied-no-gateway` — the gateway's availability at that moment.
- **T-6.9-008** — every budget class the catalogue declares is enforced, each with denials in the hash-chained log; the classes that
  do not exist at 1B (`rate`, `tokens`, `spend`, deferred to 1C under R-GW-9) are listed rather than claimed. The `objects` class was
  declared but *never demonstrated*: at 10 000 objects no bundle a session can build within its other budgets can exceed it. The
  force operation's limit is now 8, and `T-6.9-008.objects` exceeds it, so the class is enforced in evidence and not only on paper.
- **D-16** — the aggregate: every trigger in the frozen 11-trigger vocabulary must have been exercised with a declared action and a
  `session.revocation_received` record. It initially failed at 8/11, which exposed that `policy_withdrawn` was in the vocabulary but
  never signalled by any row (the catalogue's T-6.8-004 reads "policy/catalogue withdrawal" and only the catalogue half was covered);
  `T-6.8-004.policy` now covers it. Invariant 21 stays incomplete until 1C regardless, per R-LC-3.

A catalogue error found the same way: giving the Git task a partial `revocation` map made `agentbound-policy` reject every 1B request,
because the manifest schema closes over all eleven triggers. Correct behaviour, and the reason the whole 1B section went red at once.

### The twelve 1A rows WP2 recorded from prose

WP2's register carried twelve 1A rows whose evidence was an argument in prose rather than an executed assertion. All twelve now
execute. Six run inside the session (in `probe.sh`, with new syscall modes in `ab-gwclient`), six from the driver:

| Row | How it is now decided |
| --- | --- |
| T-6.1-006 | symlinks planted in the session temp dir at the host catalogue, the lifecycle store and `/` — every write through them refused, the targets absent from this mount namespace |
| T-6.1-008 | the session sets env vars and writes its own startup files; no sibling session's startup file can be written |
| T-6.1-010 | `pidfd_open` on a live host pid (passed in through the workspace) returns ESRCH; the same pair against our own init is recorded as a control so the negative cannot be a broken-syscall artefact |
| T-6.1-011 | `process_vm_readv` against that host pid returns ESRCH — denied, never a partial read |
| T-6.1-012 | the driver binds `agentbound-conf-abs` in the host netns; the session cannot connect to it (ECONNREFUSED) **and** binds the identical name itself, which proves the abstract namespaces are separate rather than merely that a connect failed |
| T-6.2-005 | the double-forked orphan is located in the session's own pid namespace with our init as its parent — contained here, reaped at termination by D-07 |
| D-03 | two concurrent sessions of *different principals*; each private-state interface of one attempted with the other's identity, judged **by effect**: A's `memory.max` unchanged, A's process count unchanged after a signal attempt, A's environ/root unreadable, no file created in A's workspace. Reverse direction too |
| D-05 | three runtimes on the same task and resource: distinct uids from one allocator range, distinct scope cgroups, an identical committed namespace set, every namespace the record commits as private in fact distinct from PID 1's, and the same boundary-establishing audit events |
| T-6.5-005 | the catalogue's `pids` limit is rewritten every 150 ms while eight requests race it; every admitted session's installed `pids.max` equals the value in **its own** committed manifest — never a mixture, never a value that existed only between decisions |
| T-6.5-008 | four confusions of a genuine committed record replayed to `commit_binding`: constructor envelope as the policy envelope, manifest mutated after signing, a valid signature over the wrong object, corrupted signature bytes. All refused, each naming the check that caught it |
| T-6.6-007 | a correctly signed binding replayed for an allocation that has advanced is refused (`binding_allocation_mismatch`), and the catalogue's `policy_version` is rolled backwards to confirm no session derives from a rolled-back policy |
| T-6.7-001 | axes measured against the committed manifest (mounts, grants, descriptors), a child of the workload gains nothing, and seven recovery paths — remount rw, fresh tmpfs, raise pids/memory/NOFILE, re-exec as root, rewrite the catalogue — are each refused |

Three of these initially "passed" for the wrong reason, and the fixes are worth recording because each was a way a suite can lie to
itself:

- **D-03 counted an empty output as a denial.** `setpriv --reuid` was receiving an empty uid, so every attempt failed with
  `failed to parse reuid` — which contains no data, looks like nothing happened, and would have been read as "denied". The row now
  judges each attempt by its **effect** on the victim (limit unchanged, process count unchanged, no file created), treats an attempt
  that could not be made as `unmeasured` rather than as a pass, and reports a setup failure as a setup failure. The empty uid itself
  was a real setup bug: the request kept alice's initiator credential while running as bob, which the platform correctly rejected.
- **D-05 compared namespaces against the driver's own.** The driver runs as a systemd service whose mount namespace is already
  private, so "not shared with the driver" was not the claim being made. It now compares against PID 1, and only for the namespaces
  the record actually commits as private (`user` is `inherited`, so claiming it would be false).
- **The T-6.1-012 host-name fixture never ran.** `pkill -f abs-holder.py` matched the shell that was starting it — the pattern
  appears in that shell's own argv — so the fixture killed itself and the row was scored against a name nobody held. It now uses a
  pidfile.

**Result** ([raw/run-05-missing-rows.md](raw/run-05-missing-rows.md)): **175 PASS, 3 WEAK, 4 RECORDED, 0 FAIL; catalogue 115/121
PASS, 0 NOT-EXECUTED, dups=0, extra=0; run verdict PASS.** This is the first run in the project whose verdict is not FAIL, and the
first in which every row the frozen catalogue names is actually executed.

That verdict is *not* the WP3.1 exit condition, and it should not be read as one. Four items of the plan remain, and two of them
exist precisely because a green suite is not yet trustworthy: D-12 is still a presence check rather than the pre-registered metric
(8 sessions × 230 effects × 10 seeded repetitions), the ten seeded repetitions per bypass row required by catalogue §4 have not been
run, **no negative control has yet been built** — that is, nothing has yet demonstrated that these rows can fail when the property
they assert is broken — and there has been no fresh-host reproduction. Until the negative controls exist, "0 FAIL" evidences that
the assertions ran, not that they discriminate.

The three WEAK rows and four RECORDED deviations are unchanged from WP3 and are listed there: T-6.9-006 (cooperative fan-out),
T-6.4-012 (no TLS upstream in this deployment), D-12 (presence check), D-02/T-6.1-003 (no PTY path exists to deny), T-6.2-008
(loader inventory), D-15 (no delegation operation exists to narrow).

## Item 5 — D-12 as pre-registered: three blocking findings

D-12's register entry has read `WEAK — presence check only` since WP3. Item 5 was to replace it with the metric the frozen
catalogue pre-registers in §5: 8 concurrent sessions × 230 atomic effects across three ontology classes, ten seeded repetitions,
a 30 s correlation deadline, ≥ 99 % over all classes and 100 % over the finite gateway-operation corpus.

The harness is built (`crates/ab-conformance/probe/d12-worker.sh` emits the ground truth, `d12-correlate.py` computes `|C|/|G|`,
`d12-run.py` drives one seeded repetition), and it was deliberately built to be able to return a number below 1.0. It does. The
metric is **not met**, and three separate findings stand in the way. None of them is a harness artefact.

### 1. The gateway protocol had no idempotency key at all (fixed)

Test-catalogue §5 defines an atomic effect as an *idempotency-keyed* workload event, and a reconstruction is correct only when it
matches the ground-truth class, outcome **and idempotency key**. Component-interfaces §5 requires *every* component request to
carry a key scoped to the receiving component, caller identity, operation and target record, and requires receivers to retain
enough completed-operation state to return the original outcome rather than repeat a non-idempotent action.

The gateway's own protocol (`agentbound.gateway.v0.1`) had no such field. Its requests carried `operation`, `operation_id`,
`payload_len` and `payload_sha256` and nothing else, so no gateway effect could be matched to a workload's own log by key, and a
retried Git push would have been executed twice. The correlator scored the gateway class at **0 %** for exactly this reason.

Fixed rather than narrated: the protocol now carries `idempotency_key`, the gateway scopes it to (allocation, operation_id, key),
a repeated key with the same operation returns the original reply and emits `gateway.operation_replayed`, a repeated key with a
different operation is a `conflict`, and the key appears on `gateway.operation_admitted`, `_completed` and `_denied`. Denials
carry it too, because §5 counts a denied operation as an in-scope effect — that required stashing the key on the connection
before the grant check, since the generic denial path never sees the parsed request. After the fix the gateway-operation corpus
reconstructs at **100 %** (15/15 in the last partial run, 88/88 gateway records in the run before it).

### 2. Two of the three effect classes have no telemetry path (open, unfixed)

R-AUD-2 (1B) requires `agentbound-audit` to reconstruct `initiator → agent → session → process → effect` for the whole ontology:
local objects in the session's world, process lifecycle events, and gateway operations. Only the third exists. The audit store
contains gateway events and session-lifecycle events and nothing else: no record names an individual file the workload created,
and none names an individual fork/exec/exit. 220 of every 230 effects — **95.7 % of the metric's denominator** — are therefore
unattributable, and the measured completeness is **3.5 %** (8/230 for a single session; the eight permitted gateway operations
were the only effects reconstructed).

This is a design gap, not a bug. Nothing in the implementation was ever built to ingest classes (a) and (b), and the WP2/WP3
registers never noticed because D-12 was scored by a presence check that only ever looked at event kinds already being emitted.
Kernel audit is enabled on the host and a path watch on a session workspace does capture syscall records carrying pid, uid and
the syscall number, so a path to class (a) and (b) ingestion exists on the pinned baseline — but building an audit-netlink
shipper, deciding how per-session rules are installed and removed, and reconciling the host-global `lost` counter of R-AUD-3 is
a work package, not a WP3.1 repair. **D-12 cannot be met at 1B without it.**

### 3. The platform cannot currently admit 8 concurrent sessions (open)

The pre-registered profile requires 8 concurrent sessions. Launched together, only 2–3 of 8 succeed. The failures are real and
of two kinds:

- `gateway_rejected: unavailable:lifecycle:record unavailable` at constructor step 8 — the gateway must fetch the committed
  record from `agentbound-lifecycle` to activate a projection, and lifecycle is busy serving another construction.
- `lifecycle_rejected: invalid:constructor_envelope:Stale` at step 8 — the launch binding must be verified within
  `BINDING_MAX_AGE_S` (60 s) of signing, and the queue ahead of it is longer than that.

The cause is that `agentbound-lifecycle` serves **one request at a time** and its handlers do blocking work inside that
serialization: `terminate` alone holds the daemon through a 2 s SIGTERM grace plus a bounded wait for cgroup emptiness and init
exit. Measured on this host: a single construction held the daemon for **17.4 s**, one session took **123 s** from authorization
to activation, and one termination held it for **61 s**. Component-interfaces §3.6 requires lifecycle to *decide, serialize and
record* transitions — serializing the *decision* is the requirement; serializing the *waiting* is an implementation choice, and
it is the one that makes the pre-registered profile unreachable.

This also corrected a fix from round 5. Bounding cross-daemon calls at 4 s was right in kind and wrong in value: 4 s is *below*
the peer's legitimate service time under load, so it turned a busy peer into a failed launch. The bound now lives in one place
(`wire::CROSS_DAEMON_MS`, 60 s) with the reasoning that it must exceed the slowest legitimate service time, because its purpose
is to stop an indefinite wait and not to impose a latency budget. The same class of bug was found and fixed in the in-session
client: `ab-gwclient`'s `recv` was unbounded, so a slow gateway was indistinguishable from a hung workload — it is now bounded
at 30 s and reports a timeout as a timeout.

### Consequence for the WP3.1 verdict

D-12 stays **WEAK**, and the honest statement is stronger than that: *the attribution-completeness metric of R-AUD-2 cannot be
met by this implementation*, because two of its three effect classes are not collected at all. This is the first finding in
WP3.1 that a repair inside the work package cannot close, and it belongs in the go/no-go as a **narrow-or-defer** recommendation
on R-AUD-2 rather than as a residual note.
