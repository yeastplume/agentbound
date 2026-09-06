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
