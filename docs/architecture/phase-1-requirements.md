# Phase 1 Normative Requirements

**Version:** 0.7  
**Status:** Frozen (WP0)  
**Date:** 28 August 2026  
**Governs:** milestones 1A–1D of the [Phase 1 plan](../plans/phase-1-reference-implementation.md)  
**Source of invariants:** [technical report](../papers/technical-report.md) §7, profile U column  
**Companion WP0 artefacts:** [manifest schema](manifest-schema.md), [session lifecycle](session-lifecycle.md), [execution-identity lifecycle](execution-identity-lifecycle.md), [component interfaces](component-interfaces.md), [test catalogue](test-catalogue.md), [ADR-0001](ADR-0001-execution-identity.md), [ADR-0002](ADR-0002-gateway-authentication.md), [ADR-0003](ADR-0003-control-substrate.md), [traceability matrix](traceability-matrix.md)

## Revision history

- **0.1** — Initial WP0 draft.
- **0.2** — Local-socket topology only (ADR-0002 0.2); `agentbound-lifecycle` daemon replaces the systemd-invoked helper; `loginuid` made corroborating evidence with one fail rule; storage bounds restated for tmpfs/images; quiesce redefined; termination deadline; SLOC accounting rules; attribution metric referenced to the test catalogue; bypass-corpus rule made non-tautological; resource-class milestone matrix; comparative decision rule referenced to ADR-0003.
- **0.3** — R-ID-8 restated with the two identifiers; R-GW-1 covers `channel_topology: none` at 1A; pinned versions aligned with ADR-0003 0.3.
- **0.4** — Open questions disposed per the open-question register; answers written into the normative text. Evaluation-arm manifests: attribution `required`, audit-loss *stop*; `nosuid,nodev` plus image digest; per-key approval sequence; sixth SLOC figure.
- **0.5** — R-LC-2 restriction on `continue-degraded`; R-LC-3 trigger list uses the split outage vocabulary.
- **0.6** — Post-freeze editorial maintenance (no normative change): R-CON-1 rollback list names the gateway-socket projection instead of a firewall rule; host firewall removed from the trusted list.
- **0.7** — Editorial pass under docs/STYLE.md; no obligation, identifier, or value changed. R-ISO-4 and §12 prose split; Oxford spelling.


---

## 0. Conventions

The key words MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are used as in RFC 2119. Each requirement has an identifier `R-<area>-<n>`, a milestone at which it becomes testable, and the technical-report invariant(s) it serves. A requirement tagged **[assumption]** is not enforced by the system; it is a documented precondition whose owner and revalidation trigger appear in the evidence table.

Requirements are stated for the Linux arm. ADR-0003 defines which apply identically to the microVM control arm.

---

## 1. Scope and claim

**R-SCOPE-1 (1A).** Phase 1 claims conformance with the Unix-governed profile only. Every published result MUST state the milestone reached. Every published result MUST list the invariants demonstrated, not demonstrated, not evaluated, and not applicable, using the technical report's five result classes.

**R-SCOPE-2 (1A).** Phase 1 MUST NOT claim any confidentiality- or integrity-propagation property on any communication edge. Its sole integrity-flow claim is the protected-object staging and promotion boundary (Invariant 19, protected-object subset), testable from 1B.

**R-SCOPE-3 (1A).** A successful milestone 1A MUST NOT be reported as "Phase 1 succeeded" or "Profile U conformant".

---

## 2. Threat model (scoped)

The in-scope adversary and exclusions are those of technical-report §8, restricted as follows for Phase 1.

**In scope.** An adversary who controls the workload inside a session (the runtime, any child process, any input to them) and may issue any syscall available to the session; an adversary who submits arbitrary requests to `agentbound-policy` and `agentbound-launch` as an authenticated but non-administrative initiator; an adversary who controls a concurrent session of the same or a different principal on the same host; an adversary who controls the remote end of an authorized gateway operation's *response*.

**Trusted.** Kernel, systemd, `agentbound-policy` and its catalogue store, `agentbound-launch` and the `agentbound-lifecycle` daemon (which contains the execution-identity allocator), `agentbound-gateway` and its adapters, the launch-record store and signing key, `agentbound-audit`, the Git host's branch protection, and (from 1C) the approved model endpoint.

**Out of scope for Phase 1.** Kernel and hardware vulnerabilities; side and covert channels; malicious administrators; misuse by an initiator of authority they legitimately hold; semantic content of prompts, tool arguments, and model responses; compromise of a remote service after an authorized operation; aggregation across authorized outputs.

**R-THREAT-1 (1A).** The evaluation report MUST include an adversary-capability matrix listing, for each suite in plan §6, the capabilities assumed and the interfaces enumerated. The evaluation report MUST state that untested interfaces carry no claim.

---

## 3. Identity and derivation

**R-ID-1 (1A) [Inv 1].** Every session MUST be bound to exactly one registered durable agent principal resolved by `agentbound-policy` from its registry; a request naming an unregistered principal MUST be rejected before any privileged operation.

**R-ID-2 (1A) [Inv 2].** Every session MUST record the authenticated initiator(s), every approver, and—for scheduled sessions—the scheduling principal and its accountable owner. A scheduled session without an accountable owner MUST be rejected.

**R-ID-3 (1A) [Inv 3].** `Auth_session` MUST be produced by `agentbound-policy` through the derivation relation of the technical report. `Auth_session` MUST be a subset of `Auth_agent` and of the task's policy-permitted authority. The launch record MUST record `Auth_session` with every derivation input identity and version.

**R-ID-4 (1A) [Inv 3].** Derivation MUST fail closed on any unauthenticated, expired, revoked, replayed, or unknown input. Derivation failure MUST emit an audit event naming the failed input. Approval objects MUST carry an expiry and a per-approver-key monotonic sequence. `agentbound-policy` MUST persist the highest accepted sequence per key in its append-only store and reject any lower or equal value.

**R-ID-5 (1A) [Inv 3].** A recipient-issued grant (for example a scoped token supplied by a service) MUST NOT expand `Auth_session` beyond `Auth_agent`.

**R-ID-6 (1A) [ADR-0001].** Every session MUST run under a per-session execution identity allocated by the constructor from the reserved range in the [execution-identity lifecycle](execution-identity-lifecycle.md). No two concurrent sessions MAY share an execution identity. The durable principal's owning UID MUST NOT execute session code.

**R-ID-7 (1A) [ADR-0001].** An execution identity MUST be reclaimed only when the reclamation condition holds across the declared managed domain, followed by quarantine. Objects exported beyond that domain MUST carry the global principal and session identifiers. Objects exported beyond that domain MUST NOT depend on the numeric UID for durable authorization.

**R-ID-8 (1A) [Inv 14].** The launch record is the verified pair of a policy-signed authorization manifest and a constructor-signed launch binding (manifest schema §3–4). Its policy-issued `authorization_id` is the pre-binding key. Its authoritative post-binding identity is `launch_record_digest = SHA-256(authorization_manifest_digest || launch_binding_digest)`. The manifest schema §4 table fixes which identifier each component uses. The authorization manifest MUST carry the policy version, catalogue version, and derivation inputs. Each object MUST be signed with a key whose identity and custody are stated in the evaluation report.

---

## 4. Request and manifest

**R-REQ-1 (1A) [Gate 1].** `agentbound-launch` MUST accept only a policy-signed, allocation-free **authorization manifest** produced by `agentbound-policy`. `agentbound-launch` MUST NOT accept an untrusted request directly. After validating the manifest, the allocator MUST atomically reserve an identity. `agentbound-launch` MUST produce a constructor-signed **launch binding** containing host allocation and substrate projection. The immutable launch record is the pair. Its authoritative identity is `SHA-256(authorization_manifest_digest || launch_binding_digest)`.

**R-REQ-2 (1A) [Gate 1].** The untrusted request MUST conform to the [request schema](manifest-schema.md): unknown, duplicate, or forbidden fields (numeric UIDs, paths, mount sources, labels, credential material, network addresses, capabilities, namespace settings) MUST cause rejection, as MUST requests exceeding the size and nesting bounds.

**R-REQ-3 (1A) [Gate 1].** Both records MUST be canonically encoded and independently signed. The constructor MUST verify the policy signature before allocation. The gateway and audit pipeline MUST verify the constructor signature and its binding to the policy digest. Digest mismatch, unsupported schema, duplicate launch binding, or rollback after a committed binding MUST fail closed.

**R-REQ-4 (1A) [Gate 1].** Every field of the manifest MUST be tagged substrate-independent or substrate-specific per the schema. Only substrate-independent fields MAY be shared with the control arm.

**R-REQ-5 (1A) [Gate 1].** The constructor MUST validate the manifest fully before performing any irreversible operation (validation rules in the manifest schema §5), including: all identities resolvable; after atomic allocation, the selected execution identity uniquely reserved and not quarantined; every mount source in the catalogue, descriptor allowlist closed, exactly one channel topology, every gateway operation in the adapter catalogue, policy and catalogue versions current, approvals unexpired.

**R-REQ-6 (1A) [Gate 1].** The constructor MUST resolve mount sources through descriptor-relative, symlink-safe operations (`openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS`, or mount file descriptors). The constructor MUST NOT re-walk a string path between validation and use.

---

## 5. Construction

**R-CON-1 (1A) [Inv 7].** Construction MUST follow the ordering in technical-report §2.1 as specified in the [session lifecycle](session-lifecycle.md) §3, using a `clone3` synchronization barrier (no kernel facility creates a stopped child). Any step that cannot be completed MUST abort the launch. An aborted launch MUST roll back every completed irreversible step. An aborted launch MUST leave no runnable process, usable credential, live grant, mounted resource, gateway-socket projection, or unsealed launch record.

**R-CON-2 (1A) [Inv 7].** The mount namespace MUST be marked recursively private before any bind. `pivot_root` MUST be used (not `chroot`). `proc` MUST be mounted only after the PID namespace exists. Host `/proc` MUST NOT be visible.

**R-CON-3 (1A) [Inv 6, 15].** Before `exec`, the constructor MUST close every descriptor not on the allowlist. Before `exec`, the constructor MUST install the execution identity and supplementary groups. Before `exec`, the constructor MUST drop the capability bounding set to the manifest's set (empty by default). Before `exec`, the constructor MUST set `no_new_privs`. Before `exec`, the constructor MUST apply the Landlock and seccomp policies named in the manifest. Seccomp filters MUST be installed with `SECCOMP_FILTER_FLAG_TSYNC`.

**R-CON-4 (1A) [Inv 6].** The session's world MUST contain no set-UID or set-GID executables (every session mount is `nosuid,nodev` and the runtime image digest is verified against the catalogue), no file capabilities, no writable path into the cgroup hierarchy, no systemd or container-runtime socket, and no broker socket other than the one named by the channel topology.

**R-CON-5 (1A) [Inv 15].** Every launch-only capability (`CAP_SETUID`, `CAP_SETGID`, `CAP_SYS_ADMIN`, `CAP_AUDIT_CONTROL`, `CAP_MAC_ADMIN`, and any other held) MUST be dropped before untrusted code executes. The evaluation report MUST enumerate the post-launch privileged operations of `agentbound-lifecycle`.

**R-CON-6 (1A) [Inv 13].** `loginuid` is **corroborating** audit metadata, not the authoritative session attribution key; that key is the signed launch record correlated with (execution UID, boot ID, PID namespace, process start time or pidfd). The constructor MUST attempt to set `loginuid` in the child before `exec` when the pinned host baseline permits it and the child's value is unset; the result (set, immutable, already-set, denied) MUST be recorded in the launch binding. When the manifest's attribution policy is `required` (mandatory for every Linux evaluation-arm manifest) and `loginuid` cannot be set, construction MUST fail; otherwise the condition is a recorded residual assumption. No other behaviour is permitted.

**R-CON-7 (1A).** Credentials and broker access MUST become usable only after every boundary is in place and the launch record is committed. The runtime MUST be exec'd last.

**R-CON-8 (1A) [Gate 1].** The privileged constructor and `agentbound-lifecycle` daemon together MUST NOT exceed the reviewability bound in §12 under its SLOC accounting rules; exceeding it is a milestone 1A stop condition.

---

## 6. Isolation and descendant control

**R-ISO-1 (1A) [Inv 17].** A session MUST NOT be able to observe, signal, trace, read the memory of, attach to, or open the private state of any concurrent session of the same or a different principal through any interface in the enumerated inventory (plan §6.1), including `/proc/<hostpid>`, `kill`, `pidfd_open`/`pidfd_send_signal`, `ptrace`, `process_vm_readv`/`writev`, `/run` and `/tmp` paths, pathname and abstract Unix sockets, shared supplementary-group permissions, broker socket reuse, and inherited descriptors (including reintroduction via `SCM_RIGHTS`, `/proc/self/fd`, and memfd).

**R-ISO-2 (1A) [Inv 17].** Each session MUST have its own mount, PID, IPC, UTS, and network namespace (the latter containing no interface, including loopback, and no route), private `/tmp` and runtime directory, private PTY if any, and private workspace mount.

**R-ISO-3 (1A) [Inv 12].** All session processes MUST remain in one systemd scope (cgroup v2) with an in-session PID-namespace init acting as subreaper. `agentbound-lifecycle` MUST hold a pidfd for the init for the session's lifetime. No session process MAY hold a writable descriptor to any cgroup file or a path to the system manager.

**R-ISO-4 (1A) [Inv 12].** Termination MUST follow the eleven-step protocol in the session lifecycle §5 in order: (1) deny gateway admission, (2) freeze, (3) `SIGTERM` with a bounded thaw so the PID-namespace init reaps, (4) refreeze and `cgroup.kill`, (5) confirm `cgroup.procs` is empty, the init has exited, and the host credential scan finds no process under the execution UID/GIDs, (6) release grant records and close connections, (7) close broker access, (8) unmount, (9) remove the gateway socket, (10) release the identity to reclamation, (11) seal. No grant-record or identity release may precede step 5. Early revocation to contain an immediate remote effect is permitted only as an audited ordering deviation. Tasks in uninterruptible sleep MUST delay termination, recorded as *termination-incomplete*; a session still incomplete at the manifest termination deadline is a non-pass for the affected test; the execution identity MUST NOT be released while any process is live.

**R-ISO-5 (1A) [Inv 6].** A child session or process delegated from a running session MUST receive authority that is a subset of the parent's on every axis (mounts, descriptors, gateway operations, budgets). A child session or process delegated from a running session MUST have no path to recover the parent's authority.

---

## 7. Gateway, egress, and credentials

**R-GW-1 (1B) [Inv 10].** Under `gateway.channel_topology: local-socket` the session MUST have exactly one channel to `agentbound-gateway`: one bind-mounted `AF_UNIX SOCK_SEQPACKET` socket (ADR-0002), and no network interface, route, or other host socket. Under `none` (the only form constructible at 1A) the session has no channel at all and the same no-network, no-host-socket boundary. The network topology is not part of Phase 1.

**R-GW-2 (1B) [Inv 10].** Seccomp MUST deny every socket family except `AF_UNIX`; the inherited-socket set MUST be empty; every Unix socket other than the manifest's gateway socket, pathname or abstract, MUST be unreachable; `SCM_RIGHTS` on the gateway protocol MUST be rejected.

**R-GW-3 (1B) [Inv 10, 11, 13].** `agentbound-gateway` MUST authenticate each connection to a single execution identity and launch record (`SO_PEERCRED` plus pidfd), MUST attribute **each operation** to a live process via the per-packet `SCM_CREDENTIALS` rules of ADR-0002 Decision 2, MUST refuse unauthenticated or unmapped connections and packets, and MUST invalidate connections at revocation and termination per ADR-0002 Decision 4.

**R-GW-4 (1B) [Inv 10].** The gateway MUST expose only named, typed operations from the adapter catalogue. The gateway MUST NOT act as an HTTP or CONNECT proxy or forward arbitrary bodies. Each operation MUST authorize destination, method, argument schema, tenant, and response size. Each operation MUST authenticate its upstream TLS peer. Each operation MUST propagate the session trace identity to the upstream service.

**R-GW-5 (1B) [Inv 19].** The Git adapter MUST accept pushes only to `refs/agentbound/<session>/…` staging refs of the manifest-named repository and MUST refuse pushes to any other ref, to another session's staging ref, or with a trace identity that does not match the authenticated session. Protected-branch enforcement at the Git host is a **[assumption]**.

**R-GW-6 (1B) [Inv 11].** No Git, cloud, model, or API credential MAY be present in the session's environment, filesystem, or descriptors. Bearer tokens MUST NOT be the primary gateway-authentication mechanism. The evaluation report MUST state, per credential, whether it is exportable from within the session.

**R-GW-7 (1B) [Inv 20].** The gateway MUST enforce the manifest's budgets for every present gateway resource class—request rate and count, bytes, network bandwidth, concurrent connections, fan-out, storage, and monetary quota. The gateway MUST record classes absent from the deployment.

**R-GW-8 (1C) [Inv 22].** The inference adapter MUST carry the session identity on every request, MUST bind requests to the approved execution binding in the manifest, and MUST refuse a change of model, endpoint, tenant, adapters, inference pool, or retention mode that is not approved for the task. Every binding change MUST produce an audit event.

**R-GW-9 (1C) [Inv 20].** From 1C the gateway MUST additionally enforce token and inference-spend budgets.

---

## 8. Resource governance

**R-RES-1 (1A) [Inv 20].** The manifest MUST use the closed resource schema in `manifest-schema.md` §3.5. The manifest MUST classify every resource class as *applicable and enforced* or *absent with evidence*. Unknown resource classes MUST be rejected.

**R-RES-2 (1A) [Inv 20].** `agentbound-launch` MUST install cgroup v2 limits for PID count, CPU, memory, and I/O; rlimits for file descriptors; disk byte and inode bounds via either a per-session filesystem image with fixed capacity and inode count or tmpfs `size=`/`nr_inodes=` (reported as *bounded volatile storage*, never as a project quota); and a bounded audit queue or loss policy before exec. A present class without an enforcement owner is a construction failure.

**R-RES-3 (1A) [Inv 20].** Delegation and child-process fan-out MUST be bounded by the manifest and monotonically decrease under delegation. Limits MUST cover children, concurrent operations, and total delegated sessions where those facilities exist.

**R-RES-4 (1B/1C) [Inv 20].** `agentbound-gateway` MUST enforce R-GW-7 at 1B and token/inference-spend limits at 1C. Accelerator budget MAY be classified absent only when no accelerator is exposed to the session.

**R-RES-5 (1A) [Inv 20].** Resource classes are allocated to milestones as follows. *Absent* means the deployment exposes no such resource at that milestone and the manifest records the evidence; it never means deferred enforcement of an exposed resource.

| Class | 1A | 1B | 1C | Enforcement owner |
|---|---|---|---|---|
| `pids`, `file_descriptors`, `cpu`, `memory_bytes`, `io_bandwidth` | enforced | enforced | enforced | `agentbound-launch` (cgroup/rlimit) |
| `disk_bytes`, `disk_inodes` | enforced | enforced | enforced | `agentbound-launch` (image or tmpfs bounds) |
| `audit_capacity` | enforced | enforced | enforced | `agentbound-audit` |
| `delegation_fanout` | enforced | enforced | enforced | `agentbound-policy` + `agentbound-lifecycle` |
| `network_bandwidth`, `connection_count`, `request_rate`, `storage_bytes`, `external_spend` | absent (no gateway) | enforced | enforced | `agentbound-gateway` |
| `model_tokens` | absent | absent (no inference adapter) | enforced | `agentbound-gateway` |
| `accelerator` | absent | absent | absent unless exposed | — |

---

## 9. Lifecycle and revocation

**R-LC-1 (1A) [Inv 21].** Every state transition of the [session lifecycle](session-lifecycle.md) MUST have an authorized actor, an audit event, a defined idempotency rule, failure cleanup, and an externally observable status.

**R-LC-2 (1A) [Inv 21] [assumption].** For each revocation trigger, the manifest MUST declare one of *terminate*, *quiesce*, or *continue-degraded*; the system MUST implement the declared behaviour. *continue-degraded* is valid only for `policy_service_unavailable` and `audit_pipeline_degraded_below_stop_threshold` (session lifecycle §6); `agentbound-policy` MUST reject any other mapping to it. The choice among permitted values is policy and is recorded as an assumption.

**R-LC-3 (1A/1B/1C) [Inv 21].** Triggers MUST be demonstrated at the milestone where the affected component exists: 1A — initiator disabled, approval expired, authority revoked, policy or catalogue withdrawn, approver cancel, policy service unavailable, audit pipeline degraded below stop threshold, lifecycle daemon unavailable; reclassification request (fail closed and audit when no labelled resource exists in profile U); 1B — Git grant withdrawn, gateway unavailable; 1C — inference grant or binding revoked. Invariant 21 MUST NOT be marked complete before 1C.

**R-LC-4 (1A).** *Quiesce* MUST mean: gateway admission denied, no new attachments or grants, then the session cgroup **frozen** for the manifest-declared bound, after which terminate applies. No-new-child semantics are provided by the freeze alone; a quiesced session is never thawed except by the termination protocol.

**R-LC-5 (1A).** After an `agentbound-lifecycle` restart, every session in any non-terminal state MUST be reconciled using the source precedence in [component interfaces](component-interfaces.md); orphaned sessions MUST be terminated or resumed per their declared behaviour, and identities without a live scope MUST enter reclamation.

---

## 10. Audit and attribution

**R-AUD-1 (1A) [Inv 13].** Every audit event MUST carry host ID, boot ID, authorization ID, launch-record digest, allocation-record ID, session and trace identities, execution UID, monotonic and wall-clock timestamps, actor, and outcome. Kernel process records MUST additionally carry the PID-namespace identity and process start time or pidfd-derived identity where supported; lack of pidfd support MUST be recorded as a residual assumption, never silently replaced by PID alone.

**R-AUD-2 (1B) [Inv 13].** For the defined effect ontology (local objects in the session's world, process lifecycle events, gateway operations), `agentbound-audit` MUST reconstruct `initiator → agent → session → process → effect` and the report MUST state the fraction reconstructed, computed by the metric definition and load profiles in the [test catalogue](test-catalogue.md), against the threshold in §12.

**R-AUD-3 (1A) [Inv 13].** `agentbound-audit` MUST expose loss counters. The manifest MUST declare audit-loss behaviour (*stop*, *quarantine*, or *continue-with-counter*); the profile that claims attribution SHOULD select *stop* or *quarantine*. Every evaluation-arm manifest MUST declare *stop*; *continue-with-counter* is exercised only by T-6.9-007 and reported separately.

**R-AUD-4 (1A).** The launch-record store MUST be append-only with a stated trust anchor, clock source, retention, and correction procedure; corrections MUST be new records referencing the original, never edits.

**R-AUD-5 (1A).** Denial diagnostics returned to the workload MUST name the requirement or policy rule and the launch-record and trace identities. Denial diagnostics returned to the workload MUST NOT disclose other sessions' identifiers.

---

## 11. Integration contract (1C)

**R-INT-1.** An existing coding-agent harness MUST run under `agentbound run --agent <id> --task <id> -- <command>` given only: a per-session working tree at a stable path, stdin/stdout/stderr and an optional PTY, Git remote operations via the gateway staging adapter, and model access via the inference adapter. Any further grant MUST be a reviewed manifest entry.

**R-INT-2.** The same manifest MUST support a local developer mode (single host, file-backed policy) and a CI mode (non-interactive, scheduled initiator with accountable owner).

**R-INT-3.** A harness that cannot run without a grant that voids any Gate 1–3 property is a milestone 1C stop condition.

---

## 12. Pre-registered thresholds

These values are fixed before any test runs and MAY be changed only by a recorded revision of this document with justification.

| Threshold | Value | Applies to |
|---|---|---|
| Interface inventory | The union of plan §6.1–§6.9 items, enumerated per suite in the traceability matrix; no claim on interfaces outside it | R-THREAT-1 |
| Adversary matrix | One row per suite, capabilities as in §2 | R-THREAT-1 |
| Bypass corpus | 100% of enumerated tests reach their expected preventive outcome for the gate to pass; any reproducible bypass fails the gate; unexplained nondeterminism is a failure pending investigation; repetitions pinned in the test catalogue; every attempt retained | Inv 6, 10, 12, 17 |
| Attribution completeness | ≥ 99% correct full-chain reconstructions / in-scope ground-truth effects under the catalogue's nominal profile; 100% over the finite gateway-operation corpus per run; overload profile reported with loss counters; denominator, dedup, and correlation deadline per test catalogue | R-AUD-2 |
| Privileged-code reviewability bound | `agentbound-launch` + `agentbound-lifecycle` (including allocator) + gateway authentication path ≤ 6 000 **direct** SLOC. SLOC accounting: pinned counting tool and version; five separately reported figures — direct privileged SLOC, generated SLOC, transitive dependency SLOC in privileged processes, configuration/rule SLOC (seccomp, Landlock, systemd units, D-Bus policy), and SLOC in a language without memory safety by default (allowed only with a listed justification per file). The bound applies to the first figure; all five are published, plus a sixth unbounded figure, gateway core SLOC (dispatch and adapters), which is reviewed line by line but not bounded in Phase 1 | R-CON-8 |
| Policy-exception rate | Zero manifest fields overridden by administrators outside the catalogue during the evaluation run; every privileged manual repair recorded | Gate 4 |
| Fault-injection coverage | Every fault point in the test catalogue's finite inventory (F-C-01…09, F-T-01…11) injected at least once; after each, no live process, usable grant, mount, or unsealed record; sealed failed records are the only permitted remnant; reconciliation completes within the catalogue deadline | R-CON-1, R-ISO-4 |
| Pinned versions | Linux 6.12 LTS series (exact patch release recorded; `openat2`, new mount API, pidfd, `SOCK_SEQPACKET` credentials verified per ADR-0002 Decision 7), systemd 258 series (exact release recorded), LSM policy digest, Firecracker v1.16.1 and the ADR-0003 `pinned-configuration.json` digest (control arm), Git host version; recorded in the evaluation report and unchanged within a run | all |
| Control-arm equivalence and decision rule | The ADR-0003 per-test classification and the pre-registered comparative decision rule, frozen before any control-arm result | Gate 4 comparative claim |

---

## 13. Requirements by milestone

| Milestone | Requirements first testable |
|---|---|
| 1A | R-SCOPE-*, R-THREAT-1, R-ID-1…8, R-REQ-1…6, R-CON-1…8, R-ISO-1…5, R-RES-1…3, 5, R-LC-1, 2, 4, 5, R-LC-3 (1A cases), R-AUD-1, 3, 4, 5 |
| 1B | R-GW-1…7, R-LC-3 (1B cases), R-AUD-2 |
| 1C | R-GW-8, 9, R-LC-3 (1C cases), R-INT-1…3 |
| 1D | Control-arm application per ADR-0003 |

---

## 14. Open questions

None. The five WP0 questions are answered in the [open-question register](open-question-register.md) and written into R-ID-4, R-CON-4, R-CON-6, R-AUD-3, and §12.
