# Phase 1 invariant-to-test traceability matrix

**Version:** 0.1  
**Status:** Draft for WP0 review  
**Date:** 28 August 2026  
**Conformance target:** [technical report](../papers/technical-report.md) §7, Unix-governed profile U  
**Requirements:** [Phase 1 normative requirements](phase-1-requirements.md)  
**Test definitions:** [Phase 1 plan](../plans/phase-1-reference-implementation.md) §§5–7

## 1. Result vocabulary

Each row receives one and only one result at execution time:

1. **enforced and passed**;
2. **enforced and failed**;
3. **detected only**;
4. **assumption** (with owner and revalidation trigger);
5. **not applicable** to profile U.

If a property is applicable but its milestone was not reached, the result is **not evaluated**, not "not applicable." `Not evaluated` is a milestone-progress status, not one of the five final conformance result classes; once evaluated, an applicable invariant receives exactly one final class. Each published result MUST carry the milestone reached.

## 2. Adversary-capability matrix

| Test group | Adversary capability | Principal interfaces exercised | Explicit exclusions |
|---|---|---|---|
| §6.1 session interference | Arbitrary unprivileged code in either of two concurrent sessions, including same-principal sessions | `/proc`, pidfd/signals, ptrace/process-vm, `/tmp`, `/run`, Unix sockets, durable partition, broker socket, inherited/received descriptors | Kernel exploit, side channels |
| §6.2 escape/persistence | Arbitrary fork, double-fork, daemonization, orphaning, signal handling, set-ID/file-cap execution attempts | PID namespace, cgroup v2, systemd scope, pidfds, capability set, `no_new_privs`, seccomp, Landlock, namespace handles | Kernel exploit, D-state prevention |
| §6.3 credential recovery | Read all session-visible environment, files, procfs, descriptors; invoke broker/gateway as an authorized session process | environment, filesystem, `/proc/self`, descriptors, broker/gateway protocol | Compromise of gateway or upstream service |
| §6.4 gateway bypass | Arbitrary socket/syscall use and typed-operation inputs from workload | IPv4/IPv6, UDP/QUIC, link-local/metadata, vsock, Unix sockets, inherited connections, redirects, DNS, Git adapter | Kernel/firewall exploit |
| §6.5 constructor inputs | Authenticated initiator sends arbitrary bytes and races server-side state | request decoder, schema, path resolution, policy/catalogue versioning, allocator, signature envelope | Compromise of policy signing key |
| §6.6 bounded derivation | Authenticated initiator supplies unauthorized, stale, replayed, conflicting, or ambiguous references | identity registry, approval store, task policy, runtime catalogue, derivation function | Malicious administrator changing policy legitimately |
| §6.7 monotonic delegation | Parent session controls child creation inputs and all inherited state available to it | mounts, descriptors, gateway grants, budgets, set-ID paths, cgroup/systemd paths | Kernel exploit |
| §6.8 lifecycle/revocation | Authorized revoker or dependency failure acts while hostile workload runs | policy/control plane, lifecycle helper, gateway, inference adapter, launch-record store | Instant termination of D-state tasks |
| §6.9 exhaustion | Hostile session consumes every present resource class up to and beyond its limit | cgroup CPU/memory/PIDs/IO, gateway requests/bytes/bandwidth/storage/spend/tokens | Resource classes absent from deployment |
| §7.3 fault injection | Evaluator interrupts every construction and cleanup step, including crash/restart | constructor, allocator, mounts, namespaces, firewall, gateway grants, launch records, lifecycle helper | Kernel/hardware corruption |

## 3. Applicable invariants

| Inv | Property | Milestone complete | Normative requirements | Mechanism | Primary tests / demos | Required result class | Residual assumption / owner / impact / compensating control / acceptance authority / trigger |
|---:|---|---|---|---|---|---|---|
| 1 | Durable identity | 1A | R-ID-1, R-ID-6 | Principal registry; signed manifest; durable ownership projection separate from execution identity | §6.5 unknown/forbidden identity; §6.6 unauthorized and ambiguous principal; demos 1–2 | enforced and passed | Registry correctness / policy owner / registry or schema change |
| 2 | Explicit initiator | 1A | R-ID-2 | Authenticated request; initiator, approver, scheduler and owner fields in launch record | §6.6 missing/unauthenticated initiator and scheduled owner; demo 10 audit chain | enforced and passed | Initiator authenticator / IAM owner / authentication mechanism change |
| 3 | Bounded derivation | 1A | R-ID-3…5, R-REQ-5 | `agentbound-policy` derivation; constructor accepts only the policy-signed authorization manifest; constructor signs allocation-bound launch binding | §6.6 complete suite; demo 11 derived-authority display | enforced and passed | Policy correctness / policy owner / policy-language change |
| 6 | Monotonic delegation | 1A | R-CON-3…4, R-ISO-5 | Capability bounding, `no_new_privs`, descriptor allowlist, narrower child manifest | §6.7; demo 15 | enforced and passed | Base image has no set-ID/file-cap path / image owner / image digest change |
| 7 | Fail-closed construction | 1A | R-REQ-1…6, R-CON-1…7 | Ordered constructor, rollback log, signed manifest, exec last | §6.5 request suite; §7.3 every construction point; demo 1 | enforced and passed | Kernel APIs behave as documented / platform owner / pinned-version change |
| 10 | Gateway-only egress | 1B | R-GW-1…4 | ADR-0002 selected topology; firewall/seccomp or single Unix socket; typed adapter | §6.4 bypass corpus; demos 7, 9, 12 | enforced and passed | Gateway and upstream TLS endpoint trusted / gateway owner / adapter or topology change |
| 11 | Credential confinement | 1B | R-GW-3, R-GW-6 | No upstream secret in session; gateway/broker holds credential | §6.3; demo 10 | enforced and passed, or assumption where hardware-backed non-exportability is claimed | Authentication credential exportability / gateway owner / mechanism change |
| 12 | Complete descendant control | 1A | R-ISO-3…4 | systemd scope, cgroup v2, PID-ns init/subreaper, pidfd, `cgroup.kill` | §6.2; demo 6; §7.3 termination faults | enforced and passed | D-state can delay but not escape; kernel trusted / platform owner / kernel change |
| 13 | Attribution of mediated effects | 1B | R-CON-6, R-AUD-1…4 | Signed launch record, loginuid, trace identity, kernel and gateway audit correlation | demo 8; §6.4 effects; §6.8 transitions; threshold ≥99% nominal and 100% gateway effects | detected only (attribution is detective) | Audit availability and loss / audit owner / load or audit configuration change |
| 14 | Policy provenance | 1A | R-ID-8, R-AUD-4 | Policy/catalogue versions and derivation inputs in signed append-only launch record | §6.5 forged/downgraded versions and signature confusion; §6.6 rollback; demo 11 | detected only for historical provenance; enforcement of version currency must pass | Signing key and launch-record store / policy owner / key or store change |
| 15 | Launch privilege disposal | 1A | R-CON-3…5 | Capability bounding/drop, `no_new_privs`, separate enumerated lifecycle helper | §6.2 capability/set-ID/systemd/cgroup attacks; line-by-line review; demo 5 | enforced and passed | Lifecycle helper trusted / implementation owner / helper operation change |
| 17 | Same-principal session isolation | 1A | R-ISO-1…2 | Per-session execution UID, namespaces, private state, descriptor discipline | §6.1 complete suite; demos 3–4 | enforced and passed | Kernel and constructor trusted / platform owner / pinned-version or constructor change |
| 19 | Integrity promotion (protected-object subset) | 1B | R-GW-5 | Git adapter restricts staging ref; independent branch protection promotes | §6.4 direct/forged/cross-session push; demo 13 | enforced and passed for staging boundary | Git host branch protection / repository owner / protection-rule change |
| 20 | Bounded external resources | 1B; inference classes 1C | R-GW-7, R-GW-9 | cgroup/rlimit/filesystem quotas, audit queue policy, delegation counters, plus per-operation gateway budgets | §6.9 for PIDs, FDs, CPU, memory, disk bytes/inodes, I/O/network bandwidth, connections, audit capacity, fan-out, storage, spend, tokens, and accelerator status; demos 7 and 14 where relevant | enforced and passed for every present class; absent classes listed | Gateway accounting and upstream billing reconciliation / service owner / resource-class addition |
| 21 | Lifecycle and revocation | Latest applicable service (1C in full programme) | R-LC-1…5 | Lifecycle helper; signed manifest behavior; per-operation gateway grant checks | §6.8 cases by milestone, including auditable fail-closed reclassification in U; demo 16; §7.3 lifecycle faults | assumption for policy choice; enforcement of declared behavior must pass | Correct choice of terminate/quiesce/degrade / policy owner / trigger or service addition |
| 22 | Execution-binding control | 1C | R-GW-8 | Inference adapter compares live operation with approved manifest binding | demo 14; §6.8 inference binding revocation; unauthorized model/endpoint/tenant/adapter/inference-pool/retention tests | enforced and passed | Gateway is only inference path / gateway owner / new inference path or adapter |

## 4. Not applicable to profile U

| Inv | Property | Why N/A to U | Future profile |
|---:|---|---|---|
| 4 | Labeled state | U does not assign formal `C`, `I`, purpose, or provenance labels to durable state | C/M/W |
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
launch_record_id
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
2. Confirm every requirement ID in `phase-1-requirements.md` appears here or is explicitly non-invariant operational support.
3. Confirm ADR-0003 has classified every demonstration and suite named above before any control-arm run.
4. Confirm the attribution threshold is paired with a fixed nominal-load profile before WP0 freezes.
5. Confirm Invariant 21 cannot be reported complete at 1A or 1B when later services remain in programme scope.
