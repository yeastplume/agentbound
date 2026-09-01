# Phase 1 Reference Implementation Plan

**Status:** Draft for review  
**Plan version:** 0.2  
**Date:** 28 August 2026  
**Related position paper:** [`../papers/position-paper.md`](../papers/position-paper.md)  
**Normative technical report:** [`../papers/technical-report.md`](../papers/technical-report.md)  
**Decision records:** [`../architecture/ADR-0001-execution-identity.md`](../architecture/ADR-0001-execution-identity.md)

---

## Revision history

- **0.1** — Initial plan: Unix-governed profile, four gates, seven components, WP0–WP6, ordinal sequence.
- **0.2** — Incorporated three independent reviews: Phase 1 claim narrowed to isolation, authority, credential confinement, descendant control, and attribution of mediated effects, with information-flow invariants marked not applicable; per-session execution identity committed (ADR-0001); gateway-only egress topology specified; components collapsed to four plus a CLI; a thin integrity slice added; container/microVM control arm made a required evaluation arm; integration contract and adoption "step 0" added.

---

## 1. Purpose

This plan defines the next project stage after review of the *Agents as Unix Principals* position paper and technical report. It proposes a bounded Phase 1 implementation intended to test the smallest important architectural claim:

> An existing agent runtime can execute inside a task-scoped Unix session whose identity, authority, descendants, resources, credentials, and mediated effects are enforced and attributed independently of the model and harness.

Phase 1 is a security experiment, not a production platform. It focuses on the **Unix-governed session** profile, which the technical report defines as an **isolation, authority, and attribution profile, not an information-flow profile**. Phase 1 therefore does not test the confidentiality- or integrity-propagation thesis; that belongs to the compartmented profile and Phase 2. It deliberately postpones full SELinux MLS, dynamic category allocation, semantic declassification, distributed workflow orchestration, durable-memory promotion, and shared-accelerator isolation.

The expected outcome is evidence: a reference implementation, adversarial conformance suite, measured evaluation, and a list of failed or residual assumptions mapped to the technical report's invariants.

---

## 2. Goals and non-goals

### 2.1 Goals

Phase 1 will demonstrate that:

1. A durable global agent identity can be bound to a session that runs under a **per-session, non-reusable execution identity** (ADR-0001), with the durable principal owning state but never executing code.
2. Every session is bound to an authenticated initiator, task, effective policy, and immutable launch record produced by the derivation relation.
3. Sessions belonging to different principals—and concurrent sessions belonging to the same principal—cannot access each other's private process, IPC, terminal, credential, or workspace state through the enumerated interface inventory.
4. Untrusted descendants cannot escape the session's namespaces, cgroup, resource controls, or termination boundary through tested techniques.
5. The cognitive runtime can be replaced without changing session identity or enforcement, and a change of model endpoint is recorded as an execution-binding event.
6. Service access is narrow, short-lived or brokered, and bound to session identity rather than inherited from a human or shared daemon.
7. Direct egress cannot bypass the approved service gateway through the tested bypass set, given the specified egress topology.
8. **Mediated** local and remote effects can be reconstructed as:

   ```text
   initiator → agent principal → session → process → object or gateway operation
   ```

   using the launch record and propagated trace identity, corroborated by kernel and gateway audit.
9. Failed construction leaves no runnable partial session, reusable credential, or ambiguous audit record.
10. The trusted and privileged code surface—launch path **and** post-launch lifecycle services—can be enumerated, measured, and reviewed.
11. One **thin integrity slice** holds: a session can stage a change to a protected object (a Git branch) but cannot reach the protected branch or production object except through a validating promotion path outside the session.
12. The same workload runs under a hardened container or microVM control arm with the substrate-independent manifest fields, and the measured delta is published.

### 2.2 Non-goals

Phase 1 will not claim to provide:

- confidentiality or integrity information-flow control on any edge (technical-report Invariants 8, 9, 16, and 18 are **not applicable** to Phase 1 and appear in the evidence table only as N/A; Invariant 19 is claimed only for the protected-object subset in goal 11);
- SELinux MLS/MCS category allocation;
- secure semantic declassification;
- persistent agent memory partitioning and trusted promotion of memory;
- arbitrary distributed workflow scheduling or recovery;
- secure process checkpoint/restore;
- shared GPU or inference-cache isolation;
- protection against kernel, host administrator, firmware, physical-host, side-channel, or covert-channel compromise;
- production-ready enterprise IAM integration;
- proof of noninterference.

These are later phases or explicit threat-model exclusions. The Phase 1 design must avoid foreclosing them.

---

## 3. Success criteria

Phase 1 succeeds only if all four gates pass.

### Gate 1 — Narrow, fail-closed construction

The privileged constructor must:

- accept only an authenticated, bounded request;
- derive the effective manifest from server-side policy and catalogues;
- establish every required boundary before running untrusted code;
- dispose of launch-only privilege;
- abort and clean up on partial failure;
- remain small enough for line-by-line security review.

### Gate 2 — Session isolation and descendant control

The conformance suite must show that:

- cross-principal sessions cannot interfere through the enumerated interface inventory;
- two sessions of the same principal, each under a distinct execution identity, cannot reach each other via `/proc/<hostpid>`, `kill`/`pidfd_send_signal`, `ptrace`/`process_vm_*`, `/run` and `/tmp` paths, pathname or abstract Unix sockets, shared durable-partition permissions, broker socket reuse, or any inherited descriptor;
- fork, double-fork, daemonization, and orphaning do not escape the supervised cgroup;
- session termination kills or reaps all descendants before credential revocation completes.

### Gate 3 — End-to-end identity and remote effect control

The gateway must authenticate and log:

```text
initiator + agent principal + session + task/purpose + delegated scope
```

The gateway exposes **named, typed operations**, not a generic HTTP or CONNECT proxy, and propagates a session trace identity to the protected service.

The egress topology of technical-report §3.2 must be present: a session network namespace with a single veth, host-side nftables or eBPF policy permitting only the gateway (and constructor-operated resolver, if any), no `CAP_NET_RAW`/`CAP_NET_ADMIN`, seccomp restrictions on socket families, and an empty inherited-socket set. Direct network paths to protected services must be unavailable through the tested bypass set, including UDP/QUIC, vsock, IPv6 link-local and metadata addresses, and pre-opened connections.

### Gate 4 — Operability and evidence

The prototype must produce:

- deterministic launch and cleanup behavior;
- actionable denial and failure diagnostics;
- correlated local and gateway audit keyed by launch record and trace identity, with audit-loss counters;
- measurements of latency, resource overhead, policy complexity, and privileged code size, for both the Linux arm and the control arm;
- an invariant-by-invariant result table using the technical report's five result classes, including failures, N/A entries, and residual assumptions with owner and revalidation trigger.

Failure of a gate pauses progression to Phase 2 until the architecture is revised or the failed property is explicitly removed from the claimed profile.

---

## 4. Scope and architecture

### 4.1 Proposed components

Phase 1 builds four components and a CLI. Review of plan 0.1 found that a bespoke supervisor duplicates systemd scopes plus a PID-namespace init, that audit is a pipeline rather than a daemon, and that a custom runtime is unnecessary when the point is to run *existing* runtimes.

```text
agentbound
    CLI/API client: requests, observes, attaches to, and terminates sessions
    e.g.  agentbound run --agent finance-agent --task redwood-analysis -- <harness command>

agentbound-policy
    unprivileged resolver: principal, initiator, task, catalogue → effective manifest
    (Phase 1: a file-backed stub with a stable interface, not an IAM integration)

agentbound-launch
    narrow privileged constructor; validated host setup only; delegates lifecycle to
    a systemd scope plus an in-session PID-namespace init/subreaper; drops privilege
    before exec; the post-launch privileged lifecycle (terminate, revoke, clean) is a
    separate small helper invoked by systemd, not the constructor

agentbound-gateway
    session-authenticating gateway with exactly one typed operation adapter in Phase 1
    (Git push to a staging ref of a protected repository); propagates trace identity

agentbound-audit
    log pipeline and correlator: launch record + kernel audit + gateway log → effect table
```

Workloads are existing artefacts: `/bin/sh`, an existing coding-agent harness, and one minimal scripted model loop for determinism. No `agentbound-runtime` component is built. Policy parsing, organizational authorization, and arbitrary network protocols must not be added to the privileged constructor merely because they are convenient.

### 4.2 Initial enforcement mechanisms

The Linux implementation should evaluate:

- global agent UUID and initiator identity;
- per-session execution identity allocator with reuse quarantine (ADR-0001);
- systemd-managed cgroup v2 scope plus in-session PID-namespace init; pidfd-based supervision;
- mount, PID, IPC, UTS, and network namespaces, with the constructor ordering in technical-report §2.1;
- private runtime directory, `/tmp`, workspace, home view, and procfs;
- immutable/read-only base filesystem where practical;
- minimal device view;
- capability bounding and ambient-capability removal;
- `no_new_privs`;
- optional Landlock filesystem and supported TCP restrictions;
- a minimal seccomp profile where it adds testable value;
- explicit file-descriptor allowlisting and closure;
- short-lived proof-of-possession credentials or brokered gateway operations;
- the gateway-only egress topology (netns → single veth → host nftables/eBPF → gateway; no `CAP_NET_RAW`; socket-family seccomp);
- immutable, signed effective launch record with stated trust anchor and clock;
- session trace identity propagated through the gateway; Linux audit plus gateway audit correlation;
- Git credential hidden from the session; pushes brokered by the gateway to a staging ref only;
- cgroup-wide termination and deterministic cleanup.

A mechanism is included because it enforces a defined property, not merely because it is available.

### 4.3 Policy request and effective manifest

The untrusted request may identify only:

- registered agent principal;
- task or purpose identifier;
- requested named resources;
- requested runtime from an approved catalogue;
- optional bounded resource budget.

The policy service resolves these into an immutable effective manifest. The request must not supply authoritative numeric UIDs, paths, mount sources, labels, credential material, network addresses, capabilities, or namespace settings.

The initial schema should include:

```text
manifest version
launch-record ID
agent global ID and durable ownership projection
per-session execution identity
session trace identity
initiator ID(s), approver ID(s), scheduler/owner if scheduled
task/purpose and approval references
derivation inputs and policy/catalogue versions
runtime identity and artifact digest
approved execution binding (model, endpoint, tenant, retention mode)
namespace and mount specification
descriptor allowlist
resource limits
network/gateway policy and permitted operations
credential or broker grants
audit requirements and audit-loss behavior
termination and retention policy
```

Fields are tagged **substrate-independent** (identity, derivation inputs, authority, budgets, gateway policy, audit requirements) or **substrate-specific** (namespaces, mounts, descriptors, seccomp). Only the former are shared with the control arm.

The schema and canonical serialization become reviewable project artifacts before the privileged constructor is implemented.

### 4.4 Session lifecycle

Phase 1 will implement and test:

```text
requested
→ authorized
→ constructing
→ active
→ quiescing
→ terminated
→ cleaned/sealed
```

Every transition must have:

- an authorized actor;
- an audit event;
- defined retry/idempotency behavior;
- failure cleanup;
- externally observable status.

Construction must not expose credentials or start the runtime before all required boundaries and audit bindings exist. Termination must stop descendants before credentials and session resources are released.

### 4.5 Integration contract for existing harnesses

Phase 1 must show that an unmodified or minimally modified coding-agent harness runs inside a session. The contract the session offers a harness is:

- **Working tree:** a per-session writable checkout of the task repository, bind-mounted at a stable path; the durable principal's storage is not mounted.
- **Credentials:** no Git, cloud, or API credential is present in the session environment, files, or descriptors. Git remote operations go through the gateway adapter, which accepts pushes only to `refs/agentbound/<session>/...` staging refs.
- **Model access:** the harness's model endpoint is reachable only through the gateway as a typed inference operation carrying the session identity; the endpoint and tenant are fixed by the execution binding.
- **Terminal and streams:** stdin/stdout/stderr and an optional PTY are the only inherited descriptors; attachment is a policy-governed event attributed to the attaching human.
- **Diagnostics:** a denied operation returns an error that names the invariant or policy rule and the launch-record and trace identity, so the harness (or a human) can distinguish policy denial from failure.
- **Modes:** the same manifest supports a local developer mode (single host, file-backed policy) and a CI mode (non-interactive, scheduled initiator with accountable owner).

Anything a harness needs beyond this contract is either added to the manifest as a reviewed grant or is out of scope for Phase 1.

---

## 5. Demonstration environment

### 5.0 Step zero: the scenario the reader should picture

A developer, Alice, runs `agentbound run --agent engineering-agent --task fix-issue-1234 -- <coding harness>`. The harness clones the repository, reads a hostile issue comment, and is induced to try three things: read Alice's `~/.ssh` and cloud credentials, push directly to `main`, and exfiltrate the repository to an external host. All three fail: the credentials are not in the session's world, the only push path is a gateway operation restricted to a staging ref, and there is no route except to the gateway. The staged branch is later promoted to `main` only by CI and review outside the session. The audit table shows Alice → engineering-agent → session → process → staged push. Every Phase 1 demonstration below is a component of this scenario.

The initial demonstration uses:

- humans Alice and Bob;
- durable `finance-agent` and `engineering-agent` principals;
- at least two concurrent sessions for `finance-agent`;
- separate private workspaces and runtime directories;
- one approved gateway operation (Git push to a staging ref);
- one protected service (the Git host) that is unreachable directly, with `main` a protected branch;
- three replaceable workloads: `/bin/sh`, an existing coding-agent harness, and a minimal scripted model loop;
- a hardened container or microVM control arm running the same substrate-independent manifest.

### Required demonstrations

1. Alice starts an authorized Finance session.
2. Bob cannot start, observe, or attach to it without an explicit grant.
3. Finance and Engineering sessions cannot read or influence each other's private state.
4. Two Finance sessions cannot inspect `/proc`, ptrace, signal, attach to, connect to private sockets, recover credentials, or read session-private files from one another.
5. Replacing the model-driven runtime with a shell preserves the same session identity, cgroup, namespace, gateway scope, and audit chain.
6. Child and grandchild processes retain the session boundary.
7. Double-fork and daemonization attempts remain supervised.
8. Killing the session terminates all descendants and then revokes or closes gateway authority.
9. The session cannot reach the protected service directly.
10. An allowed gateway operation is attributed to the initiating human, agent, session, task, and local calling process where available.
11. Forced failure at each constructor stage leaves no runnable session or credential.
12. Audit reconstruction identifies the process that created a local artifact and the process/session responsible for a remote effect, and the report states the fraction of effects in the defined ontology that were reconstructed.
13. A session pushes a patch to its staging ref; a direct push to `main`, a push to another session's staging ref, and a push with a forged trace identity all fail at the gateway.
14. Changing the model endpoint for a running session produces an execution-binding record and, where the new endpoint is not approved for the task, is refused.
15. Every demonstration 1–13 is repeated on the control arm and differences are recorded.

---

## 6. Adversarial conformance suite

The test suite is a first-class deliverable and should be developed alongside the implementation.

### 6.1 Process and session interference

Attempt:

- `/proc` enumeration and sensitive-file reads;
- `ptrace`, process memory access, and signal delivery;
- PTY discovery and attachment;
- Unix-socket and abstract-socket discovery;
- shared-memory and IPC access;
- temporary-file races and symlink attacks;
- access to another session's runtime and workspace directories;
- environment, startup-file, and shell-configuration injection;
- inherited and accidentally reopened file descriptors, including descriptors reintroduced through `SCM_RIGHTS`, `/proc/self/fd`, or memfd;
- `pidfd_open` on host PIDs and `pidfd_send_signal`;
- `process_vm_readv`/`process_vm_writev`;
- abstract-namespace sockets shared across network namespaces;
- broker socket reuse from a sibling session.

### 6.2 Escape and persistence

Attempt:

- cgroup migration;
- namespace manipulation;
- setuid and file-capability execution;
- capability and ambient-authority recovery;
- daemonization, double-forking, and orphan adoption;
- mount and procfs abuse;
- persistence outside the session workspace;
- use of interpreters, package managers, or dynamic-loader settings to cross the boundary.

### 6.3 Credential recovery and reuse

Inspect or exploit:

- environment variables;
- files and procfs;
- inherited descriptors;
- child processes;
- Unix-socket brokers;
- logs, exceptions, and crash output;
- credentials after termination;
- replay from another session.

The result must state which credentials remain exportable. Short lifetime alone is not considered non-exportability.

### 6.4 Network and gateway bypass

Attempt:

- direct IP and alternate-port connections;
- DNS rebinding and alternate resolution paths;
- IPv6 and unusual address forms;
- HTTP redirects;
- proxy environment variables;
- local forwarding and tunnelling;
- Unix sockets and host-local proxies;
- misuse of the approved gateway as a generic tunnel or SSRF oracle;
- TLS identity or tenant mismatch;
- replay of another session's gateway identity or trace identity;
- UDP and QUIC egress;
- vsock and other non-INET socket families;
- link-local and cloud-metadata addresses;
- connections opened before boundary installation and inherited;
- DNS resolver abuse where a resolver is permitted.

### 6.5 Resource exhaustion

Exercise limits for:

- PIDs and descendant fan-out;
- file descriptors;
- memory and CPU;
- disk bytes and inodes;
- I/O and network bandwidth;
- connection count;
- audit-event volume;
- gateway requests, rate limits, tokens, and monetary budget.

---

## 7. Evaluation and evidence

### 7.1 Required measurements

Measure at minimum:

- session construction and teardown latency;
- descendant-termination latency;
- steady-state memory and process overhead;
- representative syscall/workload overhead;
- audit-event volume and loss behavior;
- trusted code size and privileged code size;
- number and breadth of retained capabilities;
- seccomp/Landlock/policy rule count where used;
- administrator actions required for launch, diagnosis, and cleanup;
- proportion of tested effects enforced locally, mediated by the gateway, or dependent on remote authorization;
- attribution completeness for local and remote effects.

### 7.2 Invariant evidence table

The evaluation report will contain one row per applicable technical-report invariant:

| Invariant | Class | Enforcement mechanism | Adversarial test | Result | Residual assumption |
|---|---|---|---|---|---|
| 17 Same-principal session isolation | prevents | per-session execution identity + namespaces + descriptor discipline | interference suite (§6.1) | Pending | kernel and constructor trusted |
| 12 Complete descendant control | prevents | systemd scope + PID-ns init + pidfd + `cgroup.kill` | double-fork/daemon suite | Pending | no writable cgroup path; D-state delays |
| 10 Gateway-only egress | prevents | netns + veth + nftables/eBPF + seccomp + gateway | bypass suite (§6.4) | Pending | gateway and remote service trusted |
| 13 Attribution of mediated effects | detects | launch record + trace identity + kernel/gateway audit | reconstruct effect ontology | Pending | audit availability; lossy under load |
| 19 Integrity promotion (protected-object subset) | prevents | gateway staging-ref restriction + branch protection | direct/forged push tests | Pending | Git host enforces protection |
| 8, 9, 16, 18 | — | — | — | **N/A to Unix-governed profile** | — |

Every applicable invariant in technical-report §7 receives a row. Results use the report's five classes:

- enforced and passed;
- enforced but failed;
- detected only;
- assumption (documented and accepted, with owner and revalidation trigger);
- not applicable to this profile.

Pre-registered thresholds (interface inventory, adversary matrix, required attribution completeness, maximum policy-exception rate, pinned kernel/systemd/LSM versions) are fixed in WP0 before any test runs.

### 7.3 Fault injection

Force failures during:

- authorization and derivation;
- execution-identity allocation;
- namespace and mount setup, at each step of the §2.1 ordering;
- network path and firewall rule installation;
- cgroup setup;
- credential/gateway grant issuance;
- audit binding;
- privilege disposal;
- runtime `exec`;
- active-session supervision;
- termination and cleanup.

The implementation must either fail closed or document why the property cannot be claimed.

---

## 8. Control arm: hardened container or microVM

Phase 1 **requires** a control arm; without it the evaluation cannot show whether the Linux-process design offers anything a container or microVM with per-session workload identity, egress gateway, and audit does not. The control arm is not a second full implementation: it reuses `agentbound-policy`, `agentbound-gateway`, and `agentbound-audit`, and substitutes a minimal container or microVM launcher for `agentbound-launch`. The substrate-independent manifest fields and workload are launched through:

```text
policy decision
├── Linux process/session constructor
└── minimal microVM session constructor
```

Compare:

- launch latency;
- steady-state memory;
- cross-session isolation assumptions;
- credential delivery;
- state projection;
- descendant control;
- audit correlation;
- patching and operator complexity.

The goal is to identify when host-process isolation is sufficient, when a container or microVM materially improves assurance, which identity, gateway, storage, and audit abstractions remain substrate-independent, and what the process-session design costs or saves. Equivalent assurance must not be inferred from shared manifest fields; substrate-specific refinements are listed separately for each arm.

The control arm may lag the Linux arm within a work package, but Gate 4 cannot pass and no Phase 2 decision may be made without its results.

---

## 9. Work packages and ordinal sequence

The work proceeds by exit conditions rather than calendar estimates. A later work package begins only when its prerequisites are satisfied; independent packages may overlap when doing so does not weaken a gate or review boundary.

### WP0 — Specification freeze and threat model

Deliverables:

- Phase 1 normative requirement list using MUST/SHOULD/MAY;
- scoped threat model and non-goals;
- effective-manifest schema and canonical encoding;
- lifecycle and failure-state specification;
- invariant-to-test traceability matrix.

Exit condition: reviewers agree the prototype has a bounded claim.

### WP1 — Mechanism spikes

Prototype high-risk mechanisms independently:

- per-session execution identity allocation and the durable-ownership projection (ADR-0001);
- systemd scope + PID-namespace init containment and `cgroup.kill` behavior, including D-state tasks;
- namespace, mount, and procfs construction in the §2.1 ordering, with mount-descriptor resolution;
- descriptor closure and runtime launch ordering;
- egress topology and host-side policy; socket-family seccomp;
- workload identity, trace identity, and gateway authentication;
- Git staging-ref adapter and protected-branch behavior;
- `loginuid` and audit correlation, including loss behavior under load;
- minimal control-arm launcher.

Exit condition: no known mechanism gap makes a Phase 1 gate impossible.

### WP2 — Constructor and lifecycle

Implement the minimum request, policy-stub, construction, systemd-scope lifecycle, and cleanup path. Keep policy resolution unprivileged, the privileged constructor narrow, and the post-launch privileged helper separate and enumerated.

Exit condition: shell workload launches and terminates fail closed with an immutable launch record.

### WP3 — Gateway and end-to-end audit

Implement the Git staging-ref gateway operation, session-bound workload and trace identity, the egress topology, and correlated audit.

Exit condition: the remote effect is attributable end to end, the thin integrity slice (goal 11) holds, and bypass tests have defined outcomes.

### WP4 — Existing-harness integration and control arm

Run the same effective manifest with `/bin/sh`, an existing coding-agent harness under the §4.5 contract, and the scripted model loop; run the substrate-independent manifest through the control-arm launcher.

Exit condition: runtime replacement does not alter security identity or enforcement; an execution-binding change is recorded and, where unapproved, refused; the control arm runs the same demonstrations.

### WP5 — Adversarial suite and fault injection

Automate the tests in Section 6 and construction/lifecycle faults in Section 7.3.

Exit condition: every applicable invariant has evidence or a recorded failure.

### WP6 — Evaluation and security review

Produce measurements for both arms, review privileged code (launch path and post-launch helper) line by line, conduct an independent architecture review, and publish the Linux-arm versus control-arm delta.

Exit condition: issue a go/no-go recommendation for Phase 2.

---

## 10. Sequence and staffing

The ordinal sequence is:

1. freeze the bounded specification and threat model;
2. resolve high-risk mechanism questions with focused spikes;
3. construct and supervise a fail-closed shell session;
4. bind one protected remote effect to session identity and audit;
5. demonstrate runtime replacement without boundary changes and run the control arm;
6. execute adversarial tests and lifecycle fault injection on both arms;
7. evaluate evidence, conduct independent review, and make the Phase 2 decision.

Progress is assessed against the work-package exit conditions, not elapsed time. Reordering or overlapping work is acceptable only when prerequisite evidence and independent review boundaries remain intact.

Recommended minimum roles:

- Linux security/namespace/cgroup engineer;
- security engineer for policy, gateway, and adversarial testing;
- agentbound runtime-integration engineer;
- independent reviewer not responsible for the constructor implementation.

One person may cover multiple implementation roles, but the final security review should be independent.

---

## 11. Decision points after Phase 1

Phase 1 evidence determines the next branch.

### Branch A — Integrity provenance and partitioned memory (default)

This is the default branch if the Phase 1 gates pass. Practical agent risk is dominated by integrity failures—prompt-injected changes reaching trusted state—rather than by confidentiality labels. Extend the Phase 1 integrity slice: immutable snapshots, append-only proposals, provenance, generation checks, mechanical validators for structured state, and the structured/semantic memory split of technical-report §3.3. This branch is the first to claim Invariant 19 beyond the protected-object subset.

### Branch B — Compartmented MCS sessions

Prioritize for real project, customer, legal-matter, or tenant separation. Add category allocation, labeled storage, compartment-aware gateways, and controlled peer import.

### Branch C — VM-backed sessions

Prioritize if shared-kernel or device risks remain unacceptable. Preserve the same policy, manifest, workload identity, gateway, and audit interfaces while changing the execution substrate.

### Branch D — Trusted release and full MLS

Prioritize only with a concrete classified or regulated workflow whose required information flows and review economics can be measured, and treat the release-economics targets of technical-report §11 as a go/no-go condition. Do not adopt full MLS merely because the paper describes it.

---

## 12. Review questions

Reviewers are asked to focus on:

1. Is the Phase 1 claim narrow enough to be falsifiable?
2. Are any non-goals actually prerequisites for the claimed Unix-governed profile?
3. Is the proposed privileged/unprivileged split credible?
4. Does ADR-0001 (per-session execution identity, durable principal as ownership only) close the same-principal isolation gap, and what does it cost in identity allocation and audit mapping?
5. Is a systemd scope plus in-session PID-namespace init sufficient lifecycle machinery, and is the post-launch privileged helper small enough?
6. Is the gateway experiment sufficient to establish end-to-end identity binding?
7. Which tests are missing from the adversarial suite?
8. Which measurements determine whether process isolation or microVM isolation should be the default?
9. Are the four gates appropriate go/no-go criteria?
10. Is the work-package scope appropriately bounded, and what should be cut first if a gate requires substantially more mechanism or trusted code than proposed?
11. Is the thin integrity slice the right first integrity claim, and does the §4.5 integration contract match what real harnesses need?
12. Does the required control arm make the comparison fair, or does it favor one substrate?

---

## 13. Expected Phase 1 outputs

1. A versioned normative Phase 1 specification.
2. Reference constructor and lifecycle helper, policy stub, one gateway adapter, and audit correlator, plus a minimal control-arm launcher.
3. Machine-readable request and effective-manifest schemas.
4. Reproducible host configuration and policy examples.
5. Automated adversarial conformance and fault-injection suite.
6. Invariant-to-evidence traceability matrix.
7. Measured evaluation report, including negative results.
8. Independent mechanism and architecture review reports.
9. Published Linux-arm versus control-arm comparison.
10. A go/no-go recommendation and selected Phase 2 branch.
