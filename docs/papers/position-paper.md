# Agents as Unix Principals

## A Security Ontology for Governed Agent Sessions

**Version:** 0.7  
**Date:** 28 August 2026  
**Status:** Working position paper for external review  
**Companion:** [`technical-report.md`](technical-report.md)  
**Provenance:** Supersedes the original first draft; the companion report contains the complete invariants, threat model, operational analysis, and evaluation programme

---

## Revision history

- **0.4** — Split the concise position paper from the technical architecture and evaluation report.
- **0.5** — Restored the framework-responsibility boundary and governed Unix-composition implications; aligned provenance practice with the companion report.
- **0.6** — Incorporated three independent reviews: durable principal identity separated from per-session execution identity; the information-admission claim scoped to mediated edges and the Unix-governed profile stated as a non-information-flow profile; intersection presented as the common case of a derivation relation; model treated as an approved execution binding; attribution scoped to mediated effects; integrity provenance foregrounded as the near-term payoff; the hostile-reviewer objection stated and answered.
- **0.7** — Trimming pass: removed mechanism detail owned by the companion report and consolidated the rules; added an explicit statement that the formalism is an unproven specification.

---

## Abstract

Most AI-agent systems represent an agent as an application object, conversation, coroutine, or process owned by a shared service account. The framework must then reconstruct identity, lifecycle, authorization, isolation, resource accounting, and audit above the operating system, even when the agent ultimately acts by launching ordinary Unix programs.

This paper proposes a different foundation: **an organizational agent is a durable, accountable security principal; a session is a task-scoped shell and process tree instantiated under its own execution identity with a bounded subset of that principal's authority; an execution is an ordinary process; and a model is a replaceable cognitive component whose binding to the session is itself governed.** The durable identity owns state and carries accountability; it never runs code. Each session runs under a per-session Unix execution identity. Neither is reducible to a PID, model instance, API token, or database row.

Linux already supplies much of the local enforcement substrate: credentials and access controls, namespaces, cgroups, capabilities, `no_new_privs`, Landlock, seccomp, audit, and optional mandatory-access-control profiles. Organizational policy should compute the permissible session, while the operating system enforces its local realization. Gateways and remote services remain responsible for effects beyond the host.

The key security distinction is between **authority** and **information state**. Delegation may reduce what a child can access, but a fresh process or model does not erase the confidentiality or integrity provenance of information supplied to it. Durable memory must therefore be partitioned, communication must preserve labels and provenance, and untrusted outputs must pass through validation or review before promotion into trusted state. The architecture is tiered, and each tier claims only the edges it mediates. The baseline Unix-governed profile is deployable now as an isolation, authority, and attribution boundary; it makes no information-flow claim. Compartments and full multilevel security are later, measured profiles. The nearest practical payoff is integrity: keeping prompt-injected and model-generated material out of trusted state.

---

## Summary of claims

1. **An agent is a durable organizational security principal**, not a PID, model instance, API token, or conversation row. It owns state and carries accountability; it is not the identity under which sessions run.
2. **A session is the task-scoped security and information boundary.** It binds initiator, purpose, activated authority, information state, credentials, resources, visible world, a per-session execution identity, and process tree.
3. **Processes and models are replaceable executions, but the model binding is governed.** Replacing a runtime does not change the session's identity; changing model, endpoint, tenant, or retention mode changes its information sources and trusted base and is an audited policy event.
4. **Authority and information state are different.** Delegation may narrow authority, but a fresh child or peer does not cleanse the confidentiality or integrity provenance of its inputs.
5. **Durable memory must be partitioned.** Ownership by one agent does not make every historical memory partition safe to import into every session.
6. **Integrity requires stage, validate, and promote.** Prompt-injected or model-generated material must not directly replace trusted configuration, approved memory, or production artifacts.
7. **Linux enforces the locally realizable boundary.** Gateways, workload identity, and remote reference monitors enforce model, database, Git, SaaS, and other network effects, and attribution extends only to effects those components mediate.
8. **The architecture is tiered and each tier claims only its mediated edges.** Unix-governed sessions are an isolation, authority, and attribution profile practical now; compartments add labeled communication and partitioned memory; full MLS adds trusted release at a cost that must be measured.
9. **Containers and microVMs are complementary.** They strengthen workload isolation but do not by themselves classify outputs, partition memory, constrain remote services, or provide trusted release.

The central rules are, for the common case where a human or service delegates a subset of its own authority to an agent for one task, and for every **mediated** communication edge (higher `T` meaning more trusted):

```text
A_session  ⊆ P_agent ∩ A_initiator ∩ A_task ∩ A_policy
I_receiver ⪰ join(confidentiality labels of every admitted input)
T_receiver ⪯ meet(integrity labels of every admitted input)
untrusted input → staging → validation or review → trusted promotion
```

Intersection is the delegation case of a more general derivation relation defined in the companion report; approvals, quorum rules, scheduled initiators, and multi-caller service agents need other compositions.

**Status of the formalism.** These rules and the companion report's typed derivation and admission relations are a specification written in prose and notation. They have not been formalized as a transition system, mechanized, or proven to satisfy any noninterference or integrity property. They are stated precisely enough to be implemented and tested against, and their formalization is an explicit later deliverable, not a completed result.

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

This creates recurring mismatches: a framework UUID identifies the agent while the filesystem records every write as the daemon user; retrieval policy hides a document the process credential can still open; “spawn subagent” creates another object with whatever authority the framework assigns rather than a descendant with non-increasing authority; classification ends at retrieval although prompts, transcripts, caches, child messages, and model requests all carry the retrieved information; and the audited component emits its own tool log while direct syscalls and hidden subprocesses go unrecorded.

Mature platforms already improve this with containers, workload identities, application RBAC, short-lived credentials, and structured audit. Application policy does not disappear; organizational IAM, purpose constraints, approvals, and resource catalogues are still required. The narrower division of labor is:

```text
organizational policy → computes what may be activated
host and gateway controls → enforce the resulting boundary
```

The daemon should not need to mediate every `open(2)` call for its authorization decision to be real.

---

## 2. Ontology: principal, session, execution, model

### 2.1 Agent principal

An **agent** is a durable organizational security principal with a stable global identity and accountable owner; potential authority and clearance; partitioned durable state; credential, model, and tool eligibility policy; delegation, retention, and audit policy; and a provisioning-to-retirement lifecycle.

Across a fleet, a directory or workload-identity system remains authoritative. On one host, the durable identity may be projected into a stable UID (or a storage service) that **owns** durable state so that ordinary ownership, quota, backup, and audit tooling attribute objects to the agent. That identity never executes session code; sessions run under separate execution identities (Section 2.2) and reach the principal's state only through per-session grants.

The cognitive software is not the principal. A model or runtime may plan and act, but it does so within authority assigned to the governed identity.

### 2.2 Session

A **session** is a temporary, task-scoped realization of an agent principal. It binds the authenticated initiator; task, purpose, and approvals; activated authority; confidentiality and integrity state; filesystem, process, IPC, device, and network views; brokered credentials; budgets; and the process tree, streams, outputs, and audit identifiers. It is the natural cognitive security boundary because information read during one task may enter prompts, caches, memory, outputs, or child messages; a task requiring incompatible compartments should receive another session.

Two sessions of one agent are not isolated merely by having different session IDs or cgroups: a shared UID passes ordinary access and signal checks between its own processes, and namespaces hide identifiers without changing authorization. Each session therefore runs under a **per-session, non-reusable execution identity**—its own UID and groups, and in MAC profiles its own type—with private runtime directories, storage, descriptors, PTYs, and sockets. The durable identity owns state; the session identity acts.

### 2.3 Execution and model

An **execution** is an ordinary process. Shells, retrieval commands, compilers, cognitive runtimes, and local model clients are executions.

A **model** is a cognitive implementation invoked by an execution. It may be replaced without changing the agent or session identity, but replacement is not security-neutral: model, endpoint, provider tenant, adapters or fine-tuned weights, retention mode, and inference pool form the session's **approved execution binding**, and changing any of them changes where information can go and what is trusted. A remote or shared model is also a governed service boundary whose caches, batching, logs, accelerator memory, and tenant state must not silently cross information domains. A change of binding is a policy-controlled, audited event.

This yields the hierarchy:

```text
Organization
├── Human and service initiators
└── Agent principal (global identity + policy + partitioned state)
    └── Session (initiator + task + authority + information state)
        ├── execution identity + namespaces + cgroup + optional MAC context
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

The constructor derives `A_session` from agent, initiator, task, approvals, and current policy. Clearance is a ceiling, not ambient access, and it is a different kind of value from authority: one is a position in a confidentiality ordering, the other a set of permitted operations.

Information state follows different rules. If a session reads Finance+Redwood material, a child with fewer filesystem permissions may still receive Redwood information in its prompt: reduced authority limits future access but does not declassify what was already observed. Likewise a fresh model process does not restore integrity if its instructions derive from a malicious repository or web page.

For every communication edge—parent to child, sibling to sibling, agent to agent, pipe, socket, queue, RPC, file, artifact, or model call—the receiver must be authorized for the channel, its confidentiality domain must dominate the input label, and the input's provenance must not be silently promoted. Otherwise the input is rejected, quarantined, routed to a compatible session, or passed through a trusted release or validation boundary:

> **Every communication edge is an information-admission event. A new process boundary is not an information-cleansing boundary.**

The principle is a design obligation, not a description of what any profile enforces everywhere. A profile enforces admission only on the edges a named component mediates and must publish which edges those are. The baseline profile mediates authority, execution-world separation, credentials, and gateway operations—not the content of prompts, tool arguments, intra-session pipes, or model responses—and so makes no confidentiality- or integrity-propagation claim. The unmediated edges are where the semantic information-flow problem still lives; the ontology exposes them rather than hiding them.

Purpose also has a limit. It can determine which session is launched, which credentials are issued, and which gateway operations are available; the kernel cannot infer whether a permitted read is genuinely for “quarterly forecasting.” In audit records purpose is an authorization attribute, never evidence that an action was appropriate.

---

## 4. Durable memory and integrity

### 4.1 Partitioned memory

A single undifferentiated agent home is incompatible with task-scoped information domains. If a Finance agent stores Redwood, Bluebird, and acquisition summaries in one database, that database acquires the join of those domains and cannot be imported into a lower session merely because the same agent owns it.

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

Concurrent sessions import stable snapshots and append proposed updates with writer identity, labels, and provenance rather than mutating approved memory in place; validation or review—not locking—controls promotion.

### 4.2 Stage, validate, and promote

Confidentiality controls do not stop an authorized but prompt-injected agent from corrupting an artifact it may write. Practical integrity separates imported material, untrusted parsing, model-generated proposals in staging, validated artifacts, reviewed outputs, and protected policy, credentials, configuration, and approved memory. The default workflow is not direct mutation of trusted objects:

```text
stage → validate → review where necessary → promote
```

Promotion may require tests, schemas, reproducible transformations, independent review, branch protection, or a constrained trusted service. Confidence expressed by a model is not an integrity transition.

Validation is well-defined for structured state—configuration, schemas, test results, reproducible outputs—and undefined for natural-language memory: no validator can show a model's summary is free of confidential inference, injected instruction, or hallucination. Semantic observations therefore stay append-only, provenance-bearing, and visibly untrusted. This is the most immediately useful part of the architecture. Most real agent incidents are integrity failures—hostile input reaching a repository, deployment, or durable memory—not confidentiality-label failures, and stage–validate–promote addresses them without mandatory-access-control machinery.

### 4.3 Trusted release and aggregation

A process that reads a high domain should produce high-domain output by default. Downgrade requires a separate trusted workflow: deterministic transformation, policy-approved redaction, constrained declassifier, or human review. Ordinary agents must not label their own output downward.

Declassification is also an economic system. Conservative joins can make every useful output require review, creating bottlenecks and eventual rubber-stamping; a viable deployment must measure release volume, latency, correction rates, reviewer agreement, and automated-release coverage. Releases also cannot always be judged independently: individually low-risk outputs may jointly disclose protected information, so release services should consider cumulative history. This mitigates but cannot prove the absence of semantic inference.

---

## 5. Unix realization and its boundary

A narrow trusted constructor can translate an authenticated organizational decision into:

```text
authenticate initiator
→ resolve agent principal
→ validate task, purpose, and approvals
→ derive authority and initial information domain
→ allocate a per-session execution identity, groups, capabilities, and optional MAC context
→ construct namespaces, mounts, devices, and descriptors
→ create cgroup and resource limits
→ attach the network path to the gateway only
→ issue or broker narrow service credentials
→ bind audit and workload identity
→ apply no_new_privs, Landlock, and seccomp where appropriate
→ dispose of launch privilege
→ exec the cognitive runtime or shell
```

Linux mechanisms answer different questions:

| Mechanism | Role |
|---|---|
| Global identity projected to an owning UID or storage service | Durable ownership of state; never executes |
| Per-session execution identity (UID, groups, optional MAC type) | Session boundary between processes, including sessions of one agent |
| SID, cgroup, immutable launch record | Task-scoped session grouping and provenance |
| PID | Current execution identity |
| Namespaces and descriptors | Visible and connected world |
| Capabilities and `no_new_privs` | Privilege reduction and anti-escalation |
| Landlock and seccomp | Inherited self-restriction and attack-surface reduction |
| DAC, ACLs, host MAC, and SELinux MCS/MLS where deployed | Local authorization; compartments and declared multilevel domains |
| Linux Audit plus service logs | Process attribution and denial evidence |

A manifest is a derived launch record, not an entitlement supplied by the agent; authoritative identities, labels, mounts, credentials, and endpoints come from a server-side policy decision, and anything unavailable fails closed. The launch path can dispose of its privilege; the system cannot. Termination, revocation, cleanup, and audit remain privileged operations of small, separately authorized services. The honest claim is that the trusted computing base is enumerable and reviewable, not that it vanishes.

A network namespace by itself denies nothing, and hostnames are not policy. "Gateway-only egress" is a topology—one interface, host-side filtering that permits only the gateway, no raw-socket capability, no inherited connections—ending in a gateway that exposes named, typed operations rather than a generic proxy and verifies service, tenant, model, retention mode, credential scope, and budget. Bearer tokens remain copyable even when short-lived; proof-of-possession credentials or operation brokers reduce that risk.

Linux does not replace remote reference monitors:

| Effect | Local enforcement | Remote enforcement |
|---|---|---|
| Local file/process/IPC access | Credentials, namespaces, MAC, Landlock, descriptor policy | Policy administration |
| Model call | Reachability and credential confinement | Model/tenant selection, retention, spend, audit |
| Database query | Endpoint and credential confinement | Database authorization and query audit |
| Git or CI write | Endpoint and credential confinement | Branch protection, review, CI, repository policy |
| SaaS action | Endpoint and token confinement | Service authorization and audit |
| Release | Prevention of ordinary write-down | Trusted transformation, review, approval |

The end-to-end boundary is composed from host controls, gateways, workload identity, remote authorization, and correlated audit. Kernel audit is correlation evidence, not causal proof, and cannot see inside an authorized connection; the attribution claim is therefore scoped to **mediated effects**—local objects, process lifecycle, and gateway operations carrying a propagated session trace identity—reconstructed from a signed launch record.

---

## 6. Deployment profiles

The ontology does not require every deployment to adopt full SELinux MLS.

### Profile 1: Unix-governed session

Global agent identity with per-session execution identities, private namespaces and storage, cgroups, capability removal, `no_new_privs`, optional Landlock/seccomp and host MAC (SELinux or AppArmor), gateway-only egress, brokered credentials, and attributable launch records. Practical with current Linux and systemd machinery.

It is an **isolation, authority, and attribution profile, not an information-flow profile**: it bounds what a session can reach and do, keeps sessions of one agent apart, confines credentials, and attributes mediated effects. It does not label or propagate the content of prompts, pipes, or model responses, and its integrity claim is limited to objects behind a gateway or branch-protection boundary.

### Profile 2: Compartmented session

Adds MCS or equivalent compartments, partitioned memory, labeled channels, category allocation, and controlled import/export. The first profile that claims contamination-safe communication, and only on the edges it publishes as mediated.

### Profile 3: Multilevel session

Adds full MLS sensitivities, declared flow rules, analyzed policy, labeled persistence, and trusted declassification. A high-assurance option with a small ecosystem, whose policy correctness, administrative burden, and release economics must be measured rather than assumed.

### Profile 4: Strong workload isolation

A container, VM, microVM, or dedicated node can surround any profile. A microVM gives a simpler, stronger boundary against cross-workload kernel and device interference at a cost in latency, memory, and observability; it does not determine where information may flow after entering the VM. A likely high-assurance design is organizational identity and task policy → microVM session boundary → partitioned storage, gateways, promotion, and release. The choice should be empirical: the evaluation programme treats a hardened container or microVM with per-session workload identity, gateway, and audit as a required control arm, and the process-session design must show a measured advantage or cost, not an assumed one.

---

## 7. Relationship to prior work

This proposal composes established operating-system security ideas with recent Unix-native agent systems.

Quine identifies an agent with a POSIX process.[^quine] Orkia treats persistent agent sessions as governed shell jobs in PTYs.[^orkia] Agent-as-user projects use dedicated Unix credentials.[^aaau] agentsh places enforcement beneath agent tools.[^agentsh] These support Unix-native execution but do not define the combination of durable organizational principal, initiator-bound session, information contamination, partitioned memory, remote workload identity, and trusted promotion proposed here.

Decentralized information-flow-control systems are the closest academic ancestors. Myers and Liskov's decentralized label model and Jif address confidentiality, integrity, and owned declassification.[^dlm] Asbestos and HiStar explore kernel-enforced information flow with small trusted components.[^asbestos][^histar] Flume is a particularly close neighbor, adding process-level DIFC to familiar Linux abstractions such as pipes, sockets, and descriptors.[^flume]

This paper does not claim to invent label joining, integrity labels, process-level information flow, or controlled declassification. The confidentiality rules descend from Bell–LaPadula and Denning, the integrity rules from Biba, and stage–validate–promote from Clark–Wilson.[^blp][^biba][^clark-wilson] Its narrower contribution is their composition around a durable organizational agent identity, task-scoped session, governed model binding, partitioned memory, service egress, and attributable delegation, projected onto deployable Linux and gateway mechanisms. The companion report carries a comparison matrix against DIFC, SELinux MLS, and workload-identity systems.

The strongest objection is that this repackages DIFC, capability, and MLS ideas in agent terminology while deferring the hard parts—dynamic flow mediation and economically viable declassification. This paper does not claim to solve those parts. It claims that the ontology gives organizational policy, local enforcement, remote identity, and accountability a disciplined place to bind, is useful without information-flow machinery, and exposes the remaining semantic problem rather than hiding it inside an agent framework. SELinux MLS is valuable because it is not agent-specific, but it performs no semantic taint tracking and a targeted installation is not the multilevel profile.

---

## 8. Adoption and evaluation

Adoption need not begin with MLS or a new agent platform. The scenario to picture is ordinary: a developer runs an existing coding-agent harness against a repository; the harness reads a hostile issue comment and is induced to read the developer's credentials, push to `main`, and send the repository to an external host. Under a governed session all three fail—the credentials are not in the session's world, the only push path is a typed gateway operation limited to a staging branch, and there is no route except to the gateway—and the audit table names developer, agent, session, process, and staged push. Promotion to `main` happens outside the session, by CI and review.

An existing harness can move incrementally toward that profile by:

0. running unmodified under a session wrapper that supplies a working tree, hides credentials, and routes Git and model access through a gateway;
1. assigning explicit global agent identity and a per-session execution identity;
2. constructing a private namespace and cgroup per session;
3. removing ambient capabilities and setting `no_new_privs`;
4. isolating sessions that share the same durable principal;
5. issuing narrow credentials through an authenticated, typed gateway;
6. binding process audit to an immutable launch record and propagated trace identity;
7. separating untrusted staging from trusted promotion;
8. partitioning durable memory before adding richer classification.

Compartments, stronger MAC, and trusted release follow where risk justifies their cost. The approach extends existing harnesses rather than requiring them to become classified operating systems at once.

### 8.1 What remains for the agent framework

This architecture does not eliminate the harness; it narrows its mandate to work specific to cognition and agent behavior: forming cognitive context, invoking models, representing goals and structured tool requests, deciding when to delegate within an externally enforced ceiling, externalizing proposed memory before context loss, requesting authority changes through explicit policy interfaces, and participating in validation and review without controlling its own promotion or release. It may also provide orchestration, durable queues, retries, and distributed placement where single-host process semantics are insufficient. It should not be the sole reference monitor for resources the host or a service can enforce directly.

Unix composition remains useful, but a pipe is not merely a convenience in a governed system:

```sh
produce-material | agent-runtime "analyze" | review-filter
```

Each endpoint, inherited descriptor, and process must belong to a compatible confidentiality and integrity domain. Shell-native composition makes policy visible at process boundaries; it does not abolish that policy.

### 8.2 Evaluation path

The companion technical report defines a three-phase evaluation:

- **Phase 1:** Unix-governed identity, construction, same-principal isolation, resources, credential confinement, gateway-only egress, termination, attribution of mediated effects, one thin integrity slice, and a required container/microVM control arm. Phase 1 makes no information-flow claim.
- **Phase 2:** integrity provenance and partitioned/versioned memory first; then compartments, contamination-safe communication on mediated edges, revocation, and external budgets.
- **Phase 3:** full MLS, declassification throughput, storage continuity, shared inference, and comparison with microVM-per-session.

The most important falsification questions are:

- Does the host prevent cross-principal and same-principal cross-session interference?
- Can narrower delegation launder confidentiality or integrity across a mediated edge?
- Does the process-session design show a measured advantage over a hardened container or microVM control arm?
- Can a session bypass the model/service gateway or steal reusable credentials?
- Can untrusted input modify trusted memory or production artifacts without promotion?
- Can labels and provenance survive the actual Git, archive, backup, and remote-service workflows an organization uses?
- Can administrators and reviewers operate the policy without broad exceptions, continual privileged repair, or rubber-stamped release?

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

- **agent:** durable, accountable organizational principal that owns partitioned state and never executes;
- **session:** task-scoped process world under its own execution identity, with activated authority, information state, provenance, credentials, and budgets;
- **execution:** ordinary process;
- **model:** replaceable cognitive component whose binding to the session is governed and, when shared or remote, a governed service boundary;
- **organizational policy:** computes the permissible session and lifecycle response;
- **Unix and optional MAC:** enforce the local realization;
- **gateways and remote services:** enforce effects beyond the host;
- **trusted promotion and release:** govern integrity elevation and confidentiality downgrade.

The ontology and baseline profile are viable with mechanisms deployed today, as an isolation, authority, and attribution boundary. Integrity provenance is the nearest payoff and needs no mandatory-access-control machinery. Compartments require real engineering; full MLS is an option whose release economics decide its viability; VMs and microVMs must be measured against rather than assumed inferior to. The formal rules remain a specification to be implemented, tested, and eventually formalized—not a proven model.

The central contribution is the distinction between authority and information state. Authority can decrease when work is delegated; confidentiality and integrity provenance do not disappear because a new process, model, or agent object was created. If this principle is projected into enforceable local boundaries, partitioned memory, explicit communication edges, and remote reference monitors, agent systems can stop rebuilding weaker operating-system abstractions inside every harness while remaining honest about what Unix does not solve.

---

## References

[^blp]: D. Elliott Bell and Leonard J. LaPadula, [*Secure Computer Systems: Mathematical Foundations*](https://apps.dtic.mil/sti/citations/AD0770768), MITRE, 1973; Dorothy E. Denning, [“A Lattice Model of Secure Information Flow,”](https://doi.org/10.1145/360051.360056) *CACM*, 1976.

[^biba]: Kenneth J. Biba, [*Integrity Considerations for Secure Computer Systems*](https://apps.dtic.mil/sti/citations/ADA039324), MITRE, 1977.

[^clark-wilson]: David D. Clark and David R. Wilson, [“A Comparison of Commercial and Military Computer Security Policies,”](https://doi.org/10.1109/SP.1987.10001) *IEEE S&P*, 1987.

[^quine]: Hao Ke, [“Quine: Realizing LLM Agents as Native POSIX Processes,”](https://arxiv.org/html/2603.18030v2) 2026.

[^orkia]: Orkia, [documentation](https://orkia.dev/docs/) and [source](https://github.com/orkiaHQ/orkia).

[^aaau]: AgentaaU, [AaaU: Agent-as-User Architecture](https://github.com/AgentaaU/AaaU).

[^agentsh]: agentsh, [Execution-Layer Security documentation](https://www.agentsh.org/docs/).

[^dlm]: Andrew C. Myers and Barbara Liskov, [“Protecting Privacy using the Decentralized Label Model,”](https://cs.cornell.edu/andru/papers/iflow-tosem.pdf) *ACM TOSEM* 9(4), 2000.

[^asbestos]: Petros Efstathopoulos et al., [“Labels and Event Processes in the Asbestos Operating System,”](https://doi.org/10.1145/1095809.1095813) *SOSP*, 2005.

[^histar]: Nickolai Zeldovich et al., [“Making Information Flow Explicit in HiStar,”](https://www.usenix.org/legacy/event/osdi06/tech/zeldovich.html) *OSDI*, 2006.

[^flume]: Maxwell Krohn et al., [“Information Flow Control for Standard OS Abstractions,”](https://dl.acm.org/doi/10.1145/1323293.1294293) *SOSP*, 2007.
