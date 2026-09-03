# Architecture specifications

This directory contains WP0 implementation specifications and architecture decision records (ADRs) that refine the normative [technical report](../papers/technical-report.md) and the [Phase 1 plan](../plans/phase-1-reference-implementation.md).

## WP0 specification set and status

**WP0 is frozen.** Every artefact below is at its frozen version; ADRs retain their accepted status. A change to any frozen artefact requires a revision-history entry and, where the change touches a technical-report invariant, plan gate, profile claim, or milestone allocation, the cross-document propagation rule at the end of this page.

Decision status (is the architecture settled?) and specification completeness (are the parameters filled in?) are distinct; both are shown.

| Artefact | Version | Decision status | Specification completeness |
|---|---|---|---|
| [Phase 1 normative requirements](phase-1-requirements.md) | 0.9 | frozen (WP0); WP1 findings applied | complete; all questions answered |
| [Authorization manifest and launch binding schema](manifest-schema.md) | 0.6 | frozen (WP0) | complete, with gateway-free 1A form and worked examples; all questions answered |
| [Session lifecycle and failure states](session-lifecycle.md) | 0.7 | frozen (WP0); WP1 findings applied | complete; LC-2 answered by WP1 |
| [Execution-identity lifecycle](execution-identity-lifecycle.md) | 0.7 | frozen (WP0); WP1 findings applied | complete; ID-1 answered by WP1 |
| [Component interfaces (skeleton)](component-interfaces.md) | 0.3 | frozen (WP0); security-relevant decisions | wire formats and versioning negotiation deferred to WP1 by design |
| [Test catalogue](test-catalogue.md) | 0.6 | frozen (WP0); pre-registration | load profiles, deadlines, seeds fixed; fixtures and commands are WP1 outputs by design |
| [Invariant-to-test traceability matrix](traceability-matrix.md) | 0.6 | frozen (WP0) | complete; requirement-coverage check performed (§8) |
| [Open-question disposition register](open-question-register.md) | 0.5 | frozen (WP0) | all 32 WP0 questions disposed: 28 answered in their owning documents, 4 verified in WP1 with none taking its failure branch |
| [ADR-0001](ADR-0001-execution-identity.md) execution identity | — | **accepted** (revised) | lifecycle details in the identity-lifecycle specification |
| [ADR-0002](ADR-0002-gateway-authentication.md) gateway topology and authentication | 0.7 | **accepted for Phase 1**; Decision 7 items 1–5 verified in WP1 (items 6–9 need gateway code, WP2) | mechanism complete; one connection per process |
| [ADR-0003](ADR-0003-control-substrate.md) control substrate | 0.9 | **accepted for Phase 1**; VM-1, VM-2 answered by WP1 | configuration, thresholds, confidence method, and classification rules filled; image digests recorded in `pinned-configuration.json` at image build; per-ID `control-arm-register.md` is a 1D prerequisite; two WP1 items (VM-1, VM-2) |

**Freeze record.** Both freeze conditions were met at commit `fae6912` (no CERTAIN-class defect in the third independent review; every open question answered or assigned to a WP1 item in the [open-question register](open-question-register.md)). The canonical WP1 verification set is recorded in the register's summary (ADR-0002 Decision 7 items; VM-1, VM-2, LC-2, ID-1; implementation spike LC-1); a failed item reopens the owning ADR or specification before the constructor is built.

## Mandatory implementation contracts

Implementation contracts, with their owning section:

| Contract | Where | Milestone |
|---|---|---|
| Policy signs an allocation-free authorization manifest; the constructor signs the launch binding after atomic identity reservation; the pair is the launch record. `authorization_id` is the pre-binding key, `launch_record_digest` the post-binding identity | manifest schema §3–4 | 1A |
| 1A manifests use `gateway.channel_topology: none`: no channel, same no-network boundary; `local-socket` from 1B | manifest schema §3.4; requirements R-GW-1 | 1A |
| Construction step 1 creates the child with `clone3`; a pipe or eventfd synchronization barrier blocks it before untrusted execution; nine ordered steps; rollback ledger | session lifecycle §3, §9.3 | 1A |
| `agentbound-lifecycle` is the sole actor for quiesce, termination, reclamation, and reconciliation; systemd cannot invoke a helper for a scope | session lifecycle §4; component interfaces | 1A |
| Eleven-step termination: deny gateway admission, freeze, let init reap, `cgroup.kill`, verify emptiness plus host credential scan, then release | session lifecycle §5; requirements R-ISO-4 | 1A |
| Identity state machine `free → allocated → in-use → reclaiming → quarantined → free`; reclamation is a condition, never a period; exports never rely on the numeric UID | identity lifecycle §4–5 | 1A |
| `loginuid` is corroborating; one fail rule (R-CON-6) | requirements §5; identity lifecycle §6 | 1A |
| Every resource class is enforced or absent-with-evidence per the milestone matrix | requirements R-RES-5; manifest schema §3.5 | 1A–1C |
| One `SOCK_SEQPACKET` gateway socket; `SO_PEERCRED` + pidfd at connect; one `SCM_CREDENTIALS` per packet per operation; one connection per process; `SCM_RIGHTS` rejected | ADR-0002 Decisions 1–2; requirements R-GW-1…3 | 1B |
| Every operation re-checks live grant state; admitted-before-revocation Git pushes are complete-and-record | ADR-0002 Decision 4 | 1B |
| vsock CID bound to a non-reusable VM token and invalidated before reuse; VM arm claims session-level attribution for the process leg | ADR-0002 Decision 6; ADR-0003 | 1D |
| Observed result class is unconstrained; expected class is pre-registered | traceability matrix §1 | all |

## WP1 verification items

WP1 verification items are recorded in the [open-question register](open-question-register.md) and the owning document; a failed verification reopens the owning ADR before WP2 begins. **WP1 is complete** ([evidence register](../evidence/wp1/README.md)): no item failed and no ADR was reopened; eight findings (F-1–F-8) were applied to the frozen documents as revision entries (requirements 0.9, session lifecycle 0.7, identity lifecycle 0.7, ADR-0002 0.7, ADR-0003 0.9, technical report 0.5-TR12).

## Authority and change control

The technical report remains the source for invariants and threat model. The Phase 1 plan owns milestones, gates, work packages, and evidence scope. This directory owns concrete schemas, lifecycle rules, thresholds, interfaces, tests, and implementation decisions.

An ADR or specification that changes a technical-report invariant, plan gate, profile claim, or milestone allocation MUST also update its owning document. Changes to the frozen WP0 set require a revision-history entry and an impact review against the traceability matrix and test catalogue.
