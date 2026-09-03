# Architecture specifications

This directory contains WP0 implementation specifications and architecture decision records (ADRs) that refine the normative [technical report](../papers/technical-report.md) and the [Phase 1 plan](../plans/phase-1-reference-implementation.md).

## WP0 specification set

- [Phase 1 normative requirements](phase-1-requirements.md) — RFC 2119-style requirements, scoped threat model, and pre-registered thresholds.
- [Session request and effective-manifest schema](manifest-schema.md) — bounded untrusted request, signed effective manifest, canonical encoding, and constructor validation.
- [Session lifecycle and failure states](session-lifecycle.md) — state machine, construction and rollback ordering, revocation, termination, and recovery.
- [Execution-identity lifecycle](execution-identity-lifecycle.md) — host-local allocation, managed reclamation domain, quarantine, audit disambiguation, and exhaustion.
- [Invariant-to-test traceability matrix](traceability-matrix.md) — every Profile U invariant mapped to requirements, mechanisms, adversaries, tests, and result class.

All five are version 0.1 drafts for WP0 review. They constrain implementation only after WP0 review freezes them.

## Decision records

- [ADR-0001: Per-session execution identity is distinct from the durable principal identity](ADR-0001-execution-identity.md) — **Accepted (revised)**.
- [ADR-0002: Gateway channel topology and session authentication](ADR-0002-gateway-authentication.md) — **Proposed for WP0 review**; WP1 selects one Linux-arm candidate.
- [ADR-0003: Control substrate](ADR-0003-control-substrate.md) — **Proposed for WP0 review**; selects Firecracker by default and freezes test equivalence.

## Authority and change control

The technical report remains the source for invariants and threat model. The Phase 1 plan owns milestones, gates, work packages, and evidence scope. This directory owns concrete schemas, lifecycle rules, thresholds, and implementation decisions.

An ADR or specification that changes a technical-report invariant, plan gate, profile claim, or milestone allocation MUST also update its owning document. Draft open questions are not decisions. Once WP0 freezes the set, changes require a revision-history entry and an impact review against the traceability matrix.
