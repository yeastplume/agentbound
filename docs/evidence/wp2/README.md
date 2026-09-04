# WP2 conformance and evidence register (milestone 1A)

Evidence for the Phase 1 plan WP2 exit condition: *the 1A rows of the test catalogue run as an automated conformance suite on the pinned baseline, every row records PASS or FAIL with observed evidence, and the five R-CON-8 SLOC figures are published against the 6 000 direct-SLOC bound.*

**Pinned baseline (as run):** VM 110 `agentbound-dev` — Linux `6.12.107+deb13-cloud-amd64`, systemd `257.13-1~deb13u1`, Debian 13, cargo 1.98.1, tokei `13.0.0-alpha.8`. Run date 2026-09-04; repository commit `8752b76` (suite) — the register text was added afterwards without code change.

Status values: **PASS**, **FAIL**, **PASS (partial)** (row observed as far as the 1A topology permits; the untested half is named), **N/A-1A** (row's subject does not exist at topology `none`; listed and re-run at 1B), **RESIDUAL** (accepted assumption recorded, not tested).

## WP2 exit status

**84 rows, 84 PASS, 0 FAIL** in the final machine run ([raw/conformance-run.md](raw/conformance-run.md)). Three rows are PASS (partial) and two are N/A-1A, by written justification below. Four residuals are recorded. Gate 1 (R-CON-8 bound) passes with the direct figure at **2 166 SLOC** (36 % of the bound). Gate 2 (containment rows) passes.

Eleven implementation defects were found by the suite and fixed before the final run; they are listed in §5 because they are the actual evidence that the suite tests something.

Reproduction: `deploy/provision.sh` on a fresh Debian 13 host, then `ab-conformance` as root. Each run appends to the durable allocator, policy and audit stores; rows that depend on prior state (approval consumption, quarantined identity count) are written to hold across re-runs.

## 1. Components delivered

| Crate | Role | Privilege |
|---|---|---|
| `ab-common` | JCS canonical JSON with closed member sets and limits; SHA-256 / Ed25519 envelopes (manifest-schema §3–5); request/manifest/binding schema validation; SEQPACKET + `SO_PEERCRED` wire; audit event constructor and sink | linked into all |
| `agentbound-policy` | file-backed resolver: registries, initiator bound to peer UID, `Auth_session ⊆ Auth_agent ∩ Task`, approval expiry/subject/monotonic replay store, budget narrowing, manifest assembly and signing, ownership-checked spool handoff | unprivileged user `agentbound-policy` |
| `agentbound-launch` | constructor: session-lifecycle §3 steps in order with reverse rollback ledger; `clone3` barrier, `CLONE_INTO_CGROUP`, `openat2(RESOLVE_BENEATH\|NO_SYMLINKS)`, `open_tree`/`mount_setattr`/`move_mount`, `pivot_root`, proc after pidns, no sysfs, `close_range`, `TimeoutStopUSec` at `StartTransientUnit`, loginuid, bounding/ambient drop before `setresuid`, five cap sets verified zero, `no_new_privs`, seccomp `socket()` family filter, host-side `/proc/<pid>/status` check before signing | root, transient |
| `agentbound-lifecycle` | identity allocator (SQLite WAL `synchronous=FULL`, hash-chained, CAS state sequence, 24 h quarantine floor), pidfd holder, scope observer, §5 termination with F-3/F-4/F-5, cleanup with residue scan, seal, §8 restart reconciliation | root daemon |
| `agentbound-audit` | append-only hash-chained event store, `event_id` dedup, closed per-kind detail schema, capacity/loss counter, query | unprivileged user `agentbound-audit` |
| `agentbound` | operator CLI | initiator's own UID; may invoke the constructor via sudoers rule only |
| `ab-conformance` + `probe/probe.sh` | this suite: host-side driver plus an in-session busybox probe | root (driver) / session identity (probe) |

Wire formats, versioning and the 1A spool handoff are defined in `docs/architecture/component-wire-formats.md` 0.1 (fills component-interfaces §10 without changing frozen boundaries).

## 2. Row register

Rows are the 1A rows of `docs/architecture/test-catalogue.md`. Sub-rows (`.x`) are the suite's decomposition. Evidence text is the suite's recorded observation, abbreviated; the raw table has the full strings.

### D-* deployment scenarios

| Row | Status | Evidence |
|---|---|---|
| D-01 | **PASS** | alice's request → `launchrec:redwood-analysis-*` → `session.activated`; status `active`/`in-use`; CLI returns on activation report (≈70 ms after exec) |
| D-02 | **PASS (partial)** | No observe/attach interface exists at 1A: the descriptor allowlist is stdin/stdout/stderr projected to `/dev/null` and an identity-owned `console.log`; `status` is by digest and returns no cross-principal data. The deny-with-`operation.denied` half needs the 1B attach path |
| D-03 | **PASS** | probe rows T-6.1-007/.etc/.sibling: `/var/lib/agentbound`, `/etc/agentbound` absent from the session root; another identity's workspace file not writable |
| D-04 | **PASS** | host view: zero processes of the session UID outside its scope; probe rows T-6.1-001/002/004/005/009/010, T-6.2-001/002/006 |
| D-05 | **PASS** | two runtimes (`runtime:scripted-loop` `/bin/sh loop.sh`, `runtime:probe` `/bin/sh probe.sh`) and `runtime:sh` share the identical construction, identity range, scope naming and audit chain; the invocation profile is digested into the launch binding |
| D-06 | **PASS** | scope held 64 processes (init + shell + 62 fan-out survivors at `TasksMax=64`) |
| D-07 | **PASS** | double-fork orphan (`(sleep 1000 &)`) is in the scope; at termination `cgroup_procs_remaining=[]`, credential scan outside scope empty |
| D-08 | **PASS** | terminate with 64 descendants → `cleaned/sealed`; evidence object: `freeze_written`, `sigterm_sent`, `cgroup_kill_written`, `init_pidfd_exited`, procs empty, 2.0 s |
| D-11 | **PASS** | F-C-03/07/09 below: no runnable session, identity `reclaiming`→`quarantined`, no scope left, `session.construction_failed` with ledger and rollback list |
| D-15 | **RESIDUAL** | child delegation needs a delegation interface (1B gateway); at 1A a session cannot reach any component socket (seccomp: only `AF_UNIX`; netns private; `/run/agentbound` not projected) so no delegation path exists to be narrowed. Re-run at 1B |

### T-6.1 cross-session isolation (in-session probe)

| Row | Status | Evidence |
|---|---|---|
| T-6.1-001 | **PASS** | `/proc` shows ≤8 pids, all with our comm names; `/proc/1/environ` (session init) unreadable |
| T-6.1-002 / -010 / -011 | **PASS** | `kill -0` on host-range pid and on pid 300 (exists on host) → ESRCH; host pids unaddressable so `pidfd_open`/`process_vm_readv` have no target (private pidns) |
| T-6.1-003 | **N/A-1A** | no PTY projected; see D-02 |
| T-6.1-004 / -012 | **PASS** | `/run/agentbound` absent; netns private (only `lo`); seccomp `socket(AF_INET)` → EPERM proven by the constructor's own step-7 check |
| T-6.1-005 | **PASS** | private ipcns; no `/dev/shm` |
| T-6.1-006 | **PASS** | root tmpfs and `/tmp` are per-session; workspace file of another UID not writable (`.sibling`) |
| T-6.1-007 | **PASS** | private paths absent (`.etc`, `.sibling`) |
| T-6.1-008 | **PASS** | environment is exactly the catalogue profile's `env` list (constructor passes no inherited environment); startup = `execve` of the profile argv from `/workspace` |
| T-6.1-009 | **PASS** | fds at start: `0 1 2` (+ the shell's script fd); constructor's own leak check (`fds ...` status line) fails construction on anything else |
| T-6.1-013 | **N/A-1A** | no gateway at topology `none` |

### T-6.2 privilege and boundary (in-session probe)

| Row | Status | Evidence |
|---|---|---|
| T-6.2-001 | **PASS** | no cgroupfs mounted; `cgroup.procs` write ENOENT |
| T-6.2-002 | **PASS** | `mount tmpfs` denied (no `CAP_SYS_ADMIN`, cap sets zero); only `lo` |
| T-6.2-003 | **PASS** | setuid copy on `nosuid` tmpfs runs as the identity; `NoNewPrivs=1` |
| T-6.2-004 | **PASS** | `CapEff/Prm/Bnd/Amb/Inh` all zero (verified in-child and host-side before signing) |
| T-6.2-005 | **PASS** | orphan spawned and later killed with scope (D-07) |
| T-6.2-006 | **PASS** | `mount -t proc` denied; pid of probe ≤4 (private pidns; proc mounted after pidns) |
| T-6.2-007 | **PASS** | `/image` read-only (EROFS), root tmpfs write denied (mode 755 root-owned), workspace writable, session dir removed at cleanup, workspace root group reset to durable owner (`root:root 2770`) |
| T-6.2-008 | **PASS (partial)** | busybox `sh` is the only interpreter in the image; no package loader exists. Re-run with a real runtime at 1B |
| T-6.2-009 | **PASS** | `/sys/class/net` ENOENT: no sysfs mounted |

### T-6.5 request and interface integrity

| Row | Status | Evidence |
|---|---|---|
| T-6.5-001 | **PASS** | unknown member (`uid`) → `request_schema` "unknown-member"; duplicate member → `duplicate-member` |
| T-6.5-002 | **PASS** | depth 10 → `depth-limit` (limit 4); 20 kB → `size-limit` (16 kB) |
| T-6.5-003 | **PASS** | catalogue source `../../../etc` → step 3 `mount_source_resolve` (`openat2 RESOLVE_BENEATH` EXDEV); constructor fails closed |
| T-6.5-004 | **PASS** | two concurrent constructors for one authorization: exactly one activation, one refusal (`O_EXCL` lease) |
| T-6.5-006 | **PASS** | `schema_version` v0.0 → `version` |
| T-6.5-007 | **PASS** | request-level `mount` member → unknown-member (requests cannot name mounts) |
| T-6.5-009 | **PASS** | allocator latest states: `quarantined` only (plus the one active), never `free` reissued within the run |
| T-6.5-010 | **PASS** | initiator credential not bound to peer UID → `initiator_unauthenticated`; alice calling `reserve_identity` on lifecycle → `peer_not_permitted` |

### T-6.6 derivation

| Row | Status | Evidence |
|---|---|---|
| T-6.6-001 | **PASS** | unknown principal; resource outside `Auth_agent ∩ Task` → `authority_exceeded`; ≥15 `session.rejected` events naming `failed_input` |
| T-6.6-002 | **PASS** | expired approval; stale sequence; and a valid approval accepted once then `approval_replayed` on second presentation (durable across runs) |
| T-6.6-003 | **PASS** | task requiring one approval, none given → `approval_missing` |
| T-6.6-004 | **PASS** | scheduler credential without owner → `scheduled_without_owner`; with owner → manifest `actors.owner = human:alice` |
| T-6.6-005 | **PASS** | `budget.pids` above catalogue limit → `budget_exceeds_policy` |
| T-6.6-006 | **PASS** | unknown runtime |
| T-6.6-008 | **PASS** | identifier grammar violation |

### T-6.8 revocation behaviours

| Row | Status | Evidence |
|---|---|---|
| T-6.8-001 | **PASS** | disabled initiator rejected at request; `initiator_disabled` signal on a live session → `terminate` |
| T-6.8-002 | **PASS** | `approval_expired` → `quiesce` |
| T-6.8-003 | **PASS** | `authority_revoked` → `terminate` → `cleaned/sealed` |
| T-6.8-004 | **PASS** | `catalogue_withdrawn` → `quiesce` |
| T-6.8-005 | **PASS** | `task_cancelled` → `terminate` |
| T-6.8-006 | **PASS** | `policy_service_unavailable` → `continue-degraded`, `session.degraded` with compensating control named |
| T-6.8-007 | **PASS** | `reclassification` → `quiesce`; scope `frozen 1` observed (F-T-02) |
| T-6.8-011 | **PASS** | audit degraded below stop threshold → `continue-degraded` |
| T-6.8-012 | **PASS** | daemon SIGKILLed and held down: session processes remain contained (3 in scope), no lifecycle authority available; on restart `session.recovery_reconciled` (`contained-and-held` — live evidence without held pidfd → `cgroup.kill`, `termination-incomplete`), then poll loop completes cleanup |
| T-6.8-013 | **PASS** | manifest with `continue-degraded` for a trigger the policy forbids → `continue_degraded_not_permitted` at derivation |
| T-6.8 audit | **PASS** | four `session.revocation_received`, `session.degraded`, `session.quiesce_started` on one record |

### T-6.9 resource bounds

| Row | Status | Evidence |
|---|---|---|
| T-6.9-001 | **PASS** | fan-out of 400 stops at 63 visible (`TasksMax=64` from manifest `pids`) |
| T-6.9-002 | **PASS** | fd exhaustion stops at `RLIMIT_NOFILE` from manifest |
| T-6.9-003 | **PASS (partial)** | `MemoryMax`/`CPUQuotaPerSecUSec` installed as scope properties from enforced limits (visible in `memory.max`); enforcement is the kernel's, not exercised by an exhaustion run |
| T-6.9-004 | **PASS** | 100 MB `dd` into the 16 MB root tmpfs → ENOSPC |
| T-6.9-007 | **PASS** | audit store: chain verified on daemon start, `lost=0`, 1 014 events by end of run |

### F-C constructor faults

| Row | Status | Evidence |
|---|---|---|
| F-C-03 | **PASS** | intent source replaced by a symlink to `/etc` → `openat2` ELOOP → step 3 `mount_source_escape`; rollback `cgroup.kill → scope stopped → identity reclaiming` |
| F-C-07 | **PASS** | crash before commit at step 7 → child killed and reaped, scope stopped, identity reclaiming → quarantined after clean scan; nothing signed |
| F-C-09 | **PASS** | crash after commit at step 8 → record carries `session.launch_record_committed` + `session.construction_failed`; identity quarantined; scope gone |

### F-T termination faults

| Row | Status | Evidence |
|---|---|---|
| F-T-02 | **PASS** | quiesce freeze → `frozen 1` observed on `cgroup.events` |
| F-T-03 | **PASS** | `sigterm_sent`, `init_pidfd_exited` in evidence |
| F-T-04 | **PASS** | `cgroup.kill` written without waiting for `frozen 1` (F-4 amendment), procs empty, pidfd exited |
| F-T-05 | **PASS** | by construction: `termination-incomplete` retained when procs remain (exercised by T-6.8-012's recovery path) |
| F-T-08 | **PASS** | unmount results classified (`not-a-host-mount` — session mounts died with the namespace); session dir removed; workspace root retained |
| F-T-10 | **PASS** | identity `quarantined` after seal; never `free` in the run |
| F-T-11 | **PASS** | `session.cleanup_completed`, `session.identity_released`, `session.sealed` on every terminated record |

## 3. R-CON-8 SLOC report (Gate 1)

Tool: `tokei 13.0.0-alpha.8`, code lines only, `-t Rust` for source figures. Counted on the VM at commit `8752b76`.

| Figure | Value | Notes |
|---|---|---|
| 1. Direct privileged SLOC (`agentbound-launch` 423 + `agentbound-lifecycle` 733 + `ab-common` 1 010) | **2 166** | ≤ 6 000: **PASS**. `ab-common` is included in full although only its json/sig/schema/wire/envelope modules are linked privileged; no gateway authentication path exists at 1A |
| 2. Generated SLOC | 0 | no build-script or macro-generated source in the workspace |
| 3. Transitive dependency SLOC in privileged processes | ≈1 200 000 (tokei total over the 52 resolved crates, excluding tests/benches/examples) | dominated by `libsqlite3-sys` bundled SQLite (≈520 000 C) and `libc` (≈133 000). Rust-only excluding SQLite ≈680 000. Proc-macro crates (`syn`, `serde_derive`, `quote`, `proc-macro2`) are build-time only but counted |
| 4. Configuration/rule SLOC | 34 | three systemd units (11+11+12 lines), one sudoers line, one tmpfiles line; seccomp filter is 20 BPF instructions built in code |
| 5. Memory-unsafe-language SLOC | ≈520 000 | the bundled SQLite amalgamation in `libsqlite3-sys` (justification: allocator durability store; alternative is a hand-written WAL store — deferred to WP3 review) |
| 6. Gateway core (unbounded) | 0 | none at 1A |

**Honesty note on figure 1.** The Rust in this workspace is written densely (727 of 2 298 lines exceed 100 columns; average 65 bytes/line in `ab-common`). `rustfmt` is not on the VM; a `rustfmt`-normalised count would be roughly 1.6–2× higher, i.e. ≈3 500–4 300, still under the bound. The ADR-0003 rule counts lines as committed, but the bound is a reviewability proxy and a formatted figure will be published when `rustfmt` is pinned.

## 4. Carry-ins and residuals

| Item | Disposition |
|---|---|
| Durable-ownership projection | **Decision taken at 1A:** workspace roots keep their durable owner (`root` in the reference deployment; `storage:finance-agent` per manifest `agent.durable_ownership_projection`); the session's GID is granted for the session's duration through the directory group plus setgid bit and reset at cleanup (`workspace-root-group-reset` in `session.cleanup_completed`). Files a session creates are owned by the ephemeral UID and remain readable (0644) but not writable by later identities (T-6.1-007.sibling). The projection to a durable storage principal (chown at seal) is **not implemented**; WP3 item |
| Allocator power-loss | **RESIDUAL:** WP1 identity-store spike covered crash rounds of the ID-1 design; WP2 did not run a power-loss (VM reset) round against `lifecycle.db`. Assumption: SQLite WAL with `synchronous=FULL` on ext4 gives durability of committed transactions across power loss; chain verification on open refuses a torn store. Recorded, to be exercised in WP3 |
| D-Bus scope observation | **Deviation recorded:** scope creation uses `busctl call ... StartTransientUnit`; observation uses cgroup files and the pidfd, not a `PropertiesChanged`/`UnitRemoved` subscription. WP1 established `UnitRemoved` arrives ~1.5 s after inactivity and is confirmation only; the pidfd + `cgroup.procs` view is the authoritative one used by §5. A libsystemd/zbus binding is a WP3 item; no requirement depends on the subscription at 1A |
| D7 items 6–9 | WP3 (unchanged) |
| Seccomp filter scope | The filter forbids `socket()` families other than `AF_UNIX` and kills on foreign arch; it is not a syscall allowlist. Sufficient for R-ISO at topology `none` (netns is private anyway); a broader filter is a 1B question |

## 5. Defects found by the suite and fixed

1. `PR_SET_PDEATHSIG SIGKILL` on the session init killed every session when the transient constructor exited (all containment rows initially failed).
2. `PR_CAPBSET_DROP` after `setresuid` → EPERM (needs `CAP_SETPCAP`); reordered, five sets verified zero.
3. Constructor leak check tripped on its own `/proc/self/fd` handle.
4. Workload inherited the constructor's stdout pipe → CLI blocked until session exit; now `/dev/null` + identity-owned console.
5. Workspace was chowned to the ephemeral UID (durable ownership lost); replaced by the group-grant model above.
6. Residue scan double-counted nested paths → `hold` on every cleanup.
7. Lifecycle poll ran only on client connections; `init_exited` and quiesce deadlines were never observed without traffic.
8. Audit closed schema used `kind` where events carry `event`; every event was rejected (`seq=0` at end of first run).
9. Audit closed schema for `session.construction_failed` listed a member the emitter never sends: every constructor-failure event was silently local-only. `Sink` now writes an `unforwarded` marker.
10. Runtime directory mode prevented unprivileged daemons from creating their sockets (`1770` + tmpfiles).
11. `busybox sh` aborts a `while` loop on `fork` EAGAIN at `TasksMax` — a probe defect that masked the pids row and, running after exhaustion, produced a false workspace-write failure.

## 6. Raw

- [raw/conformance-run.md](raw/conformance-run.md) — machine table from the final run (84 rows).
