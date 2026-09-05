# Agentbound

**A policy-driven security and execution substrate for organizational AI agents.**

Agentbound treats an organizational AI agent as a durable security principal. Each task runs in a separately governed Unix session and process tree. The foundational papers retain the title *Agents as Unix Principals*.

WP0 is frozen; WP1 mechanism verification, WP2 (milestone 1A: constructor, identity allocator, lifecycle daemon, policy stub, audit receiver, conformance suite) are complete. WP3 (milestone 1B: unprivileged gateway with per-connection process authentication, Git staging-ref adapter, end-to-end audit correlation) is implemented, but independent review found its conformance evidence incomplete — the [WP3 register](docs/evidence/wp3/README.md) records the correction and Gate 3 is **not yet evaluated**. WP3.1 (conformance correction and independent test) is the active work package and a hard gate before WP4.

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
crates/           Reference implementation (Rust workspace; see crates/DESIGN.md, crates/DESIGN-1B.md)
deploy/           Catalogue, systemd units, provisioning for the pinned host
crates/ab-conformance/  Conformance driver and in-session probes (runner; catalogue completeness check is WP3.1 item 1)
```

## Current status

| Artefact | Version | State |
|---|---|---|
| Position paper | 0.10 | working draft for external review |
| Technical report | 0.5-TR11 | working draft for external review |
| Phase 1 plan | 0.16 | active; WP0–WP2 complete (1A recorded); WP3 implemented, conformance exit not met after review; WP3.1 correction gate active |
| WP0 architecture set | see [index](docs/architecture/README.md) | **frozen (WP0)** after three independent review rounds |
| `crates/`, `deploy/` | commit-pinned | 1A + 1B reference implementation; [WP2 register](docs/evidence/wp2/README.md) 84/84; [WP3 register](docs/evidence/wp3/README.md) — runner 139/139 but catalogue coverage incomplete and ≥ 8 false-positive rows found in review; Gate 3 not evaluated |

A failed spike or conformance row reopens the ADR or specification that depends on its result; WP1 exercised this twice (ADR-0002, ADR-0003 amendments).

The claim is narrow. The Unix-governed baseline provides isolation, bounded authority, credential confinement, descendant control, and attribution; it does not claim general information-flow control. Integrity provenance is the first intended application; confidentiality compartments and multilevel release are later profiles.

Demonstrated so far, on one pinned Debian 13 host at topology `none`: the 1A session boundary (Gates 1 and 2 of the plan) — see the [WP2 evidence register](docs/evidence/wp2/README.md) for what each row observed and what remains partial. A mediated remote effect through the gateway is demonstrated once (WP3 D-13: staging ref with the session's trace, `main` untouched) but Gate 3 is not yet evaluated against the frozen criteria (WP3.1).

## Reviewing the design

The most useful review comments identify:

- a contradiction or unstated assumption;
- an invariant without an enforcement mechanism;
- a test that could falsify a claim;
- an unnecessary expansion of privilege or the trusted computing base;
- an operational dependency that changes feasibility;
- a simpler mechanism or narrower defensible claim.

Until contribution and security-disclosure policies are added, treat this repository as a design-stage project rather than a production security tool.
