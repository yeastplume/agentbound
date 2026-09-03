# Agentbound

**A policy-driven security and execution substrate for organizational AI agents.**

Agentbound treats an organizational AI agent as a durable security principal. Each task runs in a separately governed Unix session and process tree. The foundational papers retain the title *Agents as Unix Principals*.

WP0 is frozen; WP1 mechanism verification is next.

## Documents and their authority

- [Position paper](docs/papers/position-paper.md) — motivation, thesis, adoption argument, and conclusions.
- [Technical report](docs/papers/technical-report.md) — mechanisms, invariants, threat model, deployment profiles, and evaluation criteria.
- [Phase 1 reference implementation plan](docs/plans/phase-1-reference-implementation.md) — implementation scope, milestones, gates, work packages, and required evidence.
- [WP0 architecture specifications](docs/architecture/README.md) — concrete requirements, schemas, lifecycle rules, component interfaces, test catalogue, traceability, and architecture decision records.
- [Writing policy](docs/STYLE.md) — editorial rules for every document in this repository.

Each document is authoritative for the subjects listed against it; where two documents overlap, the more specific one links to the owner.

## Repository layout

```text
docs/
  papers/         Position paper and technical report
  plans/          Implementation and evaluation plans
  architecture/   Frozen Phase 1 specifications and architecture decisions
implementation/   Reference implementation components (placeholder)
tests/            Test implementations, fixtures, and conformance suites (placeholder)
```

## Current status

| Artefact | Version | State |
|---|---|---|
| Position paper | 0.10 | working draft for external review |
| Technical report | 0.5-TR11 | working draft for external review |
| Phase 1 plan | 0.12 | active; WP0 complete |
| WP0 architecture set | see [index](docs/architecture/README.md) | **frozen (WP0)** after three independent review rounds |
| `implementation/`, `tests/` | — | empty until WP1 mechanism spikes pass |

A failed WP1 spike reopens the ADR or specification that depends on its result.

The claim is narrow. The Unix-governed baseline provides isolation, bounded authority, credential confinement, descendant control, and attribution; it does not claim general information-flow control. Integrity provenance is the first intended application; confidentiality compartments and multilevel release are later profiles.

No security property has been demonstrated yet. Evidence comes from the WP1 spikes and the conformance tests that follow.

## Reviewing the design

The most useful review comments identify:

- a contradiction or unstated assumption;
- an invariant without an enforcement mechanism;
- a test that could falsify a claim;
- an unnecessary expansion of privilege or the trusted computing base;
- an operational dependency that changes feasibility;
- a simpler mechanism or narrower defensible claim.

Until contribution and security-disclosure policies are added, treat this repository as a design-stage project rather than a production security tool.
