# Agentbound

**A policy-driven security and execution substrate for organizational AI agents.**

Agentbound treats an organizational AI agent as a durable security principal. Each task runs in a separately governed Unix session and process tree. The foundational papers retain the title *Agents as Unix Principals*.

The Phase 1 WP0 specifications are frozen. The project is moving into WP1 mechanism verification.

## Start here

- [Position paper](docs/papers/position-paper.md) — thesis, motivation, and adoption argument.
- [Technical report](docs/papers/technical-report.md) — mechanisms, invariants, threat model, deployment profiles, and evaluation criteria.
- [Phase 1 reference implementation plan](docs/plans/phase-1-reference-implementation.md) — implementation stages, gates, and evidence requirements.
- [WP0 architecture specifications](docs/architecture/README.md) — frozen requirements, schemas, lifecycle rules, component interfaces, tests, traceability, and ADRs.

## Repository layout

```text
docs/
  papers/         Position paper and technical report
  plans/          Implementation and evaluation plans
  architecture/   Frozen Phase 1 specifications and architecture decisions
implementation/   Reference implementation components
tests/            Test implementations, fixtures, and conformance suites
```

`implementation/` and `tests/` remain placeholders at the WP0 freeze. WP1 verifies the high-risk mechanisms before implementation begins. Failed verification reopens the relevant ADR or specification.

## Document authority

- The **position paper** is authoritative for the motivation, thesis, adoption argument, and conclusions.
- The **technical report** is authoritative for mechanisms, invariants, threats, constraints, and evaluation criteria.
- The **Phase 1 plan** is authoritative for implementation scope, gates, work packages, and required evidence.
- The documents under `docs/architecture/` are authoritative for concrete requirements, schemas, lifecycle rules, interfaces, tests, traceability, and implementation decisions.

## Current status

The position paper is version 0.9 and the technical report is version 0.5-TR10. Both remain working drafts for external review. The Phase 1 plan is version 0.11.

The WP0 specification set is **frozen (WP0)** after three independent review rounds. WP1 mechanism-verification spikes are next. A failed spike reopens the ADR or specification that depends on its result.

The current claim is deliberately narrow. The Unix-governed baseline provides isolation, bounded authority, credential confinement, descendant control, and attribution. It does not claim general information-flow control. Integrity provenance is the first intended application; confidentiality compartments and multilevel release remain later profiles to be implemented and measured.

The repository contains no implementation yet, so none of these security properties has been demonstrated. Evidence will come from the WP1 mechanism spikes and subsequent conformance tests.

## Reviewing the design

The most useful review comments identify:

- a contradiction or unstated assumption;
- an invariant without an enforcement mechanism;
- a test that could falsify a claim;
- an unnecessary expansion of privilege or the trusted computing base;
- an operational dependency that changes feasibility;
- a simpler mechanism or narrower defensible claim.

Until contribution and security-disclosure policies are added, treat this repository as a design-stage project rather than a production security tool.
