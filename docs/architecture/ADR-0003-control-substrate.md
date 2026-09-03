# ADR-0003: MicroVM control substrate and pre-registered test equivalence

**Status:** Proposed for WP0 review  
**Date:** 28 August 2026  
**Applies to:** Phase 1 milestone 1D control arm  
**Related:** [Phase 1 plan §3.3, §4.3, §5–§8, WP0, and WP4b](../plans/phase-1-reference-implementation.md); [technical report §7, §9.2, and §11](../papers/technical-report.md); [ADR-0001](ADR-0001-execution-identity.md)

## Context

Phase 1 evaluates a Unix-governed session: an agent principal and task-scoped
session whose policy, identity, authority, descendants, credentials, mediated
effects, and audit identity are independently enforced and attributed. The
Linux arm is deliberately composed from ordinary host mechanisms: an
execution identity, namespaces, cgroups, descriptor discipline, a gateway, and
lifecycle services.

That design needs a control arm. Without one, a passing Linux-arm suite cannot
answer the architectural question: whether the additional host-process
isolation machinery is justified when the same policy and workload are placed
behind a stronger workload boundary. A container is not an adequate control for
that question. A container repackages shared-kernel mechanisms; it usefully
measures packaging or operability, but does not test the shared-kernel boundary
against a stronger alternative.

The control must be defined before outcomes are known. Otherwise a failed test
could be removed, weakened, or called non-comparable after observing the
microVM result. This ADR therefore fixes the substrate, the comparison surface,
and the classification of every Phase 1 demonstration, adversarial-suite group,
and fault-injection corpus before either arm's control-arm result is seen.

The control arm is an evaluation substrate, not a claim that virtualization
solves policy derivation, information flow, credential use, remote-service
authorization, or attribution. Those controls remain necessary on both arms.

## Decision: substrate

Use **Firecracker** as the default microVM implementation for one session per
microVM. Firecracker is selected because its minimal device model, jailer,
snapshot support, and wide use as a per-tenant boundary make its trusted and
operational surface appropriate to this comparison. If Firecracker is
unavailable on the evaluation host, Cloud Hypervisor or another KVM-based
microVM implementation may be used as a documented fallback. A fallback must
preserve every requirement below and identify the substituted VMM and jailer
semantics in the evaluation record.

The microVM launcher replaces `agentbound-launch`; it consumes the same
resolved, substrate-independent manifest and creates this configuration:

- A minimal guest kernel and minimal root filesystem are used. The rootfs block
  device is read-only.
- The VM has one vCPU. Its memory bound is the memory limit in the effective
  manifest; the launcher records any host reservation or VMM overhead
  separately from the guest-visible limit.
- The only writable block device is a per-session workspace block device. It is
  never shared concurrently with another session except through an explicitly
  authorized, separately recorded channel.
- The VM exposes exactly one path to `agentbound-gateway`, selected by the
  gateway topology chosen in ADR-0002. Under the **network topology**, that
  path is one veth path: guest virtio-net ↔ host veth/policy ↔ gateway. Under
  the **non-network microVM projection** of the abstract single-channel policy,
  that path is one vsock path: guest `AF_VSOCK` ↔ host gateway endpoint. This is
  substrate-equivalent to the Linux local-socket property, not the Linux
  `AF_UNIX` topology; ADR-0002 Decision 6 defines its authoritative VM/session
  evidence and explicit exemption from Linux `AF_UNIX` mechanics. The VM
  has no second network interface, no additional vsock service, and no route
  or listener beyond that selected path.
- No other devices are exposed except the read-only rootfs block device and the
  per-session workspace block device. In particular, no host filesystem share,
  console/management service, device passthrough, entropy service, accelerator,
  balloon device, or debug channel is in the workload device inventory unless a
  future ADR changes this decision and runs both arms' added tests.
- Firecracker runs through its jailer with a per-VM host UID. The microVM host
  process itself runs under a per-session execution identity as required by
  ADR-0001. The VM boundary and the identity split therefore compose; neither
  substitutes for documenting the other's lifecycle and reuse rules.
- Host-side firewall policy is identical in intent to the Linux arm: it permits
  only the selected gateway path (and a constructor-operated resolver only
  when the selected topology permits one), and denies host, peer-session,
  metadata, link-local, alternate-family, and direct protected-service paths.

Guest root is not host root. The guest contains no credential, capability,
management endpoint, device, or host identity that gives it host authority.
The VMM, jailer, host firewall, gateway, host kernel, and host lifecycle
services remain part of the trusted computing base and are counted as such.

Snapshots are permitted only as launcher-managed lifecycle artifacts. Snapshot
files, restore paths, and metadata are session-private host surfaces and are
included in the interference and persistence equivalent tests. Snapshot/restore
is not a claim of secure checkpoint/restore or transparent session migration.

## Decision: what is held constant and what varies

The following are held constant across the Linux and microVM arms:

- policy and every derivation input, including agent, initiator, task,
  approvals, policy and catalogue versions, and the immutable launch record;
- `agentbound-policy` and its policy results;
- `agentbound-gateway`, its selected authentication mechanism and typed
  adapters, its external authorization, and its gateway-policy decisions;
- session trace identity and the identity fields propagated to protected
  services;
- `agentbound-audit`, its effect ontology, correlation rules, loss counters,
  retention behavior, and result classification;
- workloads, workload artifacts, test data, test assertions, and the hostile
  in-session adversary's intended capabilities;
- all substrate-independent manifest fields: identity, derivation inputs,
  authority, budgets, gateway policy, and audit requirements.

The following vary:

- `agentbound-launch` is replaced by a minimal microVM launcher and its
  lifecycle helper;
- the execution boundary and all substrate-specific manifest fields, including
  namespaces, mounts, descriptor realization, seccomp, cgroup/VMM limits,
  device inventory, guest kernel/rootfs, firewall attachment, and the chosen
  veth or vsock realization;
- operating evidence specific to a substrate, such as Linux kernel audit events
  versus VMM/jailer/guest audit events, while preserving the common audit
  correlation contract.

The adversary differs in one intentional respect. In the Linux arm it is a host
process running under the session UID. In the microVM arm it runs as guest root.
Guest root is strictly more capable *inside the guest* than the Linux-arm
workload identity is inside its session, but it has no host UID, host process
identity, host capabilities, host mount view, or host-management endpoint.
This makes guest-to-host containment an explicit control-arm property rather
than silently granting the guest the Linux arm's host-process position.

## Decision: pre-registered test-equivalence classification

The classifications below are fixed before control-arm results are examined.
**Identical** means the same workload-level test, inputs, assertion, policy,
gateway, and audit expectation run unchanged. **Substrate-equivalent** means
that the security property and success criterion are unchanged, but the attack
mechanism is replaced by the named VM-host-surface test. **Inapplicable** is
allowed only where a mechanism has no bearing on the host boundary; the omitted
mechanism is still measured for guest operability where relevant.

| Item | Classification | Equivalent test or reason | Property preserved |
|---|---|---|---|
| Demo 1 — authorized start | identical | Same request, derivation, launch record, and policy decision. | Durable identity and bounded derivation. |
| Demo 2 — unauthorized observation/attachment | identical | Same grant and attachment authorization test. | Explicit initiator and authorized control. |
| Demo 3 — cross-principal private state | substrate-equivalent | Test guest/host surfaces: workspace blocks, vsock/veth, exposed devices, snapshot files, and launcher/jailer state. | Cross-principal state isolation. |
| Demo 4 — same-principal sibling isolation | substrate-equivalent | Replace same-host-UID probes with cross-VM probes of virtio devices, vsock, shared storage, snapshot files, and launcher/jailer surfaces. | Invariant 17 session isolation. |
| Demo 5 — runtime replacement | identical | Run the same replacement and compare identity, gateway scope, and audit chain. | Stable session identity and attribution. |
| Demo 6 — child/grandchild boundary | substrate-equivalent | Exercise guest descendants and verify they cannot reach VMM/jailer or escape guest limits. | Descendant containment. |
| Demo 7 — double-fork/daemonization | substrate-equivalent | Verify guest init/supervision, VMM lifecycle, and teardown account for daemonized guest descendants. | Supervision and termination coverage. |
| Demo 8 — terminate then revoke | substrate-equivalent | Terminate VM/guest workload, verify descendants stop, then verify gateway authority closes. | Ordering of descendant control and revocation. |
| Demo 9 — no direct protected-service access | identical | Run the same in-workload bypass corpus through the selected single path. | Gateway-only egress. |
| Demo 10 — allowed operation attribution | identical | Same typed gateway operation and trace/audit reconstruction assertion. | Attribution of mediated effects. |
| Demo 11 — constructor failure | substrate-equivalent | Inject failures into VM allocation, jailer, devices, path/firewall, audit bind, guest boot, and cleanup. | Fail-closed construction. |
| Demo 12 — audit reconstruction | identical | Same effect ontology and completeness calculation; substrate evidence is correlated by the same pipeline. | Audit fidelity and attributable effects. |
| Demo 13 — staging-ref restrictions | identical | Same gateway requests, trace forgery attempt, and protected Git service. | Protected-object promotion boundary. |
| Demo 14 — execution-binding change | identical | Mutate model, endpoint, tenant, adapter/weights, inference pool, and retention mode independently; require compatibility decision and audit for each. | Execution-binding control. |
| Demo 15 — delegated child narrowing | identical | Same derived child manifest, authority subset, gateway grant, and audit assertions. | Monotonic delegation. |
| Demo 16 — active revocation | identical | Same initiator, approval, policy, catalogue, reclassification, gateway, and binding withdrawal cases; Profile U reclassification fails closed and audits no labeled target. | Explicit lifecycle and revocation. |
| Demo 17 — control-arm repetition | identical | This row governs all rows classified identical; equivalent rows use their stated replacement. | Pre-registered comparability. |
| §6.1 interference | substrate-equivalent | Replace `/proc`, ptrace, host-PID, Unix-socket, and sibling-UID attacks with enumeration and attacks on virtio devices, vsock, workspace/shared storage, snapshots, and launcher/jailer state. | Cross-session process, IPC, descriptor, and state isolation. |
| §6.2 escape/persistence | substrate-equivalent | Replace cgroup/namespace escape with guest-to-host escape attempts; test VMM-process confinement, jailer confinement, guest resource limits, guest persistence, and snapshot/restore artifacts. | No escape or persistence beyond session boundary. |
| §6.3 credential recovery | identical | Same inspection of environment, files, processes, logs, crash output, post-termination use, and replay against gateway credentials. | Credential confinement and non-exportability statement. |
| §6.4 network/gateway bypass | substrate-equivalent | Preserve the bypass properties and corpus; replace Linux `AF_UNIX` peer/process mechanics with the ADR-0002 vsock VM/session mapping and guest-process evidence tests. | Gateway-only egress and typed-operation confinement. |
| §6.5 constructor inputs | identical | Same malformed request, canonicalization, replay, TOCTOU, policy-version, identity, and signature corpus against the common resolver/manifest contract. | Constructor input validation and provenance. |
| §6.6 bounded derivation | identical | Same unauthorized, expired, quorum, owner, catalogue, rollback, and ambiguity cases. | Invariant 3 bounded derivation. |
| §6.7 monotonic delegation | identical | Same child authority and resource assertions, including no recovery through grants or credentials. | Invariant 6 monotonic delegation. |
| §6.8 revocation | identical | Same declared behavior and audit checks for each component milestone. | Invariant 21 lifecycle and revocation. |
| §6.9 resource exhaustion | substrate-equivalent | Map cgroup tests to guest limits plus VMM/jailer confinement; retain identical gateway-budget tests. Account for guest and VMM memory separately. | Enforced present resource classes and explicit absent classes. |
| §7.3 fault injection | substrate-equivalent | Fault VM lifecycle stages in addition to common derivation, grant, audit, privilege, supervision, termination, and cleanup paths. | No runnable partial session, usable credential, or ambiguous audit record. |

`no_new_privs` inside the guest is **inapplicable to the host-boundary
comparison**: it cannot determine whether guest root reaches the host. It is
not an omitted security observation; guest-internal `no_new_privs`, capability
reduction, and hardening remain measured for operability and defense in depth.
No catalogue item above is classified inapplicable as a whole, because every
listed demonstration and suite group has either an identical or a
substrate-equivalent property test.

No item may be reclassified after control-arm results are seen. A newly added
test after that point must run on both arms, receive a pre-result classification,
and be reported separately from this frozen catalogue.

## Decision: comparative measurements

For each arm, publish the measurement method, pinned host/kernel/VMM versions,
workload, repetitions, result distribution, and applicable resource class. The
comparison includes:

- boundary strength, as pass/fail per pre-registered item and residual
  assumption;
- launch latency and teardown latency;
- steady-state memory, including guest-visible memory and VMM/jailer overhead;
- descendant-termination latency;
- audit fidelity, attribution completeness, and audit loss behavior;
- policy and configuration complexity, including rule counts and exceptions;
- privileged and trusted code size, explicitly counting the VMM and jailer as
  trusted in the microVM arm;
- accelerator sharing, recorded as absent for this Phase 1 configuration rather
  than treated as an unmeasured equivalence;
- operator actions required to launch, diagnose, patch, recover, and clean up.

Representative workload/syscall overhead, retained privilege breadth,
credential issuance and storage continuity may be reported as supporting
measurements. Equivalent assurance is never inferred merely because the two
arms share substrate-independent manifest fields. Each arm must list its
substrate-specific refinements, evidence sources, and residual assumptions.

## Consequences

- Milestone 1D produces the Phase 2 decision input. It is not a gate that
  retroactively turns 1A–1C evidence into a comparative claim.
- Any claimed Linux-arm advantage—lower latency, lower memory, simpler
  operations, smaller trusted surface, or adequate boundary strength—must be
  measured. It must not be assumed from the use of native host mechanisms.
- The microVM arm is the reference implementation path for a future Profile W
  (strong workload isolation), while preserving the common policy, gateway,
  identity, and audit abstractions.
- The project accepts the cost of maintaining a second launcher, minimal guest
  artifacts, VMM/jailer configuration, and a paired automated test corpus.
- The broader trusted computing base becomes more explicit: a microVM reduces
  guest exposure to host kernel interfaces but adds VMM, jailer, image, and
  snapshot lifecycle responsibilities.
- A hardened-container measurement may be useful, but it cannot replace this
  control arm or be presented as evidence against shared-kernel risk.

## Alternatives considered

### Hardened container (`gVisor`/`runsc`)

Rejected as the required control arm. It is a valuable optional, explicitly
labeled third arm for operability and syscall-interposition comparison, but it
does not answer the microVM-strength boundary question as directly as a VM.

### Kata Containers

Kata is a plausible middle option because it commonly uses VM-backed workload
isolation. It is not the default because its OCI/container integration adds a
larger orchestration and device surface than the minimal direct Firecracker
configuration. It may be evaluated as an optional labeled arm, not silently
substituted for this ADR's control.

### Plain OCI runtime

Rejected as a control substrate. A plain OCI runtime primarily exercises the
same host kernel, namespace, cgroup, and capability mechanisms as the Linux
arm and therefore measures packaging rather than the strength of an
independent workload boundary.

### Dedicated node

Rejected for Phase 1. A dedicated node supplies a strong physical scheduling
boundary, but makes per-session provisioning, audit correlation, lifecycle
control, and fair per-session comparison substantially different. It remains a
possible later deployment profile or a separately labeled operational control.

### No control arm

Rejected. It would leave the central comparison untested and would prevent
milestone 1D from producing the Phase 2 decision input required by the plan.

## Open questions for WP0 review

1. ADR-0002 must select one mutually exclusive Linux-arm topology. Its
   Decision 6 classifies the single vsock path as the pre-registered
   non-network microVM projection of the abstract single-channel property and defines authoritative
   VM/session evidence. Is that mapping sufficiently strong for Gate 3?
2. What exact Firecracker and host-kernel version, jailer configuration, and
   fallback acceptance criteria are pinned before testing?
3. What guest-init and guest audit source supplies descendant and process
   attribution without introducing an unreviewed management channel?
4. How are snapshot encryption, access control, reclamation, and crash cleanup
   included in the managed reclamation domain of ADR-0001?
5. Which measurement boundary counts host page cache, KVM allocation, and VMM
   process memory consistently across arms?
6. What gateway authentication evidence binds a vsock peer to the VM session
   and immutable launch record, including connection lifetime and revocation?
7. Is guest root necessary for the intended adversary model, or should a
   second, non-root guest workload measurement be reported without replacing
   the pre-registered guest-root control?
8. What pre-registered thresholds distinguish a material assurance gain from a
   measurement difference, particularly for boundary failures, audit loss, and
   operator actions?
