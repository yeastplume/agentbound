# Execution-Identity Lifecycle Specification

**Version:** 0.3  
**Status:** Draft for WP0 review  
**Date:** 28 August 2026  
**Applies to:** Phase 1 Unix-governed sessions  
**Related:** [ADR-0001](ADR-0001-execution-identity.md), [session lifecycle](session-lifecycle.md), [Phase 1 plan](../plans/phase-1-reference-implementation.md), [technical report](../papers/technical-report.md)

## Revision history

- **0.3** — Identifier terminology aligned with manifest schema §4.
- **0.1** — Initial WP0 draft.
- **0.2** — Allocator placed inside the `agentbound-lifecycle` daemon; helper references replaced; `loginuid` restated as corroborating evidence with the single R-CON-6 fail rule; host credential scan retained as a reclamation precondition.

---

## 1. Purpose and relation to ADR-0001

This specification makes the allocator and reclamation consequences of [ADR-0001](ADR-0001-execution-identity.md) implementable. It governs the per-session local execution identity under which session processes run; it does not replace the durable agent principal, durable ownership projection, remote workload identity, or policy authorization.

The terms **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative. An execution identity comprises a UID, one allocated primary GID, its explicitly allocated supplementary groups, and, where a profile requires one, an associated MAC type or category allocation. It is unique among concurrent sessions on the allocation host.

This specification satisfies the ADR's requirement for a declared managed reclamation domain, a condition rather than a mere elapsed time before reuse, an export rule, audit disambiguation, crash recovery, exhaustion behavior, and backup/persistent-file treatment.

---

## 2. Uniqueness scope

### 2.1 Phase 1 decision: host-local uniqueness

Phase 1 SHALL use **host-local uniqueness**. `agentbound-launch` MUST ensure that a numeric execution UID is never shared by concurrent or unreclaimed sessions on one host. A UID has no cross-host authorization meaning and remote services MUST NOT treat it as a workload identity.

Host-local allocation is selected because Phase 1 is a bounded single-host/reference implementation experiment. It avoids prematurely introducing a distributed allocator, fleet availability dependency, consensus protocol, and global collision/recovery service into the trusted computing base. The session boundary is enforced by the local kernel, and the required same-principal isolation tests occur on that host.

Fleet-wide audit disambiguation MUST use this tuple:

```text
(host ID, boot ID, authorization ID)
```

Every record that reports an execution UID MUST pair it with that tuple. The durable principal ID and session trace identity MUST also be present where the event schema permits. A numeric UID without these values is insufficient evidence of a particular historical session.

### 2.2 Later fleet-wide allocation

A later fleet-wide numeric allocation design MUST replace or supplement the local allocator with a strongly consistent, auditable allocation authority; define partition behavior, leases, revocation, cross-host recovery, and collision detection; and retain the export rule. It MUST NOT claim that a fleet-wide UID alone is a durable principal or remote authorization identity.

Fleet-wide allocation MAY assign globally unique numeric values, allocate non-overlapping host blocks, or use a different substrate identity. In every case it MUST preserve per-session uniqueness, verified reclamation, reuse quarantine, and audit pairing. This specification does not authorize that extension without a reviewed revision.

---

## 3. Allocation source and records

### 3.1 Reserved range

The Phase 1 default execution-identity range is:

```text
UID range: 200000–299999 inclusive
GID range: 200000–299999 inclusive
```

The range is configurable only through host administrator configuration before allocation begins. It MUST be disjoint from local human, service, and durable-principal ownership UIDs/GIDs. `agentbound-policy` requests no numeric UID, GID, path, or group from an untrusted request; `agentbound-launch` obtains the allocation only after it verifies and accepts a committed authorization manifest; it then emits the allocation-bound launch binding.

The allocator MUST reject a configured range that overlaps known durable-owner mappings, existing system/service accounts, or another active allocator range. Host provisioning MUST reserve the range from ordinary account-management tooling.

### 3.2 Allocator store and authority

The allocator lives in the `agentbound-lifecycle` daemon. Allocator state MUST be an append-only, host-local store owned by that daemon's service identity; `agentbound-launch` requests a reservation over the authenticated interface in [component interfaces](component-interfaces.md). The running session execution UID MUST have neither read nor write access to the store. The store MUST be integrity protected and durable before an allocation becomes usable.

The append-only record stream MUST support a tamper-evident sequence or equivalent integrity check. Compaction MAY occur only through a separately authenticated maintenance action that preserves allocation history and all unreclaimed/quarantined state.

### 3.3 Allocation record

An allocation record MUST contain at least:

| Field | Requirement |
|---|---|
| allocation record ID | Globally unique record identifier. |
| host ID and boot ID | Host identity and boot instance at allocation. |
| authorization ID and trace identity | Immutable binding to this session. |
| durable principal ID | Principal for which the session is constructed. |
| execution UID | UID allocated from the reserved range. |
| primary GID | One GID allocated with the UID. |
| supplementary group set | Exact allocated group IDs and their purpose; no ambient durable-owner group. |
| state and sequence | Lifecycle state and monotonic append sequence. |
| allocator actor and timestamp | Constructor or `agentbound-lifecycle` identity and trusted timestamp. |
| scope/cgroup and PID namespace IDs | Expected containment evidence. |
| managed-domain manifest | Registered host paths, mounts, stores, grants, IPC/cgroup references. |
| reclamation and quarantine evidence | Filled on transition from `in-use` onward. |

The per-session primary GID and every supplementary group MUST be allocated with the UID. The session MUST NOT inherit a durable principal's owning group or an unreviewed host group. Phase 1 SHOULD use no supplementary groups unless a manifest-required, per-session group is necessary; each such group MUST be unique to the session and recorded.

---

## 4. Identity state machine

```text
free → allocated → in-use → reclaiming → quarantined → free
```

| State | Meaning | Entry rule | Permitted next states |
|---|---|---|---|
| `free` | Numeric identity is available for allocation. | Initial verified empty range entry or completed quarantine. | `allocated`. |
| `allocated` | Identity is reserved to one authorization-manifest digest and pending authorization ID; the launch binding is committed atomically with the reservation, but no runtime is yet established. | Durable append of allocation record before identity installation. | `in-use`, `reclaiming`. |
| `in-use` | Identity has been installed for a session or may have been installed before a crash. | Constructor installs credentials or recovery finds compatible live scope evidence. | `reclaiming`. |
| `reclaiming` | No reuse is possible while `agentbound-lifecycle` verifies and removes all managed-domain residue. | Termination, construction rollback, or crash reconciliation. | `quarantined`; remains `reclaiming` on uncertainty. |
| `quarantined` | Reclamation condition passed; reuse is delayed for late audit correlation. | Recorded successful reclamation proof. | `free`. |

No transition may skip `reclaiming` or `quarantined`. Allocator operations MUST be compare-and-set on allocation record ID and state sequence. A duplicate allocation attempt for the same UID or a launch record mapped to multiple UIDs MUST fail closed, emit `identity.double_allocation_detected`, and prevent both implicated sessions from becoming active until reconciled.

### 4.1 Verified reclamation condition

Reclamation is a **condition**, not an elapsed period. `agentbound-lifecycle` MUST evaluate it across the declared managed reclamation domain:

1. session namespaces and mounts;
2. manifest-registered host paths;
3. session runtime and workspace stores;
4. broker and storage-service grants;
5. IPC namespaces; and
6. cgroup state.

The condition is met only when all of the following are true:

- **No live process:** `cgroup.procs` is empty **and** the PID-namespace init has exited. `agentbound-lifecycle` MUST also scan host process credentials for the execution UID/GIDs and reconcile every match with held pidfds, PID namespace, systemd scope, and allocation record. Any process outside the expected scope or contradictory `/proc` evidence blocks reclamation and raises `identity.scope_escape_suspected`; scope containment is not assumed to make the scan unnecessary.
- **No owned file or IPC object:** `agentbound-lifecycle` scans each manifest-registered host path and each mounted session workspace/runtime tmpfs for objects owned by the execution UID or session GID, removes or transfers only objects explicitly covered by the cleanup policy, and records every result. The session IPC namespace MUST be destroyed; no session-owned IPC object may remain in a registered host-visible IPC location.
- **No outstanding grant:** the broker, storage service, and `agentbound-gateway` MUST confirm revocation/closure for every launch-record-bound grant. An unreachable grant authority is not confirmation.

The managed domain is deliberately bounded. The allocator MUST NOT assert discovery of numeric ownership outside it, such as arbitrary detached mounts, removable media, unregistered paths, external backups, or snapshots. Those exports are controlled by the rule in §5.

If any check cannot be completed, has contradictory evidence, or finds a live process, the identity MUST remain `reclaiming`. `agentbound-lifecycle` MUST NOT release it based on a timeout alone.

### 4.2 Quarantine

After the reclamation condition passes, the allocator MUST place the identity in `quarantined`. The quarantine's purpose is to expose late-arriving audit, kernel, gateway, and storage-correlation records before the UID is reused.

The ordinal minimum is: **do not leave quarantine until `agentbound-audit` has sealed all records referencing the authorization ID.** In addition, a host-configurable quarantine floor applies. The allocator MUST enforce whichever condition completes later. The floor MAY be increased but MUST NOT be shortened automatically under allocation pressure.

---

## 5. Export rule and durable authorization

Anything that leaves the declared managed reclamation domain—backups, snapshots, archives, artifacts pushed through a gateway, exported files, replicated storage, and restore media—MUST carry the global durable principal ID and session identifiers, including the authorization ID and trace identity where applicable. It MUST NOT rely on the numeric execution UID for durable authorization or attribution.

The export metadata SHOULD also carry the host ID, boot ID, manifest digest, object digest, classification/provenance metadata where applicable, and a reference to the sealed launch record. Numeric owner fields MAY be retained as forensic observations but MUST be treated as non-authoritative after export.

`agentbound-gateway` MUST attach the durable principal ID, authorization ID, trace identity, approved operation identity, and any required provenance metadata to gateway-mediated artifacts and remote operations. It MUST authorize the operation against the session grant, not against the source process's numeric UID alone.

A storage broker MUST translate an authorized per-session grant into durable storage access and persist global principal/session metadata with created or exported objects. It MUST NOT create a durable ACL whose continuing authorization depends on the reclaimed execution UID. Restore tooling MUST map historical numeric ownership only through exported metadata and a reviewed restoration policy; it MUST NOT reactivate the old UID as authorization.

---

## 6. Audit disambiguation and loginuid

Every `agentbound-audit` record about a session MUST pair `execution_uid` with `authorization_id`, `launch_record_digest` (mandatory once the binding is committed), and `boot_id`; fleet-correlatable records MUST additionally carry `host_id`. This rule applies to lifecycle, allocation, kernel-correlation, gateway, broker, and cleanup records.

Kernel PID/PPID values are not durable process identities because PIDs are reused. Records SHOULD include a PID namespace identifier plus process start time or pidfd-derived identity where available. The systemd scope/cgroup is useful corroboration but is not a portable standalone audit key.

Per technical-report §5, `loginuid` is useful for preserving the account that originally gained access and is inherited by descendants, but it is write-once, inherited across `clone`, and governed by host audit policy; it is therefore **corroborating** evidence only. The authoritative session attribution key is the signed launch record correlated with (execution UID, boot ID, PID namespace, process start time or pidfd). `agentbound-launch` MUST attempt to set `loginuid` in the barrier-blocked child before exec when the pinned baseline permits it and the child's value is unset, and MUST record the result (set, immutable, already-set, denied) in the launch binding. The single fail rule is R-CON-6 of the requirements: construction fails only when the manifest attribution policy is `required` and the value cannot be set; otherwise the condition is a recorded residual assumption. It MUST NOT silently attribute an inherited value to another actor.

The effective UID records the execution identity, not the durable principal. The signed/append-only launch record is the authoritative mapping between them.

---

## 7. Crash recovery and integrity failures

At boot and allocator-service restart, the allocator MUST reconcile each non-`free` record against systemd scope state, cgroup state, PID-namespace-init evidence, registered mounts and paths, grant stores, and sealed launch records.

- Any `allocated` identity without a live compatible scope MUST move to `reclaiming`.
- Any `in-use` identity without a live compatible scope MUST move to `reclaiming`; it MUST NOT return directly to `free`.
- Any identity with a live scope but missing, conflicting, or unsealed launch-record evidence MUST be treated as an orphan: deny new grants, preserve the identity hold, and request `agentbound-lifecycle` containment/termination.
- Any same-UID concurrent scope, duplicate active allocation record, or mismatched UID-to-launch-record mapping is double allocation. The allocator MUST fail closed, block new sessions, emit a high-severity audit event, and require reconciliation.

If the append-only allocator store is corrupt, its integrity chain fails, its durable sequence cannot be determined, or its host binding is unavailable, the allocator MUST fail closed: it MUST admit **no new sessions** and MUST not reuse any identity. Existing sessions MAY be observed and terminated through `agentbound-lifecycle`, but their identities MUST remain held until a trusted recovery procedure repairs or replaces the allocator state and records the decision.

---

## 8. Exhaustion and monitoring

When no `free` identity exists in the configured range, `agentbound-launch` MUST reject a new session before construction with the named error:

```text
EXECUTION_IDENTITY_RANGE_EXHAUSTED
```

The rejection MUST be audited with capacity counts and MUST NOT cause the allocator to shorten quarantine, bypass reclamation checks, reuse an `allocated`, `in-use`, `reclaiming`, or `quarantined` UID, or borrow a durable-owner UID.

The host MUST expose monitoring for at least:

- free identities and percentage free;
- allocated, in-use, reclaiming, and quarantined counts;
- oldest reclaiming identity and blocked-condition class;
- quarantine backlog and oldest unsealed launch-record dependency;
- allocation rate and high-water mark; and
- double-allocation and store-integrity failures.

Provisioning MUST configure warning and critical thresholds before use. The recommended initial thresholds are warning below 20% free and critical below 5% free; deployments MAY choose stricter values but MUST document them. Capacity response is to extend the reserved range through reviewed host provisioning or reduce demand, never to weaken lifecycle rules.

---

## 9. Durable ownership projection and session grants

The durable principal's owning UID MUST be outside the execution-identity range and MUST NOT execute Phase 1 session code. It owns durable state only as a stable ownership projection, or a storage service owns that state on its behalf.

Sessions reach durable partitions only through manifest-authorized per-session grants: read-only or scoped bind mounts, ACLs, narrowly allowlisted descriptors, or a broker. Grants MUST not introduce the durable owner UID/GID as an ambient session credential.

When a grant uses an ACL entry naming an execution UID or its allocated group, `agentbound-lifecycle` MUST remove that ACL entry during reclamation, before the identity can enter quarantine. The scan in §4.1 MUST verify removal within all manifest-registered paths. A failed ACL removal leaves the identity in `reclaiming`.

Created workspace and runtime data SHOULD be placed in session-owned tmpfs or registered disposable paths. Objects intended to survive termination MUST be exported through the metadata and authorization rules in §5 rather than left as UID-owned durable residue.

---

## 10. MAC profiles and microVM control arm

For the Phase 1 Unix-governed profile, distinct UIDs/GIDs are the primary execution-identity boundary. A MAC profile MAY associate a per-session execution identity with an allocated SELinux type or category set, but that allocation, reuse policy, and policy analysis require separate conformance evidence. A MAC type/category MUST NOT be treated as an excuse to share the Phase 1 execution UID.

For the microVM control arm, the VM boundary MAY realize execution identity as ADR-0001 permits. The control arm MUST still provide an allocation record, launch-record binding, durable principal/session metadata, verified teardown, export rule, and audit disambiguation. It MUST document which fields are substrate-independent and which UID/GID details do not apply inside the VM.

---

## 11. Test obligations

### 11.1 Same-principal interference: plan §6.1

The Phase 1 test suite MUST run concurrent sessions of one durable principal with distinct execution identities. From each session it MUST attempt `/proc/<hostpid>` access, `kill`, `pidfd_send_signal`, `ptrace`, `process_vm_readv`/`process_vm_writev`, `/run` and `/tmp` access, pathname and abstract Unix sockets, IPC/shared memory, durable-partition group access, broker socket reuse, and inherited/reintroduced descriptors through `SCM_RIGHTS`, `/proc/self/fd`, and memfd.

Tests MUST prove that identity allocation is distinct and that supporting namespaces, private paths, descriptor discipline, and grants do not collapse the boundary.

### 11.2 Fault injection: plan §7.3

Fault injection MUST cover identity allocation, reclamation, quarantine, and crash during allocation; namespace/mount setup; cgroup setup; grant issuance; audit binding; privilege disposal; runtime exec; active-session supervision; and termination/cleanup.

For each injected failure, evidence MUST show one of two outcomes:

1. construction fails with no runnable session, usable credential, or reusable identity; or
2. an identity remains safely `allocated` or `reclaiming`, with the precise blocking condition and recovery evidence recorded.

The suite MUST include allocator-store corruption, duplicate concurrent allocation, stale allocation replay, a missing scope after crash, a live D-state task preventing reclamation, an unrevoked broker grant, ACL-removal failure, and audit sealing delayed beyond the quarantine floor.

---

## 12. Allocator operational rules

### 12.1 Allocation serialization and host binding

The allocator MUST serialize `free → allocated` transitions with a host-local lock or transactional primitive whose failure semantics are documented. The durable allocation append MUST happen before `agentbound-launch` installs the UID/GID in a child. If the constructor crashes after the append but before installation, restart reconciliation MUST leave the record `allocated` until it verifies the absence of a compatible scope and then moves it to `reclaiming`.

Host ID MUST be a stable machine identity selected by provisioning, not merely a hostname. Boot ID MUST be read from the host boot instance and MUST be stored in every allocation record. Restoring an allocator store onto another host without an explicitly authorized migration procedure MUST cause a host-binding failure and fail closed.

A migration procedure, if added later, MUST preserve launch-record mapping and exported metadata, create an auditable source/destination handoff, and prevent simultaneous allocator ownership on two hosts. It is out of scope for Phase 1.

### 12.2 Group allocation semantics

The primary GID allocation MUST be one-to-one with the execution UID for the allocation's lifetime. If supplementary groups are used, the allocator MUST reserve them in the same transaction and record the intended grant relation. A session MUST NOT use a shared supplementary group to reach another session's private state.

The constructor MUST apply `setgroups`, primary GID, and UID transition before runtime exec, remove all pre-existing supplementary groups except the allocation record's exact set, and verify the resulting credentials from the child context. Failure to verify the credential set MUST fail construction and move the identity to `reclaiming`.

The allocator MAY map an execution UID and primary GID to equal numeric values, as the default range proposes, but equality is a convention rather than proof of identity. All lifecycle checks MUST use the allocation record rather than infer ownership only from numeric equality.

### 12.3 Discovery and removal record

Every reclamation scan MUST create a signed or append-only discovery record with:

- allocation record ID and authorization ID;
- host process credential scan results, including every matching PID, start time, PID namespace, and scope reconciliation;
- root paths scanned and their manifest registration version;
- each owned object found, its object type, owner UID/GID, and action taken;
- IPC namespace destruction result and any host-visible IPC inspection result;
- cgroup/PID-init/pidfd evidence;
- broker, storage, and `agentbound-gateway` grant revocation confirmations; and
- a final pass/fail/uncertain conclusion.

A path omitted from the managed-domain manifest MUST be treated as outside the proven domain, not silently scanned as if it were covered. Conversely, a manifest-registered path that cannot be accessed safely for scanning MUST block reclamation. Deletion or transfer MAY occur only under the session retention and durable-projection rules; it MUST not turn an unrecognized object into an untracked export.

### 12.4 Backup, snapshot, and restore controls

Backup and snapshot tools that can see registered paths MUST be configured to carry the metadata required by §5. They SHOULD exclude disposable session runtime/tmpfs paths once cleanup begins. A backup job MUST NOT make a snapshot copy appear to satisfy the reclamation condition; it is outside the managed domain and is safe only because numeric UID ownership is not durable authorization there.

Restore tooling MUST place recovered data under a durable owner or storage-broker policy based on the exported global principal/session identifiers. It MUST remove or map stale execution-UID ACL entries before data becomes accessible. A restore that cannot interpret the required metadata MUST fail closed for protected data and create an audit event.

### 12.5 Reclamation authorization

Only the `agentbound-lifecycle` and the allocator's authenticated recovery process MAY advance an identity from `in-use` to `reclaiming` or from `reclaiming` to `quarantined`. The `agentbound` CLI, systemd, and revocation triggers may request termination, but they MUST NOT directly mark an identity free.

An operator may place an identity in a manual hold state represented operationally as `reclaiming` with a blocking reason. Manual release requires recorded evidence satisfying the same reclamation condition; it MUST NOT override a live-process, owned-object, or outstanding-grant finding.

### 12.6 Allocator audit events

Each allocator event MUST include allocation record ID, authorization ID, host ID, boot ID, execution UID, primary GID, actor, timestamp, outcome, and state sequence. The event vocabulary MUST include:

| Event | Required additional evidence |
|---|---|
| `identity.allocated` | reserved range, exact group set, allocator transaction/sequence. |
| `identity.installed` | child/scope evidence and credential verification result. |
| `identity.reclamation_started` | trigger and managed-domain manifest reference. |
| `identity.object_discovered` | registered root, object type, action, and result. |
| `identity.grant_revoked` | grant issuer/type, confirmation or blocking failure. |
| `identity.quarantined` | reclamation proof and audit-sealing dependency. |
| `identity.released` | quarantine floor result and final free transition. |
| `identity.double_allocation_detected` | conflicting records/scopes and containment action. |
| `identity.store_corrupt` | integrity failure class and fail-closed admission state. |
| `identity.range_exhausted` | range capacity and state counts. |

`agentbound-audit` MUST correlate allocator events with session lifecycle events. Retention of this audit history MUST NOT itself block reuse once quarantine's ordinal sealing condition and configured floor have both passed.

---

## 13. Open questions for WP0 review

1. Is the default range large enough for the expected concurrency and quarantine backlog on the reference host?
2. What exact allocator-store implementation supplies append-only integrity, atomic compare-and-set, backup, and recovery properties?
3. Which host paths must the manifest require as registered to make the managed domain reviewable without becoming unbounded?
4. Which object types are allowed to be transferred rather than deleted during reclamation, and who authorizes that transfer?
5. How does backup tooling preserve required global/session metadata and prevent numeric-owner-based restore authorization?
6. What constitutes authoritative confirmation from each broker and `agentbound-gateway` implementation when it is unavailable during reclamation?
7. What configured quarantine floor and audit-sealing semantics are adequate for the reference deployment?
8. Which MAC allocation fields belong in the shared allocator record when the compartmented profile is evaluated?
