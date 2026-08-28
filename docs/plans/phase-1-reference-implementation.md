# Phase 1 Reference Implementation Plan

**Status:** Draft for review  
**Plan version:** 0.1  
**Date:** 28 August 2026  
**Related position paper:** [`../papers/position-paper.md`](../papers/position-paper.md)  
**Normative technical report:** [`../papers/technical-report.md`](../papers/technical-report.md)

---

## 1. Purpose

This plan defines the next project stage after review of the *Agents as Unix Principals* position paper and technical report. It proposes a bounded Phase 1 implementation intended to test the smallest important architectural claim:

> An existing agent runtime can execute inside a task-scoped Unix session whose identity, authority, descendants, resources, credentials, and effects are enforced and attributed independently of the model and harness.

Phase 1 is a security experiment, not a production platform. It focuses on the **Unix-governed session** profile. It deliberately postpones full SELinux MLS, dynamic category allocation, semantic declassification, distributed workflow orchestration, durable-memory promotion, and shared-accelerator isolation.

The expected outcome is evidence: a reference implementation, adversarial conformance suite, measured evaluation, and a list of failed or residual assumptions mapped to the technical report's invariants.

---

## 2. Goals and non-goals

### 2.1 Goals

Phase 1 will demonstrate that:

1. A durable global agent identity can be projected into a local Unix execution identity.
2. Every session is bound to an authenticated initiator, task, effective policy, and immutable launch record.
3. Sessions belonging to different principals—and concurrent sessions belonging to the same principal—cannot access each other's private process, IPC, terminal, credential, or workspace state through tested channels.
4. Untrusted descendants cannot escape the session's namespaces, cgroup, resource controls, or termination boundary through tested techniques.
5. The cognitive runtime can be replaced without changing session identity or enforcement.
6. Service access is narrow, short-lived or brokered, and bound to session identity rather than inherited from a human or shared daemon.
7. Direct egress cannot bypass the approved service gateway through tested paths.
8. Local and remote effects can be reconstructed as:

   ```text
   initiator → agent principal → session → process → object or service effect
   ```

9. Failed construction leaves no runnable partial session, reusable credential, or ambiguous audit record.
10. The trusted and privileged code surface can be measured and reviewed.

### 2.2 Non-goals

Phase 1 will not claim to provide:

- full confidentiality or integrity information-flow control;
- SELinux MLS/MCS category allocation;
- secure semantic declassification;
- persistent agent memory partitioning and trusted promotion;
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

- cross-principal sessions cannot interfere through tested interfaces;
- two sessions of the same principal cannot inspect, signal, trace, attach to, or read each other's private state;
- fork, double-fork, daemonization, and orphaning do not escape the supervised cgroup;
- session termination kills or reaps all descendants before credential revocation completes.

### Gate 3 — End-to-end identity and remote effect control

The gateway must authenticate and log:

```text
initiator + agent principal + session + task/purpose + delegated scope
```

Direct network paths to protected services must be unavailable through the tested bypass set.

### Gate 4 — Operability and evidence

The prototype must produce:

- deterministic launch and cleanup behavior;
- actionable denial and failure diagnostics;
- correlated local and gateway audit;
- measurements of latency, resource overhead, policy complexity, and privileged code size;
- an invariant-by-invariant result table, including failures and residual assumptions.

Failure of a gate pauses progression to Phase 2 until the architecture is revised or the failed property is explicitly removed from the claimed profile.

---

## 4. Scope and architecture

### 4.1 Proposed components

```text
agentbound-policy
    unprivileged organizational principal, initiator, task, and policy resolver

agentbound
    user-facing CLI/API client that requests and observes sessions

agentbound-launch
    narrow privileged constructor; performs only validated host setup

agentbound-supervisor
    owns lifecycle, cgroup, process reaping, termination, and cleanup

agentbound-gateway
    authenticates session workload identity and mediates approved service operations

agentbound-audit
    correlates launch, process, gateway, denial, and termination records

agentbound-runtime
    replaceable shell or minimal model-driven loop used as untrusted workload
```

The exact process decomposition is subject to implementation review. Policy parsing, organizational authorization, and arbitrary network protocols should not be added to the privileged constructor merely because they are convenient.

### 4.2 Initial enforcement mechanisms

The Linux implementation should evaluate:

- global agent UUID and initiator identity;
- local stable or ephemeral execution UID strategy;
- systemd-managed cgroup v2 scope or service;
- mount, PID, IPC, UTS, and network namespaces as required;
- private runtime directory, `/tmp`, workspace, home view, and procfs;
- immutable/read-only base filesystem where practical;
- minimal device view;
- capability bounding and ambient-capability removal;
- `no_new_privs`;
- optional Landlock filesystem and supported TCP restrictions;
- a minimal seccomp profile where it adds testable value;
- explicit file-descriptor allowlisting and closure;
- short-lived proof-of-possession credentials or brokered gateway operations;
- network routing that makes the gateway the only approved protected-service path;
- immutable effective launch record;
- Linux/process audit plus gateway audit correlation;
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
agent global ID and local projection
initiator ID
task/purpose and approval references
policy/catalogue versions
runtime identity and artifact digest
namespace and mount specification
descriptor allowlist
resource limits
network/gateway policy
credential or broker grants
audit requirements
termination and retention policy
```

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

---

## 5. Demonstration environment

The initial demonstration uses:

- humans Alice and Bob;
- durable `finance-agent` and `engineering-agent` principals;
- at least two concurrent sessions for `finance-agent`;
- separate private workspaces and runtime directories;
- one approved gateway operation;
- one protected service that is unreachable directly;
- two replaceable runtimes: an ordinary shell workload and a minimal model-driven loop.

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
12. Audit reconstruction identifies the process that created a local artifact and the process/session responsible for a remote effect.

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
- inherited and accidentally reopened file descriptors.

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
- misuse of the approved gateway as a generic tunnel;
- TLS identity or tenant mismatch;
- replay of another session's gateway identity.

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

| Invariant | Enforcement mechanism | Adversarial test | Result | Residual assumption |
|---|---|---|---|---|
| Same-principal session isolation | To be selected | ptrace/signal/procfs/socket suite | Pending | Kernel and constructor trusted |
| Complete descendant control | cgroup v2 + supervisor | double-fork/daemon suite | Pending | cgroup configuration trusted |
| Gateway-only protected service | network namespace + gateway | bypass suite | Pending | gateway and remote service trusted |
| Attribution | launch/process/gateway audit | reconstruct local and remote effects | Pending | audit path availability |

Results must distinguish:

- enforced and passed;
- enforced but failed;
- recorded or detected only;
- policy/administrative assumption;
- outside the Phase 1 threat model.

### 7.3 Fault injection

Force failures during:

- authorization;
- UID/session allocation;
- namespace and mount setup;
- cgroup setup;
- credential/gateway grant issuance;
- audit binding;
- privilege disposal;
- runtime `exec`;
- active-session supervision;
- termination and cleanup.

The implementation must either fail closed or document why the property cannot be claimed.

---

## 8. MicroVM control experiment

Phase 1 should include a small control experiment, not a second full implementation. The same effective manifest and workload should be launched through:

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

The goal is to identify when host-process isolation is sufficient, when a microVM materially improves assurance, and which identity, gateway, storage, and audit abstractions remain substrate-independent.

If the control experiment would materially delay the core Phase 1 gates, it may follow the first security review rather than block the initial demonstrator.

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

- same-principal session isolation strategy;
- systemd/cgroup descendant containment and kill behavior;
- namespace and procfs construction;
- descriptor closure and runtime launch ordering;
- workload identity and gateway authentication;
- audit correlation.

Exit condition: no known mechanism gap makes a Phase 1 gate impossible.

### WP2 — Constructor and supervisor

Implement the minimum request, policy, construction, supervision, and cleanup path. Keep policy resolution unprivileged and the privileged executor narrow.

Exit condition: shell workload launches and terminates fail closed with an immutable launch record.

### WP3 — Gateway and end-to-end audit

Implement one protected service operation, session-bound workload identity, direct-egress prevention, and correlated audit.

Exit condition: the remote effect is attributable end to end and bypass tests have defined outcomes.

### WP4 — Replaceable runtime demonstration

Run the same effective manifest with an ordinary shell workload and one minimal model-driven runtime.

Exit condition: runtime replacement does not alter security identity or enforcement.

### WP5 — Adversarial suite and fault injection

Automate the tests in Section 6 and construction/lifecycle faults in Section 7.3.

Exit condition: every applicable invariant has evidence or a recorded failure.

### WP6 — Evaluation and security review

Produce measurements, review privileged code line by line, conduct an independent architecture review, and compare the minimal microVM control if feasible.

Exit condition: issue a go/no-go recommendation for Phase 2.

---

## 10. Sequence and staffing

The ordinal sequence is:

1. freeze the bounded specification and threat model;
2. resolve high-risk mechanism questions with focused spikes;
3. construct and supervise a fail-closed shell session;
4. bind one protected remote effect to session identity and audit;
5. demonstrate runtime replacement without boundary changes;
6. execute adversarial tests and lifecycle fault injection;
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

### Branch A — Partitioned memory and integrity promotion

Prioritize if the baseline boundary works and practical agent risk is the main target. Add immutable snapshots, append-only proposals, provenance, generation checks, validation, and trusted promotion.

### Branch B — Compartmented MCS sessions

Prioritize for real project, customer, legal-matter, or tenant separation. Add category allocation, labeled storage, compartment-aware gateways, and controlled peer import.

### Branch C — VM-backed sessions

Prioritize if shared-kernel or device risks remain unacceptable. Preserve the same policy, manifest, workload identity, gateway, and audit interfaces while changing the execution substrate.

### Branch D — Trusted release and full MLS

Prioritize only with a concrete classified or regulated workflow whose required information flows and review economics can be measured. Do not adopt full MLS merely because the paper describes it.

---

## 12. Review questions

Reviewers are asked to focus on:

1. Is the Phase 1 claim narrow enough to be falsifiable?
2. Are any non-goals actually prerequisites for the claimed Unix-governed profile?
3. Is the proposed privileged/unprivileged split credible?
4. Which local UID strategy best supports same-principal session isolation and durable ownership?
5. Can systemd provide the required cgroup lifecycle without a bespoke privileged supervisor?
6. Is the gateway experiment sufficient to establish end-to-end identity binding?
7. Which tests are missing from the adversarial suite?
8. Which measurements determine whether process isolation or microVM isolation should be the default?
9. Are the four gates appropriate go/no-go criteria?
10. Is the work-package scope appropriately bounded, and what should be cut first if a gate requires substantially more mechanism or trusted code than proposed?

---

## 13. Expected Phase 1 outputs

1. A versioned normative Phase 1 specification.
2. Reference constructor, supervisor, gateway, audit correlator, and minimal runtime integration.
3. Machine-readable request and effective-manifest schemas.
4. Reproducible host configuration and policy examples.
5. Automated adversarial conformance and fault-injection suite.
6. Invariant-to-evidence traceability matrix.
7. Measured evaluation report, including negative results.
8. Independent mechanism and architecture review reports.
9. Minimal process-session versus microVM comparison, if it does not block the core gates.
10. A go/no-go recommendation and selected Phase 2 branch.
