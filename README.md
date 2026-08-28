# Agentbound

**A policy-driven security and execution substrate for organizational AI agents.**

Agentbound develops and evaluates an architecture in which an organizational AI agent is a durable security principal, while each task runs as a separately governed Unix session and process tree. The foundational papers retain the title *Agents as Unix Principals*.

The project is currently moving from reviewed architecture documents toward a bounded Phase 1 reference implementation.

## Start here

- [Position paper](docs/papers/position-paper.md) — concise statement of the thesis, motivation, and adoption argument.
- [Technical report](docs/papers/technical-report.md) — normative mechanisms, invariants, threat model, deployment profiles, and evaluation programme.
- [Phase 1 reference implementation plan](docs/plans/phase-1-reference-implementation.md) — draft implementation and evaluation plan for review.

## Repository layout

```text
docs/
  papers/         Position paper and normative technical report
  plans/          Reviewed and proposed project plans
  architecture/   Future implementation specifications and decisions
implementation/   Future reference implementation components
tests/            Future adversarial conformance and integration tests
```

`implementation/` and `tests/` are placeholders until the Phase 1 plan and mechanism choices have been reviewed. The project should avoid committing to a programming language or component decomposition before the specification-freeze and mechanism-spike work packages.

## Document ownership

- The **position paper** owns motivation, thesis, adoption, and conclusions.
- The **technical report** owns mechanisms, invariants, threats, constraints, and evaluation.
- The **Phase 1 plan** owns implementation scope, gates, work packages, and expected evidence.
- Future files under `docs/architecture/` will own concrete schemas, interfaces, lifecycle specifications, and architecture decisions.

## Current status

The papers are working drafts for external review. The Phase 1 plan is version 0.1 and specifically requests feedback before implementation begins.

No security claims should be inferred from the empty implementation layout. Claims become meaningful only when mapped to mechanisms and reproducible test evidence.

## Contributing during review

Review comments are most useful when they identify:

- a contradiction or unstated assumption;
- an invariant that lacks an enforcement mechanism;
- a test that would falsify a claim;
- a privilege or trusted-computing-base expansion;
- an operational dependency that changes feasibility;
- a simpler mechanism or narrower initial claim.

Until contribution and disclosure policies are added, please treat the repository as a design-stage project rather than a production security tool.
