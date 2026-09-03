# Architecture specifications

This directory contains WP0 implementation specifications and architecture decision records (ADRs) that refine the normative [technical report](../papers/technical-report.md) and the [Phase 1 plan](../plans/phase-1-reference-implementation.md).

## WP0 specification set and status

Decision status (is the architecture settled?) and specification completeness (are the parameters filled in?) are distinct; both are shown.

| Artefact | Version | Decision status | Specification completeness |
|---|---|---|---|
| [Phase 1 normative requirements](phase-1-requirements.md) | 0.5 | draft for WP0 review | complete; all questions answered |
| [Authorization manifest and launch binding schema](manifest-schema.md) | 0.5 | draft for WP0 review | complete, with gateway-free 1A form and worked examples; all questions answered |
| [Session lifecycle and failure states](session-lifecycle.md) | 0.5 | draft for WP0 review | complete; one WP1 item (LC-2) |
| [Execution-identity lifecycle](execution-identity-lifecycle.md) | 0.5 | draft for WP0 review | complete; one WP1 item (ID-1) |
| [Component interfaces (skeleton)](component-interfaces.md) | 0.2 | security-relevant decisions frozen | wire formats and versioning negotiation deferred to WP1 by design |
| [Test catalogue](test-catalogue.md) | 0.4 | pre-registration frozen at this version | load profiles, deadlines, seeds fixed; fixtures and commands are WP1 outputs by design |
| [Invariant-to-test traceability matrix](traceability-matrix.md) | 0.4 | draft for WP0 review | complete; requirement-coverage check performed (§8) |
| [Open-question disposition register](open-question-register.md) | 0.2 | freeze input | all 32 WP0 questions disposed: 28 answered in their owning documents, 4 assigned to named WP1 items |
| [ADR-0001](ADR-0001-execution-identity.md) execution identity | — | **accepted** (revised) | lifecycle details in the identity-lifecycle specification |
| [ADR-0002](ADR-0002-gateway-authentication.md) gateway topology and authentication | 0.4 | **accepted for Phase 1**, conditional on the Decision 7 kernel-baseline verification in WP1 | mechanism complete; one connection per process |
| [ADR-0003](ADR-0003-control-substrate.md) control substrate | 0.5 | **accepted for Phase 1** | configuration, thresholds, confidence method, and classification rules filled; image digests recorded in `pinned-configuration.json` at image build; per-ID `control-arm-register.md` is a 1D prerequisite; two WP1 items (VM-1, VM-2) |

**Freeze condition.** WP0 freezes when the review of this version raises no CERTAIN-class defect and each open question is either answered or assigned a WP1 verification item. The second condition is met by the [open-question register](open-question-register.md); the WP1 verification set is ADR-0002 Decision 7 plus register items VM-1, VM-2, LC-2, ID-1. Declaring the freeze is a project decision recorded by changing every artefact's status line from "draft for WP0 review" to "frozen (WP0)" in one commit.

## Mandatory implementation contracts

Obligations an implementer must not miss, with their owning section:

| Contract | Where | Milestone |
|---|---|---|
| Policy signs an allocation-free authorization manifest; the constructor signs the launch binding after atomic identity reservation; the pair is the launch record. `authorization_id` is the pre-binding key, `launch_record_digest` the post-binding identity | manifest schema §3–4 | 1A |
| 1A manifests use `gateway.channel_topology: none`: no channel, same no-network boundary; `local-socket` from 1B | manifest schema §3.4; requirements R-GW-1 | 1A |
| Construction step 1 is a `clone3` synchronization barrier; nine ordered steps; rollback ledger | session lifecycle §3, §9.3 | 1A |
| `agentbound-lifecycle` is the sole actor for quiesce, termination, reclamation, and reconciliation; systemd cannot invoke a helper for a scope | session lifecycle §4; component interfaces | 1A |
| Eleven-step termination: deny gateway admission, freeze, let init reap, `cgroup.kill`, verify emptiness plus host credential scan, then release | session lifecycle §5; requirements R-ISO-4 | 1A |
| Identity state machine `free → allocated → in-use → reclaiming → quarantined → free`; reclamation is a condition, never a period; exports never rely on the numeric UID | identity lifecycle §4–5 | 1A |
| `loginuid` is corroborating; one fail rule (R-CON-6) | requirements §5; identity lifecycle §6 | 1A |
| Every resource class is enforced or absent-with-evidence per the milestone matrix | requirements R-RES-5; manifest schema §3.5 | 1A–1C |
| One `SOCK_SEQPACKET` gateway socket; `SO_PEERCRED` + pidfd at connect; one `SCM_CREDENTIALS` per packet per operation; one connection per process; `SCM_RIGHTS` rejected | ADR-0002 Decisions 1–2; requirements R-GW-1…3 | 1B |
| Every operation re-checks live grant state; admitted-before-revocation Git pushes are complete-and-record | ADR-0002 Decision 4 | 1B |
| vsock CID bound to a non-reusable VM token and invalidated before reuse; VM arm claims session-level attribution for the process leg | ADR-0002 Decision 6; ADR-0003 | 1D |
| Observed result class is unconstrained; expected class is pre-registered | traceability matrix §1 | all |

## Open decisions carried into WP1

Each WP0 document ends with an open-questions section. Items that require kernel-baseline evidence are listed in ADR-0002 Decision 7 and the plan's WP1; a failed verification reopens the owning ADR before WP2 begins.

## Authority and change control

The technical report remains the source for invariants and threat model. The Phase 1 plan owns milestones, gates, work packages, and evidence scope. This directory owns concrete schemas, lifecycle rules, thresholds, interfaces, tests, and implementation decisions.

An ADR or specification that changes a technical-report invariant, plan gate, profile claim, or milestone allocation MUST also update its owning document. Draft open questions are not decisions. Once WP0 freezes the set, changes require a revision-history entry and an impact review against the traceability matrix and test catalogue.
