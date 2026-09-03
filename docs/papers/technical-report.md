# Agents as Unix Principals

## Security Architecture and Evaluation Programme

**Version:** 0.5-TR5  
**Date:** 28 August 2026  
**Status:** Working technical report for external review  
**Companion:** [`position-paper.md`](position-paper.md)  
**Provenance:** Expands the position paper with normative invariants, mechanism details, threat model, operational limits, and phased evaluation

---

## Revision history

- **0.1** — Initial principal/session/MLS proposal.
- **0.2** — Added tiered profiles, integrity, partitioned memory, declassification economics, and microVM comparison.
- **0.3** — Added DIFC lineage, peer communication, principal lifecycle, and phased evaluation.
- **0.4-TR1** — Split the position paper from the technical architecture and evaluation report; added cumulative-release and concurrency tests.
- **0.4-TR2** — Restructured this document as a normative companion, removed duplicated thesis and conclusion material, established document ownership, and corrected cross-document attribution.
- **0.5-TR3** — Incorporated three independent reviews: typed formalism and authorization-derivation relation replacing universal intersection; per-session execution identity made normative; gateway-only egress topology specified; constructor ordering and post-launch privileged TCB stated honestly; invariants profile-scoped and classified as prevention, detection, or assumption; attribution narrowed to mediated effects; model treated as an approved execution binding; structured versus semantic memory promotion separated; foundational confidentiality and integrity models cited.
- **0.5-TR4** — Added an explicit statement that the formalism is an unproven specification and listed what remains open; made the same caveat part of the conformance definition.
- **0.5-TR5** — Execution identity restated as uniquely allocated with verified reclamation and reuse quarantine; ownership/execution separation stated as an invariant with profile-specific realizations rather than a universal "never executes".

---

## Purpose, scope, and document ownership

The companion [position paper](position-paper.md) is the citable statement of the motivation, thesis, adoption argument, and conclusions. This report is the normative source for mechanism mapping, constructor behavior, information-flow rules, security invariants, threat assumptions, deployment constraints, and evaluation. It does not repeat the full argument; where motivation is required, it refers to the position paper.

The report remains self-contained at the level needed to implement and test the architecture. Normative terms and notation are restated below, but their explanatory treatment belongs to the position paper. Changes to shared concepts should be made there first and then reflected here; detailed mechanisms and tests should be changed here and only summarized in the position paper.

### Normative terminology

- **Agent principal:** a durable organizational security principal with a stable global identity, an accountable owner, potential authority and clearance, partitioned durable state, credential/model/tool policy, delegation constraints, retention rules, and lifecycle policy. The durable principal is a policy, ownership, and audit identity. A host UID that owns durable state is a local projection of it. The identity that owns durable state and the identity under which a session acts must be distinguishable to whatever enforces the session boundary; in the Unix-governed profile the owning UID does not run session processes.
- **Execution identity:** the uniquely allocated local credential (UID, supplementary groups, and where applicable MAC type) under which one session's processes execute. It is allocated per session, never shared between concurrent sessions, and reclaimed only under a verified condition (no live process, no owned object, no audit record that depends on the numeric identity alone) followed by a reuse quarantine; audit records pair it with launch, boot, and session identifiers so that finite UIDs can be reclaimed without ambiguity. Durable state is reached through per-session grants (bind mounts, ACLs, descriptors, or a storage broker), not by running as the owner.
- **Session:** a task-scoped realization of one agent principal, bound to an authenticated initiator, purpose, approvals, activated authority, confidentiality and integrity state, visible world, execution identity, credentials, budgets, process tree, outputs, and audit identity.
- **Execution:** an ordinary process within a session. A cognitive runtime, shell, compiler, retrieval command, or model client is an execution, not an independent security principal unless separately provisioned as one.
- **Model and execution binding:** a model is a cognitive implementation invoked by an execution. The session identity is stable across model replacement, but the model, endpoint, provider tenant, adapters or fine-tuned weights, retention mode, and inference pool together form an **approved execution binding**. Changing any element of the binding is a policy-controlled, auditable event that requires a compatibility decision, because it changes the session's information sources, sinks, and trusted computing base.
- **Communication edge:** any prompt, message, file, descriptor, pipe, socket, queue entry, RPC, artifact, memory import, or service result admitted by another process or session. Each edge is either **mediated** (labeled and checked by a named component) or **unmediated**; a profile's information-flow claims extend only to its mediated edges.

Agent context comprises cognitive, informational, security, and organizational components. The session is the primary task-scoped boundary because admitted information may enter prompts, process memory, summaries, transcripts, caches, outputs, child messages, and durable memory.

### Normative notation

The notation separates domains that earlier drafts conflated. Each is a distinct typed value; no operator combines values of different domains.

```text
Auth   authority: a set of (resource, operation) capabilities
Clr    clearance: a confidentiality label from lattice L_C (higher = more sensitive)
I      confidentiality state: a label from L_C
T      integrity/provenance state: a label from lattice L_T (higher = more trusted)
Purp   purpose: an organizational claim used for policy selection and audit
Appr   approvals: endorsements by identified approvers, with expiry
Bud    budgets: resource, token, spend, and fan-out limits
```

The values attached to the principal and to a session are:

```text
Auth_agent, Clr_agent            principal's potential authority and clearance ceiling
Auth_session ⊆ Auth_agent        authority activated for one task
I_session ⪯ Clr_agent            confidentiality domains already admitted
T_session                        integrity justified by every admitted input
```

`Auth_session` is written `A_session` and `Auth_agent` written `P_agent` in the position paper; the abbreviations denote the same values.

### Authorization derivation

Activated authority is produced by a policy-defined **derivation relation**, not by a single universal operator:

```text
derive(Agent, Initiators, Task, Approvals, Policy) → Auth_session
```

The relation must satisfy:

```text
Auth_session ⊆ Auth_agent
Auth_session ⊆ Auth_permitted(Task, Policy)
```

Initiator authority enters the derivation differently according to the relationship:

- **Delegation** by a human or service acting on its own behalf: `Auth_session ⊆ Auth_initiator`. This is the common case and yields the position paper's intersection form `A_session ⊆ P_agent ∩ A_initiator ∩ A_task ∩ A_policy`.
- **Endorsement or approval:** an approver contributes an approval object, not authority. Approvals may be required for the derivation to produce a non-empty result, including conjunctive (two-person) and quorum rules where no individual holds sufficient authority alone.
- **Scheduled or system initiation:** the initiator is a registered scheduling principal with a named accountable owner; its authority is a policy grant, and the session records both the scheduler and the owner.
- **Agent-initiated sessions:** when a session requests another agent's session, the requesting session is the initiator and contributes `Auth_parent_session`; monotonicity (Section 4) applies.
- **Recipient-issued rights:** a service or organization may grant a narrower capability directly to the session (for example a scoped token); such grants never expand `Auth_agent`.
- **Multi-caller service agents:** an agent serving several callers does not run one session with the intersection or union of their authority. It runs one session per caller request, or acts as an explicitly authorized reference monitor whose own authority and information state are specified by policy.

The derivation must be deterministic given its inputs, must record every input identity and version in the launch record, and must fail closed when any input is unauthenticated, expired, or unknown.

### Information admission

Every communication edge is governed by an admission relation evaluated by the component that mediates the edge:

```text
admit(receiver, input) permitted iff
    receiver is authorized for the channel
    and I_receiver' = join(I_receiver, I_input) ⪯ Clr_receiver
    and T_receiver' = meet(T_receiver, T_input)
```

Admission updates the receiver's state to `(I_receiver', T_receiver')`. The receiver's prior state participates in both operations; labels are monotone over a session's lifetime except through the trusted transitions below. Inputs whose labels are unknown are treated as the highest confidentiality and lowest integrity the channel may carry.

Labels attach to objects (files, memory partitions, artifacts) and to channels (pipes, sockets, queues, gateway operations), not only to processes. A channel carries a maximum confidentiality and a minimum integrity; a mediating component must refuse inputs outside the channel's declared range.

Consequences:

```text
I_receiver ⪰ join(confidentiality labels of every admitted input)
T_receiver ⪯ meet(integrity labels of every admitted input)
```

Narrowing authority does not remove information already observed, and a fresh process or model does not restore integrity. Raising integrity or lowering confidentiality requires an explicit trusted transition:

```text
untrusted input → staging → validation or review → trusted promotion      (raises T)
high-domain output → trusted release (transform, redact, or review)        (lowers I)
```

Each transition is performed by a named trusted component with its own identity, evidence requirement, audit record, and rollback path; a session cannot perform either transition on its own state.

Standard SELinux MLS fields should not be treated as an automatic independent confidentiality-and-integrity product lattice. Implementations may combine MAC domains with structured import, immutable inputs, staging, deterministic checks, review, and constrained promotion services. The confidentiality rules follow the Bell–LaPadula and Denning lattice tradition; the integrity rules follow Biba, and the promotion path follows Clark–Wilson's separation of unconstrained from constrained data items via certified transformation procedures.[^blp][^denning][^biba][^clark-wilson]

### Mediation coverage

A profile's confidentiality and integrity claims apply only to edges mediated by a named component. Each profile must publish a **coverage inventory**: the edges it mediates, the component that mediates each, and the edges it leaves unmediated or prohibits. The Unix-governed profile (Section 9.1) mediates authority, execution-world separation, credential use, and specified gateway operations; it does not mediate the content of prompts, tool arguments, pipes between processes of one session, logs, or model responses, and therefore makes no confidentiality- or integrity-propagation claim.

### Status of the formalism

The notation, derivation relation, and admission relation above are a **specification, not a proven model**. They are written in prose and set notation; they have not been expressed as a transition system with a defined state space, mechanized in a proof assistant or model checker, or shown to satisfy noninterference, a Biba-style integrity theorem, or any other formal security property. Several elements are deliberately left open: the concrete lattices `L_C` and `L_T`, the policy-defined body of `derive`, the semantics of label evolution across trusted transitions, and the treatment of channels whose label range changes during a session.

The specification is precise enough to implement and to write conformance tests against, and that is its intended use in Phase 1. Formalizing the state and transition system, proving that the admission relation preserves the stated bounds on every mediated edge, and identifying the assumptions such a proof requires are explicit later deliverables (Phase 2, WP0 of the corresponding plan). Until then, any claim that a deployment "satisfies the model" means only that it passes the tests derived from this specification.

---

## 1. Unix and Linux mapping

The proposal uses existing mechanisms wherever their semantics fit.

| Agent-system concept | Unix/Linux mechanism |
|---|---|
| Durable local agent principal (ownership) | Stable UID / system account, or storage service |
| Session execution identity | Per-session UID and groups, uniquely allocated with verified reclamation |
| Organizational memberships | GIDs, supplementary groups, POSIX ACLs |
| Mandatory identity and authorization range | SELinux user, role, types, MLS range |
| Agent private state | Home directory, ownership, quotas, labeled storage |
| Session | PAM/login session, `setsid(2)`, SID, PTY |
| Job or pipeline | Process group and standard job control |
| Current execution | PID and process credentials |
| Session’s visible world | Mount, network, IPC, PID, and user namespaces |
| Communication | stdin/stdout/stderr, pipes, Unix sockets, inherited FDs |
| Process lifecycle | `fork`, `clone`, `execve`, `wait`, `exit`, signals |
| Mandatory classification | SELinux MLS sensitivities and MCS categories |
| Discretionary file access | Owner/group mode bits and POSIX ACLs |
| Privilege decomposition | Linux capabilities and securebits |
| Irreversible anti-escalation | `PR_SET_NO_NEW_PRIVS` |
| Additional self-restriction | Landlock filesystem and, on supporting ABIs, TCP bind/connect rules |
| Syscall attack-surface reduction | seccomp-bpf |
| Resource limits and accounting | cgroups v2, rlimits, filesystem quotas |
| Supervision | parent process and systemd service/scope units |
| Provenance and denials | Linux Audit, login UID, SELinux AVC records |
| Durable coordination and memory | labeled filesystem objects |

These mechanisms are compositional. In a typical access decision, visibility and permission are jointly constrained:

```text
allowed =
    object is visible in the namespace
    AND DAC/ACL permits access
    AND SELinux policy permits access
    AND Landlock permits access
    AND the syscall is permitted
```

Linux process credentials already distinguish PIDs, sessions, process groups, real/effective/saved UIDs and GIDs, supplementary groups, and capabilities. The `credentials(7)` documentation describes sessions and process groups as the hierarchy supporting shell job control.[^credentials] cgroups organize process hierarchies for limits, monitoring, freezing, and accounting.[^cgroups] Namespaces provide isolated views of global resources.[^namespaces] No novel “agent scheduler” or “agent job” primitive is required to obtain these baseline semantics.

### 1.1 Why both DAC and MAC are needed

UID/GID permissions are discretionary access control (DAC): an authorized owner can often change an object’s discretionary permissions. They work well for ownership, collaboration, and stable organizational groups, but they are not sufficient for classified information. Unix session IDs and process groups likewise support terminal job control; they carry no file- or network-authorization semantics by themselves.

Mandatory access control (MAC) places policy above object owners. SELinux combines identities, roles, types, and—under an installed and selected MLS policy—sensitivity levels and categories. Red Hat’s SELinux MLS documentation explicitly describes enforcement based on the Bell–LaPadula model and the “no read up, no write down” principle.[^rhel-mls] This is not the behavior of a typical default `targeted` SELinux installation: an MLS policy must be installed, selected, and validated. MCS categories can represent compartments such as a project, customer, legal matter, or business unit.

An illustrative process context is:

```text
agent_u:agent_r:agent_session_t:s2:c17,c42
```

with a local convention such as:

```text
s2  = Confidential
c17 = Finance
c42 = Project Redwood
```

The exact category allocation and dominance rules are policy decisions, not universal meanings built into SELinux.

### 1.2 UID is not the whole identity, and the durable UID is not the runtime identity

“Agent as user” should not be read as “put the entire enterprise IAM graph in `/etc/passwd` and `/etc/group`.” A UID is the local kernel principal. At organizational scale, a directory or identity service remains authoritative, and a session broker projects a verified agent identity into a host credential, SELinux context, and workload identity.

The durable agent principal has two distinct local roles that must not be collapsed:

- **Ownership projection.** A stable UID (or a storage service acting for the principal) owns durable state so that ordinary DAC, backup, quota, and audit tooling attribute objects to the agent. In the Unix-governed profile this identity does not execute session code.
- **Execution identity.** Each session runs under a **per-session, uniquely allocated UID with verified reclamation and reuse quarantine** with its own supplementary groups and, in MAC profiles, its own type or category set. Durable partitions activated for the session are exposed through per-session bind mounts, ACL grants, inherited descriptors, or a broker operating on the session's behalf.

This split is normative because two processes sharing one UID pass ordinary DAC and signal checks against one another; `hidepid`, PID namespaces, and Yama `ptrace_scope` distinguish UIDs or hide identifiers but do not create an authorization boundary between same-UID processes. Same-principal session isolation (Invariant 17) is therefore achievable only with distinct execution identities or a rigorously allocated per-session MAC type; the baseline profile uses distinct execution identities. The architectural decision is recorded in ADR-0001.

Likewise, POSIX groups are useful for durable coarse-grained membership but are a poor fit for every transient object-level grant. NIST’s ABAC model is relevant to computing a decision from subject, object, action, and environmental attributes.[^nist-abac] The proposal’s division of labor is:

```text
organizational IAM / RBAC / ABAC  → computes what may be activated
Linux credentials and MAC         → enforce the resulting local boundary
```

---

## 2. Session construction

The main new trusted component is a small **agent session constructor**. It should be privileged enough to establish credentials, labels, namespaces, and audit provenance, but small enough to review and test. It is not an application-level reference monitor for every file access.

A launch proceeds conceptually as follows:

```text
authenticate initiator
        ↓
resolve durable agent principal
        ↓
validate task, purpose, approvals, and requested data domains
        ↓
derive effective session authority and initial information state
        ↓
allocate per-session execution identity and optional MAC context
        ↓
construct namespaces, mounts, and approved file-descriptor set
        ↓
create cgroup/systemd scope and resource limits
        ↓
attach network path to gateway only
        ↓
bind audit/session provenance and launch record
        ↓
issue or broker short-lived external credentials
        ↓
set no_new_privs and optional Landlock/seccomp policy
        ↓
drop launcher privilege
        ↓
exec shell or cognitive runtime
```

The effective authority is the output of the derivation relation defined in the front matter; in the common delegation case it reduces to:

```text
A_session ⊆ P_agent ∩ A_initiator ∩ A_task ∩ A_current_policy
```

This concerns activated authority, not information contamination. The initial session information label must dominate every prompt, memory partition, file domain, descriptor, and service result admitted at launch; subsequent import must preserve or raise it. Purpose can constrain which session is authorized and which credentials or gateway operations are issued, but the kernel cannot infer whether an otherwise permitted read is genuinely being performed for the declared business purpose. Purpose is an authorization and audit attribute, never evidence that an action was appropriate.

This is not directly a Linux formula; it is the policy computation performed before launch. Its result is projected into mechanisms the kernel understands. Failure to establish any required boundary must abort launch rather than silently degrade.

A durable Finance agent might be authorized for Finance, Forecasting, and Acquisitions. A session created to analyze Project Redwood should activate only Finance and Redwood, expose only the necessary directories, use only a model endpoint authorized for the classification, and receive credentials limited to that purpose. Clearance represents a ceiling, not ambient access.

### 2.1 Construction ordering and privileged operations

The conceptual sequence above hides ordering constraints that are security-critical. A conforming constructor:

1. creates the child stopped (or via `clone3` with the required namespace flags) before any credential or mount is visible to it;
2. unshares the mount namespace and marks all mounts recursively private before any bind operation, so nothing propagates back to the host;
3. resolves mount sources through descriptor-relative, path-safe operations (`openat2` with `RESOLVE_BENEATH`/`RESOLVE_NO_SYMLINKS`, or the new mount API with mount file descriptors), never through re-walked string paths;
4. builds the restricted tree and enters it with `pivot_root`, not `chroot`;
5. mounts `proc` only after the PID namespace exists, and never exposes host `/proc`;
6. closes every descriptor not on the effective manifest's allowlist, including descriptors that could be reintroduced through `SCM_RIGHTS`, `/proc/self/fd`, or memfd;
7. installs the execution identity, supplementary groups, LSM context, capability bounding set, `no_new_privs`, Landlock, and seccomp in an order tested against the selected LSM policy;
8. makes credentials or broker access usable only after every boundary is in place and the launch record is committed;
9. execs the runtime.

Each step requires privilege: `CAP_SETUID`/`CAP_SETGID` for credential transition; `CAP_AUDIT_CONTROL` for `loginuid`; `CAP_SYS_ADMIN` and `CAP_NET_ADMIN` in the relevant namespaces for mounts, `pivot_root`, and veth setup; MAC administrative authority for contexts; and the privileged system manager for scope creation and cgroup delegation. The launch helper disposes of these before `exec`, but **the post-launch trusted computing base does not disappear**: termination, credential revocation, mount cleanup, firewall rules, audit rule management, and systemd interaction remain privileged operations performed by separately authorized services. "Dispose privilege" is a property of the launch path, not of the whole system.

Lifecycle observation also needs more than a cgroup. cgroup v2 provides limits, accounting, freezing, and `cgroup.kill`, but does not reap processes, cannot terminate tasks in uninterruptible sleep, and contains descendants only if the workload has no writable path into the cgroup hierarchy or the system manager. The session should run a PID-namespace init or subreaper, hold pidfds for supervised processes, and treat systemd D-Bus, writable cgroup files, container-runtime sockets, and broker sockets as privileged capabilities that must not be present in the session's world.

Every irreversible operation—identity allocation, mount, cgroup creation, firewall rule, credential issuance, launch record—must have a defined rollback, and fault injection at each step must show no runnable session, usable credential, or ambiguous record survives a failure.

### 2.2 A session manifest

A portable declarative record could look like:

```yaml
agent: finance-analyst
initiator: alice@example.com
purpose: analyze-quarterly-forecast
classification:
  sensitivity: confidential
  compartments: [finance, project-redwood]
authority:
  groups: [finance-forecast-read]
filesystem:
  - path: /org/finance/common
    access: read-only
  - path: /projects/redwood
    access: read-only
  - path: /sessions/${SESSION_ID}/work
    access: read-write
network:
  allow:
    - private-model.company.internal:443
credentials:
  - service: forecast-db
    scopes: [forecast.read]
resources:
  memory_max: 8G
  cpu_quota: 200%
  runtime_max: 2h
```

The manifest is not itself the security boundary and must never be interpreted as an entitlement. The constructor accepts only an authenticated initiator, a registered agent identifier, a task/purpose identifier, and a bounded set of requested resources. It resolves all named resources through a server-side catalog, intersects the request with current agent and initiator policy, rejects unknown fields and non-canonical paths, and produces the effective manifest itself. Agent-authored text may request a future session but cannot supply authoritative UID, label, mount, credential, or network values. The resulting immutable launch record supports audit and reproducibility.

Hostnames in such a manifest are policy names rather than sufficient enforcement objects. DNS rebinding, redirects, CDNs, proxies, IPv6, local sockets, TLS identity, and multi-tenant endpoints make hostname allowlists fragile. Sensitive sessions should normally reach an authenticated gateway or workload identity whose service, tenant, model, retention mode, method, and budget can be verified; network policy should prevent bypassing it.

Credentials should likewise be treated as capabilities, not merely short-lived strings. Exportable bearer tokens placed in files or environment variables can be copied by any hostile process in the session. Where possible, sensitive operations should use proof-of-possession credentials, inherited descriptors, kernel keyrings with appropriate restrictions, or a broker that performs narrowly authorized operations without revealing a reusable token.

This constructor is a privileged reference monitor at launch and a prime confused-deputy target.[^confused-deputy] Its parser, path resolution, policy lookup, environment construction, file-descriptor handling, race resistance, and privilege dropping are in the trusted computing base. A production design should separate unprivileged request parsing from a narrow privileged executor, authenticate IPC, use descriptor-relative/path-safe operations, avoid shell interpolation, and make partial construction fail closed.

---

## 3. Classification and information flow

Classification is the largest difference between ordinary agent sandboxing and the proposed model.

A sandbox usually asks: **may this process open this object?** A classified agent system must also ask: **after the process reads the object, where may the resulting information flow?**

If a session reads `Secret:Redwood`, the following state may contain Redwood information:

- model prompts and responses;
- process memory and swap;
- transcript and summaries;
- retrieval caches and embeddings;
- stdout and stderr;
- generated files;
- crash dumps and telemetry;
- child-agent prompts;
- checkpoints and durable memory;
- requests sent to a remote model endpoint.

Let a label be `L = (s, C)`, where `s` is an ordered sensitivity and `C` is a set of unordered compartments. Define dominance componentwise: `(s₁, C₁) ⪰ (s₂, C₂)` when `s₁ ≥ s₂` and `C₁ ⊇ C₂`. The least upper bound is `(max(s₁,s₂), C₁ ∪ C₂)`. A conservative session-construction rule is then:

```text
L_session = join of all information domains visible to the session
L_output  = L_session unless a trusted downgrade occurs
```

For multiple inputs:

```text
L_output ⪰ join(L_session, L_input1, …, L_inputN)
```

This is a design rule for assigning the session and output labels; SELinux does not dynamically compute that join from the read history of an ordinary process.

SELinux can prevent a session running in a high context from writing to a lower-labeled object when the MLS policy is configured accordingly. The reference policy is commonly stricter than textbook Bell–LaPadula for ordinary writes, requiring equal levels unless a trusted-domain exception applies.[^selinux-mls] It can prevent unauthorized relabeling and constrain which domains perform transitions. It cannot inspect generated prose, infer its semantic classification, or automatically compute a derived output’s label from everything the process has read. Therefore:

> **SELinux enforces declared labels; it does not classify language.**

This limitation should shape the architecture. Sessions should be constructed at the highest classification of anything they can read, and outputs should inherit that level by default. Downgrading requires a separately trusted declassification workflow—possibly human review, deterministic transformation, policy-approved redaction, or a specially constrained and audited declassifier domain. An ordinary agent must not be trusted to label its own answer downward. SELinux policy is expressive enough to include exceptions that violate a simple Bell–LaPadula interpretation, so policy conformance should be tested and, for high-assurance use, analyzed rather than inferred from the presence of MLS labels alone.[^selinux-analysis]

### 3.1 One process cannot carry token-level labels

Once public and secret text coexist in one context window, the kernel cannot label individual tokens or regions of ordinary process memory independently. The process must be treated at the joined classification. Where finer separation matters, use distinct sessions or processes with controlled communication:

```text
Public session  ── public data
       │
       │ controlled release/import
       ▼
Secret session  ── secret data
```

This is a feature of taking information boundaries seriously, not merely an implementation inconvenience.

### 3.2 Remote inference is an egress operation

Local MAC ceases to control information once bytes are transmitted to a remote provider. Model selection and network access therefore belong to the security context:

```text
Public        → approved external or internal endpoint
Internal      → enterprise-contracted tenant endpoint
Confidential  → private endpoint with suitable controls
Secret        → local or specifically accredited endpoint
```

The table above is illustrative organizational policy, not a universal consequence of a classification level; accreditation, jurisdiction, contracts, and the organization’s threat model determine the actual mapping. Network namespaces, firewall/eBPF policy, proxy enforcement, SELinux network policy, Landlock TCP bind/connect restrictions on supporting kernel ABIs, and short-lived workload credentials can constrain local egress. Carrying MLS labels across hosts additionally requires an explicitly configured labeled-network mechanism such as NetLabel/CIPSO or an application gateway that preserves and enforces equivalent metadata; ordinary Internet TCP does not transport a trustworthy SELinux label. Landlock network rules strengthen inherited, unprivileged self-restriction but do not resolve DNS, redirects, proxies, TLS or tenant identity, non-TCP channels, or remote authorization; they complement rather than replace a gateway. Contractual and remote-system controls must govern what happens after transmission. A paper about classified agents that ignores model egress would be incomplete.

A shared local inference server should be treated as a service boundary too. GPU device access, accelerator memory, batching across security domains, prompt and KV caches, adapters, logs, crash dumps, and model-server credentials can all cross session boundaries. High-assurance deployments may require label-aware gateways, separate server instances, hardware partitioning, or dedicated accelerators rather than direct access to shared device nodes.

The model is therefore not security-neutral even though the session identity survives its replacement. The model, endpoint, provider tenant, adapters or fine-tuned weights (which are themselves information-bearing objects), retention mode, and inference pool form the session's **approved execution binding**. Changing any element is an admission event and a change to the trusted computing base: it requires a compatibility decision against the session's current `I` and `T`, an audit record, and, where classification or provider controls differ, reauthorization of the session.

#### Gateway-only egress is a topology, not a namespace flag

A network namespace creates a separate network stack; by itself it denies nothing. "Gateway-only egress" is a property of a specified graph and its enforcement points, all of which must be present for Invariant 10 to hold:

```text
session netns
  └── single veth (no other interfaces, no default route except via veth)
        └── host-side policy (nftables or eBPF on the veth):
              permit  → gateway address:port only
              permit  → resolver operated by the constructor, if DNS is needed at all
              drop    → everything else, including host, bridge, link-local,
                        metadata, and other sessions' veths
                └── gateway: narrowly typed operation API
```

Additional conditions:

- the session holds no `CAP_NET_RAW` or `CAP_NET_ADMIN`; seccomp forbids unneeded socket families (`AF_PACKET`, `AF_VSOCK`, `AF_NETLINK` beyond what the runtime needs, and raw sockets);
- the inherited descriptor set contains no pre-opened sockets or connections;
- no host Unix-domain sockets, loopback services, container-runtime sockets, or local proxies are mounted or reachable inside the session's world;
- the gateway is not a generic HTTP or CONNECT proxy. It exposes named operations with typed arguments, authorizes destination, method, body semantics, tenant, and response size per operation, authenticates its upstream TLS peer, and binds a signed per-session identity and audience to every request;
- Landlock TCP `bind`/`connect` rules and hostname allowlists are defense in depth, not the boundary: they cover neither UDP/QUIC, vsock, existing descriptors, DNS behavior, nor TLS identity.

Bypass testing must therefore cover UDP and QUIC, vsock, IPv6 and link-local addresses, cloud metadata addresses, DNS resolver behavior, pre-opened connections, host service sockets, and attempts to use the gateway itself as a tunnel or SSRF oracle. A local broker reduces token theft but not malicious in-session use: a hostile process in the session can ask the broker to perform any operation the session is authorized for, so broker operations must be narrowly typed and bound to the session's declared task.

### 3.3 Durable memory is partitioned information

A durable agent home cannot safely be one undifferentiated memory store. If a Finance agent accumulates summaries from Redwood, Bluebird, and acquisition sessions in one database, that store acquires the join of those domains and cannot be admitted to a lower session merely because the same agent owns it.

Durable state should therefore be partitioned by compatible confidentiality and integrity domains:

```text
finance-agent state
├── public or general memory
├── finance memory
├── finance + redwood memory
├── finance + bluebird memory
├── trusted configuration and approved skills
├── untrusted learned observations
└── sealed session archives
```

A session constructor exposes only partitions compatible with the session's current domain. Import from a lower confidentiality domain into a higher one may be allowed, subject to integrity checks; export downward requires trusted release. Untrusted observations and model-generated summaries must not silently modify trusted configuration, approved skills, or high-integrity organizational knowledge. Resume is therefore both reauthorization and selective import, not restoration of an agent's entire historical context.

Partition compatibility does not solve concurrent mutation. Two sessions authorized for Finance+Redwood could lose updates, expose torn state, or poison each other's future context. Durable memory should therefore favor immutable versions or append-only records with writer session, source labels, provenance, and generation identifiers. A running session should normally import a stable snapshot; it appends proposed updates rather than mutating approved memory in place. Compare-and-swap, transactional storage, or explicit merge prevents accidental conflicts, while validation or review—not locking alone—governs promotion into trusted memory.

#### Structured versus semantic memory

"Validation" is well-defined for some memory and undefined for the rest. The design therefore distinguishes three classes of durable state with different promotion semantics:

1. **Structured, mechanically validated state** — configuration, skill manifests, schemas, index entries, test results, reproducible transformation outputs. Promotion is governed by a predicate a validator can evaluate: schema conformance, test pass, reproducible build, signature by an authorized tool. This class may be promoted automatically.
2. **Untrusted semantic observations** — natural-language summaries, learned "facts," a model's interpretation of a document, retrieval-memory entries. No validator can prove such an entry is free of confidential inference, injected instruction, or hallucination. This class is retained append-only, at the confidentiality label of its session and the integrity label *untrusted*, with full provenance. Later sessions may read it as low-integrity input; it never becomes trusted configuration or instruction by review alone.
3. **Human-reviewed releases** — the narrow set of semantic outputs an organization chooses to promote or release through review. This class carries explicit capacity, error-rate, aggregation, and audit assumptions (Section 3.5) and must be measured, not assumed to scale.

A promotion predicate must name its validator identity, the evidence it consumed, the freshness window, and the rollback path. The architecture does not claim that useful cognitive memory can be promoted to trusted state at scale; it claims that the three classes must not be confused, and that the second class must remain visibly untrusted.

### 3.4 Integrity and trusted promotion

Confidentiality control alone does not address the dominant prompt-injection failure mode: an authorized agent reads hostile material and corrupts an artifact it is allowed to write. A reference design should separate:

- immutable or low-integrity imported material;
- untrusted parsing and retrieval processes;
- model-generated proposals and patches in staging areas;
- validated artifacts that passed deterministic checks;
- reviewed or approved outputs promoted into trusted repositories;
- protected runtime configuration, policy, credentials, tools, and durable memory.

The default workflow is **stage, validate, and promote**, not direct mutation of trusted objects. Integrity does not become high merely because a model asserts confidence. Promotion may require tests, schema validation, reproducible transformation, independent review, branch protection, or a constrained trusted service. Reading untrusted data need not make all future work useless if risky parsing and imports are separated into narrower processes and only structured results cross an integrity boundary.

### 3.5 Declassification is a capacity-constrained system

Conservative labeling tends to raise sessions and outputs toward the join of everything they can observe. This is secure but can produce label creep: too many useful outputs require downgrade, review queues become bottlenecks, and reviewers may begin to rubber-stamp releases. Declassification is therefore not a clause at the end of an MLS design; it is an operational subsystem whose economics may determine whether the design is viable.

A production design should minimize unnecessary joins through narrow sessions and compartmented memory, define deterministic releases where possible, preserve source provenance for reviewers, separate requesters from approvers when appropriate, and measure review latency, rejection and correction rates, reviewer agreement, automated-release coverage, accidental overclassification, and post-release incidents. If useful work cannot exit a domain at sustainable cost, the policy is operationally unsuccessful even if it is formally conservative.

Release decisions also cannot always be evaluated independently. A sequence of individually low-risk outputs may jointly reveal protected information through aggregation, differencing, or adaptive querying—an especially important risk when agents can automate many small requests. A declassifier should therefore consider cumulative release history across the session, agent principal, initiator, project, recipient, and related queries; unusual rate, volume, repetition, or adaptation is itself a policy signal.

---

## 4. Delegation

Agent delegation should be monotonic with respect to authority:

```text
A_child ⊆ A_parent_session
K_child ⊆ K_parent_delegable
V_child ⊆ V_parent
```

where `K` denotes credentials/scopes and `V` the visible world. A child may receive a narrower filesystem view, fewer groups, no network, fewer credentials, tighter resource limits, and a restricted syscall set. It must not obtain additional authority merely because its parent can request another agent.

Information labels obey different rules:

```text
I_child ⪰ join(confidentiality labels of every prompt, file, descriptor,
               memory object, message, and service result admitted to the child)

T_child must not exceed the integrity justified by the provenance and
validation state of every admitted input
```

If integrity is represented as a lattice in which higher means more trusted, the second rule can be expressed as `T_child ⪯ meet(T_input1, …, T_inputN)`. Promotion above that bound requires the trusted validation or review transition described in Section 3.4.

A parent contaminated by Finance+Redwood cannot create a Finance-only child, pass arbitrary parent-generated text to it, and treat the child's output as Finance-only. Likewise, a fresh child does not cleanse low-integrity instructions generated from hostile parent inputs. Narrower authority limits new access; it is neither declassification nor integrity restoration. A child can run at a lower confidentiality or higher integrity domain only when its inputs are independently known to be compatible or cross an appropriate trusted release, validation, or import boundary.

Unix inheritance gives a useful default: children inherit process credentials, file descriptors, namespaces, `no_new_privs`, and many restrictions. The `no_new_privs` bit is inherited across `fork`, `clone`, and `execve` and cannot be unset; it prevents `execve` from granting privilege that would not otherwise have been available.[^nnp] Landlock rules restrict the enforcing thread and future children and are designed to stack with other access controls.[^landlock]

This monotonicity is not automatic for every Unix property. A process retaining `CAP_SETUID`, `CAP_SETGID`, broad capabilities, dangerous set-ID executables, or an allowed SELinux domain transition may create a child with different or broader effective authority. The session policy must prohibit upward transitions, remove launch capabilities and privileged entrypoints, and constrain cgroup migration before delegation is exposed.

Two caveats matter. First, already-open file descriptors are capabilities in practice: they may retain access even after path-based policy or membership changes, so delegation and revocation must inventory and close them. Second, `no_new_privs` can interact with LSM domain transitions; it should be applied only after the intended SELinux transition is established and tested.

Inheritance alone is not least privilege: a child initially inherits too much. A delegation launcher should close unnecessary descriptors, narrow mounts and network access, drop groups and capabilities, issue narrower credentials, apply additional Landlock/seccomp restrictions, and enter an authorized SELinux domain. Expansion of authority requires an external trusted decision, not cooperation by the parent model.

A delegated “agent” need not always receive another durable principal. A temporary helper with no independent organizational authority can remain a child execution inside the parent session, or a narrower child session with its own execution identity under the parent agent's durable principal. A new durable principal is justified when the child has independent policy, ownership, credentials, lifecycle, or audit responsibility.

### 4.1 Sessions sharing one durable principal

A UID distinguishes principals, not sessions belonging to the same principal. Two concurrent sessions running directly under one UID can inspect or influence each other through `/proc`, signals, ptrace, `process_vm_readv`, shared home state, Unix and abstract sockets, temporary files, terminals, IPC, and inherited descriptors, because same-UID access passes ordinary DAC and signal permission checks. SID and cgroup membership aid grouping and accounting but are not authorization boundaries. PID namespaces hide identifiers but do not change authorization for a process that obtains a usable PID or pidfd; `hidepid` and Yama `ptrace_scope` are configurable mitigations that distinguish UIDs, not sessions.

The reference design therefore requires **distinct execution identities per session** as the primary boundary (Section 1.2, ADR-0001). Mount, PID, and IPC namespaces, private procfs and runtime directories, private sockets and PTYs, partitioned storage, and explicit descriptor passing are necessary supporting controls, but none substitutes for the identity split. A profile may instead use a rigorously allocated per-session SELinux type with a scalable allocator, which is a compartmented-profile mechanism rather than a baseline one. The durable agent identity owns state; the per-session execution identity accesses only the partitions activated for that session through per-session grants.

Conformance testing for Invariant 17 must attempt, from one session against a concurrent sibling: `/proc/<hostpid>` access, `kill` and `pidfd_send_signal`, `ptrace` and `process_vm_*`, `/run` and `/tmp` paths, pathname and abstract Unix sockets, shared supplementary-group permissions on durable partitions, broker socket reuse, and every descriptor inherited at launch.

### 4.2 Peer and cross-principal communication

Delegation is only one communication topology. Sibling sessions and agents belonging to different principals also exchange messages, artifacts, queue entries, RPC calls, repository changes, and file descriptors. Every such edge is an authorization decision, an information admission, a possible confidentiality join, an integrity/provenance transition, and an auditable causal link.

A receiver may import a message or artifact only when it is authorized for the named channel, its confidentiality domain dominates the message label, and the message's provenance and integrity state are preserved without silent promotion. If those conditions are not met, the receiver must reject the input, enter or create a compatible session, or use a trusted release/validation boundary. The audit record should bind sender session, receiver session, channel, object digest or message identifier, labels, and policy decision. Pipes and Unix sockets are useful transports, not exceptions to the rule.

### 4.3 Human attachment and interactive control

Attaching to a PTY is a bidirectional information-flow and authority event, not merely a user-interface feature. Policy must separately govern who may observe output, inject input, approve an operation, interrupt work, or take interactive control. Each of these is a distinct relationship to the session: an **observer** is a receiver on an admission edge and must be cleared for `I_session`; an **injector** acts under delegated authority and every injected command is attributed to the injecting human, never to the agent; an **approver** contributes an approval object without becoming a receiver; a **controller** temporarily becomes a co-initiator, which changes the derivation inputs and must be re-derived and recorded. Terminal escape sequences, concurrent attachments, transcript classification, and the possibility that attachment reveals everything visible to the session must be addressed. Non-interactive sessions should avoid PTYs when they are unnecessary.

A trusted control plane that receives session output (a Web UI, orchestration service, or collaboration server) is itself a receiver on an admission edge. Its confidentiality and integrity state, and the state of every human it displays output to, are governed by the same admission relation; the control plane cannot be exempt merely because it is trusted for other purposes.

---

## 5. Attribution and audit

A meaningful audit chain must preserve multiple identities:

```text
initiating human/service
    → durable agent principal
        → session and purpose
            → process
                → syscall / external action
                    → labeled object or service
```

Linux Audit’s login UID is useful because it is intended to track the account that originally gained access and is inherited by child processes.[^loginuid] Setting it requires `CAP_AUDIT_CONTROL`; because `loginuid` is set once and inherited, a long-lived launcher must set it in an unset child immediately before `exec`, and must handle systems that make it immutable. The effective UID identifies the per-session execution identity after the credential transition, and a mapping from execution identity to durable principal and session is part of the launch record. PID/PPID identify the execution, but PIDs are reused; a PID namespace plus process start time or a pidfd is needed to make process identity unambiguous. SELinux source and target contexts identify mandatory domains. A cgroup or systemd scope supplies a stable session grouping even as individual processes come and go, but it is not a portable audit key.

Kernel audit is **correlation evidence, not causal attribution**. It records numeric identifiers and event-time observations; it does not encode agent, session, purpose, or remote delegation, its delivery is lossy under load, and a single authorized TLS or database connection can carry many semantic operations it cannot distinguish. The primary attribution mechanism is therefore the signed launch record plus a **session trace identity** propagated through every gateway operation and logged by the remote service; kernel audit corroborates and fills in local effects.

No single existing field expresses the entire agent provenance chain. The session constructor should emit a signed or append-only launch record binding:

- globally durable agent identity;
- per-session execution identity and SELinux identity/context;
- initiator and authentication event;
- session ID, trace identity, and cgroup/unit;
- purpose and approvals;
- policy version and manifest digest;
- mounted resources and credential issuances;
- approved execution binding (model, endpoint, tenant, adapters, retention mode);
- start/end times and termination reason.

The launch record needs a stated trust anchor: who signs it, how its append-only integrity is protected, which clock it uses, how corrections are authorized, and how long it is retained.

Kernel audit and service-side logs can then be correlated with this record. Tamper-evident application records, such as Orkia’s signed SEAL chains, are complementary to kernel audit rather than substitutes for it.

The attribution claim is scoped to a **defined effect ontology**: local object creation and modification within the session's world, process lifecycle events, and gateway-mediated remote operations. Kernel audit cannot reveal the semantic operation performed inside an already-authorized database connection, SaaS API, model server, or multiplexed gateway; those services must preserve the session trace identity in their own authorization and audit records. Audit availability is also a security property: deployments must specify behavior when buffers fill, collectors fail, clocks diverge, or event volume exceeds capacity, and must expose loss counters. The profile that claims attribution should stop or quarantine sessions rather than continue without required evidence.

### 5.1 Local and remote enforcement boundary

| Effect | Local enforcement | Remote or organizational enforcement |
|---|---|---|
| Local file access | Namespace, DAC, SELinux, Landlock | Label and policy administration |
| Process and IPC access | UID, SELinux, namespaces, ptrace and descriptor policy | Session policy |
| Model invocation | Network reachability and credential confinement | Gateway identity, model/tenant selection, retention, spend and audit |
| Database query | Endpoint and credential confinement | Database authorization, row/column policy and query audit |
| Git or CI write | Credential and endpoint confinement | Branch protection, review, CI and repository audit |
| SaaS action | Endpoint and token confinement | Service authorization and service-side audit |
| Classification release | Prevention of ordinary write-down | Trusted transformation, review and approval |

Linux therefore protects more than the local filesystem, but it does not replace service-side reference monitors. The end-to-end boundary is composed from host controls, gateways, workload identity, remote authorization, and correlated audit.

---

## 6. Related work

The proposal is assembled from established OS security ideas and several emerging Unix-native agent projects. Its novelty claim should be narrow: not invention of the primitives, but their composition around a particular ontology and enterprise information-security model. Most agent-project descriptions below rely on preprints or project self-documentation rather than independent security evaluations; they describe the projects’ stated designs as of this draft date.

### 6.1 Quine: agent as POSIX process

Quine argues that current frameworks rebuild isolation, scheduling, communication, and lifecycle above services already provided by operating systems. It maps agent identity to a PID, interface to standard streams and exit status, state to memory/environment/filesystem, and lifecycle to `fork`/`exec`/`exit`.[^quine-paper] Its implementation describes permissions, signals, pipes, files, resource limits, process groups, and job control as the agent’s native substrate.[^quine-repo]

Quine is the closest conceptual predecessor, but the ontologies differ:

| Dimension | Quine | This proposal |
|---|---|---|
| Agent | POSIX process | Durable security principal and context domain |
| Live execution identity | PID | PID |
| Durable local security identity | Not the central mapping | UID + SELinux identity |
| Session | Process invocation/runtime | Task-scoped shell/process world |
| Classification | Not a primary model | SELinux MLS/MCS plus output rules |
| Main concern | Cognitive execution and composition | Organizational identity, authority, and information governance |

The approaches compose naturally: Quine can be a cognitive runtime inside a governed session. In short, **Quine Unix-nativeizes agent execution; this proposal Unix-nativeizes agent identity and information governance.**

### 6.2 Orkia: agents as governed shell jobs

Orkia is an interactive shell built on a POSIX shell engine that hosts Claude Code, Codex, and Gemini sessions as persistent jobs in isolated PTYs. It exposes dispatch, attach, follow-up, kill, pipelines, and a signed tamper-evident audit chain.[^orkia] This strongly supports “sessions as shells” and Unix job semantics.

Orkia’s current public framing treats named agent sessions as governed jobs and adds a capability/audit layer. The proposal here differs by making the durable agent a kernel security principal and placing MLS/MCS classification, initiator-agent intersection, and session output labeling at the center.

### 6.3 AaaU and agent-as-unix-user

AaaU describes a PTY bridge in which an agent runs as a dedicated system user, using `setuid`, a per-user home, cgroups through systemd, Unix-socket authorization, and JSON session audit.[^aaau] The `agent-as-unix-user` package similarly creates a Unix user and group, configures setgid/default ACLs, scrubs the environment, drops inherited groups, and provides explicitly shared read-only or read-write directories.[^a4u2]

These projects are direct demonstrations of the local principal idea and likely the closest implementations to the phrase “agent as user.” They focus primarily on coding-agent sandboxing on a single Linux system. They do not yet constitute a multi-user enterprise architecture for task-scoped authority, MLS/MCS classification, information-flow constraints, remote workload identity, or trusted declassification.

### 6.4 agentsh: enforcement below tools

agentsh positions itself as an execution-layer security shell. It intercepts or constrains file, network, process, signal, database, and outbound LLM API activity; applies policy to subprocess trees; and emits structured audit events. Its Linux modes combine mechanisms including seccomp, eBPF, FUSE, Landlock, and capability dropping.[^agentsh]

This shares the principle that controls should be enforced below prompt and tool abstractions. Its policy/interception approach is complementary to principal- and label-based enforcement. The distinction is that agentsh does not by itself define the durable organizational principal and MLS session ontology proposed here.

### 6.5 Agent OS: filesystem-native orchestration

Agent OS exposes agents, processes, conversations, memory, channels, tools, and external services through a FUSE filesystem and CLI rather than framework-specific APIs. It states that capabilities “intersect down,” treats cost as a resource, and makes conversation state searchable and versionable.[^agentos]

Its thesis—that agents already understand files and shell conventions—is aligned with the position paper. However, it implements an agent-oriented virtual filesystem and daemon that rematerialize OS-like abstractions. This proposal asks which of those abstractions can instead be ordinary host objects protected by ordinary credentials and MAC policy. FUSE may remain useful as a gateway to remote services, but should not be confused with the underlying principal or classification mechanism.

### 6.6 Agor and the limits of projecting RBAC into POSIX groups

Agor is especially instructive because its current documentation separates application authorization from execution isolation. Its earlier Unix-user/group modes were removed; current sandbox mode derives a fail-closed Bubblewrap filesystem view from tenant and branch authorization, while delegated mode leaves isolation to an external substrate.[^agor]

This is a warning against an overly literal design. Enterprise RBAC graphs should not be lossily encoded entirely as inode owner/group bits, and host accounts/groups should not require continuous privileged reconciliation with application state. The lesson is not that Unix enforcement is unsuitable; it is that **organizational policy and host enforcement are separate layers**. Stable agent principals and coarse memberships can map to UIDs/GIDs, while a session constructor realizes dynamic grants through labels, namespaces, mounts, file descriptors, and short-lived credentials.

### 6.7 Workload identity and delegated authorization

SPIFFE, a CNCF-hosted standard with SPIRE as a reference implementation, defines platform-neutral workload identities and short-lived identity documents for authenticating software workloads across heterogeneous infrastructure.[^spiffe] OAuth 2.0 Token Exchange standardizes obtaining tokens for impersonation and delegation scenarios.[^rfc8693] These are relevant once an agent session calls APIs that do not understand Unix credentials.

A local UID is not a network identity. A production system should bind the local session provenance to an attested workload identity and exchange it for narrow, short-lived service credentials. This extends the same principle beyond the host: the session should not inherit a human’s broad bearer token or a daemon’s static secret.

Authenticated-delegation research similarly argues that a service must distinguish the human principal, the delegated agent, and limitations on the delegated scope.[^delegation] This report specifies the complementary local execution boundary: remote tokens express what a service should accept; the UID/session/MAC context constrains what the local process can reach and do.

### 6.8 Agent-specific access-control research

Recent access-control research is converging on the observation that static tool allowlists are insufficient. The Agent Access Control vision frames the problem as governance of dynamic information flow rather than only binary resource access.[^aac] SEAgent proposes an application-layer mandatory-access-control framework for LLM agents, including labeled subjects, a policy database, decision engine, and protected memory, with the aim of constraining privilege escalation in single- and multi-agent systems.[^seagent]

These efforts overlap with the position paper’s concern for context, delegation, and derived information. Their mechanisms are principally agent-runtime policy abstractions. The proposal here explores a different enforcement placement: encode coarse, auditable information domains in ordinary OS principals and SELinux labels, then reserve semantic reasoning for policy selection and trusted release. The approaches may be layered; kernel MAC cannot replace semantic governance, while an application decision engine should not be the only barrier protecting filesystem objects and subprocesses.

### 6.9 Decentralized information-flow control

Decentralized information-flow-control (DIFC) systems are the closest academic ancestors of this proposal's labels, joins, integrity provenance, partitioned state, and controlled release. Myers and Liskov's decentralized label model gives owners authority over confidentiality and declassification; Jif applies that model through language-level static checking.[^dlm-jif] Asbestos and HiStar explore kernel-enforced labels, information-flow control, and small trusted components in new operating systems.[^asbestos][^histar] Flume is a particularly close neighbor: it adds process-level DIFC to familiar operating-system abstractions including processes, pipes, sockets, and file descriptors through a user-level reference monitor on Linux.[^flume]

The position paper does not claim to invent label joining, process-level information flow, integrity labels, or controlled declassification. Its narrower contribution is to compose related ideas around a durable organizational agent principal, an initiator- and task-bound cognitive session, partitioned agent memory, model and service egress, delegated workload identity, and attributable execution. It initially favors controls deployable on conventional Linux, while recognizing that ordinary SELinux MLS/MCS provides less dynamic label ownership and propagation than purpose-built DIFC systems. DIFC also makes clear why declassification authority and every message or descriptor crossing a boundary must be explicit.

The strongest objection to this proposal is that it repackages DIFC, capability, and MLS ideas in agent terminology while deferring the hard parts—dynamic flow mediation and economically viable declassification. The honest response is that this report does not claim to solve those parts. It claims that a durable-principal/task-session ontology provides a disciplined place to bind organizational policy, local enforcement, remote identity, and accountability, and that it exposes the remaining semantic information-flow problem rather than hiding it inside an agent framework. The Unix-governed profile is explicitly a non-IFC isolation and attribution profile; the compartmented and multilevel profiles inherit DIFC's obligations and the release-economics condition.

The following table positions the proposal against the traditions it draws on:

| Property | DIFC (Flume, HiStar, Asbestos) | SELinux MLS/MCS | Capability / workload identity (SPIFFE, scoped tokens) | This proposal |
|---|---|---|---|---|
| Label ownership | Per-principal, decentralized | Central policy | None (authority, not labels) | Central policy; principal owns partitions |
| Dynamic propagation | Yes, on every mediated IPC | No; declared per object/domain | No | Only on mediated edges per profile |
| Persistent labeled state | Labeled files/objects | Labeled inodes | No | Partitioned memory with provenance |
| Authority delegation | Capabilities/privileges | Domain transitions | Token scopes | Derivation relation; monotonic sessions |
| Declassification authority | Label owner | Trusted subject | N/A | Named trusted release service |
| Remote/distributed services | Mostly single host | Labeled networking (rare) | Yes, core strength | Gateway + trace identity |
| Model/cache state | Not modeled | Not modeled | Not modeled | Execution binding as admission event |
| Durable organizational identity | Process/principal, not organizational | User/role | Workload, not agent | Agent principal + session |
| Attribution chain | Process-level | Process-level | Service-side | Launch record + trace identity + audit |

The confidentiality rules descend from Bell–LaPadula and Denning's lattice model; the integrity rules from Biba; and stage–validate–promote from Clark–Wilson's certified transformation procedures over constrained data items.[^blp][^denning][^biba][^clark-wilson] Provenance and supply-chain integrity work (in-toto, SLSA) addresses the same promotion problem for build artifacts and is directly reusable for the structured-memory class.[^slsa]

### 6.10 SELinux MLS and historical multilevel security

SELinux MLS is not agent-specific; that is precisely its value. It is an existing mandatory-control substrate developed for sensitivity-based information separation. The proposal applies it to cognitive sessions whose in-memory context and outputs become part of the classified domain. SELinux type enforcement also permits separate domains for launchers, sessions, model gateways, audit components, and trusted declassifiers.

The strongest claim warranted here is feasibility of coarse, declared, kernel-enforced information domains—not automatic semantic taint tracking or a proof of noninterference.

---

## 7. Security invariants

A reference design should make the following invariants testable. Each invariant is stated with:

- **Profiles** — the deployment profiles (Section 9.1) in which it is claimed: **U** Unix-governed, **C** compartmented, **M** multilevel, **W** strong workload isolation. An invariant not listed for a profile is **not applicable** to that profile and must not appear as a pass in its evidence table.
- **Layer** — the primary enforcement layer: **K** kernel after launch, **C** trusted constructor, gateway, or other named trusted service, **P** organizational policy or administration.
- **Class** — whether the mechanism **prevents** the violation, **detects** it after the fact, or is an administrative **assumption** that must hold for other invariants to be meaningful.

Every invariant also has configuration preconditions and excluded interfaces; the conformance suite must state both for each test. A K tag means the kernel enforces an already-correct policy choice; it never means the kernel would detect a wrong choice.

| # | Invariant | Profiles | Layer | Class | Preconditions and exclusions |
|---|---|---|---|---|---|
| 1 | **Durable identity.** Every session is bound to exactly one durable agent principal. | U C M W | C | prevents | Principal registry authenticated and available. |
| 2 | **Explicit initiator.** Every session records the authenticated initiator(s), approvers, and, for scheduled sessions, the accountable owner. | U C M W | C | prevents | Initiator authentication is trusted. |
| 3 | **Bounded derivation.** Activated authority is the output of the derivation relation and never exceeds agent or policy authority. | U C M W | P C | prevents | Correctness of the derivation is a policy property; the kernel enforces only the installed result. |
| 4 | **Clearance ceiling.** Policy selects an authorized MAC context; the installed policy prevents labels and transitions it does not allow. | C M | P C K | prevents | Requires an installed, analyzed MLS/MCS policy; not claimed in U. |
| 5 | **No ambient categories.** Authorization for a category does not activate it in every session. | C M | P C | prevents | Category allocator and policy correct. |
| 6 | **Monotonic delegation.** The launcher narrows authority; capability bounds, `no_new_privs`, Landlock, namespaces, descriptor discipline, and MAC transition rules prevent specified re-expansion. | U C M W | C K | prevents | Excludes: already-open descriptors not inventoried; set-ID binaries or permitted transitions left in the session's world. |
| 7 | **Fail-closed construction.** If a required identity, label, namespace, mount, limit, network path, credential restriction, or audit binding cannot be established, the session does not start and no partial effect survives. | U C M W | C | prevents | Requires rollback for every irreversible step (Section 2.1). |
| 8 | **Classified persistence.** Objects created through supported paths receive at least the session's label. | C M | C K | prevents | Excludes: archives, Git objects, object stores, backups, hard links, and any path not covered by a type transition or gateway. |
| 9 | **No ordinary downgrade.** The session lacks relabel and write-down permission; only a separate trusted declassifier may release lower-labeled output. | M | K | prevents | Requires a complete MLS policy with all trusted-subject exceptions analyzed; excludes covert channels. |
| 10 | **Gateway-only egress.** The topology in Section 3.2 is present, so a session reaches only its gateway and approved resolver. | U C M W | C K | prevents | Excludes: side channels; misuse of an authorized gateway operation; requires the full topology, not a namespace alone. |
| 11 | **Credential confinement.** Issuers create minimally scoped, short-lived credentials bound to the session and revoke them at termination. | U C M W | C P | prevents / **assumption** | Non-exportability holds only for proof-of-possession or brokered credentials; bearer tokens are copyable within the session. |
| 12 | **Complete descendant control.** All descendants remain in the supervised cgroup and PID namespace; termination kills and reaps them before credential revocation. | U C M W | C K | prevents | Requires no writable cgroup or system-manager path in the session; D-state tasks may delay, not escape, termination. |
| 13 | **Attribution of mediated effects.** For the defined effect ontology (Section 5), the launch record, trace identity, and corroborating kernel/service events identify initiator, agent, session, and process. | U C M W | C K | **detects** | Excludes: semantic operations inside an authorized connection not carrying the trace identity; events lost under audit overload unless the profile fails closed. |
| 14 | **Policy provenance.** The launch record identifies the policy, catalogue, and effective-manifest versions used. | U C M W | C P | detects | Launch-record trust anchor stated. |
| 15 | **Launch privilege disposal.** The launch helper drops every launch-only capability before untrusted code executes. | U C M W | C | prevents | Post-launch lifecycle, network, and audit services remain privileged and are separately authorized (Section 2.1). |
| 16 | **Contamination-safe communication.** On every **mediated** edge, admission follows the admission relation; narrowing authority or using a fresh process never lowers `I` or raises `T`. | C M | C K | prevents | Applies only to edges in the profile's coverage inventory; unmediated edges carry no claim. |
| 17 | **Same-principal session isolation.** Concurrent sessions of one durable principal cannot inspect, signal, trace, attach to, or access each other's private state and credentials unless an explicit channel is authorized. | U C M W | C K | prevents | Requires distinct execution identities (or a per-session MAC type in C/M); excludes shared durable partitions deliberately granted to both. |
| 18 | **Partitioned durable memory.** A session imports only partitions compatible with its `I` and `T`; ownership does not grant ambient access to all memory. | C M | C K P | prevents | Requires a partitioned store and per-session grants; not claimed in U beyond per-session workspace separation. |
| 19 | **Integrity promotion.** Untrusted inputs and model-generated artifacts cannot directly replace trusted configuration, approved memory, protected branches, or production objects; promotion crosses a named validator. | U (protected-object subset) C M | C K P | prevents | In U the claim is limited to objects behind a gateway or branch-protection boundary; memory promotion is C/M. |
| 20 | **Bounded external resources.** Gateways and supervisors enforce token, spend, rate, fan-out, storage, accelerator, and audit-capacity budgets. | U C M W | C P | prevents | Budgets not visible to cgroups require gateway enforcement. |
| 21 | **Explicit lifecycle and revocation.** Policy defines active-session behavior for initiator disablement, authority revocation, reclassification, approval expiry, and control-plane failure. | U C M W | C P | **assumption** | Defined and tested, but behavior is a policy choice. |
| 22 | **Execution-binding control.** Changing the model, endpoint, tenant, adapters, retention mode, or inference pool of a session is a policy-controlled, audited event with a compatibility decision. | U C M W | C P | prevents | Requires the gateway to be the only path to inference (Invariant 10). |

Evidence tables must distinguish five results for each applicable invariant: **enforced and passed**, **enforced but failed**, **detected only**, **assumption (documented and accepted)**, and **not applicable to this profile**.

---

## 8. Threat model

The in-scope adversary controls untrusted documents, prompts, repositories, packages, tool output, and potentially the cognitive runtime or a child process. It may issue arbitrary syscalls available to the session and seek unauthorized reads, writes, privilege expansion, persistence, audit evasion, or overt exfiltration. It does not begin with kernel, policy-administrator, session-constructor, model-gateway, or physical-host control.

A **malicious or careless authorized initiator** is also in scope for a defined subset of properties: a non-administrator human who uses an agent to perform actions they could not perform directly, to launder attribution, or to move information between domains they are separately cleared for. The derivation relation (Invariant 3), attribution (Invariant 13), and admission rules on mediated edges bound what such an initiator can achieve; the architecture does not prevent an initiator from misusing authority they legitimately hold.

The system should assume:

- the model is non-deterministic and may be manipulated by prompt injection;
- inputs, repositories, web pages, tool output, and packages may be adversarial;
- model-generated commands and code are untrusted;
- tool plugins may be buggy or malicious;
- a session may attempt to inspect credentials, escape its world, signal other processes, disable logging, or exfiltrate information;
- the organizational policy service, session constructor and post-launch lifecycle services, kernel, MAC policy, identity infrastructure, credential issuer, gateways, validators and declassifiers, launch-record store, and approved model endpoints form the trusted computing base. This is not small; the design goal is that it be **enumerable and separately reviewable**, not that it vanish.

The initial scope does not claim to prevent:

- kernel vulnerabilities or physical compromise;
- hardware side channels and covert timing or storage channels through scheduling, shared caches, memory pressure, cgroup statistics, filesystem metadata, or other shared state; the primary claim concerns overt flows through kernel-mediated labeled objects and configured gateways;
- flows across **unmediated edges** in a given profile (see the coverage inventory in the front matter and Section 9.1);
- semantic leakage through an approved output that is mislabeled by a trusted declassifier;
- malicious administrators with sufficient host and policy authority;
- misuse by an authorized initiator of authority they legitimately hold;
- use of information already observed before revocation;
- compromise of a remote model provider after authorized transmission;
- proof that multiple individually authorized releases cannot be combined to infer protected information; cumulative monitoring and release policy mitigate rather than eliminate this aggregation risk.

seccomp reduces syscall attack surface but is explicitly not a complete sandbox by itself; filters are per-thread unless installed with `SECCOMP_FILTER_FLAG_TSYNC`, and seccomp user-notification introduces a privileged, TOCTOU-sensitive broker.[^seccomp] Capabilities divide traditional root powers but must be aggressively minimized, especially broad powers such as `CAP_SYS_ADMIN`.[^capabilities] User namespaces are not a free isolation primitive: namespace-scoped "root" can mount and manipulate namespaced objects, so mappings, `setgroups`, and mounts must be tightly controlled or user namespaces avoided. `no_new_privs` blocks privilege gain through `execve`; it does not remove held capabilities, revoke open descriptors, or block every LSM transition. Defense in depth is required.

### 8.1 Availability, resource exhaustion, and supply chain

A hostile session may exhaust more than CPU and memory. Limits and accounting should cover process and descriptor counts, disk bytes and inodes, I/O and network bandwidth, connections, audit volume, child-session fan-out, accelerator memory and time, model tokens, API rate limits, and monetary spend. cgroups enforce only part of this set; model and service gateways must enforce external budgets.

Containment also does not establish executable integrity. The session base filesystem, models and adapters, tools, plugins, dependency installation, compiler caches, startup files, `PATH`, dynamic-loader environment, and writable configuration are supply-chain inputs. High-integrity profiles should use an immutable or measured base, scrub the environment, verify eligible artifacts, isolate build caches, and direct untrusted package installation into disposable staging rather than the trusted runtime or durable agent memory.

---

## 9. Deployment profiles, alternatives, and open questions

### 9.1 Deployment profiles

The ontology does not require every deployment to adopt full SELinux MLS. Four composable profiles clarify the claims:

1. **Unix-governed session:** global agent identity with a per-session execution identity; per-session namespaces, cgroup, private storage, capability removal, `no_new_privs`, optional Landlock/seccomp and host MAC, gateway-only egress topology, brokered or session-bound credentials, and an attributable launch record. This profile is viable with current Linux and systemd machinery. SELinux type enforcement is not the only useful baseline MAC: AppArmor or other supported LSM controls may provide meaningful confinement on distributions where SELinux is impractical, although they are not drop-in implementations of the multilevel profile.

   **Coverage inventory.** This profile mediates: authority activation, execution-world separation, same-principal isolation, descendant control, credential use, gateway operations, and the launch record. It does **not** mediate the content of prompts, tool arguments, intra-session pipes and files, logs, or model responses, and therefore makes **no confidentiality- or integrity-propagation claim**. It is an isolation, authority, and attribution profile, not an information-flow profile. Integrity claims are limited to objects protected behind a gateway or branch-protection boundary (Invariant 19, protected-object subset).
2. **Compartmented session:** the baseline plus MCS or equivalent project/tenant separation, partitioned durable memory, labeled channels, authenticated gateways, category allocation, and controlled import/export. This is the first profile that claims Invariants 16 and 18, and only for the edges in its published coverage inventory. Existing container practice supports category isolation, but does not by itself prove multilevel information-flow control.
3. **Multilevel session:** full MLS sensitivities, declared flow rules, labeled persistence and networking or equivalent gateways, analyzed policy, and trusted declassification. This is a high-assurance research and deployment profile whose staffing, tooling, interoperability, and operating cost must be measured. Its viability depends on the release economics of Section 3.5, which are a conformance condition, not a footnote.
4. **Strong workload isolation:** a container, VM, microVM, or dedicated node around any of the above. This reduces shared-kernel risk but does not by itself label outputs, partition memory, constrain remote services, or provide declassification.

LSM stacking and feature availability vary by kernel and distribution; combinations of SELinux, AppArmor, Landlock, BPF LSM, and other modules must be tested rather than assumed to compose with identical hooks and semantics. The multilevel profile in this report is specifically described using SELinux MLS/MCS or a system shown to provide equivalent declared information-flow properties.

### 9.2 MicroVM-per-session baseline

A serious evaluation must compare the shared-host design with a microVM or VM per session plus per-session cloud IAM. MicroVMs commonly offer a simpler and stronger boundary against cross-workload kernel and device interference, at the cost of startup time, memory, image management, and more complex state and observability. They do not answer where information may flow after it enters the VM. The likely high-assurance architecture may combine both ideas: organizational identity and task policy create a microVM session, while labeled storage, gateways, scoped credentials, partitioned memory, and trusted release preserve the information domain.

The comparison should measure boundary strength, launch latency, steady-state memory, accelerator sharing, policy complexity, credential issuance, storage continuity, audit fidelity, patching, and operator skill requirements rather than assuming either labels or virtualization wins universally.

### 9.3 Principal scale and portability

Creating a host account for every ephemeral helper is unnecessary and may not scale operationally. Durable organizational agents merit stable principals; transient delegations can use processes, sessions, dynamically allocated UIDs, or namespace-local mappings. Fleet-wide identity requires centralized allocation or host-local projections that avoid UID collision. The design is Linux-specific where it depends on SELinux, Landlock, cgroups v2, and Linux Audit.

The durable principal itself needs a lifecycle: `proposed → provisioned → active → suspended → retired → archived/deleted`. Provisioning binds an accountable organizational owner, policy, credential eligibility, and memory custody; rotation updates keys, attestations, endpoint eligibility, and local projections without breaking audit identity. Retirement revokes new sessions and credentials, seals or transfers each labeled memory partition under an explicit custodian and retention policy, and reconciles files and ACLs across the fleet before any host UID is reclaimed. Globally durable principal identifiers and historical audit identities must never be reused even if a host-local numeric UID eventually is.

### 9.4 POSIX groups are not enterprise IAM

One owning group per inode and supplementary-group membership cannot represent every dynamic, multi-tenant access graph. ACL inheritance, shared storage, Git operations, backups, and file creation require careful handling. Groups should represent stable collaboration domains, not the full policy language.

### 9.5 Labels do not understand meaning

MLS controls declared information domains; it does not recognize that an innocuous-looking paragraph reveals a secret. Conservative session labeling and trusted release are necessary. More precise semantic provenance remains a research problem.

### 9.6 Revocation and session lifecycle

Removing a group or credential can prevent future access, but cannot make a running context forget information already read. Strong revocation may require terminating the session and discarding or sealing its state. Durable outputs remain governed by their labels.

The implementation should define an explicit lifecycle such as `requested → authorized → constructing → active → quiescing → terminated → sealed/archive/deleted`. Policy must state what happens when an initiator is disabled, an object is reclassified, a category is revoked, an approval expires, or an audit or policy service fails. Active sessions may be killed, frozen for review, or allowed a bounded grace period, but the maximum exposure window and credential-revocation ordering must be explicit. Mid-session approval should grant a specific action or transition rather than silently becoming indefinite shell authority.

### 9.7 Network and service integration

Databases, SaaS applications, Git hosts, and cloud APIs do not authorize UIDs. A binding among agent identity, initiator, purpose, session, workload attestation, and delegated token scopes is required. The host boundary and service authorization must fail closed together.

### 9.8 SELinux operational complexity

A normal `targeted` SELinux installation is not this architecture. A prototype requires the MLS policy, purpose-built agent and gateway domains, reviewed transitions and constraints, a category-allocation service, labeled storage and networking decisions, and tests for policy exceptions. RHEL exposes a finite category space (commonly 1024 categories), so assigning one permanent category to every tenant-project-session combination does not scale without reuse or structured allocation. Fleet-wide dynamic allocation becomes a privileged distributed service requiring consistency, collision avoidance, lifecycle rules, and auditable reuse.

The ecosystem risk is substantial: full MLS has a much smaller operator, tooling, application-compatibility, and policy-authoring base than mainstream container or VM isolation. Mechanism feasibility must therefore be evaluated separately from policy correctness and ongoing operational viability. The multilevel profile should not be presented as the default until administrators can maintain it without broad exceptions or continual privileged repair.

Label continuity is also a deployment boundary. Containers require coordinated host/container policy and volume labels; archives and Git do not intrinsically preserve SELinux xattrs; NFS, SMB, object stores, backup systems, and cross-host restore have different label behavior; and many enterprise applications are not tested under MLS. A system must either preserve labels end to end or pass through a trusted gateway that reconstructs and validates equivalent metadata. These are major deployment prerequisites, not incidental configuration. A prototype must measure administrative complexity and failure modes as well as runtime overhead.

### 9.9 Session continuity

A stateless resume should rerun authorization and session construction, then import only still-authorized labeled cognitive state. The authorization decision may have expired, group membership may have changed, and resources may have been reclassified. Full process checkpoint/restore is different: CRIU-style restoration requires substantial privilege, namespace fidelity, cgroup reconstruction, and correct SELinux contexts, especially across hosts. This report does not assume secure transparent process migration; resume means a reauthorized launch unless a future checkpoint mechanism proves preservation of every boundary.

### 9.10 “Shell” need not mean interactive Bash

A governed session can be interactive through a PTY, non-interactive through standard streams, or supervised as a service. “Session as shell” means Unix session semantics and composability, not a requirement that every agent expose a human command prompt.

---

## 10. Phased reference implementation and evaluation

The complete evaluation is a research programme, not one minimal prototype. It should be staged so that the baseline ontology can succeed or fail independently of full MLS, shared accelerators, and comparative virtualization work.

### 10.1 Components

```text
agent-principald   organizational principal/session policy resolver
agent-login        privileged, narrow session constructor
agent-shell        ordinary shell or minimal LLM loop
agent-audit        launch-record and Linux Audit correlator
agent-declassify   optional trusted release boundary
```

The implementation can use local configuration rather than enterprise IAM and should avoid claiming production readiness.

### 10.2 Phases

**Phase 1 — Unix-governed execution boundary.** Implement global agent identity and local projection, the constructor, namespaces and cgroups, private per-session state, capability disposal and `no_new_privs`, same-principal isolation, descriptor discipline, cgroup-wide termination, a basic credential/egress gateway, and correlated launch/process audit. This phase tests whether the principal/session/process ontology improves a conventional agent sandbox without depending on MLS.

**Phase 2 — compartments, memory, and remote effects.** Add MCS or equivalent compartments, partitioned and versioned durable memory, contamination-safe delegation and peer messaging, brokered credentials, integrity staging and promotion, policy-change behavior, and external spend/fan-out budgets.

**Phase 3 — multilevel and comparative evaluation.** Add full MLS and analyzed policy, declassification workflows and throughput measurement, cross-host and collaborative-storage tests, shared-inference/accelerator isolation, and a parallel microVM-per-session implementation. This phase compares high-assurance options rather than assuming shared-host MLS is the default.

### 10.3 Demonstration programme

Create:

- humans Alice and Bob;
- durable Finance and Engineering agent principals;
- Finance and Engineering groups;
- Public, Confidential, and Secret levels;
- Redwood and Bluebird categories;
- labeled source, work, transcript, and output directories;
- one approved internal model endpoint and one forbidden public endpoint.

Then demonstrate the following across the phases above; the numbering is a test catalogue, not a claim that every test belongs in the first prototype:

1. Alice can start the Finance agent for Redwood under an authorized context.
2. Bob cannot start or attach to that session without the corresponding policy grant.
3. The Redwood session can read Finance/Redwood data but cannot see Bluebird or Engineering data.
4. Files, transcripts, and logs created by the session receive the correct context through policy/type transitions.
5. The session cannot write Confidential/Redwood material to Public output.
6. A delegated child receives a smaller mount view, no service credential, and no additional categories.
7. Replacing the cognitive runtime does not alter the session execution identity, MAC context, cgroup, or audit identity; changing the model endpoint or tenant is recorded as an execution-binding change.
8. The session cannot contact a model endpoint unauthorized for its classification.
9. Killing the systemd scope/cgroup terminates all descendants and triggers credential revocation.
10. Audit reconstruction shows:

   ```text
   Alice → finance-agent → session R-1042 → PID 8134 → report.md
   ```
11. Two concurrent Finance sessions cannot inspect, signal, trace, attach to, or access the private state and credentials of one another.
12. After a Redwood parent reads classified material, it cannot create a nominally Finance-only child, pass arbitrary parent text, and obtain a lower-labeled output.
13. Redwood memory is not visible to a later Bluebird session merely because both sessions belong to the Finance agent.
14. A prompt-injected session may stage a proposed patch but cannot directly modify trusted configuration, approved durable memory, or a protected production branch; promotion requires validation and attribution.
15. DNS rebinding, redirects, proxy variables, alternate address forms, direct sockets, and TLS mismatch cannot bypass the approved model gateway.
16. Revoking the initiator or project authorization during execution produces the documented freeze, termination, and credential-revocation behavior.
17. PID, descriptor, disk/inode, audit, child-fan-out, accelerator, model-token, and monetary budgets are enforced at the appropriate cgroup, supervisor, or gateway.
18. The same scenario is exercised under a microVM-per-session baseline for comparison.
19. A message from a Redwood session to a Bluebird-only peer is rejected, quarantined, or routed through trusted release, and the decision is audited with both session identities, channel, and labels.
20. Two concurrent sessions appending to the same memory partition cannot lose or tear approved state, and neither session's unvalidated proposals enter the other's imported snapshot.

### 10.4 Evaluation questions

- Which security invariants are enforced entirely by the kernel after launch?
- What is the trusted code size and privilege of the constructor?
- What launch latency and runtime overhead do MLS, namespaces, Landlock, seccomp, audit, and cgroups add?
- Can policy changes revoke future access for running sessions, and what requires restart?
- Are labels preserved across Git workflows, archives, backups, and supported shared filesystems?
- Can every observed operation be attributed to initiator, principal, session, and process?
- How often does policy administration require relabeling or privileged repair?
- Which operations cannot be expressed without userspace mediation?
- Does the design degrade safely when a kernel facility is unavailable?
- What fraction of meaningful effects are fully host-mediated, gateway-mediated, or dependent on a remote service's reference monitor?
- Can a hostile session copy or reuse issued credentials, and which credentials are non-exportable or brokered?
- Does shared inference preserve separation across GPU memory, batching, caches, adapters, logs, and crash recovery?
- How many outputs require declassification, what are review latency and rejection/correction rates, and does reviewer behavior degrade into routine approval?
- Can lower-classification sessions use useful durable memory without importing higher-domain or low-integrity state?
- What are the trusted code size, privilege, policy complexity, and change rate of the constructor, policy resolver, gateways, category allocator, and declassifier together?
- Compared with a microVM, what are boundary strength, startup latency, steady-state memory, storage complexity, audit fidelity, and operator skill requirements?
- Under Git and collaborative workflows, where do joins occur, how are merges labeled, and how are trusted artifacts promoted?

Negative results are valuable. In particular, Agor’s experience suggests testing the operational burden of maintaining Unix ownership and group projections under collaborative workflows.

---

## 11. Conformance and evaluation status

An implementation conforms to a deployment profile only if it identifies that profile, publishes its coverage inventory, satisfies every invariant applicable to that profile in Section 7 with the five-way result classification, documents every unavailable or substituted mechanism, and passes the corresponding Phase 1–3 tests in Section 10. Enabling SELinux, assigning a UID, or launching a container is not by itself evidence of conformance. Nor is conformance a proof: the formal rules in the front matter are an unproven specification, and passing its derived tests shows agreement with the specification on the tested interfaces, not satisfaction of a verified security property.

Conformance claims must be pre-registered with operational thresholds rather than adjectives. At minimum a profile's evaluation states: the covered-interface inventory and the adversary-capability matrix its bypass corpus exercises; kernel, LSM, systemd, and policy versions; the effect ontology against which attribution completeness is measured and the required completeness; the maximum acceptable policy-exception and privileged-repair rate; and, for profiles claiming release or promotion, reviewer throughput, disagreement, correction, and false-release targets. For the multilevel profile, failure to meet the release-economics targets is a conformance failure, not an operational note. Each residual assumption carries an owner, impact, compensating control, acceptance authority, and revalidation trigger.

A control arm is a required part of any evaluation: the same workload and the same abstract manifest run through a microVM (a stronger boundary; a container tests packaging rather than shared-kernel risk) with per-session workload identity, egress gateway, and audit. "Same manifest" is meaningful only for the substrate-independent fields (identity, derivation inputs, authority, budgets, gateway policy, audit requirements); substrate-specific refinements must be listed separately and equivalent assurance must not be inferred from shared fields.

The architecture remains experimentally falsifiable. Evaluation must determine whether the constructor and surrounding control plane prevent cross-session access, authority expansion, gateway bypass, unpromoted mutation of protected objects, incomplete descendant termination, and unattributed mediated effects; whether, in profiles that claim it, confidentiality and integrity laundering is prevented on mediated edges; and whether administrators can preserve labels and provenance through real storage and collaboration workflows at a sustainable policy-maintenance and review cost.

Negative results should identify the failed invariant, enforcement layer, deployment profile, and threat assumption. Failure of the full MLS profile does not by itself invalidate the principal/session ontology; failure of identity binding, same-principal isolation, gateway-only egress, or attribution of mediated effects would challenge the core architecture, and failure to show a measurable advantage over the control arm would challenge its justification.

For motivation, adoption implications, and the thesis-level conclusion, see the companion [position paper](position-paper.md).

---

## Notes and references

[^quine-paper]: Hao Ke, [“Quine: Realizing LLM Agents as Native POSIX Processes,”](https://arxiv.org/html/2603.18030v2) arXiv:2603.18030, 2026.

[^quine-repo]: Hao Ke, [Quine repository and runtime overview](https://github.com/kehao95/quine).

[^orkia]: Orkia, [documentation](https://orkia.dev/docs/) and [source repository](https://github.com/orkiaHQ/orkia). The project describes agent sessions as governed, persistent Unix-style jobs in PTYs with audit chains.

[^aaau]: AgentaaU, [AaaU: Agent-as-User Architecture](https://github.com/AgentaaU/AaaU).

[^a4u2]: Emmanuel Leblond, [`agent-as-unix-user`](https://pypi.org/project/agent-as-unix-user/), a Linux coding-agent sandbox based on dedicated users, groups, ACLs, and a credential-switching entrypoint.

[^agentsh]: agentsh, [Execution-Layer Security documentation](https://www.agentsh.org/docs/) and [source repository](https://github.com/canyonroad/agentsh).

[^agentos]: jimmc414, [Agent OS](https://github.com/jimmc414/AgentOS), a FUSE-based Unix-native interface for agent orchestration, state, and external services.

[^agor]: Agor, [Multiplayer Execution Isolation](https://agor.live/guide/multiplayer-unix-isolation) and [discussion of leaving Unix impersonation](https://agor.live/blog/why-agor-is-leaving-unix-impersonation-behind). Agor’s current design separates application authorization from the execution boundary.

[^credentials]: Linux man-pages project, [`credentials(7)`](https://man7.org/linux/man-pages/man7/credentials.7.html).

[^cgroups]: Linux man-pages, [`cgroups(7)`](https://man7.org/linux/man-pages/man7/cgroups.7.html).

[^namespaces]: Linux man-pages, [`namespaces(7)`](https://man7.org/linux/man-pages/man7/namespaces.7.html).

[^capabilities]: Linux man-pages, [`capabilities(7)`](https://man7.org/linux/man-pages/man7/capabilities.7.html).

[^nnp]: Linux kernel documentation, [No New Privileges Flag](https://docs.kernel.org/userspace-api/no_new_privs.html).

[^landlock]: Linux kernel documentation, [Landlock: unprivileged access control](https://docs.kernel.org/userspace-api/landlock.html).

[^seccomp]: Linux kernel documentation, [Seccomp BPF](https://docs.kernel.org/userspace-api/seccomp_filter.html). The documentation explicitly cautions that syscall filtering alone is not a sandbox.

[^loginuid]: Linux-PAM / man-pages, [`pam_loginuid(8)`](https://man7.org/linux/man-pages/man8/pam_loginuid.8.html) and [`audit_setloginuid(3)`](https://man7.org/linux/man-pages/man3/audit_setloginuid.3.html).

[^rhel-mls]: Red Hat, [Using Multi-Level Security (MLS)](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/10/html/using_selinux/using-multi-level-security-mls).

[^selinux-mls]: SELinuxProject, [MLS and MCS](https://github.com/SELinuxProject/selinux-notebook/blob/main/src/mls_mcs.md), *SELinux Notebook*. This documents MLS dominance, categories, trusted subjects, and reference-policy write constraints.

[^selinux-analysis]: Boniface Hicks, Sandra Rueda, Luke St.Clair, Trent Jaeger, and Patrick McDaniel, [“A Logical Specification and Analysis for SELinux MLS Policy,”](https://doi.org/10.1145/1805974.1805982) *ACM Transactions on Information and System Security* 13(3), 2010.

[^nist-abac]: NIST SP 800-162, second update (2019), [Guide to Attribute Based Access Control Definition and Considerations](https://csrc.nist.gov/pubs/sp/800/162/upd2/final).

[^spiffe]: Cloud Native Computing Foundation, [SPIFFE Overview](https://spiffe.io/docs/latest/spiffe-about/overview/).

[^rfc8693]: M. Jones et al., [RFC 8693: OAuth 2.0 Token Exchange](https://www.rfc-editor.org/rfc/rfc8693.html), January 2020.

[^delegation]: Shahar Avin et al., [“Authenticated Delegation and Authorized AI Agents,”](https://arxiv.org/abs/2501.09674) arXiv:2501.09674, 2025.

[^aac]: Xinfeng Li et al., [“A Vision for Access Control in LLM-based Agent Systems,”](https://arxiv.org/abs/2510.11108) arXiv:2510.11108, 2025.

[^seagent]: Zimo Ji et al., [“Taming Various Privilege Escalation in LLM-Based Agent Systems: A Mandatory Access Control Framework,”](https://arxiv.org/abs/2601.11893) arXiv:2601.11893, 2026.

[^confused-deputy]: Norm Hardy, [“The Confused Deputy: (or why capabilities might have been invented),”](https://doi.org/10.1145/54289.871709) *ACM SIGOPS Operating Systems Review* 22(4), 1988.

[^dlm-jif]: Andrew C. Myers and Barbara Liskov, [“Protecting Privacy using the Decentralized Label Model,”](https://cs.cornell.edu/andru/papers/iflow-tosem.pdf) *ACM Transactions on Software Engineering and Methodology* 9(4), 2000. The paper develops decentralized confidentiality and integrity policies and introduces Jif's statically checked information-flow model.

[^asbestos]: Petros Efstathopoulos et al., [“Labels and Event Processes in the Asbestos Operating System,”](https://doi.org/10.1145/1095809.1095813) *SOSP*, 2005.

[^histar]: Nickolai Zeldovich et al., [“Making Information Flow Explicit in HiStar,”](https://www.usenix.org/legacy/event/osdi06/tech/zeldovich.html) *OSDI*, 2006.

[^flume]: Maxwell Krohn et al., [“Information Flow Control for Standard OS Abstractions,”](https://dl.acm.org/doi/10.1145/1323293.1294293) *SOSP*, 2007.

[^blp]: D. Elliott Bell and Leonard J. LaPadula, [*Secure Computer Systems: Mathematical Foundations*](https://apps.dtic.mil/sti/citations/AD0770768), MITRE Technical Report 2547, Vol. I, 1973.

[^denning]: Dorothy E. Denning, [“A Lattice Model of Secure Information Flow,”](https://doi.org/10.1145/360051.360056) *Communications of the ACM* 19(5), 1976.

[^biba]: Kenneth J. Biba, [*Integrity Considerations for Secure Computer Systems*](https://apps.dtic.mil/sti/citations/ADA039324), MITRE Technical Report 3153, 1977.

[^clark-wilson]: David D. Clark and David R. Wilson, [“A Comparison of Commercial and Military Computer Security Policies,”](https://doi.org/10.1109/SP.1987.10001) *IEEE Symposium on Security and Privacy*, 1987.

[^slsa]: OpenSSF, [Supply-chain Levels for Software Artifacts (SLSA)](https://slsa.dev/spec/) and [in-toto](https://in-toto.io/).

---

## Appendix A: Concise comparison matrix

| System or tradition | First-class abstraction | Uses native OS enforcement? | Durable agent principal? | Shell/session semantics? | MAC classification central? |
|---|---|---:|---:|---:|---:|
| Typical agent framework | Agent object / conversation | Partly | Application-only | Usually no | No |
| Quine | POSIX agent process | Yes | No; PID/run lineage | Process and stream composition | No |
| Orkia | Governed agent job | Partly/yes | Named application agent | Yes, PTY and jobs | No |
| AaaU / agent-as-unix-user | Dedicated Unix user | Yes, DAC | Yes, local UID | Yes/PTY or command | No |
| agentsh | Policy-controlled execution session | Yes/interception stack | Session-oriented | Drop-in shell | Not MLS-centered |
| Agent OS | FUSE-exposed agent/filesystem object | Daemon + FUSE | Application/filesystem object | CLI/filesystem native | Custom security model |
| Agor | Application principal + sandbox view | Bubblewrap/mount boundary | Application identity | Web/agent sessions | No MLS core |
| SPIFFE | Workload identity | Attestation and credentials | Workload, not agent ontology | No | No |
| DIFC (Asbestos, HiStar, Flume) | Labeled process/object and flow | Kernel or reference-monitor enforcement | Information-flow principal, not agent ontology | Process/IPC-level | Yes, dynamic/decentralized |
| SELinux MLS | Labeled subject and object | Yes | OS security context | Process-level | Yes |
| **This proposal** | **Agent principal + governed session** | **Yes, composed Linux controls** | **Global identity; per-session execution UID** | **Yes** | **Profile-dependent** |

