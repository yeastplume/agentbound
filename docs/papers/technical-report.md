# Agents as Unix Principals

## Security Architecture and Evaluation Programme

**Version:** 0.4-TR2  
**Date:** 27 August 2026  
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

---

## Purpose, scope, and document ownership

The companion [position paper](position-paper.md) is the citable statement of the motivation, thesis, adoption argument, and conclusions. This report is the normative source for mechanism mapping, constructor behavior, information-flow rules, security invariants, threat assumptions, deployment constraints, and evaluation. It does not repeat the full argument; where motivation is required, it refers to the position paper.

The report remains self-contained at the level needed to implement and test the architecture. Normative terms and notation are restated below, but their explanatory treatment belongs to the position paper. Changes to shared concepts should be made there first and then reflected here; detailed mechanisms and tests should be changed here and only summarized in the position paper.

### Normative terminology

- **Agent principal:** a durable organizational security principal with a stable global identity, potential authority and clearance, partitioned durable state, credential/model/tool policy, delegation constraints, retention rules, and lifecycle policy. A host UID is a local projection, not necessarily the global source of truth.
- **Session:** a task-scoped realization of one agent principal, bound to an authenticated initiator, purpose, approvals, activated authority, confidentiality and integrity state, visible world, credentials, budgets, process tree, outputs, and audit identity.
- **Execution:** an ordinary process within a session. A cognitive runtime, shell, compiler, retrieval command, or model client is an execution, not an independent security principal unless separately provisioned as one.
- **Model:** a replaceable cognitive implementation. A shared or remote model is also a governed service boundary; changing it does not change the session identity.
- **Communication edge:** any prompt, message, file, descriptor, pipe, socket, queue entry, RPC, artifact, memory import, or service result admitted by another process or session.

Agent context comprises cognitive, informational, security, and organizational components. The session is the primary task-scoped boundary because admitted information may enter prompts, process memory, summaries, transcripts, caches, outputs, child messages, and durable memory.

### Normative notation and flow rules

```text
P_agent   = principal's potential authority and clearance
A_session = authority activated for one task
I_session = confidentiality domains already admitted
T_session = integrity and provenance of admitted inputs and mutable outputs
```

Activated authority is bounded by:

```text
A_session ⊆ P_agent ∩ A_initiator ∩ A_task ∩ A_policy
```

Every receiving process or session must satisfy:

```text
I_receiver ⪰ join(confidentiality labels of every admitted input)
T_receiver must not exceed the integrity justified by every admitted input
```

If higher `T` means more trusted:

```text
T_receiver ⪯ meet(T_input1, …, T_inputN)
```

Narrowing authority does not remove information already observed, and a fresh process or model does not restore integrity. Raising integrity requires the explicit promotion path specified in this report:

```text
untrusted input → staging → validation or review → trusted promotion
```

Standard SELinux MLS fields should not be treated as an automatic independent confidentiality-and-integrity product lattice. Implementations may combine MAC domains with structured import, immutable inputs, staging, deterministic checks, review, and constrained promotion services.

---

## 1. Unix and Linux mapping

The proposal uses existing mechanisms wherever their semantics fit.

| Agent-system concept | Unix/Linux mechanism |
|---|---|
| Durable local agent principal | UID / system account |
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

### 1.2 UID is not the whole identity

“Agent as user” should not be read as “put the entire enterprise IAM graph in `/etc/passwd` and `/etc/group`.” A UID is the local kernel principal. At organizational scale, a directory or identity service remains authoritative, and a session broker projects a verified agent identity into a host credential, SELinux context, and workload identity.

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
compute effective session authority and classification
        ↓
select UID, groups, capabilities, and SELinux context
        ↓
construct namespace and approved file-descriptor set
        ↓
create cgroup/systemd scope and resource limits
        ↓
issue short-lived external credentials
        ↓
set audit/session provenance
        ↓
set no_new_privs and optional Landlock/seccomp policy
        ↓
drop launcher privilege
        ↓
exec shell or cognitive runtime
```

The effective authority should be no broader than the intersection of relevant constraints:

```text
A_session ⊆ P_agent ∩ A_initiator ∩ A_task ∩ A_current_policy
```

This formula concerns activated authority, not information contamination. The initial session information label must dominate every prompt, memory partition, file domain, descriptor, and service result admitted at launch; subsequent import must preserve or raise it. Purpose can constrain which session is authorized and which credentials or gateway operations are issued, but the kernel cannot infer whether an otherwise permitted read is genuinely being performed for the declared business purpose.

This is not directly a Linux formula; it is the policy computation performed before launch. Its result is projected into mechanisms the kernel understands. Failure to establish any required boundary must abort launch rather than silently degrade.

A durable Finance agent might be authorized for Finance, Forecasting, and Acquisitions. A session created to analyze Project Redwood should activate only Finance and Redwood, expose only the necessary directories, use only a model endpoint authorized for the classification, and receive credentials limited to that purpose. Clearance represents a ceiling, not ambient access.

### 2.1 A session manifest

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

A delegated “agent” need not always receive another durable UID. A temporary helper with no independent organizational authority can remain a child execution or narrower session under the parent agent’s UID. A new durable principal is justified when the child has independent policy, ownership, credentials, lifecycle, or audit responsibility.

### 4.1 Sessions sharing one durable principal

A UID distinguishes principals, not sessions belonging to the same principal. Two concurrent sessions running directly under one agent UID may otherwise inspect or influence each other through `/proc`, signals, ptrace, shared home state, Unix sockets, temporary files, terminals, IPC, or inherited descriptors. SID and cgroup membership aid grouping and accounting but are not authorization boundaries.

The reference design must therefore isolate same-principal sessions unless policy deliberately creates a sharing channel. Depending on the profile, this can require per-session execution UIDs or SELinux types, PID and IPC namespaces, private procfs and runtime directories, ptrace restrictions, private sockets and PTYs, partitioned storage, and explicit descriptor passing. A stable agent identity may own durable state while an ephemeral execution identity accesses only the partitions activated for one session.

### 4.2 Peer and cross-principal communication

Delegation is only one communication topology. Sibling sessions and agents belonging to different principals also exchange messages, artifacts, queue entries, RPC calls, repository changes, and file descriptors. Every such edge is an authorization decision, an information admission, a possible confidentiality join, an integrity/provenance transition, and an auditable causal link.

A receiver may import a message or artifact only when it is authorized for the named channel, its confidentiality domain dominates the message label, and the message's provenance and integrity state are preserved without silent promotion. If those conditions are not met, the receiver must reject the input, enter or create a compatible session, or use a trusted release/validation boundary. The audit record should bind sender session, receiver session, channel, object digest or message identifier, labels, and policy decision. Pipes and Unix sockets are useful transports, not exceptions to the rule.

### 4.3 Human attachment and interactive control

Attaching to a PTY is a bidirectional information-flow and authority event, not merely a user-interface feature. Policy must separately govern who may observe output, inject input, approve an operation, interrupt work, or take interactive control. Injected commands should be attributable to the controlling human rather than appearing indistinguishably as autonomous agent actions. Terminal escape sequences, concurrent attachments, transcript classification, and the possibility that attachment reveals everything visible to the session must be addressed. Non-interactive sessions should avoid PTYs when they are unnecessary.

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

Linux Audit’s login UID is useful because it is intended to track the account that originally gained access and is inherited by child processes.[^loginuid] Setting it requires `CAP_AUDIT_CONTROL`; the constructor should set it once from authenticated provenance and immediately drop that capability before executing untrusted code. Effective UID identifies the local agent principal after a credential transition. PID/PPID identify the execution. SELinux source and target contexts identify mandatory domains. A cgroup or systemd scope supplies a stable session grouping even as individual processes come and go.

No single existing field expresses the entire agent provenance chain. The session constructor should emit a signed or append-only launch record binding:

- globally durable agent identity;
- local UID and SELinux identity/context;
- initiator and authentication event;
- session ID and cgroup/unit;
- purpose and approvals;
- policy version and manifest digest;
- mounted resources and credential issuances;
- model endpoint classification;
- start/end times and termination reason.

Kernel audit and service-side logs can then be correlated with this record. Tamper-evident application records, such as Orkia’s signed SEAL chains, are complementary to kernel audit rather than substitutes for it.

The completeness claim must be scoped to defined mediated effects. Kernel audit cannot reveal the semantic operation performed inside an already-authorized database connection, SaaS API, model server, or multiplexed gateway. Those services must preserve the session and delegation identity in their own authorization and audit records. Audit availability is also a security property: deployments must specify behavior when buffers fill, collectors fail, clocks diverge, or event volume exceeds capacity; high-assurance profiles may need to stop or quarantine sessions rather than continue without required evidence.

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

### 6.10 SELinux MLS and historical multilevel security

SELinux MLS is not agent-specific; that is precisely its value. It is an existing mandatory-control substrate developed for sensitivity-based information separation. The proposal applies it to cognitive sessions whose in-memory context and outputs become part of the classified domain. SELinux type enforcement also permits separate domains for launchers, sessions, model gateways, audit components, and trusted declassifiers.

The strongest claim warranted here is feasibility of coarse, declared, kernel-enforced information domains—not automatic semantic taint tracking or a proof of noninterference.

---

## 7. Security invariants

A reference design should make the following invariants testable. The tags identify the primary enforcement layer: **K** kernel after launch, **C** trusted constructor/gateway, and **P** organizational policy or administration.

1. **[C] Durable identity:** Every session is bound to exactly one durable agent principal.
2. **[C] Explicit initiator:** Every session records the authenticated human or service that caused its creation.
3. **[P,C] Authority intersection:** Activated authority does not exceed agent, initiator, task, or current policy authority.
4. **[P,C,K] Clearance ceiling:** Policy selects an authorized context; SELinux prevents use of labels and transitions not allowed by the installed policy.
5. **[P,C] No ambient categories:** Authorization for a category does not activate it in every session.
6. **[C,K] Monotonic delegation:** The launcher narrows authority, while capability bounds, `no_new_privs`, Landlock, namespaces, descriptor discipline, and SELinux transition rules prevent specified forms of re-expansion.
7. **[C] Fail-closed construction:** If a required label, namespace, limit, credential restriction, or audit binding cannot be established, the session does not start.
8. **[C,K] Classified persistence:** Type transitions/default labeling and MAC checks ensure that transcripts, outputs, logs, checkpoints, and memory objects created through supported paths receive at least the session’s label.
9. **[K] No ordinary downgrade:** The session lacks relabel/write-down permission; only a separately authenticated, constrained, and audited trusted declassifier may release lower-labeled output.
10. **[C,K] Model egress compatibility:** Namespace/firewall/gateway policy lets a session reach only model endpoints approved for its current information domain.
11. **[C,P] Credential confinement:** Issuers create minimally scoped, short-lived credentials and revoke them after session termination; non-exportability depends on the credential mechanism.
12. **[C,K] Complete descendant control:** All descendants remain in a non-escapable supervised cgroup and termination uses cgroup-wide kill/reaping before credential revocation.
13. **[C,K] Attribution:** Kernel/service events and the launch record correlate an action to initiator, agent, session, and process.
14. **[C,P] Policy provenance:** Audit records identify the policy and effective-manifest versions used to construct the session.
15. **[C] Privilege disposal:** After setting provenance and the execution context, the constructor drops `CAP_AUDIT_CONTROL`, `CAP_SETUID`, `CAP_SETGID`, `CAP_MAC_ADMIN`, and every other launch-only privilege before untrusted code executes.
16. **[C,K] Contamination-safe communication:** Narrowing a child's authority or using a fresh peer process never lowers confidentiality or raises integrity of supplied information; every delegated or peer input retains compatible labels and provenance unless it crosses a trusted release or validation boundary.
17. **[C,K] Same-principal session isolation:** Concurrent sessions sharing one durable principal cannot inspect, signal, trace, attach to, or access private state and credentials of one another unless an explicit channel is authorized.
18. **[C,K,P] Partitioned durable memory:** A session imports only memory partitions compatible with its confidentiality and integrity domain; ownership by the same agent does not grant ambient access to all historical memory.
19. **[C,K,P] Integrity promotion:** Untrusted inputs and model-generated artifacts cannot directly replace trusted configuration, approved memory, protected branches, or production objects; promotion crosses a validated and attributable boundary.
20. **[C,P] Bounded external resources:** Gateways and supervisors enforce token, spend, rate, fan-out, storage, accelerator, and audit-capacity budgets not visible to cgroups alone.
21. **[C,P] Explicit lifecycle and revocation:** Policy defines active-session behavior for initiator disablement, authority revocation, reclassification, approval expiry, and control-plane failure.

The kernel cannot detect that the constructor selected the wrong authority; it only enforces the state actually installed. The C and P properties therefore require adversarial policy tests and independent audit, while K properties require SELinux-policy and kernel-configuration tests. None should be inferred merely from enabling SELinux.

---

## 8. Threat model

The in-scope adversary controls untrusted documents, prompts, repositories, packages, tool output, and potentially the cognitive runtime or a child process. It may issue arbitrary syscalls available to the session and seek unauthorized reads, writes, privilege expansion, persistence, audit evasion, or overt exfiltration. It does not begin with kernel, policy-administrator, session-constructor, model-gateway, or physical-host control.

The system should assume:

- the model is non-deterministic and may be manipulated by prompt injection;
- inputs, repositories, web pages, tool output, and packages may be adversarial;
- model-generated commands and code are untrusted;
- tool plugins may be buggy or malicious;
- a session may attempt to inspect credentials, escape its world, signal other processes, disable logging, or exfiltrate information;
- the organizational policy service, session constructor, kernel, SELinux policy, identity infrastructure, credential issuer, and approved model endpoints form part of the trusted computing base.

The initial scope does not claim to prevent:

- kernel vulnerabilities or physical compromise;
- hardware side channels and covert timing or storage channels through scheduling, shared caches, memory pressure, cgroup statistics, filesystem metadata, or other shared state; the primary claim concerns overt flows through kernel-mediated labeled objects and configured gateways;
- semantic leakage through an approved output that is mislabeled by a trusted declassifier;
- malicious administrators with sufficient host and policy authority;
- use of information already observed before revocation;
- compromise of a remote model provider after authorized transmission;
- proof that multiple individually authorized releases cannot be combined to infer protected information; cumulative monitoring and release policy mitigate rather than eliminate this aggregation risk.

seccomp reduces syscall attack surface but is explicitly not a complete sandbox by itself.[^seccomp] Capabilities divide traditional root powers but must be aggressively minimized, especially broad powers such as `CAP_SYS_ADMIN`.[^capabilities] Defense in depth is required.

### 8.1 Availability, resource exhaustion, and supply chain

A hostile session may exhaust more than CPU and memory. Limits and accounting should cover process and descriptor counts, disk bytes and inodes, I/O and network bandwidth, connections, audit volume, child-session fan-out, accelerator memory and time, model tokens, API rate limits, and monetary spend. cgroups enforce only part of this set; model and service gateways must enforce external budgets.

Containment also does not establish executable integrity. The session base filesystem, models and adapters, tools, plugins, dependency installation, compiler caches, startup files, `PATH`, dynamic-loader environment, and writable configuration are supply-chain inputs. High-integrity profiles should use an immutable or measured base, scrub the environment, verify eligible artifacts, isolate build caches, and direct untrusted package installation into disposable staging rather than the trusted runtime or durable agent memory.

---

## 9. Deployment profiles, alternatives, and open questions

### 9.1 Deployment profiles

The ontology does not require every deployment to adopt full SELinux MLS. Four composable profiles clarify the claims:

1. **Unix-governed session:** global agent identity projected into local credentials; per-session namespaces, cgroup, private storage, capability removal, `no_new_privs`, optional Landlock/seccomp and host MAC, credential and egress controls, and an attributable launch record. This profile is viable with current Linux and systemd machinery. SELinux type enforcement is not the only useful baseline MAC: AppArmor or other supported LSM controls may provide meaningful confinement on distributions where SELinux is impractical, although they are not drop-in implementations of the multilevel profile.
2. **Compartmented session:** the baseline plus MCS or equivalent project/tenant separation, partitioned durable memory, authenticated gateways, category allocation, and controlled import/export. Existing container practice supports category isolation, but does not by itself prove multilevel information-flow control.
3. **Multilevel session:** full MLS sensitivities, declared flow rules, labeled persistence and networking or equivalent gateways, analyzed policy, and trusted declassification. This is a high-assurance research and deployment profile whose staffing, tooling, interoperability, and operating cost must be measured.
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

The ecosystem risk is substantial: full MLS has a much smaller operator, tooling, application-compatibility, and policy-authoring base than mainstream container or VM isolation. Mechanism feasibility must therefore be evaluated separately from policy correctness and day-two operational viability. The multilevel profile should not be presented as the default until administrators can maintain it without broad exceptions or continual privileged repair.

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
7. Replacing the cognitive runtime does not alter the session UID, MAC context, cgroup, or audit identity.
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

An implementation conforms to a deployment profile only if it identifies that profile, satisfies every applicable invariant in Section 7, documents every unavailable or substituted mechanism, and passes the corresponding Phase 1–3 tests in Section 10. Enabling SELinux, assigning a UID, or launching a container is not by itself evidence of conformance.

The architecture remains experimentally falsifiable. Evaluation must determine whether the constructor and surrounding control plane prevent cross-session access, authority expansion, confidentiality or integrity laundering, gateway bypass, unpromoted mutation of trusted state, incomplete descendant termination, and unattributed effects. It must also determine whether administrators can preserve labels and provenance through real storage and collaboration workflows, and whether policy maintenance and release review are sustainable.

Negative results should identify the failed invariant, enforcement layer, deployment profile, and threat assumption. Failure of the full MLS profile does not by itself invalidate the principal/session ontology; failure of identity binding, contamination-safe communication, same-principal isolation, or attributable enforcement would challenge the core architecture.

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
| **This proposal** | **Agent principal + governed session** | **Yes, composed Linux controls** | **UID/local projection + global identity** | **Yes** | **Yes** |

