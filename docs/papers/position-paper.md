# Agents as Unix Principals

## A Security Ontology for Governed Agent Sessions

**Version:** 0.5  
**Date:** 27 August 2026  
**Status:** Working position paper for external review  
**Companion:** [`technical-report.md`](technical-report.md)  
**Provenance:** Supersedes the original first draft; the companion report contains the complete invariants, threat model, operational analysis, and evaluation programme

---

## Revision history

- **0.4** — Split the concise position paper from the technical architecture and evaluation report.
- **0.5** — Restored the framework-responsibility boundary and governed Unix-composition implications; aligned provenance practice with the companion report.

---

## Abstract

Most AI-agent systems represent an agent as an application object, conversation, coroutine, or process owned by a shared service account. The framework must then reconstruct identity, lifecycle, authorization, isolation, resource accounting, and audit above the operating system, even when the agent ultimately acts by launching ordinary Unix programs.

This paper proposes a different foundation: **an organizational agent is a durable security principal and governed context domain; a session is a task-scoped shell and process tree instantiated with a bounded subset of that principal's authority; an execution is an ordinary process; and a model is a replaceable cognitive component inside the session.** A fleet-wide agent identity may be projected into a stable, dynamic, or namespace-local Unix UID, but it is not reducible to a PID, model instance, API token, or database row.

Linux already supplies much of the local enforcement substrate: credentials and access controls, namespaces, cgroups, capabilities, `no_new_privs`, Landlock, seccomp, audit, and optional mandatory-access-control profiles. Organizational policy should compute the permissible session, while the operating system enforces its local realization. Gateways and remote services remain responsible for effects beyond the host.

The key security distinction is between **authority** and **information state**. Delegation may reduce what a child can access, but a fresh process or model does not erase the confidentiality or integrity provenance of information supplied to it. Durable memory must therefore be partitioned, communication must preserve labels and provenance, and untrusted outputs must pass through validation or review before promotion into trusted state. The architecture is tiered: a baseline Unix-governed profile is deployable with current mechanisms, while full multilevel security is an optional high-assurance profile whose operational viability must be demonstrated.

---

## Summary of claims

1. **An agent is a durable organizational security principal**, not a PID, model instance, API token, or conversation row.
2. **A session is the task-scoped security and information boundary.** It binds initiator, purpose, activated authority, information state, credentials, resources, visible world, and process tree.
3. **Processes and models are replaceable executions.** Replacing a cognitive runtime does not change the session's security identity.
4. **Authority and information state are different.** Delegation may narrow authority, but a fresh child or peer does not cleanse the confidentiality or integrity provenance of its inputs.
5. **Durable memory must be partitioned.** Ownership by one agent does not make every historical memory partition safe to import into every session.
6. **Integrity requires stage, validate, and promote.** Prompt-injected or model-generated material must not directly replace trusted configuration, approved memory, or production artifacts.
7. **Linux enforces the locally realizable boundary.** Gateways, workload identity, and remote reference monitors enforce model, database, Git, SaaS, and other network effects.
8. **The architecture is tiered.** Unix-governed sessions are practical now; compartments and full MLS add stronger information-domain controls at increasing operational cost.
9. **Containers and microVMs are complementary.** They strengthen workload isolation but do not by themselves classify outputs, partition memory, constrain remote services, or provide trusted release.

The central rules are:

```text
A_session ⊆ P_agent ∩ A_initiator ∩ A_task ∩ A_policy
```

```text
I_receiver ⪰ join(confidentiality labels of every admitted input)
```

```text
T_receiver must not exceed the integrity justified by every admitted input
```

If higher `T` means more trusted, the last rule may be written:

```text
T_receiver ⪯ meet(T_input1, …, T_inputN)
```

Promotion above that integrity bound requires:

```text
untrusted input → staging → validation or review → trusted promotion
```

---

## 1. Why application-only agents are insufficient

A common deployment looks like this:

```text
agent-daemon (one service UID)
├── application object: finance
├── application object: engineering
├── application object: legal
└── shared command executor and credentials
```

The application describes three agents, but the kernel sees one principal. Unless another isolation layer intervenes, every command inherits the daemon's UID, groups, filesystem reach, network access, environment, and credentials. Authorization becomes a convention in framework code. Prompt injection, a confused-deputy path, a malicious plugin, or an unmediated subprocess can cross boundaries that do not exist at the enforcement layer.

This creates recurring mismatches:

- A framework UUID identifies the agent, while the filesystem records every write as the shared daemon user.
- UI or retrieval policy hides a document, while the process credential can still open it.
- Artifacts from different agents have the same owner and weak durable attribution.
- “Spawn subagent” often means “create another object with whatever authority the framework assigns,” rather than a descendant with non-increasing authority.
- Classification ends at retrieval even though prompts, transcripts, caches, generated files, child messages, telemetry, and model requests may all contain the retrieved information.
- The component being audited emits its own tool log, while direct syscalls and hidden subprocess effects may be absent.

Mature platforms already improve this with containers, workload identities, application RBAC, short-lived credentials, and structured audit. The claim here is not that application policy should disappear. Organizational IAM, RBAC or ABAC, purpose constraints, approvals, and resource catalogues are still required. The narrower division of labor is:

```text
organizational policy → computes what may be activated
host and gateway controls → enforce the resulting boundary
```

The daemon should not need to mediate every `open(2)` call for its authorization decision to be real.

---

## 2. Ontology: principal, session, execution, model

### 2.1 Agent principal

An **agent** is a durable organizational security principal and governed context domain containing:

- a stable global identity and accountable owner;
- potential authority, clearance, and organizational membership;
- durable but partitioned private and shared state;
- credential, model, and tool eligibility policy;
- delegation, retention, and audit policy;
- lifecycle rules for provisioning, suspension, rotation, and retirement.

On one host, this identity may be represented by a dedicated UID plus an optional MAC identity and authorized range. Across a fleet, a directory or workload-identity system remains authoritative; the UID is a local projection. Stable, dynamically allocated, and namespace-local UIDs are all possible implementations if durable ownership and audit remain bound to the global principal.

The cognitive software is not the principal. A model or runtime may plan and act, but it does so within authority assigned to the governed identity.

### 2.2 Session

A **session** is a temporary, task-scoped realization of an agent principal. It binds:

- the authenticated initiating human or service;
- declared task, purpose, and approvals;
- authority activated for this invocation;
- confidentiality and integrity/provenance state;
- filesystem, process, IPC, device, and network views;
- short-lived or brokered service credentials;
- resource, token, and cost budgets;
- process tree, terminal or streams, outputs, and audit identifiers.

The session is the natural cognitive security boundary because information read during one task may enter prompts, summaries, model caches, memory, outputs, or child messages. A task requiring incompatible compartments should normally receive another session.

Two sessions belonging to the same agent are not automatically isolated merely because they have different session IDs or cgroups. A UID identifies a principal, not a session. Same-principal sessions require private runtime directories, storage and descriptors, process/IPC isolation, ptrace and signal controls, separate PTYs and sockets, or distinct execution UIDs or MAC types.

### 2.3 Execution and model

An **execution** is an ordinary process with a PID, executable image, address space, environment, file descriptors, signal state, and exit status. Shells, retrieval commands, compilers, cognitive runtimes, and local model clients are executions.

A **model** is a cognitive implementation invoked by an execution. It may be replaced without changing the agent or current session. When inference is remote—or provided by a shared local server—the model boundary is also a governed service boundary. Prompt caches, KV state, batching, adapters, logs, accelerator memory, crash dumps, tenant identity, and retention policy must not silently cross information domains.

This yields the hierarchy:

```text
Organization
├── Human and service initiators
└── Agent principal (global identity + policy + partitioned state)
    └── Session (initiator + task + authority + information state)
        ├── namespace + cgroup + optional MAC context
        ├── cognitive runtime (PID)
        ├── shell and tools (PIDs)
        └── delegated processes or narrower sessions
```

---

## 3. Authority is not information state

The most important formal distinction is among:

```text
P_agent   = principal's potential authority and clearance
A_session = authority activated for one task
I_session = confidentiality domains already admitted
T_session = integrity and provenance of admitted inputs and mutable outputs
```

The constructor computes `A_session` as an intersection of agent, initiator, task, and current policy. Clearance is a ceiling, not ambient access.

Information state follows different rules. If a session reads Finance+Redwood material, a new child with fewer filesystem permissions may still receive Redwood information in its prompt. Reduced authority limits future access; it does not declassify what the parent already observed. Similarly, creating a fresh model process does not restore integrity if its instructions were derived from a malicious repository or web page.

For every communication edge—parent to child, sibling to sibling, agent to agent, pipe, socket, queue, RPC, file, artifact, or model call—the receiver must be authorized for the channel, its confidentiality domain must dominate the input label, and the input's provenance must not be silently promoted. Otherwise the input must be rejected, quarantined, admitted to a compatible session, or passed through a trusted release or validation boundary.

This makes a general principle explicit:

> **Every communication edge is an information-admission event. A new process boundary is not an information-cleansing boundary.**

Purpose also has a limit. It can determine which session is launched, which credentials are issued, and which gateway operations are available. The kernel cannot infer whether an otherwise permitted read is genuinely for “quarterly forecasting.” Purpose remains an organizational and service-side policy claim unless every meaningful operation is mediated by a purpose-aware reference monitor.

---

## 4. Durable memory and integrity

### 4.1 Partitioned memory

A single undifferentiated agent home is incompatible with task-scoped information domains. If a Finance agent stores summaries from Redwood, Bluebird, and acquisitions in one memory database, that database acquires the join of those domains. It cannot safely be imported into a lower session merely because the same agent owns it.

Durable state should instead be partitioned by compatible confidentiality and integrity domains:

```text
finance-agent state
├── public/general memory
├── finance memory
├── finance + redwood memory
├── finance + bluebird memory
├── trusted configuration and approved skills
├── untrusted learned observations
└── sealed session archives
```

A constructor exposes only compatible partitions. Import into a higher confidentiality domain may be allowed subject to integrity checks; export downward requires trusted release. Resume means reauthorization plus selective import, not restoration of the agent's entire historical context.

Concurrent sessions also need explicit consistency and poisoning controls. A running session should normally import a stable snapshot. Sessions append proposed memory updates with writer identity, labels, provenance, and generation information rather than mutating approved memory in place. Transactions or compare-and-swap prevent torn and lost updates; validation or review—not locking alone—controls promotion into trusted memory.

### 4.2 Stage, validate, and promote

Confidentiality controls do not stop an authorized but prompt-injected agent from corrupting an artifact it may write. Practical integrity therefore separates:

- immutable or low-integrity imported material;
- untrusted parsing and retrieval;
- model-generated proposals in staging;
- deterministically validated artifacts;
- reviewed outputs promoted into trusted repositories;
- protected policy, credentials, tools, configuration, and approved memory.

The default workflow is not direct mutation of trusted objects. It is:

```text
stage → validate → review where necessary → promote
```

Promotion may require tests, schemas, reproducible transformations, independent review, branch protection, or a constrained trusted service. Confidence expressed by a model is not an integrity transition.

### 4.3 Trusted release and aggregation

A process that reads a high domain should produce high-domain output by default. Downgrade requires a separate trusted workflow: deterministic transformation, policy-approved redaction, constrained declassifier, or human review. Ordinary agents must not label their own output downward.

Declassification is also an economic system. Conservative joins can make every useful output require review, creating bottlenecks and eventual rubber-stamping. A viable deployment must measure release volume, latency, rejection and correction rates, reviewer agreement, automated-release coverage, overclassification, and incidents.

Releases cannot always be judged independently. Many individually low-risk outputs may jointly disclose protected information through aggregation, differencing, or adaptive querying. Release services should consider cumulative history across the session, agent, initiator, project, recipient, and related queries; rate and volume are policy signals. This mitigates but cannot prove the absence of semantic inference.

---

## 5. Unix realization and its boundary

A narrow trusted constructor can translate an authenticated organizational decision into:

```text
authenticate initiator
→ resolve agent principal
→ validate task, purpose, and approvals
→ compute authority and initial information domain
→ select UID, groups, capabilities, and optional MAC context
→ construct namespaces, mounts, devices, and descriptors
→ create cgroup and resource limits
→ issue or broker narrow service credentials
→ bind audit and workload identity
→ apply no_new_privs, Landlock, and seccomp where appropriate
→ dispose of launch privilege
→ exec the cognitive runtime or shell
```

Linux mechanisms answer different questions:

| Mechanism | Role |
|---|---|
| Global identity projected to UID/GID | Durable local ownership and principal boundary |
| SID, cgroup, immutable launch record | Task-scoped session grouping and provenance |
| PID | Current execution identity |
| Namespaces and descriptors | Visible and connected world |
| Capabilities and `no_new_privs` | Privilege reduction and anti-escalation |
| Landlock and seccomp | Additional inherited self-restriction and attack-surface reduction |
| DAC, ACLs, and optional host MAC | Local object and process authorization |
| SELinux MCS/MLS, where deployed | Compartments and declared multilevel domains |
| Linux Audit plus service logs | Process attribution and denial evidence |

A manifest is a derived launch record, not an entitlement supplied by the agent. Authoritative UIDs, labels, mounts, credentials, and endpoints must come from a server-side catalogue and policy decision. Unknown resources, partial construction, and unavailable required controls must fail closed.

Hostnames alone are not adequate network policy because DNS, redirects, CDNs, proxies, TLS identity, IPv6, local sockets, and multi-tenant endpoints complicate enforcement. Sensitive sessions should reach authenticated gateways that verify service, tenant, model, retention mode, method, credential scope, and budget. Exportable bearer tokens remain copyable even when short-lived; proof-of-possession credentials, inherited capabilities, or operation brokers can reduce that risk.

Linux does not replace remote reference monitors:

| Effect | Local enforcement | Remote enforcement |
|---|---|---|
| Local file/process/IPC access | Credentials, namespaces, MAC, Landlock, descriptor policy | Policy administration |
| Model call | Reachability and credential confinement | Model/tenant selection, retention, spend, audit |
| Database query | Endpoint and credential confinement | Database authorization and query audit |
| Git or CI write | Endpoint and credential confinement | Branch protection, review, CI, repository policy |
| SaaS action | Endpoint and token confinement | Service authorization and audit |
| Release | Prevention of ordinary write-down | Trusted transformation, review, approval |

The end-to-end boundary is composed from host controls, gateways, workload identity, remote authorization, and correlated audit.

---

## 6. Deployment profiles

The ontology does not require every deployment to adopt full SELinux MLS.

### Profile 1: Unix-governed session

This baseline uses global agent identity, local credential projection, private namespaces and storage, cgroups, capability removal, `no_new_privs`, optional Landlock/seccomp and host MAC, credential/egress controls, and attributable launch records. It is practical with current Linux and systemd machinery.

SELinux type enforcement is not the only useful baseline MAC. AppArmor and other supported LSM controls can contribute on distributions where SELinux is impractical, although they are not drop-in implementations of multilevel information flow.

### Profile 2: Compartmented session

This adds MCS or equivalent project/tenant compartments, partitioned memory, authenticated gateways, category allocation, and controlled import/export. Container practice demonstrates that category isolation is operationally possible, but it does not by itself prove dynamic label propagation or multilevel release semantics.

### Profile 3: Multilevel session

This adds full MLS sensitivities, declared flow rules, analyzed policy, labeled persistence and networking or equivalent gateways, and trusted declassification. Full MLS has a small tooling, staffing, and application-compatibility ecosystem. It is a high-assurance option whose policy correctness, administrative burden, storage interoperability, and release economics must be measured rather than assumed.

### Profile 4: Strong workload isolation

A container, VM, microVM, or dedicated node can surround any profile. A microVM commonly provides a simpler, stronger boundary against cross-workload kernel and device interference, at the cost of launch latency, memory, image management, and state/observability complexity. It still does not determine where information may flow after entering the VM.

A likely high-assurance design combines both ideas:

```text
organizational identity and task policy
→ microVM session boundary
→ partitioned storage, scoped credentials, gateways, promotion, and release
```

The choice should be empirical rather than ideological.

---

## 7. Relationship to prior work

This proposal composes established operating-system security ideas with recent Unix-native agent systems.

Quine identifies an agent with a POSIX process and uses streams, files, signals, process groups, and job control.[^quine] Orkia treats persistent agent sessions as governed shell jobs in PTYs.[^orkia] Agent-as-user projects demonstrate dedicated Unix credentials and filesystem separation.[^aaau] agentsh places enforcement beneath agent tools.[^agentsh] These projects support Unix-native execution, but do not alone define the combination of durable organizational principal, initiator-bound task session, information contamination, partitioned memory, remote workload identity, and trusted promotion/release proposed here.

Decentralized information-flow-control systems are the closest academic ancestors. Myers and Liskov's decentralized label model and Jif address confidentiality, integrity, and owned declassification.[^dlm] Asbestos and HiStar explore kernel-enforced information flow with small trusted components.[^asbestos][^histar] Flume is a particularly close neighbor, adding process-level DIFC to familiar Linux abstractions such as pipes, sockets, and descriptors.[^flume]

This paper does not claim to invent label joining, integrity labels, process-level information flow, or controlled declassification. Its narrower contribution is their composition around a durable organizational agent identity, task-scoped cognitive session, replaceable model, partitioned memory, model/service egress, and attributable delegation.

SELinux MLS is valuable precisely because it is not agent-specific. It can enforce coarse, declared information domains using a mature kernel substrate. It does not perform semantic taint tracking, dynamically compute labels from read history, or prove noninterference. A normal targeted SELinux installation is not the multilevel profile described here.

---

## 8. Adoption and evaluation

Adoption need not begin with MLS or a new agent platform. An existing coding-agent harness can move incrementally toward the Unix-governed profile by:

1. assigning explicit global workload identity and local credentials;
2. constructing a private namespace and cgroup per session;
3. removing ambient capabilities and setting `no_new_privs`;
4. isolating sessions that share the same durable principal;
5. issuing narrow credentials through an authenticated gateway;
6. binding process audit to an immutable launch record;
7. separating untrusted staging from trusted promotion;
8. partitioning durable memory before adding richer classification.

Compartmented memory, stronger MAC, and trusted release can then be introduced where the risk justifies their operational cost. The approach extends existing harnesses rather than requiring them to become classified operating systems at once.

### 8.1 What remains for the agent framework

This architecture does not eliminate the harness. It narrows its mandate. The framework remains responsible for work genuinely specific to cognition and agent behavior:

- forming and managing cognitive context;
- invoking models and handling model-specific protocols;
- representing goals, uncertainty, and structured tool requests;
- deciding when to delegate within an externally enforced ceiling;
- externalizing proposed memory and useful state before context loss;
- requesting authority changes through explicit policy interfaces;
- participating in validation, review, and declassification without controlling its own promotion or release.

The framework may also provide orchestration, durable queues, retries, workflow recovery, and distributed placement where ordinary single-host process semantics are insufficient. What it should not do is become the sole reference monitor for resources the host or a service can enforce directly.

Unix composition remains useful, but a pipe is not merely a convenience in a governed system:

```sh
produce-material | agent-runtime "analyze" | review-filter
```

Each endpoint, inherited descriptor, and process must belong to a compatible confidentiality and integrity domain. Shell-native composition makes policy visible at process boundaries; it does not abolish that policy.

### 8.2 Evaluation path

The companion technical report defines a three-phase evaluation:

- **Phase 1:** Unix-governed identity, construction, isolation, resources, credentials, termination, and audit.
- **Phase 2:** compartments, partitioned/versioned memory, contamination-safe communication, integrity promotion, revocation, and external budgets.
- **Phase 3:** full MLS, declassification throughput, storage continuity, shared inference, and comparison with microVM-per-session.

The most important falsification questions are:

- Does the host prevent cross-principal and same-principal cross-session interference?
- Can narrower delegation launder confidentiality or integrity?
- Can a session bypass the model/service gateway or steal reusable credentials?
- Can untrusted input modify trusted memory or production artifacts without promotion?
- Can labels and provenance survive the actual Git, archive, backup, and remote-service workflows an organization uses?
- Can administrators and reviewers operate the policy without broad exceptions, continual privileged repair, or rubber-stamped release?
- Does a microVM produce a stronger or simpler boundary at acceptable cost?

Negative results are valuable. The ontology may remain useful even if full MLS proves uneconomic.

---

## 9. Conclusion

Multi-user agent systems need durable answers to five questions:

1. Who is this agent?
2. Under whose authority is this session operating?
3. Which information and integrity domain does it inhabit?
4. How are delegation and communication prevented from laundering authority or provenance?
5. Can effects be attributed and governed after the model and process exit?

A coherent division is available:

- **agent:** durable organizational principal with potential authority and partitioned state;
- **session:** task-scoped process world with activated authority, information state, provenance, credentials, and budgets;
- **execution:** ordinary process;
- **model:** replaceable cognitive component and, when shared or remote, a governed service boundary;
- **organizational policy:** computes the permissible session and lifecycle response;
- **Unix and optional MAC:** enforce the local realization;
- **gateways and remote services:** enforce effects beyond the host;
- **trusted promotion and release:** govern integrity elevation and confidentiality downgrade.

The principal/session ontology and baseline Unix-governed profile are viable with mechanisms deployed today. Compartments require real engineering. Full MLS is an optional high-assurance profile with serious operational costs. VMs and microVMs strengthen isolation but remain complementary to information-flow governance.

The central contribution is the distinction between authority and information state. Authority can decrease when work is delegated; confidentiality and integrity provenance do not disappear because a new process, model, or agent object was created. If this principle is projected into enforceable local boundaries, partitioned memory, explicit communication edges, and remote reference monitors, agent systems can stop rebuilding weaker operating-system abstractions inside every harness while remaining honest about what Unix does not solve.

---

## References

[^quine]: Hao Ke, [“Quine: Realizing LLM Agents as Native POSIX Processes,”](https://arxiv.org/html/2603.18030v2) 2026.

[^orkia]: Orkia, [documentation](https://orkia.dev/docs/) and [source](https://github.com/orkiaHQ/orkia).

[^aaau]: AgentaaU, [AaaU: Agent-as-User Architecture](https://github.com/AgentaaU/AaaU).

[^agentsh]: agentsh, [Execution-Layer Security documentation](https://www.agentsh.org/docs/).

[^dlm]: Andrew C. Myers and Barbara Liskov, [“Protecting Privacy using the Decentralized Label Model,”](https://cs.cornell.edu/andru/papers/iflow-tosem.pdf) *ACM TOSEM* 9(4), 2000.

[^asbestos]: Petros Efstathopoulos et al., [“Labels and Event Processes in the Asbestos Operating System,”](https://doi.org/10.1145/1095809.1095813) *SOSP*, 2005.

[^histar]: Nickolai Zeldovich et al., [“Making Information Flow Explicit in HiStar,”](https://www.usenix.org/legacy/event/osdi06/tech/zeldovich.html) *OSDI*, 2006.

[^flume]: Maxwell Krohn et al., [“Information Flow Control for Standard OS Abstractions,”](https://dl.acm.org/doi/10.1145/1323293.1294293) *SOSP*, 2007.
