# Agentbound

**A policy-driven security and execution substrate for organizational AI agents.**

Agentbound develops and evaluates an architecture in which an organizational AI agent is a durable security principal, while each task runs as a separately governed Unix session and process tree. The foundational papers retain the title *Agents as Unix Principals*.

The project is currently in WP0 specification review for a bounded Phase 1 reference implementation.

## Start here

- [Position paper](docs/papers/position-paper.md) — concise statement of the thesis, motivation, and adoption argument.
- [Technical report](docs/papers/technical-report.md) — normative mechanisms, invariants, threat model, deployment profiles, and evaluation programme.
- [Phase 1 reference implementation plan](docs/plans/phase-1-reference-implementation.md) — staged implementation and evaluation programme.
- [WP0 architecture specifications](docs/architecture/README.md) — normative requirements, schemas, lifecycle specifications, component interfaces, test catalogue, traceability, and ADRs.

## Repository layout

```text
docs/
  papers/         Position paper and normative technical report
  plans/          Reviewed and proposed project plans
  architecture/   Architecture decision records and future implementation specifications
implementation/   Future reference implementation components
tests/            Future adversarial conformance and integration tests
```

`implementation/` and `tests/` are placeholders until the Phase 1 plan and mechanism choices have been reviewed. The project should avoid committing to a programming language or component decomposition before the specification-freeze and mechanism-spike work packages.

## Document ownership

- The **position paper** owns motivation, thesis, adoption, and conclusions.
- The **technical report** owns mechanisms, invariants, threats, constraints, and evaluation.
- The **Phase 1 plan** owns implementation scope, gates, work packages, and expected evidence.
- **Architecture specifications and decision records** under `docs/architecture/` own concrete requirements, schemas, lifecycle rules, traceability, and decisions that constrain implementation.

## Current status

The papers are working drafts for external review (position paper 0.9, technical report 0.5-TR9), revised after three independent reviews. The Phase 1 plan is version 0.7. Its WP0 architecture specifications (requirements, manifest schema, session and identity lifecycles, component interfaces, test catalogue, traceability matrix, and three ADRs) are drafts under review after a second independent review round; implementation begins only after that set is frozen.

The shortest honest summary of the current claim: the Unix-governed baseline is an isolation, authority, and attribution profile that makes no information-flow claim; integrity provenance is the nearest practical payoff; confidentiality compartments and multilevel release are later, measured profiles. The formal rules are a specification to implement and test against, not a proven model.

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
