# ADR-0003: MicroVM control substrate and pre-registered test equivalence

**Version:** 0.8
**Status:** Accepted for Phase 1 (substrate, configuration, classification rules, and decision rule); image digests are recorded in `pinned-configuration.json` when the images are built
**Date:** 28 August 2026
**Applies to:** Phase 1 milestone 1D control arm
**Related:** [Phase 1 plan §3.3, §4.3, §5–§8, WP0, and WP4b](../plans/phase-1-reference-implementation.md); [ADR-0001](ADR-0001-execution-identity.md); [ADR-0002](ADR-0002-gateway-authentication.md); [traceability matrix](traceability-matrix.md); [test catalogue](test-catalogue.md)

## Revision history

- **0.1** — Initial Firecracker control-arm definition and group-level pre-registration.
- **0.2** — Froze the Linux local-socket and Firecracker/vsock configurations; added vsock CID-lifetime binding, per-atomic-test classification rules, comparative decision rule, per-arm code accounting; configuration values left open for 0.3.
- **0.3** — Status reconciled with the architecture README; version pins, guest init, guest audit source, comparative thresholds, confidence method, operator script, VM instance token format, and SLOC comparability rule filled in; open questions reduced to WP1 verification items.
- **0.4** — Unblinding and operator-familiarity recording; per-ID `control-arm-register.md` as a 1D prerequisite; open questions disposed via the register.
- **0.5** — T-6.8 register row extended to 013.
- **0.6** — Post-freeze editorial maintenance (no normative change): catalogue described as frozen; typo.
- **0.7** — Editorial pass under docs/STYLE.md; no obligation, identifier, or value changed. Configuration and comparability rules split; owns the microVM, VM-attribution, and register rationale.
- **0.8** — systemd pin changed from the 258 series to the 257 series as shipped by Debian 13, the reference host distribution; no Phase 1 facility differs between them. Pinned-value change to a frozen artefact, made before any WP1 evidence is recorded.


## Context

Phase 1 evaluates a Unix-governed session whose identity, authority,
descendants, credentials, mediated effects, and audit identity are independently
enforced and attributed. The Linux arm deliberately uses ordinary host
mechanisms: a per-session execution identity, namespaces, cgroups, descriptor
discipline, lifecycle services, and a gateway.

A passing Linux-only suite cannot answer whether that shared-kernel design is
justified when the same policy and workload are behind a stronger workload
boundary. A container is not an adequate control: it repackages shared-kernel
mechanisms and measures packaging or operability, rather than the boundary
question.

This control arm is an evaluation substrate, not a claim that virtualization
solves policy derivation, information flow, credential use, remote-service
authorization, or attribution. Those controls remain necessary in both arms.
The comparison MUST be fixed before results are examined; otherwise a failed
test could be weakened, removed, or retrospectively called non-comparable.

## Decision: frozen substrate configurations

### Linux arm

The Linux arm MUST use only the ADR-0002 Candidate L local-socket topology.
It has no network interface or route and exposes exactly one explicitly mounted,
single-purpose `AF_UNIX SOCK_SEQPACKET` gateway endpoint. The gateway MUST use
the authoritative per-operation Linux process witness required by ADR-0002:
`SO_PASSCRED`/`SCM_CREDENTIALS`, bound to a pidfd (start time read through it), UID,
GID, PID namespace, scope, boot ID, active allocation, and active launch
record. It MUST reject `SCM_RIGHTS` and fail closed when that witness cannot be
obtained or verified.

No Linux veth, network-topology, resolver, or network alternative is part of
this evaluation arm. The candidate remains documented in ADR-0002, but MUST NOT
appear in an effective manifest or result labelled as this ADR's Linux arm.

### VM arm

The VM arm MUST use **Firecracker v1.16.1** with its bundled `jailer`. It
creates one microVM per session. The pinned set is:

| Item | Pinned value |
|---|---|
| VMM | Firecracker v1.16.1, release binary, SHA-256 recorded |
| Host kernel (both arms) | Linux 6.12 LTS series, one exact patch release chosen at image build, `CONFIG_SECCOMP_FILTER`, `CONFIG_CGROUP_FREEZER`, `CONFIG_VSOCKETS`, `CONFIG_VHOST_VSOCK`, `CONFIG_AUDIT`, Landlock enabled |
| Guest kernel | Same 6.12 patch release, Firecracker microvm config, `CONFIG_VIRTIO_VSOCKETS`, `CONFIG_AUDIT` |
| systemd (host) | 257 stable series as shipped by Debian 13, one exact release recorded. *Rationale:* the reference host is a Debian 13 VM; every systemd facility Phase 1 depends on (`cgroup.freeze`/`cgroup.kill` via scopes, pidfd-based process tracking, scope `UnitRemoved`/`PropertiesChanged` signals) is present and unchanged in 257, and a stable distribution pin is more reproducible than a hand-backported 258 |
| Guest rootfs | Minimal Debian-derived image built reproducibly from a committed manifest; ext4, read-only; SHA-256 recorded |
| Guest init | `agentbound-guest-init`: a static, single-binary init from this repository (subreaper, vsock forwarder, workload exec, audit shipper); its build digest is recorded |
| Guest audit source | Linux Audit (`auditd` not used); `agentbound-guest-init` reads the audit netlink socket directly and ships records over the single vsock service, tagged with the VM instance token |
| Jailer | `--chroot-base-dir` per session, dedicated UID/GID per session from the same allocator, `--cgroup-version 2`, `--new-pid-ns`, seccomp default filter, no `--netns` |
| vCPU / memory | 1 vCPU, 512 MiB, no balloon |
| Snapshot/restore | disabled |

Before the first 1D run, the per-ID expanded classification register (one row per test-catalogue ID) MUST be generated from the group rules in this ADR and committed as `control-arm-register.md`. It is an execution prerequisite of milestone 1D. No reclassification is permitted after it is committed. Every value MUST be written to `pinned-configuration.json` (schema: item, value, digest, source) before the first control-arm run. Every control-arm evidence record MUST cite the file's own digest. A change to any item starts a new, separately labelled run set.

The VM device inventory is closed:

- one read-only `virtio-blk` rootfs;
- one per-session writable `virtio-blk` workspace;
- one `AF_VSOCK` device and exactly one guest-to-host vsock service; and
- nothing else.

In particular, the VM MUST NOT expose virtio-net, a second vsock service, a
host filesystem share, console or management service, entropy device,
accelerator, balloon device, debug channel, device passthrough, or another
listener. The workspace MUST NOT be concurrently shared except through an
explicitly authorized and separately recorded channel.

The guest init and guest audit source are those in the pinned-set table above. Both MUST be included in the pinned configuration and evidence record. Guest root is not host root. It MUST have no host credential, capability, management endpoint, device, or host identity.

The VM exposes exactly one vsock path to `agentbound-gateway`. This vsock path
is the **non-network microVM projection of the single-channel property**. It is
substrate-equivalent to Linux `AF_UNIX`; it is never identical to Linux
`AF_UNIX`. `AF_VSOCK` does not transfer Unix peer credentials or the Linux
per-operation process witness.

Snapshot and restore are disabled. No snapshot image, restore path, or
checkpoint lifecycle is in scope for this arm. A fallback VMM is a separate,
explicitly labelled arm with its own frozen configuration and test register;
its results MUST NEVER be pooled with Firecracker results.

### VM identity, CID lifetime, and vsock admission

The host endpoint MUST bind every accepted vsock connection to all of:

- the host-observed guest CID;
- a non-reusable VM instance token: 128 bits from the host CSPRNG, generated
  by `agentbound-lifecycle` at VM launch, recorded in the launch binding's
  `host_binding`, passed to the guest only via the Firecracker boot arguments,
  and never reissued for another instance on the same host and boot ID;
- the VMM pidfd and VMM process start time;
- the jailer identity;
- the active immutable launch record; and
- the Firecracker configuration digest and pinned-version set.

A guest-supplied CID, trace identifier, or authorization ID MUST NOT be
authentication evidence. The binding record MUST be created before admission of the first typed operation. It MUST be immutable while the launch record is active. The gateway MUST re-check active launch-record and grant state on every typed operation, as ADR-0002 requires.

Mappings and indexed connections MUST be invalidated before a CID can be
reassigned. Termination MUST mark the launch record terminating, deny new and
next-operation admission, close indexed vsock connections, terminate the VM,
verify VMM/jailer exit and guest teardown evidence, invalidate the CID mapping,
and only then permit CID reuse. Failure to establish or invalidate any binding
MUST fail closed and leave the CID unavailable for reassignment.

Vsock has no host-kernel counterpart to the Linux `SCM_CREDENTIALS` witness.
Therefore the VM arm claims **session-level attribution for the process leg**
over vsock, not per-process attribution, unless a guest-side witness is added
and pre-registered. This is a pre-registered difference. Guest process audit MAY correlate a guest process to a request. It MUST NOT upgrade the cross-boundary claim without that witness and its validation test.

### Held constant and varied

Both arms MUST hold constant policy and derivation inputs; authorization
manifest and launch-record semantics; gateway and typed adapters; trace
identity; audit ontology and correlation rules; workloads and workload
artefacts; test data and assertions; hostile-workload objective; and all
substrate-independent manifest fields.

Only the execution boundary and substrate-specific projection may vary. Linux
uses namespaces, mounts, cgroups, descriptor realization, and `AF_UNIX` process
evidence. The VM uses Firecracker, jailer, guest kernel/init/audit, closed
virtio inventory, vsock mapping, and VM lifecycle evidence. Each result MUST
list its substrate-specific refinements and residual assumptions.

## Decision: adversary corpora and comparability

The Linux attacker is an unprivileged session process. The VM arm MUST run two
separate corpora: one with guest root and one with a guest unprivileged process.
Guest root is stronger inside the guest but has no host authority. Results from the two VM corpora MUST NOT be merged. Each evidence record MUST state the corpus and privilege class.

A test is **identical** only when its test, inputs, expected outcome, attacker
privilege class, policy, gateway, and audit expectation run unchanged. A test
is **substrate-equivalent** only when it preserves the SAME property against the
SAME attacker privilege class, while replacing a substrate mechanic with its
named VM realization. If an attack mechanic has no VM counterpart, it is **not
directly comparable**. It remains arm-specific evidence but MUST be excluded
from the common boundary-strength score.

The frozen [test catalogue](test-catalogue.md) is the authoritative list of
IDs: `T-<suite>-<nnn>` for §6.x bullets, `D-<nn>` for demonstrations 1–17, and
`F-C-<n>`/`F-T-<n>` for construction/termination fault points. The register
below fixes classifications at plan-bullet granularity. Before milestone 1D,
these group-level classifications MUST be expanded mechanically into one
committed row per catalogue ID (`control-arm-register.md`); no result,
outcome, or observation may cause reclassification.

### Demonstration register

| ID | Class | Linux attacker / realization | VM attacker / realization | Preserved property |
|---|---|---|---|---|
| D-01 | identical | unprivileged; same authorized request | guest root; guest unprivileged; same request | durable identity and bounded derivation |
| D-02 | identical | unprivileged; observation/attach grant | both corpora; same grant | explicit initiator and authorized control |
| D-03 | substrate-equivalent | unprivileged; private state probes | both; workspace, vsock, VMM/jailer probes | cross-principal state isolation |
| D-04 | substrate-equivalent | unprivileged; sibling UID/process probes | both; cross-VM state and host-surface probes | same-principal session isolation |
| D-05 | identical | unprivileged; runtime replacement | both; same replacement | stable identity and attribution |
| D-06 | substrate-equivalent | unprivileged; child/grandchild escape | both; guest descendants to host boundary | descendant containment |
| D-07 | substrate-equivalent | unprivileged; daemonization | both; guest init/VMM teardown | supervision coverage |
| D-08 | substrate-equivalent | unprivileged; terminate/revoke | both; VM terminate/revoke | ordered control and revocation |
| D-09 | substrate-equivalent | unprivileged; `AF_UNIX` bypass corpus | both; sole-vsock-path bypass corpus | gateway-only egress |
| D-10 | substrate-equivalent | unprivileged; process-witness attribution | both; session-level process-leg attribution | mediated-effect attribution, at stated granularity |
| D-11 | substrate-equivalent | evaluator; constructor faults | evaluator; VM construction faults | fail-closed construction |
| D-12 | substrate-equivalent | unprivileged; local/gateway audit reconstruction | both; guest/VMM/jailer/gateway correlation | audit fidelity at stated attribution scope |
| D-13 | identical | unprivileged; Git gateway corpus | both; same typed requests | protected-object promotion boundary |
| D-14 | identical | unprivileged; binding mutations | both; same mutations | execution-binding control |
| D-15 | identical | unprivileged; narrowed child manifest | both; same authority derivation | monotonic delegation |
| D-16 | identical | unprivileged; live revocation cases | both; same cases | lifecycle and revocation |
| D-17 | identical | unprivileged; register execution | both; registered execution | pre-registered comparison discipline |

For D-01, D-02, D-05, D-13 through D-17, the VM workload corpus is separately
run as guest root and guest unprivileged; “identical” identifies the common
logical test, not a claim that guest and Linux credentials are the same kernel
artefact. D-10 and D-12 MUST label Linux per-process evidence and VM
session-level process-leg evidence separately.

### Suite register

Every row below represents the plan bullet(s) named in the final catalogue.
`I` means identical, `SE` substrate-equivalent, and `NDC` not directly
comparable. All Linux entries use the unprivileged session process; every VM
entry MUST be executed separately as guest root and guest unprivileged.

| Catalogue keys / plan bullets | Class, in plan-bullet order | Linux realization / VM realization | Preserved property |
|---|---|---|---|
| T-6.1-001…013: `/proc`; ptrace/process-vm/signals; PTY; Unix/abstract sockets; shared IPC; temp races; runtime/workspace; environment injection; inherited/`SCM_RIGHTS` FDs; pidfd; process-vm; abstract netns; broker reuse | NDC, NDC, SE, NDC, SE, SE, SE, I, SE, NDC, NDC, NDC, NDC | session process probes each Linux interface / both VM corpora probe guest state, workspace, vsock, VMM, jailer, and exposed host surfaces | cross-session process, IPC, descriptor, and state isolation; NDC excluded |
| T-6.2-001…008: cgroup; namespace; set-ID/file-cap; capability; daemon/orphan; mount/procfs; persistence; interpreter/loader | NDC, NDC, SE, SE, SE, SE, SE, SE | session process attacks / both corpora attack guest controls and guest-to-host boundary | no escape, persistence, or lifecycle escape |
| T-6.3-001…008: environment; files/procfs; descriptors; children; brokers; logs; post-termination; replay | I, I, I, I, SE, I, I, I | session-visible secret probes / both corpora run same probes; vsock broker replaces Unix broker | credential confinement and stated exportability |
| T-6.4-001…015: socket families; interfaces/routes; host Unix sockets; abstract sockets; inherited descriptors; `SCM_RIGHTS`; cross-session descriptor; credential-message count/forgery; PID reuse; stream/dgram connect; SSRF/tenant; redirect/TLS; identity replay; post-revocation; vsock realization | SE, SE, NDC, NDC, I, NDC, NDC, NDC, NDC, NDC, I, I, SE, I, VM-arm only | session process exercises `AF_UNIX` gateway and prohibited families / both corpora exercise sole-vsock-path admission, typed adapter, no-device surface, and CID reuse | gateway-only egress, per-operation process attribution (Linux only), and typed-operation confinement |
| T-6.5-001…010: encoding; size; path/TOCTOU; replay; policy/catalogue change; version downgrade; smuggled fields; signatures; allocation; caller identity | I for each catalogue-expanded bullet | authenticated initiator attacks common resolver/constructor / same common resolver and VM launcher binding | constructor validation and provenance |
| T-6.6-001…008: principal/task; approval; quorum; owner; excessive grant; catalogue; rollback; ambiguity | I for each catalogue-expanded bullet | authenticated initiator attacks common derivation / same common derivation | bounded derivation |
| T-6.7-001: narrowed mounts, descriptors, grants, limits, and recovery paths | I | parent session derives child / both corpora derive same authority subset | monotonic delegation |
| T-6.8-001…013: initiator; approval; authority; policy/catalogue; cancellation; policy-service loss; reclassification; Git grant; gateway loss; inference binding; audit degradation; lifecycle-daemon loss; forbidden degraded mapping | I for each milestone bullet (T-6.8-012: VM arm kills the same host daemon; identical) | hostile session during common revocation / both VM corpora during same revocation | declared lifecycle and revocation behaviour |
| T-6.9-001…008: PIDs; FDs; memory/CPU; disk; I/O/network; connections; audit; gateway budgets | SE, SE, SE, SE, SE, SE, SE, I | session process exhausts applicable limits / both corpora exhaust guest limits; guest RSS plus VMM RSS recorded | enforced present resource classes and absent-class disclosure |
| F-C-01…09: pipe/eventfd barrier; mount namespace; source resolution; root; proc; descriptor closure; UID/LSM/cap/seccomp; record/grant/socket; exec | SE for each point | evaluator fault-injects Linux construction / evaluator fault-injects Firecracker/jailer/guest construction | no runnable partial session, credential, or ambiguous audit record |
| F-T-01…11: admission; freeze; kill; reap; no-live proof; grant close; broker close; unmount; socket unmount; identity reclaim; record seal | SE for each point | evaluator interrupts Linux teardown / evaluator interrupts termination, CID invalidation, and cleanup | ordered termination, invalidation, and cleanup |

The `…` notation is fixed mechanical expansion notation, not a range that may be
silently changed. The catalogue MUST assign every plan bullet one or more IDs
within the shown scheme, retain this class, name its precise VM realization, and
state its property. Any catalogue item whose mechanic lacks a VM counterpart
MUST be NDC, retained as arm-specific evidence, and excluded from the common
boundary-strength score.

## Decision: frozen measurement protocol

WP0 MUST pin the host kernel; Firecracker version; guest kernel; rootfs digest;
jailer settings and identity; device inventory; guest init; guest audit source;
and measurement tooling before any result is collected. The Linux pinned kernel
and its selected local-socket configuration MUST be recorded in the same form.

Memory accounting is fixed: steady-state RAM per session is **guest RSS plus VMM
RSS**, reported separately and summed for the VM arm. The report MUST state the Linux process/cgroup RSS method separately. It MUST NOT conceal VMM overhead in a host-only aggregate. Snapshot/restore is disabled and therefore MUST NOT be
measured as a supported lifecycle capability.

Trusted-code size MUST be reported per arm under identical, tool-pinned SLOC
rules. The report MUST separately count direct privileged code, generated code,
transitive dependencies, configuration-rule code, and unsafe-language code. VM
trusted code MUST include the VMM, jailer, guest init, and configuration as well
as launcher and gateway-related trusted components. If these accounting rules
cannot yield comparable measurements, code size MUST NOT be used as a cross-arm
score; per-arm disclosure remains required.

## Pre-registered comparative decision rule

The Linux design is **justified as default for the Profile U threat model** only
if both conditions below hold.

1. Every common boundary test—every identical or substrate-equivalent test—MUST
   reach its expected preventive outcome on the Linux arm, with no unresolved
   preventive-invariant failure.
2. Across `N = 10` repetitions per arm on the same host, Linux MUST beat the
   Firecracker VM arm on **all three** of the following, each judged by the
   95% bootstrap confidence interval (10 000 resamples) of the paired
   per-repetition difference lying entirely on the Linux side of the threshold:

   | Measure | Threshold (Linux better by at least) |
   |---|---|
   | p95 session construction-plus-teardown latency (D-01 workload, cold start, `requested` → `sealed`) | 50% |
   | steady-state RSS per idle session after 60 s (Linux: cgroup `memory.current`; VM: VMM RSS + guest `MemTotal − MemAvailable`, reported separately and summed) | 50% |
   | scripted operator task: "find why session X was denied, terminate it, confirm reclamation" — wall time and step count, run by three operators blind to arm; operational artefacts (unit names, process trees, latency) may reveal the substrate, so the report MUST record any observed unblinding and each operator's prior familiarity with either arm | 25% on both time and steps |

   The workload is the D-01 scenario with the T-6.9-001 fan-out disabled; the
   operator script is committed alongside the test catalogue before any run.

Any Linux boundary failure selects the VM regardless of performance. If no
threshold is met by either arm, the result is **“no measured advantage; VM
preferred on assurance.”** If condition 1 passes but condition 2 fails, the
result is **“Linux viable but not justified on operational grounds.”** A result
that does not satisfy both conditions MUST NOT call Linux the default.

The common boundary-strength score includes only identical and
substrate-equivalent tests. NDC tests remain published as arm-specific evidence.
A VM failure does not erase a Linux failure, and a retry does not erase either
negative result. Confidence intervals and all individual repetitions MUST be
published with the evidence set.

## Consequences

- Milestone 1D produces the Phase 2 decision input; it does not retrospectively
  convert 1A–1C evidence into a comparative claim.
- Firecracker configuration drift requires a new labelled arm or a new frozen
  evaluation; it MUST NOT be treated as a minor implementation variation.
- VM assurance includes VMM, jailer, host kernel, guest kernel, image, guest
  init, audit source, and host lifecycle assumptions.
- The control arm accepts the cost of a second launcher, guest artefacts,
  Firecracker/jailer configuration, and paired corpora.
- A hardened-container measurement MAY be useful, but MUST be a separately
  labelled arm and cannot replace this microVM control.

## Alternatives considered

### Network/veth VM topology

Rejected. The evaluation VM exposes one vsock path only. Adding virtio-net or a
veth path would add a second topology and defeat the frozen single-channel
comparison.

### Fallback VMM

Rejected as a fallback within this arm. Cloud Hypervisor, Kata, or another KVM-based implementation MAY be evaluated only as a separate labelled arm. Results MUST NEVER be pooled with the Firecracker arm.

### Hardened container (`gVisor`/`runsc`)

Rejected as the required control. It is an optional labelled third arm for
operability or syscall-interposition comparison, not evidence against the
shared-kernel boundary question.

### Dedicated node

Rejected for Phase 1. It changes per-session provisioning, audit correlation,
lifecycle control, and fair per-session comparison too substantially.

### No control arm

Rejected. It would leave the central architecture comparison untested and
prevent milestone 1D from supplying the Phase 2 decision input.

## Open questions carried to WP1

Items 1 and 3 are WP1 verification items *VM-1* and *VM-2* in the [open-question register](open-question-register.md); item 2 is answered there.

1. Does the pinned 6.12 patch release expose vsock peer-CID reporting sufficient for the host endpoint binding, or does the binding require the VMM's own connection table?
2. *(answered — no witness in Phase 1; the pre-registered arm difference stands; see the [open-question register](open-question-register.md))*
3. Are the five SLOC figures comparable across arms once the VMM, jailer, and guest init are counted? If the pinned tool cannot attribute Firecracker's transitive Rust dependencies consistently, cross-arm SLOC is disclosed per arm and excluded from the decision rule (already the rule in the accounting section; WP1 confirms whether the exclusion is triggered).
