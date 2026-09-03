# Phase 1 Test Catalogue

**Version:** 0.4  
**Status:** Draft for WP0 review  
**Date:** 28 August 2026  
**Governs:** Agentbound milestones 1A–1D  
**Companion documents:** [requirements](phase-1-requirements.md), [traceability matrix](traceability-matrix.md), [session lifecycle](session-lifecycle.md), [Phase 1 plan](../plans/phase-1-reference-implementation.md), and [technical report](../papers/technical-report.md)

---

## Revision history

- **0.1** — Initial WP0 pre-registration.
- **0.2** — Gateway-free 1A form (`channel_topology: none`) applied to 1A tests; load profiles, correlation deadlines, and repetition seeds fixed; T-6.4 rows rewritten for the local-socket corpus; one-connection-per-process rule in T-6.4-007; control-arm column declared to be populated by ADR-0003; identifier terminology aligned.
- **0.3** — Control-arm column points to the committed per-ID register as a 1D prerequisite.
- **0.4** — T-6.8-006 narrowed to policy-service outage; T-6.8-011–013 added (audit degradation, lifecycle-daemon outage, forbidden degraded mapping).


## 1. Purpose and status

This catalogue is the WP0 pre-registration of the Agentbound Phase 1 test
population. It fixes the test identifiers, atomic test intent, expected
preventive result, required evidence, repetition rules, and source coverage
before implementation results are available. It MUST be read with the
normative requirements and lifecycle specification.

This catalogue is a test definition, not a fixture or command specification.
Fixtures, harness commands, workload implementations, test runners, and CI
wiring are explicitly out of scope here; they are WP1 outputs. A later test
implementation MUST preserve the IDs and assertions in this catalogue, or a
recorded WP0 revision MUST explain the change.

The applicable Profile U invariants are **1, 2, 3, 6, 7, 10, 11, 12, 13, 14,
15, 17, 19 (protected-object subset), 20, 21, and 22**. Invariants 4, 5, 8,
9, 16, and 18 are not applicable to Profile U and are not test targets here.
An applicable invariant that has not reached its milestone is **not evaluated**,
not not-applicable.

## 2. Fixed Phase 1 decisions

A gateway-enabled Linux session (milestone 1B onward, `gateway.channel_topology:
local-socket`) MUST use only the local-socket topology; a 1A session uses
`none` and has no channel at all. Every session MUST have no network interface.
Under `local-socket` the only gateway channel MUST be exactly one explicitly
bind-mounted, single-purpose `AF_UNIX` `SOCK_SEQPACKET` gateway socket. The
gateway MUST enable `SO_PASSCRED` and authenticate every received packet using
its `SCM_CREDENTIALS`, mapping that evidence to the execution identity and
immutable launch record. `SCM_RIGHTS` MUST be rejected on that socket. No veth,
mTLS, network namespace interface, firewall topology, or alternate network
channel is in Phase 1.

A privileged `agentbound-lifecycle` daemon, rather than a systemd-invoked
helper, MUST own termination, reclamation, and post-launch lifecycle state. It
MUST subscribe to systemd D-Bus signals and retain session pidfds. Systemd MAY
report scope state but MUST NOT bypass the daemon's serialized lifecycle
interface. Milestones are only **1A**, **1B**, **1C**, and **1D**.

Every table's **Control arm** field is intentionally blank. ADR-0003 MUST fill
it before control-arm results exist with exactly `identical`,
`substrate-equivalent`, or `not directly comparable`; this catalogue MUST NOT
prejudge the classification.

## 3. Conventions and common assertions

Every 1A test runs against a `gateway.channel_topology: none` authorization
manifest; the constructor MUST refuse `local-socket` until the gateway component
exists. Tests marked 1A/1B run once per form. Steps or assertions that concern
gateway state are recorded as `not applicable` under `none`, never as passed.

RFC 2119 terms are normative. A test identifier is permanent. `D-xx` denotes a
plan §5 demonstration; `T-6.x-nnn` an atomic plan §6 adversarial test;
`F-C-nn` and `F-T-nn` construction and termination fault tests respectively.
A row can name multiple requirements only where one atomic attempt necessarily
asserts all of them.

**Privileges** use only these labels: **USP** (unprivileged session process),
**CSSP** (concurrent same-principal session), **COSP** (concurrent
other-principal session), **ANI** (authenticated non-admin initiator), and
**RR** (remote response). “Evidence” is a required retained artefact class,
not an implementation filename.

All denials MUST return a safe diagnostic containing the applicable requirement
ID (or stable policy-rule ID), authorization ID, launch-record digest, and trace ID. Each denial MUST
also emit `operation.denied` or the more specific lifecycle/derivation event
named by the row. No diagnostic MAY disclose another session's identifiers.

| Evidence shorthand | Minimum retained content |
|---|---|
| `LR` | Signed launch record, manifest and binding digests, allocation record |
| `LC` | Ordered lifecycle events, rollback ledger, daemon pidfd/systemd-D-Bus observations |
| `KA` | Kernel/process evidence: PID namespace, cgroup, audit, descriptor or mount evidence |
| `GA` | Gateway packet authentication, operation decision, upstream trace and connection record |
| `AR` | Audit correlator reconstruction and loss-counter snapshot |
| `DR` | Stable denial response plus requirement/policy ID, launch record, and trace ID |

## 4. Gate pass rules and repetition

Each enumerated bypass attempt in §§6.1–6.7 and the local-socket-relevant
§6.4 inventory MUST reach its stated preventive outcome in **100%** of pinned
repetitions. A reproducible bypass fails its gate. Unexplained nondeterminism
is a failure pending investigation; it MUST NOT be labeled flaky or removed by
retry. Every attempt, including retry, failure, and infrastructure abort, MUST
be retained and linked by test ID, run ID, input digest, and idempotency key.

Repetition counts, kernel/systemd/LSM versions, interface inventory, and test
seeds are pinned here: bypass and fault tests run 10 repetitions each; the seed for each repetition is `SHA-256(test_id || repetition_index)` truncated to 64 bits. Each row executes its pinned count; where a
row participates in the nominal profile it MUST execute at least `N` times.
An attempt is passing only if its preventive result, required audit event, and
all required evidence are present. A missing required audit record is a failed
attempt unless the manifest declares and records its fail-closed audit-loss
behavior.

**Gate 1** passes only if all applicable request, derivation, construction, and
`F-C-*` tests pass, no partial session/grant/resource survives, and the
reviewability bound holds. **Gate 2** passes only if every isolation,
delegation, descendant, and `F-T-*` test passes. **Gate 3** passes only if all
local-socket bypass, gateway authentication, credential, staging, and remote
attribution tests pass. **Gate 4** passes only if deterministic-behaviour and
diagnostic assertions below pass, all evidence is retained, measurement and
loss counters are present, and each applicable invariant has its required
result record. Milestone 1D additionally requires ADR-0003's frozen
classification table and no post-result reclassification.

### 4.1 Gate 4 deterministic and diagnostic assertions

For `N` identically pinned runs with the same canonical request, policy,
catalogue, workload seed, and fault schedule, launch records MUST be bytewise
identical after excluding only: authorization ID, session/trace ID, allocation
record/UID, host and boot ID, wall/monotonic timestamps, pidfd/PID values,
systemd unit instance, cryptographic signature nonce, and explicit run ID.
The derivation output, manifest digest, selected runtime, grants, resource
projection, construction-step sequence, and terminal outcome MUST be identical.

For identically pinned termination schedules, lifecycle event names, order,
state transitions, cleanup-result classes, and denial classes MUST be identical
modulo the listed nondeterministic fields. Every denial MUST carry the
requirement/policy ID, authorization ID, launch-record digest, and trace ID; every state transition
MUST carry its authorized actor and causation ID. The test evidence MUST show
that the lifecycle daemon, not a systemd-invoked helper, consumed systemd D-Bus
signals and used the retained pidfd evidence.

## 5. Attribution-completeness pre-registration

An **atomic effect ID** is an idempotency-keyed workload event with a monotonic
sequence number. The ontology classes are: (a) local object create or modify
within the session world; (b) process lifecycle events; and (c) gateway
operations. The ground truth for **each** class is the instrumented workload
log containing atomic effect ID, class, sequence number, idempotency key,
process identity, session/trace ID, and intended outcome.

Denied operations are in-scope effects and MUST be included. Retries sharing an
idempotency key are deduplicated to one intended effect; their attempts remain
retained as evidence. Effects outside those three classes, effects after a
workload's declared end marker, and uninstrumented host-maintenance actions are
excluded. A reconstruction is correct only when it recovers the full chain
`initiator → agent → session → process → effect`, matches the ground-truth
class/outcome/idempotency key, and reaches the correlator by the correlation
deadline.

The correlation deadline is **30 s** after the workload end marker for nominal
runs and **120 s** for overload runs. Let
`G` be all in-scope, deduplicated ground-truth effects whose deadlines expire,
and `C` those correctly reconstructed full chains. Attribution completeness is
`|C| / |G|`. It MUST be at least **99%** over all classes under the nominal
profile and **100%** for the finite gateway-operation corpus in every nominal
run. Loss under overload MUST be reported with counters; it MUST NOT silently
change the denominator.

| Profile | Fixed workload mix | Concurrent sessions | Operation rate | Duration | Repetitions |
|---|---|---:|---:|---:|---:|
| NOMINAL | per session: 200 local create/modify, 20 process lifecycle events (fork/exec/exit), 10 Git gateway operations (8 `push-staging-ref` permitted, 2 denied); 230 effects | 8 | 20 effects/s aggregate | 300 s | `N = 10` |
| OVERLOAD | per session: 2 000 local create/modify, 200 lifecycle events, 50 gateway operations (40 permitted, 10 denied) | 32 | 500 effects/s aggregate | 300 s | `N = 5` |

## 6. Demonstration catalogue (plan §5)

| ID | Milestone | Inv. | Requirement IDs | Privilege | Interface exercised | Expected outcome | Evidence | Control arm |
|---|---|---|---|---|---|---|---|---|
| D-01 | 1A | 1,2,3,7 | R-ID-1..4,R-CON-1 | ANI | request, derivation, constructor | active authorized Finance session; `session.activated` | LR,LC,KA | |
| D-02 | 1A | 2,17 | R-ID-2,R-ISO-1 | ANI | observe/attach status and PTY | deny Bob absent grant; `operation.denied` | LR,DR,LC | |
| D-03 | 1A | 17 | R-ISO-1..2 | COSP | private state interfaces | cross-principal reads/influence denied | KA,DR,AR | |
| D-04 | 1A | 12,17 | R-ISO-1..4 | CSSP | proc, signals, ptrace, sockets, files, credentials | every sibling action denied | KA,DR,LC | |
| D-05 | 1A | 15 | R-CON-3..5 | USP | shell/runtime substitution | same identity/boundary/scope/audit chain | LR,KA,AR | |
| D-06 | 1A | 12 | R-ISO-3..4 | USP | child/grandchild process tree | descendants remain contained | KA,LC | |
| D-07 | 1A | 12 | R-ISO-3..4 | USP | double-fork and daemonize | still supervised and reaped | KA,LC | |
| D-08 | 1A | 12,21 | R-ISO-4,R-LC-1..4 | USP | lifecycle termination (topology `none` at 1A; `local-socket` re-run at 1B) | descendants stop before ordinary grant closure | LC,KA,GA(1B) | |
| D-09 | 1B | 10 | R-GW-1..4 | USP | direct protected-service access | deny/fail closed; no network interface exists | KA,DR,GA | |
| D-10 | 1B | 11,13 | R-GW-3..4,R-AUD-2 | USP | typed Git operation | full attributed gateway operation | LR,GA,AR | |
| D-11 | 1A | 7 | R-CON-1 | ANI | constructor faults | no runnable session or usable credential | LC,KA,LR | |
| D-12 | 1B | 13 | R-AUD-1..3 | USP | local and gateway effect generation | reported completeness meets §5 metric | AR,KA,GA | |
| D-13 | 1B | 19 | R-GW-5 | USP | Git staging adapter | staging allowed; main/cross-session/forged trace denied | GA,DR,AR | |
| D-14 | 1C | 22 | R-GW-8 | USP | inference binding members | unapproved changes refused and audited | GA,DR,AR | |
| D-15 | 1A | 6 | R-ISO-5,R-RES-3 | USP | child delegation | strictly narrower authority; no recovery path | LR,KA,DR | |
| D-16 | 1A/1B/1C | 21 | R-LC-1..4 | ANI | active revocation triggers | declared action, grant effect, and audit occur | LC,GA,AR | |
| D-17 | 1D | applicable | R-REQ-4 | USP | ADR-0003 registered set | run only per frozen ADR classification | ADR-0003 evidence | |

## 7. Adversarial catalogue (plan §6)

### 7.1 Process and session interference

| ID | Milestone | Inv. | Requirement IDs | Privilege | Interface exercised | Expected outcome | Evidence | Control arm |
|---|---|---|---|---|---|---|---|---|
| T-6.1-001 | 1A | 17 | R-ISO-1 | CSSP | `/proc` enumeration, sensitive reads | deny; `operation.denied` | KA,DR | |
| T-6.1-002 | 1A | 17 | R-ISO-1 | CSSP | ptrace, process memory, signals | deny all operations | KA,DR | |
| T-6.1-003 | 1A | 17 | R-ISO-1..2 | CSSP | PTY discovery/attach | deny absent authorized attachment | KA,DR,LC | |
| T-6.1-004 | 1A | 17 | R-ISO-1..2 | CSSP | pathname and abstract Unix sockets | sibling sockets unreachable | KA,DR | |
| T-6.1-005 | 1A | 17 | R-ISO-1..2 | CSSP | shared memory and IPC | deny/unreachable | KA,DR | |
| T-6.1-006 | 1A | 17 | R-ISO-1..2 | CSSP | temp races and symlink attacks | no cross-session influence | KA,DR | |
| T-6.1-007 | 1A | 17 | R-ISO-1..2 | CSSP | runtime/workspace directories | deny private paths | KA,DR | |
| T-6.1-008 | 1A | 17 | R-ISO-1 | CSSP | environment/startup/shell injection | no sibling influence | KA,DR | |
| T-6.1-009 | 1A | 6,17 | R-CON-3,R-ISO-1 | CSSP | inherited/reopened FD, `SCM_RIGHTS`, procfd, memfd | no unauthorized FD capability | KA,DR | |
| T-6.1-010 | 1A | 17 | R-ISO-1 | CSSP | `pidfd_open`, `pidfd_send_signal` | deny/no usable host pidfd | KA,DR | |
| T-6.1-011 | 1A | 17 | R-ISO-1 | CSSP | `process_vm_readv/writev` | deny | KA,DR | |
| T-6.1-012 | 1A | 17 | R-ISO-1..2 | CSSP | abstract sockets across namespaces | deny/unreachable | KA,DR | |
| T-6.1-013 | 1A | 17 | R-ISO-1 | CSSP | gateway broker socket reuse | peer/session mapping rejects sibling use | GA,DR | |

### 7.2 Escape and persistence

| ID | Milestone | Inv. | Requirement IDs | Privilege | Interface exercised | Expected outcome | Evidence | Control arm |
|---|---|---|---|---|---|---|---|---|
| T-6.2-001 | 1A | 12 | R-ISO-3 | USP | cgroup migration | deny; scope unchanged | KA,DR | |
| T-6.2-002 | 1A | 12 | R-ISO-2..3 | USP | namespace manipulation | deny; boundary unchanged | KA,DR | |
| T-6.2-003 | 1A | 6,15 | R-CON-3..4 | USP | setuid/file-cap execution | no privilege gain | KA,DR | |
| T-6.2-004 | 1A | 6,15 | R-CON-3..5 | USP | capability/ambient recovery | deny/no recovered capability | KA,DR | |
| T-6.2-005 | 1A | 12 | R-ISO-3..4 | USP | daemonize/double-fork/orphan | contained and later reaped | KA,LC | |
| T-6.2-006 | 1A | 12,17 | R-CON-2,R-ISO-1 | USP | mount and procfs abuse | deny; host proc absent | KA,DR | |
| T-6.2-007 | 1A | 17 | R-ISO-1..2 | USP | persistence outside workspace | deny/no retained object | KA,DR,LC | |
| T-6.2-008 | 1A | 6,15 | R-CON-3..4 | USP | interpreter/package-loader escape | no boundary crossing | KA,DR | |

### 7.3 Credential recovery and reuse

| ID | Milestone | Inv. | Requirement IDs | Privilege | Interface exercised | Expected outcome | Evidence | Control arm |
|---|---|---|---|---|---|---|---|---|
| T-6.3-001 | 1B | 11 | R-GW-6 | USP | environment variables | no credential present | KA,AR | |
| T-6.3-002 | 1B | 11 | R-GW-6 | USP | files and procfs | no credential present | KA,AR | |
| T-6.3-003 | 1B | 11 | R-GW-6 | USP | inherited descriptors | no reusable credential | KA,AR | |
| T-6.3-004 | 1B | 11 | R-GW-6 | USP | child processes | no credential recovery | KA,AR | |
| T-6.3-005 | 1B | 11 | R-GW-3,R-GW-6 | USP | gateway socket broker | authenticated use only; no export | GA,DR | |
| T-6.3-006 | 1B | 11 | R-GW-6 | USP | logs/exceptions/crash output | no secret disclosure | KA,AR | |
| T-6.3-007 | 1B | 11,12 | R-GW-3,R-ISO-4 | USP | post-termination authority | deny; stale connection closed | GA,LC,DR | |
| T-6.3-008 | 1B | 11,17 | R-GW-3 | CSSP | replay from other session | peer credentials mismatch; deny | GA,DR | |

### 7.4 Local-socket gateway bypass and misuse

| ID | Milestone | Inv. | Requirement IDs | Privilege | Interface exercised | Expected outcome | Evidence | Control arm |
|---|---|---|---|---|---|---|---|---|
| T-6.4-001 | 1B | 10 | R-GW-1..2 | USP | `socket()` for INET, INET6, PACKET, NETLINK, VSOCK | seccomp deny `EPERM`; `boundary.socket_family_denied` | KA,DR | |
| T-6.4-002 | 1B | 10 | R-GW-1..2 | USP | interface/route enumeration and creation in session netns | no interface, no route; creation denied | KA,DR | |
| T-6.4-003 | 1B | 10 | R-GW-1..3 | USP | host pathname Unix sockets | `ENOENT`/`EACCES`; only gateway mount reachable | KA,DR | |
| T-6.4-004 | 1B | 10,17 | R-GW-2,R-ISO-2 | CSSP | abstract-namespace sockets of host and sibling sessions | `ECONNREFUSED`; abstract namespace isolated by netns | KA,DR | |
| T-6.4-005 | 1B | 10 | R-CON-3,R-GW-2 | USP | pre-opened connections and inherited descriptors | none inherited; allowlist verified at exec | KA,LR | |
| T-6.4-006 | 1B | 10,13 | R-GW-2..3 | USP | `SCM_RIGHTS` on the gateway socket | packet rejected; connection closed; `gateway.descriptor_transfer_rejected` | GA,DR | |
| T-6.4-007 | 1B | 11,13 | R-GW-3 | USP, CSSP | connected gateway descriptor inherited by a child, and passed to another session | establishing-PID mismatch closes connection; `gateway.process_mismatch` | GA,DR,AR | |
| T-6.4-008 | 1B | 13 | R-GW-3 | USP | packets with zero or multiple `SCM_CREDENTIALS`, forged fields | reject; connection closed | GA,DR | |
| T-6.4-009 | 1B | 13 | R-GW-3 | USP | PID reuse against the per-operation check | start-time/pidfd mismatch deny | GA,DR,AR | |
| T-6.4-010 | 1B | 10 | R-GW-1 | USP | `SOCK_STREAM`/`SOCK_DGRAM` connect to gateway path | `EPROTOTYPE`/refused; no operation admitted | KA,GA,DR | |
| T-6.4-011 | 1B | 10 | R-GW-4 | USP | gateway as tunnel/SSRF oracle: alternate destination, tenant/repo mismatch | typed adapter rejects argument | GA,DR | |
| T-6.4-012 | 1B | 10 | R-GW-4 | RR | upstream redirect or TLS identity mismatch | adapter refuses; `gateway.upstream_rejected` | GA,DR | |
| T-6.4-013 | 1B | 10,11,13 | R-GW-3..4 | CSSP | replay of another session's launch-record, trace, or grant identity | per-packet credential mismatch deny | GA,DR,AR | |
| T-6.4-014 | 1B | 11,21 | R-GW-3,R-LC-3 | USP | operation after committed revocation on established connection; new connection after revocation | next operation denied; new connection refused | GA,DR,LR | |
| T-6.4-015 | 1D | 10,11 | R-GW-1,R-GW-3 | guest root / guest unprivileged | control arm: vsock-path realization of T-6.4-001..014 and CID reuse after teardown | per ADR-0003 classification; stale CID mapping invalidated | GA,DR,ADR-0003 evidence | |

### 7.5 Constructor and request inputs

| ID | Milestone | Inv. | Requirement IDs | Privilege | Interface exercised | Expected outcome | Evidence | Control arm |
|---|---|---|---|---|---|---|---|---|
| T-6.5-001 | 1A | 7,14 | R-REQ-2..3 | ANI | unknown/duplicate/reordered/noncanonical fields | reject before privilege; `session.rejected` | LR,DR | |
| T-6.5-002 | 1A | 7 | R-REQ-2 | ANI | oversized/deep request | bounded reject | LR,DR | |
| T-6.5-003 | 1A | 7 | R-REQ-5..6 | ANI | traversal, symlink, mount substitution | reject/no host path use | LR,KA,DR | |
| T-6.5-004 | 1A | 7 | R-LC-1 | ANI | replay/concurrent duplicate | one result; no second scope/UID | LR,LC,DR | |
| T-6.5-005 | 1A | 7,14 | R-REQ-5 | ANI | policy/catalogue/filesystem TOCTOU | reject or serialized current decision | LR,KA,DR | |
| T-6.5-006 | 1A | 14 | R-ID-8,R-REQ-3 | ANI | forged/downgraded versions | fail closed and audit | LR,DR | |
| T-6.5-007 | 1A | 7 | R-REQ-2 | ANI | smuggled UID/cap/path/net/ns setting | reject forbidden field | LR,DR | |
| T-6.5-008 | 1A | 7,14 | R-REQ-1..3 | ANI | manifest/signature confusion | fail closed | LR,DR | |
| T-6.5-009 | 1A | 1,7 | R-ID-6..7,R-REQ-5 | ANI | stale/double allocated identity | deny/hold/quarantine correctly | LR,LC,DR | |
| T-6.5-010 | 1A | 2,3 | R-ID-2..4 | ANI | unauthenticated/wrong caller | reject before derivation | LR,DR | |

### 7.6 Bounded derivation

| ID | Milestone | Inv. | Requirement IDs | Privilege | Interface exercised | Expected outcome | Evidence | Control arm |
|---|---|---|---|---|---|---|---|---|
| T-6.6-001 | 1A | 3 | R-ID-3..4 | ANI | unauthorized principal/task | no session; failed-input audit | LR,DR | |
| T-6.6-002 | 1A | 3 | R-ID-4 | ANI | expired/revoked/replayed approval | no session; audit names input | LR,DR | |
| T-6.6-003 | 1A | 3 | R-ID-3..4 | ANI | conflicting/incomplete quorum | no session; derivation denial | LR,DR | |
| T-6.6-004 | 1A | 2,3 | R-ID-2..4 | ANI | scheduler without owner | no session; derivation denial | LR,DR | |
| T-6.6-005 | 1A | 3 | R-ID-5 | ANI | recipient grant exceeds agent | no session/excess authority | LR,DR | |
| T-6.6-006 | 1A | 3,14 | R-ID-3,R-REQ-5 | ANI | catalogue/runtime substitution | no session; audit version | LR,DR | |
| T-6.6-007 | 1A | 3,14 | R-ID-4,R-ID-8 | ANI | policy/version rollback | fail closed; audit | LR,DR | |
| T-6.6-008 | 1A | 1,2,3 | R-ID-1..4 | ANI | ambiguous identities | no session; failed-input audit | LR,DR | |

### 7.7 Monotonic delegation

| ID | Milestone | Inv. | Requirement IDs | Privilege | Interface exercised | Expected outcome | Evidence | Control arm |
|---|---|---|---|---|---|---|---|---|
| T-6.7-001 | 1A | 6 | R-ISO-5,R-RES-3 | USP | child mounts, FDs, grants, budgets and recovery paths | every axis non-increasing; recovery denied | LR,KA,DR | |

### 7.8 Active revocation and lifecycle

| ID | Milestone | Inv. | Requirement IDs | Privilege | Interface exercised | Expected outcome | Evidence | Control arm |
|---|---|---|---|---|---|---|---|---|
| T-6.8-001 | 1A | 21 | R-LC-1..4 | ANI | initiator disabled | declared action and audited transition | LC,AR | |
| T-6.8-002 | 1A | 21 | R-LC-1..4 | ANI | approval expiry | declared action and audit | LC,AR | |
| T-6.8-003 | 1A | 21 | R-LC-1..4 | ANI | authority revoked | declared action and audit | LC,AR | |
| T-6.8-004 | 1A | 21 | R-LC-1..4 | ANI | policy/catalogue withdrawal | declared action and audit | LC,AR | |
| T-6.8-005 | 1A | 21 | R-LC-1..4 | ANI | approver cancellation | declared action and audit | LC,AR | |
| T-6.8-006 | 1A | 21 | R-LC-2..4 | ANI | policy service unavailable | declared terminate/quiesce, or `continue-degraded` with no fresh-policy operation admitted and no new authority | LC,DR,AR | |
| T-6.8-007 | 1A | 21 | R-LC-3 | ANI | U reclassification request | fail closed, unchanged projection, policy audit | LR,DR,AR | |
| T-6.8-008 | 1B | 21 | R-LC-3 | ANI | Git grant withdrawn | new Git operation denied | GA,LC,DR | |
| T-6.8-009 | 1B | 21 | R-LC-3 | RR | gateway unavailable | declared behavior plus availability event | GA,LC,AR | |
| T-6.8-010 | 1C | 21,22 | R-LC-3,R-GW-8 | ANI | inference grant/binding revoked | new inference denied; declared action | GA,LC,DR | |
| T-6.8-011 | 1A | 21 | R-LC-2..4 | ANI | audit pipeline degraded below stop threshold | declared terminate/quiesce, or `continue-degraded` with loss counters exposed and attribution-requiring effects denied | LC,DR,AR | |
| T-6.8-012 | 1A | 21 | R-LC-1,R-LC-4..5 | ANI | lifecycle daemon unavailable (daemon killed while session active) | containment holds; no new authority; no transition until restart; §7 reconciliation completes | LC,KA,LR | |
| T-6.8-013 | 1A | 21 | R-LC-2 | ANI | manifest declaring `continue-degraded` for a non-permitted trigger | rejected by `agentbound-policy`; `request.rejected` | DR,AR | |

### 7.9 Resource exhaustion

| ID | Milestone | Inv. | Requirement IDs | Privilege | Interface exercised | Expected outcome | Evidence | Control arm |
|---|---|---|---|---|---|---|---|---|
| T-6.9-001 | 1A | 20 | R-RES-1..3 | USP | PIDs/descendant fan-out | bounded/denied; class recorded | KA,AR | |
| T-6.9-002 | 1A | 20 | R-RES-1..2 | USP | file descriptors | bounded/denied; class recorded | KA,AR | |
| T-6.9-003 | 1A | 20 | R-RES-1..2 | USP | memory and CPU | bounded/denied; class recorded | KA,AR | |
| T-6.9-004 | 1A | 20 | R-RES-1..2 | USP | disk bytes/inodes | bounded/denied; class recorded | KA,AR | |
| T-6.9-005 | 1B | 20 | R-GW-7 | USP | I/O/network bandwidth | bounded at gateway; class recorded | GA,AR | |
| T-6.9-006 | 1B | 20 | R-GW-7 | USP | connection count | bounded/denied | GA,AR | |
| T-6.9-007 | 1A | 20,13 | R-RES-2,R-AUD-3 | USP | audit-event volume | declared loss behavior; counters retained | AR,LC | |
| T-6.9-008 | 1B/1C | 20 | R-GW-7,R-GW-9 | USP | requests/rate/tokens/spend | all present classes bounded; absent listed | GA,AR | |

## 8. Fault-injection inventory (plan §7.3)

Faults MUST be injected at every construction and termination step below. The
construction step 1 barrier is a parent/child synchronization barrier through a
pipe or `eventfd`: the parent retains release authority and the child MUST NOT
execute untrusted code before release. It is not described or implemented as a
“stopped clone3 child.” Each fault must prove reverse-order cleanup, retained
failure evidence, and no unsafe identity reuse.

| ID | Milestone | Inv. | Requirement IDs | Privilege | Interface exercised | Expected outcome | Evidence | Control arm |
|---|---|---|---|---|---|---|---|---|
| F-C-01 | 1A | 7 | R-CON-1,R-LC-1 | ANI | step 1 parent/child pipe/eventfd barrier | child never released; reap/scope rollback | LC,KA,LR | |
| F-C-02 | 1A | 7 | R-CON-1..2 | ANI | step 2 private mount namespace | abort; no propagation | LC,KA | |
| F-C-03 | 1A | 7 | R-REQ-6,R-CON-1 | ANI | step 3 descriptor-safe source resolution | abort; close FDs; no path race | LC,KA | |
| F-C-04 | 1A | 7 | R-CON-1..2 | ANI | step 4 restricted tree/pivot root | abort; unmount tracked tree | LC,KA | |
| F-C-05 | 1A | 7,17 | R-CON-1..2 | ANI | step 5 proc after PID namespace | abort; host proc absent | LC,KA | |
| F-C-06 | 1A | 6,7,15 | R-CON-1,R-CON-3 | ANI | step 6 descriptor closure | abort; no FD survives/reintroduced | LC,KA | |
| F-C-07 | 1A | 6,7,15 | R-CON-1,R-CON-3..5 | ANI | step 7 UID/LSM/cap/seccomp install | abort; identity safely held/reclaimed | LC,KA,LR | |
| F-C-08 | 1B | 7,11,13 | R-CON-1,R-CON-7,R-GW-3 | ANI | step 8 record, grant, socket bind | abort; grant/socket unusable | LC,GA,LR | |
| F-C-09 | 1A | 7 | R-CON-1,R-CON-7 | ANI | step 9 runtime exec | no runtime; failure record sealed | LC,KA,LR | |
| F-T-01 | 1A/1B | 12,21 | R-ISO-4,R-LC-1 | ANI | step 1 admission closure | 1A (`none`): step recorded as not applicable with no gateway state present; 1B: new gateway operation denied | LC,GA(1B),DR | |
| F-T-02 | 1A | 12 | R-ISO-4 | ANI | step 2 cgroup freeze | containment retained/fail closed | LC,KA | |
| F-T-03 | 1A | 12 | R-ISO-4 | ANI | step 3 `SIGTERM` and bounded thaw for init reaping | no process escapes; no premature resource release | LC,KA | |
| F-T-04 | 1A | 12 | R-ISO-4 | ANI | step 4 refreeze and `cgroup.kill`, init pidfd exit wait | identity held until proof | LC,KA | |
| F-T-05 | 1A | 12 | R-ISO-4 | ANI | step 5 no-live-process confirmation | `termination-incomplete`; no release | LC,KA,LR | |
| F-T-06 | 1B | 11,12,21 | R-ISO-4,R-GW-3 | ANI | step 6 gateway grant/connection closure | retain safe state; audit failure | LC,GA | |
| F-T-07 | 1B | 11,12 | R-ISO-4,R-GW-6 | ANI | step 7 broker/credential closure | retain safe state; no reuse | LC,GA | |
| F-T-08 | 1A | 12 | R-ISO-4 | ANI | step 8 unmount session filesystems | retain cleanup ledger and identity | LC,KA | |
| F-T-09 | 1B | 10,12 | R-ISO-4,R-GW-2 | ANI | step 9 gateway socket unmount | gateway inaccessible; ledger retained | LC,KA,GA | |
| F-T-10 | 1A | 1,12 | R-ID-7,R-ISO-4 | ANI | step 10 identity reclamation | no release absent proof; quarantine | LC,LR | |
| F-T-11 | 1A | 13,21 | R-LC-1,R-AUD-4 | ANI | step 11 launch-record sealing | append-only incomplete record; no silent completion | LC,LR,AR | |

The §7.3 named points map as follows: authorization/derivation → `T-6.6-*` and
`F-C-01`; identity allocation, crash, reclamation, and quarantine →
`T-6.5-009`, `F-C-07`, `F-T-10`; namespace/mount setup → `F-C-01..05`;
network path/firewall installation is replaced by the single gateway-socket
bind/mount → `F-C-08` and `F-T-09`; cgroup setup → `F-C-01`, `F-T-02..05`;
credential/grant issuance → `F-C-08`; audit binding → `F-C-08`, `F-T-11`;
privilege disposal → `F-C-06..07`; runtime exec → `F-C-09`; active-session
supervision → `T-6.8-*`; and termination/cleanup → `F-T-01..11`.

## 9. Coverage check

This section is a visible completeness check. A source item MUST NOT be
considered covered merely because it is mentioned in prose; it must map here to
one or more atomic IDs.

| Plan source | Coverage IDs |
|---|---|
| §5 demos 1–4 | D-01, D-02, D-03, D-04 |
| §5 demos 5–8 | D-05, D-06, D-07, D-08 |
| §5 demos 9–12 | D-09, D-10, D-11, D-12 |
| §5 demos 13–17 | D-13, D-14, D-15, D-16, D-17 |
| §6.1 bullets 1–13 | T-6.1-001..013 |
| §6.2 bullets 1–8 | T-6.2-001..008 |
| §6.3 bullets 1–8 | T-6.3-001..008 |
| §6.4 bullets 1–10 | T-6.4-001..015 (bullets 1, 4, 5, and 7 each split into two IDs) |
| §6.5 bullets 1–10 | T-6.5-001..010 |
| §6.6 bullets 1–8 | T-6.6-001..008 |
| §6.7 sole bullet | T-6.7-001 |
| §6.8 1A cases | T-6.8-001..007, T-6.8-011..013 |
| §6.8 1B cases | T-6.8-008..009 |
| §6.8 1C case | T-6.8-010 |
| §6.9 bullets 1–8 | T-6.9-001..008 |
| §7.3 authorization/identity | T-6.5-009, T-6.6-*, F-C-01, F-C-07, F-T-10 |
| §7.3 construction and grant points | F-C-01..09 |
| §7.3 supervision and termination points | T-6.8-*, F-T-01..11 |

## 10. Execution record and review obligations

Every execution MUST emit the evidence record shape in the
[traceability matrix](traceability-matrix.md) §6 plus `test_id`, run ID,
control-arm field, effect-ground-truth digest where relevant, and the pinned
profile/version set. The result MUST preserve negative results and every
attempt. A test MAY be rerun to diagnose a result, but no later pass erases an
earlier failure.

The **Control arm** column in every table above is intentionally empty in this
document: the authoritative, populated classification of each test ID is the
per-test register in [ADR-0003](ADR-0003-control-substrate.md), which MUST be
expanded into the committed per-ID `control-arm-register.md` before any microVM
result is recorded (ADR-0003 execution prerequisite). WP0 review MUST confirm that
every applicable invariant has a named test and that the finite local-socket
gateway corpus includes every row in §7.4. Load-profile constants and
correlation deadlines in §5 are frozen by this version; changing them
requires a revision-history entry before any run.
