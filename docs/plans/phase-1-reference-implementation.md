# Phase 1 Reference Implementation Plan

**Status:** Active Phase 1 plan; WP0 specification set frozen (architecture README freeze record)  
**Plan version:** 0.12  
**Date:** 28 August 2026  
**Related position paper:** [`../papers/position-paper.md`](../papers/position-paper.md)  
**Normative technical report:** [`../papers/technical-report.md`](../papers/technical-report.md)  
**Architecture specifications:** [`../architecture/README.md`](../architecture/README.md)

---

## Revision history

- **0.1** — Initial plan: Unix-governed profile, four gates, seven components, WP0–WP6, ordinal sequence.
- **0.2** — Incorporated three independent reviews: Phase 1 claim narrowed to isolation, authority, credential confinement, descendant control, and attribution of mediated effects, with information-flow invariants marked not applicable; per-session execution identity committed (ADR-0001); gateway-only egress topology specified; components collapsed to four plus a CLI; a thin integrity slice added; container/microVM control arm made a required evaluation arm; integration contract and adoption "step 0" added.
- **0.3** — Second independent review: resolved the one-adapter/inference-adapter contradiction (Git only in the core; inference adapter and execution-binding control moved to milestone 1C); reworded the integrity non-goal; added adversarial suites for bounded derivation, monotonic delegation, active revocation, and constructor inputs; execution identity restated as "uniquely allocated with verified reclamation and reuse quarantine" with a WP0 lifecycle specification; gateway authentication and the control substrate made required ADRs; control arm fixed as a microVM; internal milestones 1A–1D with stop points; full Profile U conformance target made explicit.
- **0.4** — Follow-up review: active-revocation evidence split by milestone; control-arm test equivalence pre-registered in ADR-0003; ADR-0002 candidate set reconciled with the egress topology (two mutually exclusive channel topologies, each with tests); Invariant 20 evidence distinguishes absent resource classes; gates numbered §§3.1–3.4; programme framing stated in §1.
- **0.5** — Stale §4.2 egress bullet replaced with the ADR-0002 topology choice; Branch D cross-reference corrected to technical-report §3.5; ADR-0002 scope extended to peer-credential evidence and connection lifetime.
- **0.6** — WP0 drafts added; effective manifest split into a policy-signed allocation-free authorization manifest and constructor-signed launch binding to remove allocation circularity; execution binding includes inference pool.
- **0.7** — WP0 decisions applied: local-socket topology selected and network topology withdrawn from Phase 1 (ADR-0002 0.2); systemd-invoked helper replaced by the `agentbound-lifecycle` daemon; WP0 deliverables extended with the test catalogue and component-interface skeleton; Gate 3 no longer provisional; gate language, bypass-corpus rule, comparative decision rule, and Profile U wording tightened; demo 12 scoped to the effect ontology.
- **0.8** — Second WP0 review: gateway-free 1A manifest form; one connection per process; policy component emits the authorization manifest, not the effective manifest; ADR-0003 accepted with pinned configuration and thresholds.
- **0.9** — WP1 spike list extended with the open-question register items VM-1, VM-2, LC-2, ID-1.
- **0.10** — §6.8 1A case list uses the split outage-trigger vocabulary and the `continue-degraded` restriction.
- **0.11** — Post-freeze maintenance: status reflects the WP0 freeze; WP0 section marked complete and in past tense; fault-point list uses the empty-network-namespace/gateway-socket wording.
- **0.12** — Editorial pass under docs/STYLE.md: demonstration 16 split by milestone; control-arm rationale consolidated in §8; review narrative removed; Oxford spelling. No milestone, gate, deliverable, or criterion changed.


---

## 1. Purpose

This document defines the next project stage after the *Agents as Unix Principals* position paper and technical report. Phase 1 is a bounded implementation that tests the smallest architectural claim:

> An existing agent runtime can execute inside a task-scoped Unix session whose identity, authority, descendants, resources, credentials, and mediated effects are enforced and attributed independently of the model and harness.

Phase 1 is a security experiment, not a production platform. It focuses on the **Unix-governed session** profile, which the technical report defines as an **isolation, authority, and attribution profile, not an information-flow profile**. Phase 1 therefore does not test the confidentiality- or integrity-propagation thesis; that belongs to the compartmented profile and Phase 2. It postpones full SELinux MLS, dynamic category allocation, semantic declassification, distributed workflow orchestration, durable-memory promotion, and shared-accelerator isolation.

The expected outcome is evidence: a reference implementation, adversarial conformance suite, measured evaluation, and a list of failed or residual assumptions mapped to the technical report's invariants.

Phase 1 is a staged implementation and evaluation programme, not a single increment. Milestone 1A is the smallest first implementation step; 1B–1D are contingent extensions with explicit stop conditions (Section 3.5). A successful 1A is not "Phase 1 succeeded," and a stopped programme is not "Profile U conformant": every published result carries the milestone reached and the invariant subset actually demonstrated.

---

## 2. Goals and non-goals

### 2.1 Goals

Phase 1 will demonstrate that:

1. A durable global agent identity can be bound to a session that runs under a **per-session, uniquely allocated execution identity with verified reclamation and reuse quarantine** (ADR-0001), with the durable principal owning state but not executing session code.
2. Every session is bound to an authenticated initiator, task, effective policy, and immutable launch record produced by the derivation relation.
3. Sessions belonging to different principals—and concurrent sessions belonging to the same principal—cannot access each other's process, IPC, terminal, credential, or workspace state through the enumerated interface inventory, absent an explicitly authorized and recorded shared channel (technical-report Invariant 17).
4. Untrusted descendants cannot escape the session's namespaces, cgroup, resource controls, or termination boundary through tested techniques.
5. The cognitive runtime can be replaced without changing session identity or enforcement (milestone 1A); a change of model endpoint is recorded as an execution-binding event and refused when unapproved (milestone 1C).
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
12. The same workload runs under a **microVM control arm** with the substrate-independent manifest fields, and the measured delta is published (milestone 1D).

### 2.2 Non-goals

Phase 1 will not claim to provide:

- general confidentiality or integrity label propagation across communication edges (technical-report Invariants 8, 9, 16, and 18 are **not applicable** to the Unix-governed profile and appear in the evidence table only as N/A). Phase 1's **sole integrity-flow claim** is the protected-object staging and promotion boundary of goal 11 (Invariant 19, protected-object subset); no dynamic `T` propagation through arbitrary edges is claimed;
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

### 2.3 Conformance target

Phase 1 targets **full conformance with the Unix-governed profile** (technical-report §9.1), not a subset. Every invariant the report marks applicable to profile U—1, 2, 3, 6, 7, 10, 11, 12, 13, 14, 15, 17, 19 (protected-object subset), 20, 21, and 22—must receive a test and a row in the evidence table (Section 7.2). Invariant 22 depends on the inference adapter and is reachable only at milestone 1C; if the programme stops before 1C, the report claims a stated Profile U subset and marks 22 as **not evaluated**, never as N/A.

---

## 3. Success criteria

Phase 1 succeeds only if all four gates pass.

### 3.1 Gate 1 — Narrow, fail-closed construction

The privileged constructor must:

- accept only an authenticated, bounded request;
- derive the effective manifest from server-side policy and catalogues;
- establish every required boundary before running untrusted code;
- dispose of launch-only privilege;
- abort and clean up on partial failure;
- stay within the privileged-code reviewability bound of the requirements §12 (≤ 6 000 direct SLOC under its accounting rules; all five SLOC figures published).

### 3.2 Gate 2 — Session isolation and descendant control

The conformance suite must show that:

- cross-principal sessions cannot interfere through the enumerated interface inventory;
- two sessions of the same principal, each under a distinct execution identity, cannot reach each other via `/proc/<hostpid>`, `kill`/`pidfd_send_signal`, `ptrace`/`process_vm_*`, `/run` and `/tmp` paths, pathname or abstract Unix sockets, shared durable-partition permissions, broker socket reuse, or any inherited descriptor;
- fork, double-fork, daemonization, and orphaning do not escape the supervised cgroup;
- session termination kills or reaps all descendants before credential revocation completes.

### 3.3 Gate 3 — End-to-end identity and remote effect control

The gateway must authenticate and log:

```text
initiator + agent principal + session + task/purpose + delegated scope
```

The gateway exposes **named, typed operations**, not a generic HTTP or CONNECT proxy, and propagates a session trace identity to the protected service.

Gateway authentication is a high-risk mechanism. ADR-0002 (version 0.2) selects the **local-socket topology** for Phase 1. The session has **no network interface**. The gateway is reached through exactly one bind-mounted, single-purpose `AF_UNIX SOCK_SEQPACKET` socket. The connection is authenticated by `SO_PEERCRED` plus a peer pidfd. **Every operation** is attributed to a live process by kernel-supplied per-packet `SCM_CREDENTIALS`. `SCM_RIGHTS` is rejected. Each connection is bound to one process: a packet from any other PID, however the descriptor was acquired, closes the connection. The "empty inherited-socket" and "no host Unix sockets" rules have this one named exception, recorded in the manifest.

The network topology (veth, mTLS/proof-of-possession, or host broker) is **withdrawn from Phase 1** because none of its mechanisms identifies the operation-issuing process, so it cannot satisfy the process leg of Invariant 13; it is deferred to a future multi-host ADR. Gate 3 is therefore no longer provisional: it can fail only on evidence. WP1 verifies the kernel-baseline assumptions listed in ADR-0002 Decision 7 (`SOCK_SEQPACKET` credential semantics, pidfd availability, abstract-socket isolation); a failed verification reopens the ADR.

### 3.4 Gate 4 — Operability and evidence

The prototype must produce:

- deterministic launch and cleanup behaviour: N repeated launches of one authorization manifest (N per test catalogue) produce launch bindings identical modulo the catalogue's listed nondeterministic fields; every termination reaches `sealed` within the manifest deadline;
- denial and failure diagnostics that carry the requirement ID, authorization ID, launch-record digest, and trace ID, and leak no other session's identifiers (assertions per test catalogue);
- correlated local and gateway audit keyed by launch record and trace identity, with audit-loss counters;
- measurements of latency, resource overhead, policy complexity, and privileged code size;
- an invariant-by-invariant result table using the technical report's five result classes, including failures, N/A entries, and residual assumptions with owner and revalidation trigger;
- for the **comparative claim** and the Phase 2 decision only: the same measurements and demonstrations on the microVM control arm (milestone 1D), evaluated by the pre-registered decision rule in ADR-0003.

The control arm blocks architectural conclusions. It does not block publication: milestone 1A–1C results may be published as intermediate or negative findings labelled as lacking the comparative arm. For the control-arm rationale and decision rule, see Section 8 and ADR-0003.

A gate passes only when every test the catalogue assigns to it reaches its expected outcome; any reproducible bypass fails it, and unexplained nondeterminism is a failure pending investigation. Failure of a gate pauses progression to Phase 2 until the architecture is revised or the failed property is explicitly removed from the claimed profile.

### 3.5 Milestones and stop points

The work packages of Section 9 are grouped into four milestones. Each is a reporting boundary and a termination point; a failure at one stops the programme before the next is built.

| Milestone | Scope | Stop condition |
|---|---|---|
| **1A — Session boundary** | manifest and canonical encoding; identity allocator and `agentbound-lifecycle` daemon; constructor; `/bin/sh` and scripted-loop workloads; same-principal isolation; delegation narrowing; local revocation cases; descendant control; fault injection on construction; Gates 1 and 2 | Gate 1 or 2 fails, or the privileged surface exceeds the reviewability bound fixed in WP0 |
| **1B — Mediated effect** | local-socket channel; ADR-0002 gateway authentication (`SOCK_SEQPACKET`, per-operation process evidence); Git staging-ref adapter; trace propagation; thin integrity slice; audit correlation; Gate 3 | Gate 3 fails, or no authentication mechanism satisfies ADR-0002 without enlarging the TCB beyond the WP0 bound |
| **1C — Real harness and binding** | inference adapter (second typed operation); existing coding-agent harness under the §4.5 contract; execution-binding control (Invariant 22); full Profile U conformance | the harness cannot run under the contract without grants that void a Gate 1–3 property |
| **1D — Comparative arm** | microVM control arm (ADR-0003); comparative measurements; Gate 4 comparative claim; Phase 2 decision | — (1D produces the decision; it is not itself gated) |

A stop at 1A is a result about the architecture and is published as such. A stop at 1B or 1C is a result about a mechanism and is published with the Profile U subset actually demonstrated.

---

## 4. Scope and architecture

### 4.1 Proposed components

Phase 1 builds five components and a CLI. A bespoke supervisor would duplicate systemd scopes plus a PID-namespace init. Audit is a pipeline rather than a daemon. Existing runtimes run without a custom runtime.

```text
agentbound
    CLI/API client: requests, observes, attaches to, and terminates sessions
    e.g.  agentbound run --agent finance-agent --task redwood-analysis -- <harness command>

agentbound-policy
    unprivileged resolver: principal, initiator, task, catalogue → signed authorization manifest
    (the constructor adds the launch binding; the verified pair is the effective manifest)
    (Phase 1: a file-backed stub with a stable interface, not an IAM integration)

agentbound-launch
    narrow, short-lived privileged constructor; validated host setup only; creates the
    systemd scope plus an in-session PID-namespace init/subreaper; drops privilege
    before exec; hands pidfds and the allocation record to agentbound-lifecycle and exits

agentbound-lifecycle
    one privileged long-running daemon: identity allocator (ADR-0001), holder of session
    pidfds, subscriber to systemd D-Bus scope signals; sole actor for quiesce, termination,
    reclamation, and restart reconciliation (a transient scope has no ExecStop=, so systemd
    cannot invoke a helper)

agentbound-gateway
    on-host gateway over one AF_UNIX SOCK_SEQPACKET socket (ADR-0002) with typed operation adapters:
      1B: exactly one — Git push to a staging ref of a protected repository
      1C: a second — model inference carrying session identity and execution binding
    propagates trace identity; never a generic HTTP or CONNECT proxy

agentbound-audit
    log pipeline and correlator: launch record + kernel audit + gateway log → effect table
```

Workloads are existing artefacts: `/bin/sh` and a minimal scripted model loop (no network) in 1A–1B, and an existing coding-agent harness in 1C. No `agentbound-runtime` component is built. Policy parsing, organizational authorization, and arbitrary network protocols must not be added to the privileged constructor merely because they are convenient.

### 4.2 Initial enforcement mechanisms

The Linux implementation should evaluate:

- global agent UUID and initiator identity;
- per-session execution identity allocator with verified reclamation and reuse quarantine (ADR-0001; lifecycle specified in WP0);
- systemd-managed cgroup v2 scope plus in-session PID-namespace init; pidfd-based supervision;
- mount, PID, IPC, UTS, and network namespaces, with the constructor ordering in technical-report §2.1;
- private runtime directory, `/tmp`, workspace, home view, and procfs;
- immutable/read-only base filesystem where feasible;
- minimal device view;
- capability bounding and ambient-capability removal;
- `no_new_privs`;
- optional Landlock filesystem and supported TCP restrictions;
- a minimal seccomp profile where it adds testable value;
- explicit file-descriptor allowlisting and closure;
- gateway authentication per ADR-0002: `SO_PEERCRED` plus peer pidfd at connection, per-packet `SCM_CREDENTIALS` per operation over a single mounted `AF_UNIX SOCK_SEQPACKET` socket, no session network interface; bearer tokens are excluded as the primary mechanism;
- the local-socket channel topology (§3.3): empty network namespace, socket-family seccomp permitting only `AF_UNIX`, exactly one mounted single-purpose gateway socket;
- immutable, signed effective launch record with stated trust anchor and clock;
- session trace identity propagated through the gateway; Linux audit plus gateway audit correlation;
- Git credential hidden from the session; pushes brokered by the gateway to a staging ref only;
- cgroup-wide termination and deterministic cleanup.

A mechanism is included only when it enforces a defined property.

### 4.3 Policy request and effective manifest

The untrusted request may identify only:

- registered agent principal;
- task or purpose identifier;
- requested named resources;
- requested runtime from an approved catalogue;
- optional bounded resource budget.

The policy service resolves these into a policy-signed, allocation-free **authorization manifest**. After validating it, the constructor atomically reserves an execution identity and produces a constructor-signed **launch binding** containing host allocation and substrate-specific projections. Together they are the immutable effective manifest and launch record; the launch binding is cryptographically bound to the authorization-manifest digest. This two-stage form prevents policy from signing a host UID that does not exist yet. The request must not supply authoritative numeric UIDs, paths, mount sources, labels, credential material, network addresses, capabilities, or namespace settings.

The initial schema should include:

```text
manifest version
authorization ID
agent global ID and durable ownership projection
per-session execution identity (launch binding only)
session trace identity
initiator ID(s), approver ID(s), scheduler/owner if scheduled
task/purpose and approval references
derivation inputs and policy/catalogue versions
runtime identity and artifact digest
approved execution binding (model, endpoint, tenant, adapters, inference pool, retention mode)
namespace and mount specification
descriptor allowlist
resource limits
network/gateway policy and permitted operations
credential or broker grants
audit requirements and audit-loss behavior
termination and retention policy
```

Fields are tagged **substrate-independent** (identity, derivation inputs, authority, budgets, gateway policy, audit requirements) or **substrate-specific** (namespaces, mounts, descriptors, seccomp). Only the former are shared with the control arm.

The schema and canonical serialization are frozen project artefacts (manifest schema 0.5) and precede the privileged constructor.

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
- defined retry/idempotency behaviour;
- failure cleanup;
- externally observable status.

Construction must not expose credentials or start the runtime before all required boundaries and audit bindings exist. Termination must stop descendants before credentials and session resources are released.

### 4.5 Integration contract for existing harnesses

Phase 1 must show that an unmodified or minimally modified coding-agent harness runs inside a session. The contract the session offers a harness is:

- **Working tree:** a per-session writable checkout of the task repository, bind-mounted at a stable path; the durable principal's storage is not mounted.
- **Credentials:** no Git, cloud, or API credential is present in the session environment, files, or descriptors. Git remote operations go through the gateway adapter, which accepts pushes only to `refs/agentbound/<session>/...` staging refs.
- **Model access (milestone 1C):** the harness's model endpoint is reachable only through the gateway's inference adapter, a typed operation carrying the session identity; the endpoint and tenant are fixed by the execution binding. Before 1C the only workloads are `/bin/sh` and a scripted loop that needs no model network access.
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
- workloads: `/bin/sh` and a minimal scripted model loop (1A–1B); an existing coding-agent harness (1C);
- a microVM control arm (1D) running the same substrate-independent manifest.

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
12. (1B) For effects in the **defined ontology only** — local object creation/modification in the session's world, process lifecycle events, and gateway operations — audit reconstruction identifies the responsible process and session, and the report states the fraction reconstructed by the test-catalogue metric. No attribution claim is made for intra-session files, pipes, prompt content, or logs outside that ontology.
13. A session pushes a patch to its staging ref; a direct push to `main`, a push to another session's staging ref, and a push with a forged trace identity all fail at the gateway.
14. (1C) Mutate or revoke each execution-binding member—model, endpoint, tenant, adapters or weights, inference pool, and retention mode. Each unapproved change is refused; every approved or refused change creates an audit event and compatibility decision; a changed binding is unusable until reauthorized.
15. (1A) A child session or process delegated from a running session receives fewer mounts, a reduced descriptor set, no or narrower gateway grant, and lower resource limits, and cannot recover the parent's authority.
16. Lifecycle and revocation cases produce the declared behaviour:
    - **1A:** A running session is terminated or quiesced, per policy, when its initiator is disabled, an approval expires, authority is revoked, a policy or catalogue version is withdrawn, or an approver cancels the task. When the policy service is unavailable or the audit pipeline degrades below its stop threshold, the manifest's declared response applies; only these two triggers may declare `continue-degraded`. If `agentbound-lifecycle` stops, installed containment remains in place, no new authority is issued, and reconciliation completes after restart. A reclassification request is rejected fail closed.
    - **1B:** Withdrawal of the Git gateway grant and gateway unavailability produce the declared behaviour.
    - **1C:** Revocation of the inference grant or execution binding produces the declared behaviour.
17. (1D) The demonstrations and suites that ADR-0003 pre-registers as **must run identically** are repeated on the control arm unchanged; those pre-registered as **substrate-equivalent** are run through their registered equivalent test; only those pre-registered as **inapplicable to a microVM**, with a stated reason, are omitted. No demonstration may be reclassified after control-arm results are seen. Demonstration 14 is in the identical set because the control arm shares the policy and gateway components.

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

### 6.4 Channel and gateway bypass

The session has no network interface, so the corpus first confirms that no network path exists, then attacks the single permitted channel and the gateway itself. Attempt:

- creation of any socket family other than `AF_UNIX` (INET, INET6, PACKET, NETLINK, VSOCK), and any interface or route inside the session network namespace;
- pathname and abstract-namespace Unix sockets of the host and of sibling sessions;
- connections opened before boundary installation and inherited descriptors;
- `SCM_RIGHTS` on the gateway socket; a passed connected gateway descriptor used from another process and from another session;
- packets carrying zero or multiple `SCM_CREDENTIALS`, forged credential fields, and PID reuse against the per-operation check;
- `SOCK_STREAM` or `SOCK_DGRAM` connection attempts to the gateway path;
- misuse of the approved gateway as a generic tunnel or SSRF oracle (redirects, alternate destinations, tenant or repository mismatch in operation arguments);
- replay of another session's launch-record, trace, or grant identity;
- operations after revocation on an established connection, and new connections after revocation;
- for the control arm: the ADR-0003 vsock-path realization of each item above, including CID reuse after teardown.

### 6.5 Constructor and request inputs

The privileged constructor is the primary confused-deputy target and receives at least the adversarial attention given to the session. Fault injection (Section 7.3) is not a substitute. Submit:

- unknown, duplicate, and reordered fields; malformed or non-canonical encodings;
- oversized and deeply nested requests;
- path traversal, non-canonical paths, symlinks, and mount-source substitution between resolution and use;
- request replay and concurrent duplicate requests;
- TOCTOU between policy resolution and launch (policy, catalogue, or filesystem changed in between);
- forged or downgraded policy and catalogue versions;
- smuggled numeric UIDs, capabilities, paths, network addresses, or namespace settings in fields that must not carry them;
- manifest/signature confusion (valid signature over a different manifest; unsigned manifest presented as signed);
- stale or double-allocated execution identities;
- requests from an unauthenticated or wrong-identity caller.

### 6.6 Bounded derivation (Invariant 3)

Against the policy resolver and constructor, attempt:

- unauthorized principal/task combinations;
- expired, revoked, or replayed approvals;
- conflicting approvers or an incomplete quorum;
- a scheduled initiator without an accountable owner;
- a recipient-issued grant exceeding `Auth_agent`;
- catalogue or runtime substitution;
- policy or version rollback;
- duplicate or ambiguous principal, initiator, or task identities.

Each must yield no session and an audit record naming the failed derivation input.

### 6.7 Monotonic delegation (Invariant 6)

From a running session, create a child session or process and verify the child has strictly non-increasing authority: fewer or equal mounts, a subset of descriptors, no or narrower gateway operations, lower or equal resource limits, and no path—via inherited descriptors, set-ID binaries, cgroup or systemd access, broker reuse, or the parent's credentials—to recover the parent's authority.

### 6.8 Active revocation and lifecycle (Invariant 21)

While a session is active, trigger each case and verify that the manifest's declared behaviour (terminate, quiesce, or continue with recorded degradation) occurs, that any applicable gateway authority is withdrawn, and that each transition is audited. Cases are allocated to the milestone at which the affected component exists:

- **1A:** initiator disabled; approval expired; authority revoked; policy or catalogue version withdrawn; task cancelled by an approver; local termination and quiescing; policy service unavailable; audit pipeline degraded below stop threshold; lifecycle daemon unavailable (not manifest-selectable: containment holds, transitions wait); a manifest declaring `continue-degraded` for any trigger other than the two permitted ones is rejected; reclassification request, which Profile U rejects fail closed with no changed authority/resource projection and audits with its policy basis.
- **1B:** Git gateway grant withdrawn; gateway unavailable.
- **1C:** inference grant or execution binding revoked.

Invariant 21 begins evaluation at 1A and is marked complete only when the latest applicable service exists; before that its row records the cases demonstrated.

### 6.9 Resource exhaustion

Each resource class is recorded as **applicable and enforced**, **applicable but failed**, or **absent from this deployment**. At 1B the Git adapter exposes bytes, requests, bandwidth, storage, and possibly monetary quota; model tokens and inference spend exist only from 1C; accelerator budgets may be absent entirely. "Full Profile U conformance" for Invariant 20 means every class *present* is enforced, and the absent classes are listed. Exercise limits for:

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
- audit-event volume and loss behaviour;
- trusted code size and privileged code size;
- number and breadth of retained capabilities;
- seccomp/Landlock/policy rule count where used;
- administrator actions required for launch, diagnosis, and cleanup;
- proportion of tested effects enforced locally, mediated by the gateway, or dependent on remote authorization;
- attribution completeness for local and remote effects.

### 7.2 Invariant evidence table

The evaluation report will contain one row per applicable technical-report invariant:

| Invariant | Milestone | Class | Enforcement mechanism | Adversarial test | Result | Residual assumption |
|---|---|---|---|---|---|---|
| 1 Durable identity | 1A | prevents | registry lookup in resolver; manifest binding | §6.5, §6.6 | Pending | registry trusted |
| 2 Explicit initiator | 1A | prevents | authenticated request; approver/owner fields | §6.6 | Pending | initiator authentication trusted |
| 3 Bounded derivation | 1A | prevents | resolver derivation; constructor accepts only resolved manifest | §6.6 | Pending | policy correctness |
| 6 Monotonic delegation | 1A | prevents | capability bounds, `no_new_privs`, descriptor allowlist, narrower child manifest | §6.7 | Pending | no set-ID binaries in world |
| 7 Fail-closed construction | 1A | prevents | ordered construction with rollback | §7.3, §6.5 | Pending | — |
| 10 Gateway-only egress | 1B | prevents | empty netns + seccomp + single `SOCK_SEQPACKET` socket + gateway | §6.4 | Pending | gateway and remote service trusted |
| 11 Credential confinement | 1B | prevents / assumption | brokered operations; no credential in session (ADR-0002) | §6.3 | Pending | non-exportability per mechanism |
| 12 Complete descendant control | 1A | prevents | systemd scope + PID-ns init + pidfd + `cgroup.kill` | §6.2 | Pending | no writable cgroup path; D-state delays |
| 13 Attribution of mediated effects | 1B | detects | launch record + trace identity + kernel/gateway audit | effect-ontology reconstruction | Pending | audit availability; lossy under load |
| 14 Policy provenance | 1A | detects | versions in signed launch record | §6.5 (forged versions) | Pending | launch-record trust anchor |
| 15 Launch privilege disposal | 1A | prevents | capability drop before exec; `agentbound-lifecycle` operations enumerated | §6.2, code review | Pending | `agentbound-lifecycle` trusted |
| 17 Same-principal session isolation | 1A | prevents | per-session execution identity + namespaces + descriptor discipline | §6.1 | Pending | kernel and constructor trusted |
| 19 Integrity promotion (protected-object subset) | 1B | prevents | gateway staging-ref restriction + branch protection | §6.4, demo 13 | Pending | Git host enforces protection |
| 20 Bounded external resources | 1B (1C for inference classes) | prevents | cgroup limits + gateway budgets | §6.9, per resource class | Pending; absent classes listed | gateway enforces budgets |
| 21 Lifecycle and revocation | begins 1A; complete at latest applicable service (1C) | assumption (policy choice) + prevents (declared behaviour) | `agentbound-lifecycle`; manifest-declared behaviour | §6.8, per milestone | Pending; cases recorded per milestone | policy choice documented |
| 22 Execution-binding control | 1C | prevents | inference adapter binding check | demo 14 | Not evaluated until 1C | gateway is the only inference path |
| 4, 5, 8, 9, 16, 18 | — | — | — | — | **N/A to Unix-governed profile** | — |

Every invariant applicable to profile U receives a row (Section 7.2). Results use the report's five classes:

- enforced and passed;
- enforced but failed;
- detected only;
- assumption (documented and accepted, with owner and revalidation trigger);
- not applicable to this profile.

`Not evaluated` is a milestone-progress status, not one of the five final conformance result classes. Once evaluated, each applicable invariant receives exactly one final class. Every residual assumption records its owner, impact, compensating control, acceptance authority, and revalidation trigger.

Pre-registered thresholds (interface inventory, adversary matrix, required attribution completeness, maximum policy-exception rate, pinned kernel/systemd/LSM versions) were fixed in WP0 (requirements §12, test catalogue §5, ADR-0003) before any test run.

### 7.3 Fault injection

Force failures during:

- authorization and derivation;
- execution-identity allocation, reclamation, and quarantine (including crash during allocation);
- namespace and mount setup, at each step of the §2.1 ordering;
- empty-network-namespace verification and gateway-socket mounting;
- cgroup setup;
- credential/gateway grant issuance;
- audit binding;
- privilege disposal;
- runtime `exec`;
- active-session supervision;
- termination and cleanup.

The implementation must either fail closed or document why the property cannot be claimed.

---

## 8. Control arm: microVM

Phase 1 **requires** a control arm for its comparative claim. Without it, the evaluation cannot show whether the Linux-process design offers anything that a stronger substrate with per-session workload identity, egress gateway, and audit does not.

The control arm is a **microVM**, not a container. A container repackages the same shared-kernel mechanisms and would test packaging rather than boundary strength. A hardened-container arm may be added as a third, labelled arm to measure operability only.

ADR-0003 fixes the implementation and configuration: Firecracker with a minimal guest and one vsock path to the gateway, with configuration pinned. It also fixes the **pre-registered comparative decision rule**: what result justifies the Linux design as default, what selects the microVM, and how ties and partial outcomes are reported.

The control arm reuses `agentbound-policy`, `agentbound-gateway`, and `agentbound-audit`, and substitutes a minimal microVM launcher for `agentbound-launch`. Held constant across arms are policy and derivation inputs, the gateway and its adapters, trace identity, the audit pipeline, workloads, adversary capabilities inside the session, and the substrate-independent manifest fields. The execution boundary and everything substrate-specific vary.

ADR-0003 **pre-registers test equivalence** before any control-arm result is seen. Each demonstration and suite item is classified as *must run identically*, *substrate-equivalent*, or *inapplicable to a microVM*, with the reason. A substrate-equivalent item names the equivalent test that preserves the property when attack mechanics differ (worked example in ADR-0003). The ADR states any difference in the adversary's host access or privileges between arms. Reclassification after results are seen is prohibited. The substrate-independent manifest fields and workload are launched through:

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

The comparison identifies when host-process isolation is sufficient and when a microVM materially improves assurance. It identifies which identity, gateway, storage, and audit abstractions remain substrate-independent, and what the process-session design costs or saves. Equivalent assurance must not be inferred from shared manifest fields. Substrate-specific refinements are listed separately for each arm.

The control arm is milestone 1D. It may lag the Linux arm, and 1A–1C results may be published without it. The comparative claim in Gate 4 and the Phase 2 decision require its results.

---

## 9. Work packages and ordinal sequence

The work proceeds by exit conditions rather than calendar estimates. A later work package begins only when its prerequisites are satisfied; independent packages may overlap when doing so does not weaken a gate or review boundary.

### WP0 — Specification freeze and threat model (complete)

WP0 is complete. The [architecture README](../architecture/README.md) indexes the frozen set, and its freeze record names the commit at which the exit condition was met. Deliverables (as frozen):

- Phase 1 normative requirement list using MUST/SHOULD/MAY;
- scoped threat model and non-goals;
- effective-manifest schema and canonical encoding;
- session lifecycle and failure-state specification;
- **execution-identity lifecycle specification** (ADR-0001 open items): host-local versus fleet-wide uniqueness; allocation source; the declared managed reclamation domain and its *condition* (no live process, owned object, or grant within the domain) rather than a fixed period; the rule that exports beyond the domain never rely on the numeric UID; discovery or elimination of owned objects at reclamation; disambiguation of audit history by execution UID plus boot/session identity; crash-recovery and exhaustion behaviour; interaction with backups and persistent files carrying numeric ownership;
- **ADR-0002** gateway authentication: the local-socket topology and its mechanism (`SOCK_SEQPACKET`, `SO_PEERCRED` plus pidfd at connection, per-packet `SCM_CREDENTIALS` per operation), binding of that evidence to the immutable launch record, connection-lifetime behaviour (before/after revocation, caller exit, descriptor passing prohibited, stale-connection invalidation at termination), and the WP1 kernel-baseline verification list;
- **component-interface skeleton**: per component pair, transport, peer identity, authorization matrix, trust anchors and key custody, launch-record and allocator store commit models, idempotency and replay rules, error classes, and restart-reconciliation precedence (wire formats deferred to WP1);
- **test catalogue**: every demonstration, suite bullet, and fault point as an atomic test with ID, adversary privilege, interface, expected outcome, and evidence artefact; the attribution-completeness metric with denominator, dedup, and correlation deadline; nominal and overload load profiles; gate pass rules (fixtures and commands deferred to WP1);
- **ADR-0003** control substrate: pinned microVM implementation and configuration; the held-constant list; the per-test equivalence classification keyed to the test catalogue with separate guest-root and guest-unprivileged corpora; cross-arm SLOC accounting; and the pre-registered comparative decision rule;
- pre-registered thresholds: interface inventory, adversary-capability matrix, attribution completeness by the catalogue metric, maximum policy-exception rate, privileged-code reviewability bound with SLOC accounting rules, pinned kernel/systemd/LSM versions;
- invariant-to-test traceability matrix covering every profile U invariant.

Exit condition (met): every profile U invariant maps to at least one catalogue test ID; every threshold in the requirements §12 has a defined measurement; every open question in the WP0 documents is either answered or explicitly deferred to WP1 with a named verification item.

### WP1 — Mechanism spikes

Prototype high-risk mechanisms independently:

- per-session execution identity allocation and the durable-ownership projection (ADR-0001);
- systemd scope + PID-namespace init containment and `cgroup.kill` behaviour, including D-state tasks;
- namespace, mount, and procfs construction in the §2.1 ordering, with mount-descriptor resolution;
- descriptor closure and runtime launch ordering;
- socket-family seccomp and abstract-socket isolation in an empty network namespace;
- ADR-0002 Decision 7 verification: `SOCK_SEQPACKET` credential semantics, pidfd from credential PID, descriptor-transfer rejection, revocation latency;
- `agentbound-lifecycle` D-Bus scope-signal subscription and pidfd-watch fallback, including the systemd-kills-first race;
- the four register items of the WP0 [open-question register](../architecture/open-question-register.md): VM-1 vsock peer-CID reporting, VM-2 cross-arm SLOC comparability, LC-2 frozen-cgroup connection behaviour, ID-1 allocator-store crash consistency;
- Git staging-ref adapter and protected-branch behaviour;
- `loginuid` and audit correlation, including loss behaviour under load;
- minimal control-arm launcher.

Exit condition: every ADR-0002 Decision 7 item and every WP1 spike above records pass with evidence on the pinned baseline; any fail reopens the relevant ADR before WP2 begins.

### WP2 — Constructor and lifecycle (milestone 1A)

Implement the minimum request, policy-stub, construction, identity allocator, systemd-scope lifecycle, active-revocation handling, delegation narrowing, and cleanup path. Keep policy resolution unprivileged, the privileged constructor narrow and short-lived, and the `agentbound-lifecycle` daemon separate and enumerated. Run the §6.1, §6.2, §6.5–6.8 suites and §7.3 fault injection.

Exit condition: Gates 1 and 2 pass with `/bin/sh` and the scripted loop; the milestone 1A stop condition is not met.

### WP3 — Gateway and end-to-end audit (milestone 1B)

Implement the ADR-0002 authentication mechanism, the Git staging-ref gateway operation, session-bound trace identity, the egress topology, and correlated audit. Run §6.3, §6.4, and §6.9.

Exit condition: Gate 3 passes; the remote effect is attributable end to end; the thin integrity slice (goal 11) holds; bypass tests have defined outcomes.

### WP4 — Inference adapter and existing-harness integration (milestone 1C)

Implement the inference adapter and execution-binding check; run an existing coding-agent harness under the §4.5 contract.

Exit condition: the harness runs without grants that void a Gate 1–3 property; an execution-binding change is recorded and, where unapproved, refused; full Profile U conformance table complete.

### WP4b — Control arm (milestone 1D)

Build the ADR-0003 microVM launcher and run every supported demonstration and suite on it. Section 8 and ADR-0003 define the rationale and decision rule.

Exit condition: the comparative measurements of Section 8 exist for both arms.

### WP5 — Suite consolidation

Automate and consolidate the Section 6 suites and Section 7.3 fault injection across both arms so they run unattended and reproducibly.

Exit condition: every profile U invariant has evidence, a recorded failure, or a "not evaluated" entry explained by the milestone reached.

### WP6 — Evaluation and security review

Produce measurements for both arms. Review privileged code—the launch path, `agentbound-lifecycle` including the allocator, and the gateway authentication path—line by line under the SLOC accounting rules. Apply the ADR-0003 decision rule, conduct an independent architecture review, and publish the Linux-arm versus microVM-arm delta.

Exit condition: issue a go/no-go recommendation for Phase 2.

---

## 10. Sequence and staffing

The ordinal sequence is:

1. froze the bounded specification, threat model, and ADRs 0002–0003 (WP0 — complete);
2. resolve high-risk mechanism questions with focused spikes (WP1);
3. **1A** — construct, isolate, delegate, revoke, and terminate a fail-closed shell session (WP2); stop point;
4. **1B** — bind one protected remote effect to session identity and audit (WP3); stop point;
5. **1C** — add the inference adapter and run a real harness (WP4); stop point;
6. **1D** — run the microVM control arm (WP4b) and consolidate suites (WP5);
7. evaluate evidence, conduct independent review, and make the Phase 2 decision (WP6).

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

Prioritize for real project, customer, legal-matter, or tenant separation. Add category allocation, labelled storage, compartment-aware gateways, and controlled peer import.

### Branch C — VM-backed sessions

Prioritize if shared-kernel or device risks remain unacceptable. Preserve the same policy, manifest, workload identity, gateway, and audit interfaces while changing the execution substrate.

### Branch D — Trusted release and full MLS

Prioritize only with a concrete classified or regulated workflow whose required information flows and review economics can be measured, and treat the release-economics targets of technical-report §3.5 (and their conformance status in §11) as a go/no-go condition. Do not adopt full MLS merely because the paper describes it.

---

## 12. Review questions

Reviewers are asked to focus on:

1. Is the Phase 1 claim narrow enough to be falsifiable?
2. Are any non-goals actually prerequisites for the claimed Unix-governed profile?
3. Is the proposed privileged/unprivileged split credible?
4. Does ADR-0001 (per-session execution identity, durable principal as ownership only) close the same-principal isolation gap, and what does it cost in identity allocation and audit mapping?
5. Is a systemd scope plus in-session PID-namespace init plus the `agentbound-lifecycle` daemon the right lifecycle machinery, and does the daemon fit the reviewability bound?
6. Is the gateway experiment sufficient to establish end-to-end identity binding?
7. Which tests are missing from the adversarial suite?
8. Which measurements determine whether process isolation or microVM isolation should be the default?
9. Are the four gates appropriate go/no-go criteria?
10. Is the work-package scope appropriately bounded, and what should be cut first if a gate requires substantially more mechanism or trusted code than proposed?
11. Is the thin integrity slice the right first integrity claim, and does the §4.5 integration contract match what real harnesses need?
12. Does fixing the control arm as a microVM, with the held-constant list in Section 8, make the comparison fair?
13. Are the 1A–1D stop points placed where a failure is most informative and least expensive?
14. Is deferring the inference adapter and Invariant 22 to 1C the right trade between a small core experiment and real-harness evidence?

---

## 13. Expected Phase 1 outputs

1. A versioned normative Phase 1 specification, including the execution-identity lifecycle specification and ADRs 0002 and 0003.
2. Reference constructor, `agentbound-lifecycle` daemon (with identity allocator), policy stub, Git and (1C) inference gateway adapters, and audit correlator, plus a minimal microVM control-arm launcher.
3. Machine-readable request and effective-manifest schemas.
4. Reproducible host configuration and policy examples.
5. Automated adversarial conformance and fault-injection suite.
6. Invariant-to-evidence traceability matrix.
7. Measured evaluation report, including negative results.
8. Independent mechanism and architecture review reports.
9. Published Linux-arm versus microVM-arm comparison.
10. A go/no-go recommendation and selected Phase 2 branch.
