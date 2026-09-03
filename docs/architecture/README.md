# Architecture specifications

This directory contains WP0 implementation specifications and architecture decision records (ADRs) that refine the normative [technical report](../papers/technical-report.md) and the [Phase 1 plan](../plans/phase-1-reference-implementation.md).

## WP0 specification set

| Document | Version | Owns |
|---|---|---|
| [Phase 1 normative requirements](phase-1-requirements.md) | 0.2 | RFC 2119 requirements, scoped threat model, resource-class milestone matrix, pre-registered thresholds and SLOC accounting rules |
| [Authorization manifest and launch binding schema](manifest-schema.md) | 0.2 | bounded untrusted request; the two signed objects that form the effective manifest; canonical encoding; constructor validation; complete examples |
| [Session lifecycle and failure states](session-lifecycle.md) | 0.2 | state machine, construction barrier and rollback ordering, `agentbound-lifecycle` daemon, termination protocol, quiesce, recovery |
| [Execution-identity lifecycle](execution-identity-lifecycle.md) | 0.2 | host-local allocation inside the daemon, managed reclamation domain and condition, quarantine, audit disambiguation, exhaustion |
| [Component interfaces (skeleton)](component-interfaces.md) | 0.1 | per-pair transport, peer identity, authorization; trust anchors and key custody; store commit models; idempotency; error classes; restart precedence. Wire formats are WP1 outputs |
| [Test catalogue](test-catalogue.md) | 0.1 | atomic test IDs for every demonstration, suite bullet, and fault point; attribution metric and load profiles; gate pass rules. Fixtures and commands are WP1 outputs |
| [Invariant-to-test traceability matrix](traceability-matrix.md) | 0.2 | every Profile U invariant mapped to requirements, mechanisms, catalogue tests, expected mechanism class, and residual assumption |

All are drafts for WP0 review. They constrain implementation only after WP0 review freezes them.

## Decision records

- [ADR-0001: Per-session execution identity is distinct from the durable principal identity](ADR-0001-execution-identity.md) — **Accepted (revised)**.
- [ADR-0002: Gateway channel topology and session authentication](ADR-0002-gateway-authentication.md) — **Accepted for Phase 1 (0.2)**: local-socket topology, `AF_UNIX SOCK_SEQPACKET`, per-operation kernel process evidence; network topology withdrawn from Phase 1. WP1 verifies the listed kernel-baseline assumptions.
- [ADR-0003: Control substrate](ADR-0003-control-substrate.md) — **Accepted for Phase 1 (0.2)**: pinned Firecracker configuration, per-test equivalence keyed to the test catalogue, pre-registered comparative decision rule.

## Mandatory implementation contracts

Obligations an implementer must not miss, with their owning section:

| Contract | Where | Milestone |
|---|---|---|
| Policy signs an allocation-free authorization manifest; the constructor signs the launch binding after atomic identity reservation; the pair is the launch record | manifest schema §3–4 | 1A |
| Construction step 1 is a `clone3` synchronization barrier; nine ordered steps; rollback ledger | session lifecycle §3, §9.3 | 1A |
| `agentbound-lifecycle` is the sole actor for quiesce, termination, reclamation, and reconciliation; systemd cannot invoke a helper for a scope | session lifecycle §4; component interfaces | 1A |
| Eleven-step termination: deny gateway admission, freeze, let init reap, `cgroup.kill`, verify emptiness plus host credential scan, then release | session lifecycle §5; requirements R-ISO-4 | 1A |
| Identity state machine `free → allocated → in-use → reclaiming → quarantined → free`; reclamation is a condition, never a period; exports never rely on the numeric UID | identity lifecycle §4–5 | 1A |
| `loginuid` is corroborating; one fail rule (R-CON-6) | requirements §5; identity lifecycle §6 | 1A |
| Every resource class is enforced or absent-with-evidence per the milestone matrix | requirements R-RES-5; manifest schema §3.5 | 1A–1C |
| One `SOCK_SEQPACKET` gateway socket; `SO_PEERCRED` + pidfd at connect; one `SCM_CREDENTIALS` per packet per operation; `SCM_RIGHTS` rejected | ADR-0002 Decisions 1–2; requirements R-GW-1…3 | 1B |
| Every operation re-checks live grant state; admitted-before-revocation Git pushes are complete-and-record | ADR-0002 Decision 4 | 1B |
| vsock CID bound to a non-reusable VM token and invalidated before reuse; VM arm claims session-level attribution for the process leg | ADR-0002 Decision 6; ADR-0003 | 1D |
| Observed result class is unconstrained; expected class is pre-registered | traceability matrix §1 | all |

## Open decisions carried into WP1

Each WP0 document ends with an open-questions section. Items that require kernel-baseline evidence are listed in ADR-0002 Decision 7 and the plan's WP1; a failed verification reopens the owning ADR before WP2 begins.

## Authority and change control

The technical report remains the source for invariants and threat model. The Phase 1 plan owns milestones, gates, work packages, and evidence scope. This directory owns concrete schemas, lifecycle rules, thresholds, interfaces, tests, and implementation decisions.

An ADR or specification that changes a technical-report invariant, plan gate, profile claim, or milestone allocation MUST also update its owning document. Draft open questions are not decisions. Once WP0 freezes the set, changes require a revision-history entry and an impact review against the traceability matrix and test catalogue.
