# Documentation guide

Start with [Status and assessment](STATUS.md). It explains the implemented capabilities, evidence gaps, current defects, and recommendation on further work. You do not need to read the papers or learn the work-package numbering first.

## Choose a reading path

| You want to… | Read |
|---|---|
| Decide whether to keep investing | [Status and assessment](STATUS.md) |
| Understand how a task runs | [Launcher and lifecycle design](../crates/DESIGN.md), then [gateway design](../crates/DESIGN-1B.md); check claims against current code |
| Review a security rule | [Requirements](architecture/phase-1-requirements.md), then the relevant [architecture specification](architecture/README.md) |
| Inspect reported measurements | [Ten audit-test results](evidence/wp3.1/raw/d12/README.md) and [latest retained full-suite report](evidence/wp3.1/raw/run-07-negative-controls.md); their dates and scopes differ |
| Understand why earlier green results were withdrawn | [WP3 correction](evidence/wp3/README.md), then [WP3.1 development history](evidence/wp3.1/README.md) |
| Explore the larger idea | [Position paper](papers/position-paper.md), then [technical report](papers/technical-report.md) — proposals and contracts, not shipped features |
| Understand the old development sequence | [Phase 1 plan](plans/phase-1-reference-implementation.md) — its status passages need reconciliation with later requirements/evidence |

## What each kind of document means

- **Status:** the present assessment. It links to evidence and names uncertainty; it does not change requirements or certify conformance.
- **Specification:** what the implementation is required to do. “Frozen” means changes require review, not that code implements or tests prove every rule.
- **Design note:** an implementation explanation. Check it against code when they differ.
- **Evidence:** what a particular run reported. A PASS belongs to a test assertion and a build, not to the project forever.
- **Plan/paper:** intended scope, rationale and future work. Existence of a section does not establish an implementation.

Historical registers are retained because failed tests and withdrawn claims matter. Their chronological text is not the current status dashboard. Raw run output should remain unchanged when later work disproves its conclusions.

## Old labels in plain English

| Label | Meaning |
|---|---|
| 1A | Launch, isolate and clean up shell tasks without a gateway |
| 1B | Add the process-authenticated gateway and restricted Git operation |
| 1C | Planned real-agent/model integration; now also carries the missing full-attribution work |
| 1D | Planned comparison with a microVM-based deployment |
| WP0 / WP1 | Specifications / early mechanism experiments |
| WP2 / WP3 | Initial launcher-lifecycle / gateway implementation work |
| WP3.1 | Repair and reassess the implementation and its tests after overclaims were found |
| WP4 | Planned model adapter and existing-harness integration |
| D-12 | The attribution-completeness test; ten repetitions of this test are not ten repetitions of the full suite |
| Gate | A decision to proceed based on specified evidence, not a component or a feature |
| ADR | Architecture decision record: a choice and its rationale |

## Maintaining the docs

Keep capabilities and current blockers in [STATUS.md](STATUS.md), contracts in their existing specification owners, and run-specific results beside their raw evidence. Link between them rather than copying status paragraphs. Follow the [writing policy](STYLE.md); put the reader's question and direct answer before identifiers or review history.
