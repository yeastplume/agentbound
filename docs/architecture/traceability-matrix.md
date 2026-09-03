# Phase 1 invariant-to-test traceability matrix

**Version:** 0.6  
**Status:** Frozen (WP0)  
**Date:** 28 August 2026  
**Conformance target:** [technical report](../papers/technical-report.md) §7, Unix-governed profile U  
**Requirements:** [Phase 1 normative requirements](phase-1-requirements.md)  
**Test definitions:** [test catalogue](test-catalogue.md); source enumerations in [Phase 1 plan](../plans/phase-1-reference-implementation.md) §§5–7

## Revision history

- **0.1** — Initial WP0 draft.
- **0.2** — Result column renamed to expected mechanism class / pass criterion with observed class unconstrained; Inv 14 and 21 split into detective/assumption and enforced halves; Inv 10/13 rows aligned with ADR-0002 0.2; test references point to the test catalogue.
- **0.3** — Requirement-coverage check performed and recorded (§8); R-ID-7 and R-AUD-5 traced; test-catalogue and identifier terminology aligned.
- **0.4** — T-6.8 references extended to 013.
- **0.5** — Post-freeze editorial maintenance (no normative change): residual column defined as three fields; rows 14a and 21a completed; requirements version reference updated.
- **0.6** — Editorial pass under docs/STYLE.md; no obligation, identifier, or value changed. §1 split; control-arm attribution pointer to ADR-0003.


## 1. Result vocabulary

Each row receives one and only one result at execution time:

1. **enforced and passed**;
2. **enforced and failed**;
3. **detected only**;
4. **assumption** (with owner and revalidation trigger);
5. **not applicable** to profile U.

The "expected mechanism class / pass criterion" column in §3 is the pre-registered expectation. The **observed** `result_class` in every evidence record is unconstrained among the five classes above and MUST be recorded as observed. If a property is applicable but its milestone was not reached, the result is **not evaluated**, not "not applicable." `Not evaluated` is a milestone-progress status, not one of the five final conformance result classes; once evaluated, an applicable invariant receives exactly one final class. Each published result MUST carry the milestone reached.

## 2. Adversary-capability matrix

| Test group | Adversary capability | Principal interfaces exercised | Explicit exclusions |
|---|---|---|---|
| §6.1 session interference | Arbitrary unprivileged code in either of two concurrent sessions, including same-principal sessions | `/proc`, pidfd/signals, ptrace/process-vm, `/tmp`, `/run`, Unix sockets, durable partition, broker socket, inherited/received descriptors | Kernel exploit, side channels |
| §6.2 escape/persistence | Arbitrary fork, double-fork, daemonization, orphaning, signal handling, set-ID/file-cap execution attempts | PID namespace, cgroup v2, systemd scope, pidfds, capability set, `no_new_privs`, seccomp, Landlock, namespace handles | Kernel exploit, D-state prevention |
| §6.3 credential recovery | Read all session-visible environment, files, procfs, descriptors; invoke broker/gateway as an authorized session process | environment, filesystem, `/proc/self`, descriptors, broker/gateway protocol | Compromise of gateway or upstream service |
| §6.4 channel/gateway bypass | Arbitrary socket/syscall use, ancillary-data manipulation, and typed-operation inputs from workload | socket families, netns interfaces/routes, pathname and abstract Unix sockets, inherited descriptors, `SCM_RIGHTS`/`SCM_CREDENTIALS`, gateway protocol, Git adapter | Kernel or seccomp exploit |
| §6.5 constructor inputs | Authenticated initiator sends arbitrary bytes and races server-side state | request decoder, schema, path resolution, policy/catalogue versioning, allocator, signature envelope | Compromise of policy signing key |
| §6.6 bounded derivation | Authenticated initiator supplies unauthorized, stale, replayed, conflicting, or ambiguous references | identity registry, approval store, task policy, runtime catalogue, derivation function | Malicious administrator changing policy legitimately |
| §6.7 monotonic delegation | Parent session controls child creation inputs and all inherited state available to it | mounts, descriptors, gateway grants, budgets, set-ID paths, cgroup/systemd paths | Kernel exploit |
| §6.8 lifecycle/revocation | Authorized revoker or dependency failure acts while hostile workload runs | policy/control plane, `agentbound-lifecycle`, gateway, inference adapter, launch-record store | Instant termination of D-state tasks |
| §6.9 exhaustion | Hostile session consumes every present resource class up to and beyond its limit | cgroup CPU/memory/PIDs/IO, gateway requests/bytes/bandwidth/storage/spend/tokens | Resource classes absent from deployment |
| §7.3 fault injection | Evaluator interrupts every construction and cleanup step, including crash/restart | constructor, `agentbound-lifecycle` (allocator), mounts, namespaces, gateway socket, gateway grants, launch records | Kernel/hardware corruption |

## 3. Applicable invariants

The last column is a compact cell of exactly three slash-separated fields: *residual assumption / owner / revalidation trigger*. The full six-field `residual_assumption` record (adding impact, compensating control, and acceptance authority) is produced per test in the report record of §6.

| Inv | Property | Milestone complete | Normative requirements | Mechanism | Primary tests / demos (IDs per test catalogue) | Expected mechanism class / pass criterion | Residual assumption / owner / revalidation trigger |
|---:|---|---|---|---|---|---|---|
| 1 | Durable identity | 1A | R-ID-1, R-ID-6, R-ID-7 | Principal registry; signed manifest; durable ownership projection separate from execution identity | T-6.5-010; T-6.6-001, T-6.6-008; D-01, D-02 | enforced and passed | Registry correctness / policy owner / registry or schema change |
| 2 | Explicit initiator | 1A | R-ID-2 | Authenticated request; initiator, approver, scheduler and owner fields in launch record | T-6.6-002, T-6.6-004; D-10 | enforced and passed | Initiator authenticator / IAM owner / authentication mechanism change |
| 3 | Bounded derivation | 1A | R-ID-3…5, R-REQ-5 | `agentbound-policy` derivation; constructor accepts only the policy-signed authorization manifest; constructor signs allocation-bound launch binding | T-6.6-001..008; D-11 | enforced and passed | Policy correctness / policy owner / policy-language change |
| 6 | Monotonic delegation | 1A | R-CON-3…4, R-ISO-5 | Capability bounding, `no_new_privs`, descriptor allowlist, narrower child manifest | T-6.7-001; D-15 | enforced and passed | Base image has no set-ID/file-cap path / image owner / image digest change |
| 7 | Fail-closed construction | 1A | R-REQ-1…6, R-CON-1…7 | Ordered constructor, rollback log, signed manifest, exec last | T-6.5-001..010; F-C-01..09; D-01, D-11 | enforced and passed | Kernel APIs behave as documented / platform owner / pinned-version change |
| 10 | Gateway-only egress | 1B | R-GW-1…4 | Single `SOCK_SEQPACKET` gateway socket, no network interface, seccomp socket-family deny (ADR-0002); typed adapter | T-6.4-001..014; D-09 | enforced and passed | Gateway and upstream TLS endpoint trusted / gateway owner / adapter or topology change |
| 11 | Credential confinement | 1B | R-GW-3, R-GW-6 | No upstream secret in session; gateway/broker holds credential | T-6.3-001..008; D-10 | enforced and passed, or assumption where hardware-backed non-exportability is claimed | Authentication credential exportability / gateway owner / mechanism change |
| 12 | Complete descendant control | 1A | R-ISO-3…4 | systemd scope, cgroup v2, PID-ns init/subreaper, pidfd, `cgroup.kill` | T-6.2-001..008; D-06, D-07, D-08; F-T-01..11 | enforced and passed | D-state can delay but not escape; kernel trusted / platform owner / kernel change |
| 13 | Attribution of mediated effects | 1B | R-CON-6, R-GW-3, R-AUD-1…5 | Signed launch record; per-operation `SCM_CREDENTIALS` process evidence; trace identity; corroborating loginuid; kernel and gateway audit correlation | D-10, D-12; T-6.4-006..009, T-6.4-013; T-6.8-001..013; metric and load profiles per test catalogue §5; pass at ≥99% nominal and 100% gateway corpus | detected only (attribution is detective); VM arm records session-level for the process leg (see ADR-0003) | Audit availability and loss / audit owner / load or audit configuration change |
| 14a | Policy provenance — historical record | 1A | R-ID-8, R-AUD-4 | Policy/catalogue versions and derivation inputs in signed append-only launch record | T-6.5-008; D-11 | detected only | Launch-record store integrity / policy owner / store change |
| 14b | Policy provenance — version currency | 1A | R-ID-8, R-REQ-5 | Constructor rejects withdrawn or superseded versions | T-6.5-006; T-6.6-007 | enforced and passed | Signing key and launch-record store / policy owner / key or store change |
| 15 | Launch privilege disposal | 1A | R-CON-3…5 | Capability bounding/drop, `no_new_privs`, separate enumerated `agentbound-lifecycle` daemon | T-6.2-001, T-6.2-003, T-6.2-004; privileged-code review under requirements §12; D-05 | enforced and passed | `agentbound-lifecycle` trusted / implementation owner / daemon operation change |
| 17 | Same-principal session isolation | 1A | R-ISO-1…2 | Per-session execution UID, namespaces, private state, descriptor discipline | T-6.1-001..013; D-03, D-04 | enforced and passed | Kernel and constructor trusted / platform owner / pinned-version or constructor change |
| 19 | Integrity promotion (protected-object subset) | 1B | R-GW-5 | Git adapter restricts staging ref; independent branch protection promotes | T-6.4-011, T-6.4-013; D-13 | enforced and passed for staging boundary | Git host branch protection / repository owner / protection-rule change |
| 20 | Bounded external resources | 1B; inference classes 1C | R-RES-1…5, R-GW-7, R-GW-9 | cgroup/rlimit/storage bounds, audit queue policy, delegation counters, plus per-operation gateway budgets | T-6.9-001..008 (class-by-milestone per R-RES-5); D-07, D-14 where relevant | enforced and passed for every present class; absent classes listed | Gateway accounting and upstream billing reconciliation / service owner / resource-class addition |
| 21a | Lifecycle and revocation — policy choice | 1A | R-LC-2 | Manifest declares terminate/quiesce/continue-degraded per trigger | manifest review | assumption | Declared choice is appropriate / policy owner / trigger or service addition |
| 21b | Lifecycle and revocation — implementation of declared behaviour | Latest applicable service (1C in full programme) | R-LC-1, 3…5 | `agentbound-lifecycle`; per-operation gateway grant checks | T-6.8-001..013 by milestone (T-6.8-007 is the U reclassification case; 011–013 are the outage and forbidden-degraded cases); D-16; F-T-01..11 | enforced and passed | Correct choice of terminate/quiesce/degrade / policy owner / trigger or service addition |
| 22 | Execution-binding control | 1C | R-GW-8 | Inference adapter compares live operation with approved manifest binding | D-14; T-6.8-010 | enforced and passed | Gateway is only inference path / gateway owner / new inference path or adapter |

## 4. Not applicable to profile U

| Inv | Property | Why N/A to U | Future profile |
|---:|---|---|---|
| 4 | Labelled state | U does not assign formal `C`, `I`, purpose, or provenance labels to durable state | C/M/W |
| 5 | Information admission | U has no label-aware receive operation or edge propagation | C/M/W |
| 8 | Non-widening confidentiality delegation | No confidentiality lattice in U | C/M/W |
| 9 | No-write-down / trusted release | No confidentiality lattice or declassifier in U | M/W |
| 16 | Edge-aware authority derivation | U mediates named effects but does not derive authority from propagated edge labels | C/M/W |
| 18 | Integrity-aware operation | No general integrity propagation in U; Invariant 19 is the narrow protected-object substitute | C/M/W |

## 5. Gate and milestone roll-up

| Gate / milestone | Passing evidence |
|---|---|
| Gate 1 / 1A construction | Invariants 1, 2, 3, 7, 14, 15 plus constructor request suite and all construction fault points pass; privileged code within bound |
| Gate 2 / 1A isolation | Invariants 6, 12, 17 pass; 1A cases of 21 pass but 21 remains incomplete |
| Gate 3 / 1B remote effects | Invariants 10, 11, 13, 19 and present 1B resource classes of 20 pass; 1B cases of 21 pass |
| 1C full Profile U | Invariant 22, inference classes of 20, and 1C cases of 21 pass; every applicable invariant now has a terminal result |
| Gate 4 / 1A–1C operability | Deterministic lifecycle, diagnostics, audit counters, measurements, result table, owner and trigger for every assumption |
| 1D comparative claim | ADR-0003's frozen equivalence table run; no post-result reclassification; comparative measurements complete |

## 6. Evidence record template

Each automated test MUST emit or reference an evidence record containing:

```text
test_id
invariant_ids[]
requirement_ids[]
milestone
arm = linux | microvm
manifest_digest
authorization_id
host_id
boot_id
pinned_version_set
adversary_capabilities[]
interfaces_exercised[]
expected_outcome
observed_outcome
result_class
artifacts[]
residual_assumption { statement, owner, impact, compensating_control, acceptance_authority, revalidation_trigger } | null
```

The report MUST preserve negative results. Retrying a failed test does not erase the failure; both attempts remain linked, with the cause of nondeterminism stated.

## 7. WP0 review checks

1. Confirm every Profile U invariant in technical-report §7 appears exactly once in §3 or §4.
2. Confirm every requirement ID in `phase-1-requirements.md` appears here or is explicitly non-invariant operational support. **Performed for requirements 0.5:** all 58 IDs are accounted for — 48 traced to invariants in §3 and 10 recorded as non-invariant support in §8.
3. Confirm ADR-0003 has classified every demonstration and suite named above before any control-arm run.
4. Confirm the attribution threshold is paired with the fixed nominal-load profile in the test catalogue §5 (done in catalogue 0.2).
5. Confirm Invariant 21 cannot be reported complete at 1A or 1B when later services remain in programme scope.

## 8. Non-invariant requirements (recorded, not traced to an invariant)

| Requirement | Class | Where it is checked |
|---|---|---|
| R-SCOPE-1, R-SCOPE-2, R-SCOPE-3 | claim discipline | evaluation-report review against plan §2 and §3.4 |
| R-THREAT-1 | evaluation completeness | adversary-capability matrix in this document §2 and the report |
| R-CON-8 | reviewability bound | SLOC accounting per requirements §12; Gate 1 |
| R-INT-1, R-INT-2, R-INT-3 | integration and stop condition | milestone 1C harness run; Gate 4 operability |

R-ID-7 (reclamation condition) and R-AUD-5 (denial diagnostics) are traced to Invariants 1 and 13 respectively.
