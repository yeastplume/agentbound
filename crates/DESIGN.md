# WP2 implementation brief (derived from the frozen WP0 set; not normative)

This file condenses the contracts the 1A code implements so that the frozen
documents need not be re-read during development. Where this brief and a frozen
document differ, the frozen document governs and this brief is corrected.

## Crates

| Crate | Role | Privileged | Counts toward R-CON-8 |
|---|---|---|---|
| `ab-common` | JCS, digests, Ed25519 envelopes, keyring, schemas, SEQPACKET wire, audit event | no | yes (linked into launch and lifecycle) |
| `agentbound-policy` | file-backed resolver stub; signs authorization manifests | no (service UID) | no |
| `agentbound-launch` | constructor: verify manifest → reserve identity → steps 1–9 → hand over → exit | yes, short-lived | yes |
| `agentbound-lifecycle` | allocator, pidfd holder, D-Bus scope observer, §5 termination, reconciliation | yes, long-running | yes |
| `agentbound-audit` | SEQPACKET receiver, dedup by event ID, hash-chained append-only store | no | no |
| `agentbound` | CLI | no | no |
| `ab-conformance` | 1A test driver run as root on VM 110 | test only | no |

## Service identities and sockets (component-interfaces §2)

Provisioned users: `agentbound-policy`, `agentbound-audit` (1A). Launch and
lifecycle run as root (privileged). Sockets under `/run/agentbound/`:

| Socket | Listener | Accepts peers | Mode |
|---|---|---|---|
| `policy.sock` | policy | CLI UID(s) from config | 0660, group `agentbound-cli` |
| `lifecycle.sock` | lifecycle | UID 0 (launch) and configured CLI UID for status/terminate | 0600 / 0660 |
| `audit.sock` | audit | root (launch, lifecycle), policy UID | 0660 |

`SO_PEERCRED` read at accept; UID checked against config before parsing.
Path permissions are defence in depth only.

## Message flow (1A)

```
agentbound ──request──▶ policy ──(verify request, derive, sign)──▶ manifest+envelope
agentbound ──manifest+envelope──▶ launch (spawned by CLI as root via sudo/setuid provisioning; §3.2 requires
                                          policy→launch delivery, see wire-format doc §5 for the 1A shape)
launch ──reserve(manifest digest, authz id)──▶ lifecycle ──▶ {allocation_id, uid, gid}
launch: steps 1–9; commits binding via lifecycle (`commit_binding`), hands pidfds via SCM_RIGHTS (`register_session`)
launch ──activated | construction_failed──▶ lifecycle; launch exits
lifecycle: watches scope (D-Bus PropertiesChanged/UnitRemoved) + init pidfd; terminate/quiesce on request or signal
agentbound ──status/terminate──▶ lifecycle
all ──events──▶ audit
```

## Lifecycle operations (wire-format doc §3)

`reserve_identity`, `commit_binding`, `register_session`, `report_activation`,
`report_construction_failed`, `terminate`, `quiesce`, `status`, `list`,
`revocation_signal` (1A local revocation cases). Every request has a scoped
idempotency key; same key + same body → original result; same key + different body → `conflict`.

## Identity state machine (identity lifecycle §4)

`free → allocated → in-use → reclaiming → quarantined → free`.
- `allocated`: durable append before any UID install; bound to authz id + manifest digest.
- `in-use`: set by `report_activation` (or recovery finding live scope).
- `reclaiming`: set on termination, construction rollback, or crash.
- `quarantined`: only after reclamation condition (§4.1) holds; floor 24 h after seal (config `quarantine_floor_s`, may only be raised).
- `free`: after quarantine floor and audit seal.
Allocation record fields: §3.3 table (host/boot id, authz id, trace, principal, uid, gid, groups, state, seq, actor, ts, scope/pidns, managed domain, reclamation evidence).
CAS on (allocation_id, state_seq). Store: SQLite WAL, `synchronous=FULL`, append-only rows with prev-hash chain (WP1 ID-1 design).

## Reclamation condition (§4.1)

1. `cgroup.procs` empty **and** init pidfd readable (exited); 2. host `/proc` credential scan for the UID/GIDs finds nothing outside scope (else `identity.scope_escape_suspected`); 3. scan the closed five-path managed domain (workspace image, runtime tmpfs, launch-record store, allocator store, audit spool) for UID/GID-owned objects, remove per policy, record; 4. IPC namespace destroyed; 5. grants confirmed revoked (none at 1A topology `none`); 6. ACL entries naming the UID removed (durable-ownership carry-in).

## Construction steps (session-lifecycle §3) and rollback

| # | Step | Rollback |
|---|---|---|
| 0 | verify envelope, freshness, keyring; validate manifest; CAS ownership lease; `reserve_identity`; `StartTransientUnit` scope with `Delegate=yes`, `TimeoutStopUSec`, `PIDs`/`Memory`/`CPU` props | release lease; allocation → reclaiming |
| 1 | `clone3(NEWNS|NEWPID|NEWIPC|NEWUTS|NEWNET|PIDFD|INTO_CGROUP)` child blocks on pipe barrier | kill via pidfd, reap, stop scope |
| 2 | child: `mount(/, MS_REC|MS_PRIVATE)` | kill child |
| 3 | parent: `openat2(RESOLVE_BENEATH|NO_SYMLINKS|NO_MAGICLINKS)` from catalogue base fds; `open_tree(OPEN_TREE_CLONE)`; `mount_setattr` | close fds |
| 4 | child: tmpfs root via `fsopen/fsmount`, `move_mount` trees, `pivot_root`, `umount2(oldroot, MNT_DETACH)` | kill child |
| 5 | child: mount `proc` (nosuid,nodev,noexec) after pidns; fresh `sysfs` only if needed, after netns; never inherited | kill child |
| 6 | child: `close_range(3, ~0, 0)` except allowlist; verify via `/proc/self/fd` before proc is hidden — prove no memfd/SCM_RIGHTS path | kill child |
| 7 | child: `setgroups(exact set)`, `setresgid`, `setresuid`, verify; `PR_SET_NO_NEW_PRIVS`; drop bounding set + ambient; `PR_SET_CHILD_SUBREAPER`; loginuid write; seccomp socket-family filter with `TSYNC` | kill child; identity → reclaiming |
| 8 | commit binding (lifecycle appends + fsyncs); grants usable (none at 1A) | failure record |
| 9 | release barrier; child execs runtime from invocation profile; parent hands pidfds + ledger to lifecycle, emits `session.activated`, exits | kill/reap; reverse cleanup |

The child is a minimal init (PID 1 in pidns, subreaper) that forks the workload and reaps.
Steps 0–3, 8, 9(parent) are parent-side; child-side steps report over a status pipe and the parent verifies each before proceeding.

## Termination protocol (session-lifecycle §5, with F-3/F-4/F-5)

1 deny admission (gateway; none at 1A) → 2 `cgroup.freeze=1` → 3 thaw + `SIGTERM` to init via pidfd, bounded → 4 `cgroup.freeze=1` and `cgroup.kill=1` **without waiting for `frozen 1`** → 5 bounded wait for `cgroup.procs` empty and init pidfd; host credential scan; else `termination-incomplete` → 6 grant records → 7 broker → 8 unmount session fs → 9 remove gateway socket → 10 identity → reclaiming (then condition, quarantine) → 11 seal.
No grant or identity release before 5. D-state member ⇒ `termination-incomplete`, identity held.

## Session states (§2.1)

`requested, authorized, constructing, active, quiescing, termination-incomplete, terminated, cleaned/sealed, rejected, construction-failed, aborted, degraded(overlay)`.
Never back to `active`.

## Audit events (§8) emitted at 1A

policy: `session.requested`, `session.authorized`, `session.rejected`, `derivation.failed_input`
launch: `session.construction_started`, `session.construction_step`, `session.launch_record_committed`, `session.activated`, `session.construction_failed`
lifecycle: `identity.allocated`, `identity.state_changed`, `identity.scope_escape_suspected`, `session.quiesce_started/completed`, `session.revocation_received`, `session.termination_started`, `session.terminated`, `session.termination_incomplete`, `session.cleanup_completed`, `session.identity_released`, `session.sealed`, `session.recovery_reconciled`, `session.orphan_detected`
Every event carries R-AUD-1 fields (see `ab_common::audit::event`).

## Freshness (component-interfaces §4.1)

`issued_at` ≤ now+30 s; manifest consumed ≤ 10 min; binding committed ≤ 60 s; clock unreadable ⇒ fail closed.

## Catalogue (policy stub, file-backed, `/etc/agentbound/catalogue.json`)

Principals, tasks, runtimes (artifact digest + invocation profile argv/env), mount sources (host base path + id; resolved by launch via openat2 from a base fd), mount targets, resource limit defaults, initiator credential refs, approvals (with expiry + per-key sequence), agent authority sets, task permissions. Policy computes `Auth_session = Auth_agent ∩ Task ∩ Initiator` and rejects anything outside.

## 1A limits

Topology `none` only; no gateway, no grants; workloads `/bin/sh` and scripted loop; `user` namespace `inherited` (Profile U, root-constructed); LSM `mac_context: null`; Landlock deferred to 1B unless time permits (recorded as residual if absent).
