# WP0 open-question disposition register

**Version:** 0.1  
**Status:** Freeze input — every open question in the WP0 set is listed here with exactly one disposition  
**Date:** 28 August 2026  
**Related:** [architecture README](README.md) freeze condition; all WP0 documents

## Purpose

The freeze condition in the [architecture README](README.md) requires each open question to be either **answered** (the answer is written into the owning document and the question is closed there) or **assigned to WP1** (a named verification item whose failure reopens the owning document). This register is the record of that disposition. "Deferred" without an item is not a permitted disposition.

Dispositions:

- **A** — answered; the owning document has been edited to state the answer and the question is removed from its open list.
- **W1** — assigned to WP1; the verification item is named and the failure consequence stated. The question remains listed in the owning document under "carried to WP1".

## ADR-0002 — gateway topology and authentication

| # | Question | Disposition | Answer / WP1 item |
|---|---|---|---|
| 1 | Default attribution policy `required` for the Linux arm? | **A** | Yes. R-CON-6 / ADR-0002 Decision 2: for every Linux evaluation-arm run the manifest attribution policy is `required`; a connection without pidfd evidence is refused. Invariant 13 is measured without a residual assumption. Production manifests may choose `best-effort`, recorded as a residual assumption. |
| 2 | Guest-side witness in 1D? | **A** | No. The pre-registered arm difference stands: the VM arm claims session-level attribution for the process leg. A witness would change the VM arm's TCB and is out of Phase 1 scope; recorded in ADR-0003 as a future separately pre-registered claim. |

## ADR-0003 — control substrate

| # | Question | Disposition | Answer / WP1 item |
|---|---|---|---|
| 1 | vsock peer-CID reporting on 6.12 | **W1** | WP1 item *VM-1*: verify that the host `AF_VSOCK` endpoint reports the guest CID for each accepted connection and that the CID matches the VMM's configured `guest_cid`. Failure: binding uses the VMM connection table and the ADR records the change. |
| 2 | Guest-side witness | **A** | Same as ADR-0002 Q2: none in Phase 1. |
| 3 | Cross-arm SLOC comparability | **W1** | WP1 item *VM-2*: run the pinned SLOC tool over Firecracker v1.16.1, jailer, guest init, and configuration; report whether transitive-dependency attribution is consistent with the Linux arm. Failure: per-arm disclosure only, excluded from the decision rule (already the rule). |

## Requirements

| # | Question | Disposition | Answer / WP1 item |
|---|---|---|---|
| 1 | Gateway core in the 6 000-line bound? | **A** | No. The bound covers `agentbound-launch`, `agentbound-lifecycle` (with allocator), and the gateway *authentication and mapping path*. Gateway dispatch and adapters are reported separately as figure six, "gateway core SLOC", and reviewed line by line but not bounded in Phase 1. |
| 2 | R-AUD-3 *stop* for the evaluation arm? | **A** | Yes. Evaluation-arm manifests declare *stop*; attribution completeness is measured with zero tolerated loss. *continue-with-counter* is exercised only by T-6.9-007 and reported separately. |
| 3 | Nonce store vs signed sequence for approvals | **A** | Signed monotonic sequence per approver key, with `agentbound-policy` persisting the highest accepted sequence per key in its append-only store. No separate nonce store in Phase 1. |
| 4 | `nosuid` mount vs image verification for R-CON-4 | **A** | Both: the base filesystem and every session mount are mounted `nosuid,nodev`, *and* the runtime catalogue records the image digest, which the constructor verifies. `nosuid` is the enforcement; digest verification is the provenance check. |
| 5 | Attribution policy default | **A** | See ADR-0002 Q1. |

## Manifest schema

| # | Question | Disposition | Answer / WP1 item |
|---|---|---|---|
| 1 | Schema language | **A** | JSON Schema 2020-12 is normative for both signed objects and the request; a CDDL rendering may be published as informative. Schemas are WP1 outputs; the prose in this document remains authoritative where they disagree until 1A. |
| 2 | Clock, freshness, key rotation | **A** | Component interfaces §4: host realtime clock disciplined by the administrator's time service; freshness 30 s skew / 10 min manifest age / 60 s binding commit; keyring entries carry `key_id`, `not_before`, `not_after` with overlapping validity and a revocation list distributed as integrity-protected configuration. |
| 3 | `mac_context` reserved as `null`? | **A** | Yes for Profile U: `mac_context` MUST be `null`; a non-null value is a construction failure in Phase 1. The compartmented profile defines its content. |
| 4 | Absent resource classes in the first deployment | **A** | Requirements R-RES-5 matrix: at 1A `network_bandwidth`, `connection_count`, `request_rate`, `storage_bytes`, `external_spend`, `model_tokens`, `accelerator` are absent; at 1B `model_tokens`, `accelerator`; at 1C `accelerator` unless exposed. |
| 5 | Immutable representation of a policy-approved runtime command | **A** | `runtime.invocation_profile` is a catalogue identifier whose catalogue entry holds the argv template and environment allowlist; the constructor records the catalogue entry digest in the launch binding's `constructor` member. No caller-supplied command line is ever recorded as authoritative. |
| 6 | Launch-record retention class | **A** | Records are retained at least until the execution identity has left quarantine *and* every durable object in the managed domain that references the numeric UID has been reconciled or deleted; the reference deployment sets this to the quarantine floor plus the workspace retention period, and never deletes a sealed record with an unreconciled reference. |

## Session lifecycle

| # | Question | Disposition | Answer / WP1 item |
|---|---|---|---|
| 1 | systemd/kernel version set for freeze/kill/pidfd semantics | **A** | Pinned in ADR-0003 and requirements §12: Linux 6.12 LTS series, systemd 258 series. Behaviour verification is WP1 item *LC-1* (already in the plan's WP1 list). |
| 2 | Frozen cgroup holding a `SOCK_SEQPACKET` connection open | **W1** | WP1 item *LC-2*: measure whether a frozen peer delays the gateway's zero-connection acknowledgement. Failure: quiesce closes idle gateway connections before freezing and the lifecycle §6 text is revised. |
| 3 | Trust anchor, correction, retention for the launch-record store | **A** | Component interfaces §4–5: append-only, hash-chained, fsync commit, correction by new record referencing the original; retention per manifest-schema Q6 above. |
| 4 | Which outage modes may use `continue-degraded` | **A** | Only *policy-service unavailable* and *audit-pipeline degraded below stop threshold*, and only when the manifest declared it. `agentbound-lifecycle` unavailable is never `continue-degraded`: sessions keep running under their installed boundary but no new authority is issued and no transition is possible until it returns. |
| 5 | Audit backpressure representation | **A** | Manifest `audit.loss_behaviour` plus `audit_capacity` resource class; status API exposes `audit_queue_depth`, `audit_dropped_total`, and the loss-behaviour state. |
| 6 | Operator escalation for persistent D-state tasks | **A** | `termination-incomplete` at the manifest deadline emits `session.escalation_required` naming the pidfds; the operator's only permitted actions are to continue observation or reboot the host; identity remains held across reboot via the allocator store and is reconciled at boot. |
| 7 | Synchronously durable events | **A** | Component interfaces §6 commit-point table is authoritative: `authorized`, `constructing`, `active`, `terminated`, `cleaned/sealed`, and every failure outcome are fsync-durable before the externally visible status advances. |
| 8 | microVM mapping of lifecycle states | **A** | ADR-0003: identical state names; `constructing` spans VMM launch through guest-init readiness; termination steps 2–5 are realized as VMM freeze, guest SIGTERM via vsock control, VMM kill, and VMM pidfd exit; the per-test register records the mapping. |

## Execution-identity lifecycle

| # | Question | Disposition | Answer / WP1 item |
|---|---|---|---|
| 1 | Default range sufficient? | **A** | Reference range 200000–299999 (100 000 identities). With the nominal profile's 8 concurrent sessions and overload's 32, and a quarantine floor of 24 h, exhaustion is unreachable in Phase 1; exhaustion behaviour is still tested by T-6.9-001 with an artificially small range. |
| 2 | Allocator-store implementation | **W1** | WP1 item *ID-1*: prototype the append-only store with compare-and-set (candidate: single-writer SQLite in WAL mode with a hash-chained record table, owned by `agentbound-lifecycle`), verifying crash-consistency under the F-C/F-T fault points. Failure: alternative store; identity lifecycle §3 revised. |
| 3 | Registered host paths for the managed domain | **A** | Exactly: the per-session workspace image, the per-session runtime tmpfs, the launch-record store, the allocator store, and the audit spool. No other host path may carry a session UID; discovery scans these and the process table only. |
| 4 | Objects transferred rather than deleted | **A** | Only the session workspace image, transferred as a whole to the durable-principal ownership projection by the reclamation policy named in the manifest's `termination_retention`; everything else is deleted. |
| 5 | Backup tooling | **A** | Backups of the workspace image carry the `authorization_id`, `launch_record_digest`, and durable-principal ID as image metadata; restore tooling MUST resolve ownership from that metadata and MUST NOT grant access by numeric UID (already R-ID-7). |
| 6 | Broker/gateway confirmation when unavailable | **A** | Reclamation blocks; identity remains `reclaiming`. There is no substitute confirmation. |
| 7 | Quarantine floor and sealing semantics | **A** | Floor 24 h after `cleaned/sealed`, plus the ordinal condition that `agentbound-audit` has sealed every record referencing the `authorization_id`. |
| 8 | MAC fields in the allocator record | **A** | None in Profile U (`mac_context` is `null`); the compartmented profile adds them under its own ADR. |

## Summary

| Owning document | Answered | Assigned to WP1 |
|---|---:|---:|
| ADR-0002 | 2 | 0 |
| ADR-0003 | 1 | 2 (VM-1, VM-2) |
| Requirements | 5 | 0 |
| Manifest schema | 6 | 0 |
| Session lifecycle | 7 | 1 (LC-2) |
| Execution-identity lifecycle | 7 | 1 (ID-1) |
| **Total** | **28** | **4** |

Together with the ADR-0002 Decision 7 kernel-baseline list and the plan's WP1 spikes, the WP1 verification set is: Decision 7 items, VM-1, VM-2, LC-1, LC-2, ID-1. Every item names the document it reopens on failure.
