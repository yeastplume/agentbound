# Session Lifecycle Specification

**Version:** 0.7  
**Status:** Frozen (WP0)  
**Date:** 28 August 2026  
**Applies to:** Phase 1 Unix-governed sessions  
**Related:** [Phase 1 plan](../plans/phase-1-reference-implementation.md), [technical report](../papers/technical-report.md), [ADR-0001](ADR-0001-execution-identity.md)

## Revision history

- **0.1** — Initial WP0 draft.
- **0.2** — Replaced the systemd-invoked lifecycle helper with the `agentbound-lifecycle` daemon (D-Bus scope signals plus held pidfds); construction step 1 restated as a `clone3` synchronization barrier; termination protocol reordered so the PID-namespace init reaps before `cgroup.kill`, with a host credential scan and a termination deadline; quiesce redefined as admission denial plus freeze; local-socket topology only; two-stage launch record terminology.
- **0.3** — Identifier terminology aligned (`authorization_id` pre-binding, `launch_record_digest` post-binding); systemd stated as observation source only.
- **0.4** — Open questions disposed per the open-question register; answers written into the normative text. D-state escalation path; LC-2 carried to WP1.
- **0.5** — §6: `continue-degraded` restricted to `policy_service_unavailable` and `audit_pipeline_degraded_below_stop_threshold`; trigger table rewritten; generic control-plane trigger split into policy-service, audit-pipeline, and lifecycle-daemon outages (the last not manifest-selectable).
- **0.6** — Editorial pass under docs/STYLE.md; no obligation, identifier, or value changed. §5 deadline paragraph split; Oxford spelling.
- **0.7** — WP1 findings F-2, F-3, F-4, F-5 ([evidence](../evidence/wp1/)): §3 step 5 forbids inherited `sysfs`; scope creation sets `TimeoutStopUSec` at `StartTransientUnit`; §4 names `PropertiesChanged`/pidfd as prompt triggers and `UnitRemoved` as confirmation; §5 step 4 does not wait for the frozen state before `cgroup.kill`. No state, event, or deadline changed.


---

## 1. Purpose and normative language

This specification defines the lifecycle of an `agentbound` session from a request through construction, execution, revocation, termination, cleanup, and sealing. It implements the lifecycle required by the Phase 1 plan §4.4 and gives concrete lifecycle meaning to technical-report Invariants 7, 12, 15, and 21.

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative. A session is not active merely because a process exists: it is active only when its effective manifest, launch record, execution identity, containment boundary, and required audit binding have all been established.

`agentbound-policy` resolves policy and signs the allocation-free authorization manifest. `agentbound-launch` is the narrow, short-lived privileged constructor that reserves an execution identity and signs the launch binding. `agentbound-lifecycle` is the single privileged long-running daemon that owns the identity allocator, holds session pidfds, and is the sole actor for quiesce, termination, reclamation, and restart reconciliation. `agentbound-gateway` mediates only named approved service operations. `agentbound-audit` records and correlates evidence. The `agentbound` CLI is a policy-governed client; it is not itself privileged lifecycle authority.

---

## 2. Session model and identifiers

Each session MUST have one immutable authorization ID, session trace identity, authorization-manifest digest, launch-binding digest, durable principal ID, execution UID/GID allocation mapping, host ID, boot ID, and systemd scope or equivalent cgroup identity. Before allocation, the execution and launch-binding fields are explicitly absent.

The launch record MUST bind the authenticated initiator, task/purpose, approvals, policy and catalogue versions, runtime digest, grants, execution binding where applicable, and termination policy. It MUST be committed before any credential or broker access is usable.

A session status API MUST expose the current state, the latest transition event, and a safe reason code. It MUST NOT expose credentials, mount-source paths not already authorized for the observer, or unredacted audit payloads.

### 2.1 State table

| State | Entry condition | Required invariant while in state | Externally observable status |
|---|---|---|---|
| `requested` | `agentbound` submits an authenticated bounded request. | No execution identity, scope, mount, credential, or runtime exists. | `requested` or request rejection reason. |
| `authorized` | `agentbound-policy` derives and signs/commits the allocation-free authorization manifest. | Authorization inputs and versions are fixed; no execution identity exists yet. | `authorized`; launch may be scheduled. |
| `constructing` | `agentbound-launch` accepts the committed manifest and reserves construction resources. | Child is blocked on the construction barrier; no untrusted runtime executes and no usable grant exists. | `constructing`, with current construction step only to authorized operators. |
| `active` | All mandatory boundaries exist, launch record is committed, grants are usable, privilege is dropped, and runtime exec succeeds. | Session authority is no greater than the manifest. | `active`, plus policy-governed attach/observe capability. |
| `quiescing` | A terminate or quiesce decision is accepted. | No new child and no new gateway operation may start. | `quiescing`, with reason and configured bound. |
| `termination-incomplete` | The termination bound expired while live or contradictory process evidence remains. | Nonterminal: grants are revoked where safe, identity remains held, observation and retries continue; record MUST NOT be sealed final. | `termination-incomplete`, blocking evidence, and next action. |
| `terminated` | No live process remains in the supervised cgroup or PID namespace. | All processes are dead or reaped; identity remains held pending cleanup. | `terminated`, with termination reason. |
| `cleaned/sealed` | Cleanup and reclamation prerequisites complete; launch record is sealed. | No managed grant, mount, gateway socket, or identity allocation remains usable. | `sealed` with immutable final outcome. |
| `rejected` | Request or derivation is invalid, unauthenticated, expired, or unauthorized. | No construction side effect is retained. | `rejected` with safe derivation failure code. |
| `construction-failed` | A required construction step, audit binding, privilege drop, or exec fails. | Rollback is in progress or completed; no runnable partial session is permitted. | `construction-failed`; cleanup outcome is shown separately. |
| `aborted` | Authorized actor cancels before `active`, or recovery aborts an unsafe incomplete launch. | Runtime MUST NOT become usable. | `aborted` with actor/reason. |
| `degraded` | Policy permits continuing after a declared dependency loss. | Degradation, remaining authority, and compensating controls are recorded. | `degraded`, with cause and affected operations. |

`cleaned/sealed`, `rejected`, `construction-failed` after successful rollback, and `aborted` after successful rollback are terminal. `degraded` is an overlay status on an otherwise `active` session; it MUST NOT conceal a required termination or quiescing action.

### 2.2 Transition table

| From → to | Authorized actor | Audit event | Idempotency and retry | Failure cleanup | Observable status |
|---|---|---|---|---|---|
| none → `requested` | authenticated `agentbound` client or scheduler | `session.requested` | Client retry uses an idempotency key; duplicate key returns existing request result. | Persist no partial request on validation failure. | request ID and `requested`/`rejected`. |
| `requested` → `authorized` | `agentbound-policy` | `session.authorized` | Same canonical inputs produce one manifest/authorization ID; conflicting replay is rejected. | Discard uncommitted derivation data. | `authorized` or `rejected`. |
| `requested`/`authorized` → `rejected` | `agentbound-policy` | `session.rejected` | Terminal for the request key; a corrected request requires a new key. | No UID or grant may be allocated. | `rejected` and reason class. |
| `authorized` → `constructing` | `agentbound-launch` | `session.construction_started` | Compare-and-set on authorization ID; only one constructor owns an attempt. | Release reservations if ownership cannot be established. | `constructing`. |
| `authorized`/`constructing` → `aborted` | initiator/approver under policy, systemd recovery, or `agentbound-lifecycle` | `session.aborted` | Repeated abort converges on rollback and sealed outcome. | Kill barrier-blocked child, revoke provisional grants, release resources. | `aborted` then final cleanup state. |
| `constructing` → `active` | `agentbound-launch` | `session.activated` | Commit is once-only; retry observes committed launch record rather than execing twice. | If acknowledgment is uncertain, recovery treats session as constructing until scope and record reconcile. | `active`. |
| `constructing` → `construction-failed` | `agentbound-launch` or fault injector | `session.construction_failed` | Rollback actions are individually idempotent and may be retried. | Execute reverse-order rollback; retain identity if safe release cannot be proven. | `construction-failed`. |
| `active` → `quiescing` | `agentbound-lifecycle` after policy decision, systemd, or authorized CLI action | `session.quiesce_started` | Repeated request preserves earliest reason and tightest bound. | If freeze/quiesce fails, escalate to termination when policy requires it. | `quiescing`. |
| `active` → `degraded` | `agentbound-lifecycle` applying manifest-declared behaviour | `session.degraded` | Same cause updates one degradation record; recovery re-evaluates policy. | Revoke affected grants and reject affected operations. | `degraded`. |
| `degraded` → `quiescing`/`terminated` | `agentbound-lifecycle` | `session.degradation_escalated` | Idempotent escalation. | Apply termination ordering. | `quiescing` or `terminated`. |
| `active`/`quiescing`/`degraded` → `terminated` or `termination-incomplete` | `agentbound-lifecycle` (triggered by request, revocation, or systemd scope signal) | `session.terminated` | Repeated termination is successful only after no live process is confirmed. | Hold identity and mark incomplete if proof cannot be obtained. | `terminated` or `termination-incomplete`. |
| `termination-incomplete` → `terminated` | `agentbound-lifecycle` after retry/recovery proves no live process | `session.termination_completed` | Repeated proof is idempotent. | Keep identity and containment state on uncertainty. | `terminated` only after proof. |
| `terminated` → `cleaned/sealed` | `agentbound-lifecycle` and identity allocator | `session.cleaned` and `session.sealed` | Cleanup and sealing are retry-safe; sealing is append-only and once-only. | Do not release identity on cleanup uncertainty. | `sealed` with final reason. |
| any nonterminal → `construction-failed`/`aborted` during recovery | recovery controller | `session.recovery_aborted` | Reconciliation is repeatable from persisted facts. | Prefer containment and identity hold over speculative release. | failure state and recovery reason. |

An actor MAY request a transition but `agentbound-lifecycle` MUST authorize and serialize every post-launch state change. A transition audit event MUST be emitted before or atomically with the externally visible state update; audit pipeline failure follows the manifest's fail-closed rule.

---

## 3. Construction protocol

`agentbound-launch` MUST perform these nine sub-steps in this exact order from technical-report §2.1. The child is created with `clone3` carrying the required namespace flags and `CLONE_PIDFD`; because no kernel facility creates a child in a stopped state, the child MUST block on a **synchronization barrier** (a pipe or eventfd held by the constructor) before executing anything other than its bootstrap, and the constructor releases the barrier only after every step below is complete and verified. No untrusted code runs between steps.

| # | Required construction sub-step | Rollback action | Plan §7.3 fault-injection point |
|---|---|---|---|
| 1 | `clone3` with namespace flags and `CLONE_PIDFD`; child blocks on the synchronization barrier before any credential or mount is visible. | Kill/reap blocked child via pidfd; remove provisional scope. | Namespace and mount setup at each §2.1 step. |
| 2 | Unshare mount namespace and recursively mark mounts private before any bind operation. | Destroy child mount namespace by killing child; verify no propagation to host. | Namespace and mount setup. |
| 3 | Resolve mount sources descriptor-relatively with `openat2` safe resolution or mount FDs; never re-walk string paths. | Close source and mount FDs; unregister provisional path references. | Namespace/mount setup and constructor-input path/TOCTOU tests. |
| 4 | Build restricted tree and enter it with `pivot_root`, never `chroot`. | Unmount restricted tree from host-side tracked mount FD; destroy child. | Namespace and mount setup. |
| 5 | Mount `proc` only after PID namespace exists; never expose host `/proc`. Mount `sysfs`, if at all, as a fresh instance only after the network namespace exists; never carry an inherited host `sysfs` into the session root. | Unmount session procfs before namespace destruction. | Namespace and mount setup. |
| 6 | Close every descriptor outside the effective manifest allowlist, including reintroduction paths through `SCM_RIGHTS`, `/proc/self/fd`, and memfd. | Close remaining tracked descriptors; kill child if closure cannot be proven. | Privilege disposal and constructor-input descriptor tests. |
| 7 | Install execution identity, supplementary groups, LSM context, capability bounding set, `no_new_privs`, Landlock, and seccomp in an order tested for the selected LSM policy. | Kill child; revoke identity allocation only after reclamation condition; remove provisional LSM/grant state. | Execution-identity allocation; namespace/mount; privilege disposal. |
| 8 | Make credentials or broker access usable only after every boundary is in place and the launch record is committed. | Revoke/close provisional broker or credential grant; record issuance and revocation outcome. | Credential/gateway grant issuance and audit binding. |
| 9 | Exec the runtime last. | If exec fails, kill/reap child and execute all reverse-order cleanup. | Runtime `exec`. |

The constructor MUST create the cgroup/systemd scope with `TimeoutStopUSec` set at `StartTransientUnit` time (a PID-namespace init discards an external `SIGTERM` unless it installs a handler, so an operator or systemd `stop` of a scope without this property waits `DefaultTimeoutStopSec`, 90 s, before `SIGKILL`; systemd does not permit setting it on a scope afterwards), resource limits, the single `SOCK_SEQPACKET` gateway-socket mount when `gateway.channel_topology` is `local-socket` (and no gateway socket, mount, projection, or grant when it is `none`), and audit/session provenance as prerequisites incorporated in these steps; a required prerequisite failure MUST cause `construction-failed`.

Credentials, `agentbound-gateway` authority, and broker access MUST become usable **only after** the launch record is committed. The runtime MUST be exec'd last. `agentbound-launch` MUST drop launch-only privileges before that exec. A launch record committed for a failed exec MUST be sealed with a failure outcome, not deleted.

---

## 4. Post-launch privileged lifecycle daemon

`agentbound-lifecycle` is one privileged, long-running daemon, separately authorized and reviewable, and counted in the privileged-code accounting. It MUST be separate from `agentbound-launch`. The constructor hands each session's pidfds, scope name, and allocation record to the daemon before exit. It MUST NOT retain a post-launch control channel.

A transient systemd scope has no `ExecStop=`. It can only stop or kill its cgroup and cannot invoke a helper. The ordered termination protocol in §5 needs an actor that outlives the constructor, holds pidfds durably, and survives restarts. That actor is the daemon.

Its enumerated privileged operations are:

1. terminate a session and its descendants;
2. quiesce a session and enforce the quiescing restrictions;
3. instruct `agentbound-gateway` to deny admission, then release credentials, broker access, and gateway grant records;
4. clean session mounts and the mounted gateway socket;
5. allocate, reclaim, quarantine, and release execution identities (ADR-0001); and
6. reconcile persisted state after restart (§7).

The daemon subscribes to systemd's D-Bus `UnitRemoved` and `PropertiesChanged` signals for session scopes and holds the PID-namespace-init pidfd as an independent liveness source; either signal triggers the §5 protocol. `PropertiesChanged` with `ActiveState` in `inactive` or `failed`, and the held pidfd, are the prompt triggers; systemd emits `UnitRemoved` only at unit garbage collection, seconds later, so the daemon MUST treat it as confirmation rather than wait for it. If systemd kills a scope before the daemon acts, the daemon MUST still execute the full protocol. It MUST record `session.ordering_deviation` with the systemd event that pre-empted it. The daemon accepts requests from the `agentbound` CLI only through a policy-authorized request and from authenticated revocation triggers. It MUST record the invoking actor and delegated authority. It MUST NOT accept arbitrary cgroup paths, mount paths, UIDs, or shell commands from any caller. Peer identity and authorization for each caller are specified in [component interfaces](component-interfaces.md).

---

## 5. Termination and cleanup ordering

For every normal stop, cancellation, revocation requiring termination, construction rollback after process creation, and recovery stop, `agentbound-lifecycle` MUST perform this order:

1. mark the session terminating and instruct `agentbound-gateway` to deny admission of new operations without yet releasing grant records;
2. freeze the session cgroup so no process can fork while the protocol runs;
3. deliver `SIGTERM` to the workload via the PID-namespace init and thaw briefly within a bounded window so init can reap exited children;
4. request a second freeze and write `cgroup.kill` without waiting for `cgroup.events` to report `frozen 1` (a cgroup with an uninterruptible member never reaches the frozen state; `cgroup.kill` acts on frozen and unfrozen members alike); wait on cgroup emptiness and the init pidfd under the bounded wait below;
5. confirm that `cgroup.procs` is empty, the init has exited and been reaped, and the host credential scan of the [identity lifecycle](execution-identity-lifecycle.md) §4.1 finds no process under the execution UID/GIDs;
6. release `agentbound-gateway` grant records and close indexed connections; the gateway MUST acknowledge zero connections;
7. close broker access and any session credential capability;
8. unmount session filesystems;
9. remove the mounted gateway socket;
10. release the execution identity to reclamation; and
11. seal the launch record with the termination reason and cleanup outcome.

Denying new operation admission is mandatory on entry and is distinct from releasing grant records. `agentbound-lifecycle` MUST NOT release gateway/credential records before descendant termination is complete unless early revocation is needed to contain an immediate remote effect. In that exception it MUST record the ordering deviation, revoke early, and continue process termination.

`cgroup.kill` sends `SIGKILL` to current cgroup members; it neither reaps zombies nor terminates an uninterruptible D-state task, and membership is containment evidence rather than proof against a task that left the cgroup (hence step 5). `agentbound-lifecycle` MUST use a configurable bounded wait for cgroup emptiness, PID-namespace-init exit, and relevant pidfds. If a task remains live at the bound, it MUST mark the session `termination-incomplete`, retain all identity allocation and grants needed for safe containment decisions, and continue observation or operator escalation. A manifest-declared **termination deadline** bounds this state: a session still `termination-incomplete` at the deadline is a **non-pass** for the affected test and emits `session.escalation_required` naming the held pidfds; the operator's only permitted actions are continued observation or a host reboot, and the identity remains held across reboot via the allocator store. It MUST NEVER release an execution identity while a live process remains in its declared managed reclamation domain.

---

## 6. Active revocation and degradation

A manifest MUST declare one behaviour for each applicable trigger: `terminate`, `quiesce`, or `continue-degraded`. The declaration is a policy choice, but the implementation MUST execute it deterministically and audit the trigger, decision, affected grants, and result.

**Quiesce** means: `agentbound-lifecycle` instructs the gateway to deny admission of new operations, denies new attachments and grants, and then **freezes** the session cgroup for the manifest-declared bound; the frozen session can create no new gateway operation and no new process. At bound expiry `agentbound-lifecycle` MUST terminate the session. Quiesce does not promise that thawed processes could not fork: no-new-child semantics are provided only by the freeze, so a quiesced session is never thawed except by the termination protocol. Quiesce is not a promise that already-read information is forgotten.

**Restriction on `continue-degraded`.** In Phase 1, `continue-degraded` is valid only for the triggers `policy_service_unavailable` and `audit_pipeline_degraded_below_stop_threshold`, and only when predeclared in the manifest. Every other trigger MUST resolve to `terminate` or `quiesce`; `agentbound-policy` MUST reject a manifest that declares `continue-degraded` for any other trigger. An `agentbound-lifecycle` outage (`lifecycle_daemon_unavailable`) is not a manifest-selectable trigger and never enters `continue-degraded`: installed containment remains in force, no new authority is issued, and every transition waits for daemon recovery, after which §7 reconciliation runs.

| Milestone | Trigger | Declared behaviour requirements |
|---|---|---|
| 1A | Initiator disabled | Terminate or quiesce as declared. |
| 1A | Approval expired | Terminate or quiesce as declared; revoke approval-dependent future operations. |
| 1A | Authority revoked | Terminate or quiesce as declared. |
| 1A | Policy or catalogue withdrawn | Stop new use of the withdrawn item; terminate or quiesce as declared. |
| 1A | Approver cancels task | Terminate or quiesce as declared; record approver identity and authority. |
| 1A | Policy service unavailable | Terminate, quiesce, or `continue-degraded` as declared; under `continue-degraded` no operation requiring fresh policy evaluation is admitted and no new authority is granted. |
| 1A | Audit pipeline degraded below stop threshold | Terminate, quiesce, or `continue-degraded` as declared; under `continue-degraded` loss counters are exposed and effects requiring attribution are denied. |
| 1A | Lifecycle daemon unavailable | Not manifest-selectable: containment holds, no new authority, transitions wait for recovery. |
| 1B | Git gateway grant withdrawn | `agentbound-gateway` MUST deny new Git operations; terminate or quiesce as declared. |
| 1B | Gateway unavailable | `agentbound-gateway` failure MUST produce the declared terminate or quiesce behaviour and a distinct availability event. |
| 1C | Inference grant revoked | Gateway MUST deny new inference operations; terminate or quiesce as declared. |
| 1C | Approved execution binding revoked | Deny the binding; terminate or quiesce as declared; record binding identity. |

At each milestone, only cases for components that exist are testable. The evidence table MUST identify demonstrated cases. It MUST NOT claim later gateway or inference cases before the relevant component exists.

---

## 7. Crash recovery and orphan handling

On restart, the control plane MUST treat persisted launch records, systemd scope state, cgroup contents, PID-namespace init status, pidfds where recoverable, grant stores, and identity allocator state as evidence to reconcile rather than assuming in-memory state was durable.

| Persisted state | Restart behaviour |
|---|---|
| `requested` | Revalidate request idempotency and either resume authorization or reject stale/incomplete request per retention policy. |
| `authorized` | Resume construction only through a new serialized constructor ownership lease; otherwise abort safely. |
| `constructing` | Discover scope, child, mounts, grants, and committed record. If activation cannot be proven complete, abort and roll back; never blindly re-exec. |
| `active`/`degraded` | Discover the systemd scope and launch-record store. Reattach observation and re-evaluate revocation policy; if required evidence or containment cannot be restored, quiesce or terminate. |
| `quiescing` | Resume the stored deadline/bound and complete termination; never return automatically to active. |
| `terminated` | Complete cleanup and identity reconciliation. |
| `termination-incomplete` | Maintain identity hold and containment observation; require explicit successful no-live-process confirmation before cleanup. |
| terminal/sealed | Verify seal integrity only; do not recreate session resources. |

An orphan is a live scope, cgroup, mount, grant, or allocator allocation lacking a compatible active launch record, or a launch record claiming a session whose scope evidence conflicts with it. Recovery MUST mark an orphan audit event, deny new grants, and contain/terminate it. Authoritative-source precedence when systemd scope state, the launch-record store, the allocator store, the gateway connection index, and audit disagree is specified in [component interfaces](component-interfaces.md); the default is fail closed (hold identity, deny authority).

---

## 8. Required audit events

Every event below MUST include: `host_id`, `boot_id`, `authorization_id`, `launch_record_digest` (`null` before the launch binding is committed, mandatory after), `allocation_record_id`, session and trace identities, `execution_uid`, timestamp, actor, and outcome. Where allocation or execution fields do not yet exist, they MUST be explicitly `null` rather than omitted. Process events additionally require PID-namespace identity and process start time or pidfd-derived identity where supported; unavailable pidfd evidence is a recorded residual assumption, never replaced by PID alone. Events MUST also include a stable event ID and causation/correlation ID.

| Event name | Additional required fields |
|---|---|
| `session.requested` | request idempotency key, initiator, requested principal, task/purpose. |
| `session.authorized` | manifest digest, policy/catalogue versions, approvals, derivation result. |
| `session.rejected` | safe reason code and failed input class. |
| `session.construction_started` | constructor instance, scope/cgroup ID, identity allocation record. |
| `session.construction_step` | step number, operation result, fault-injection marker where used. |
| `session.launch_record_committed` | record trust anchor, commit sequence, manifest digest. |
| `session.grant_issued` / `session.grant_revoked` | grant type, issuer, scope, gateway/broker result; never secret material. |
| `session.activated` | runtime digest, exec result, privilege-disposal result. |
| `session.quiesce_started` / `session.quiesce_completed` | trigger, configured bound, child/gateway admission state. |
| `session.degraded` | trigger, remaining authority, compensating control. |
| `session.revocation_received` | source, trigger class, manifest behaviour selected. |
| `session.termination_started` | reason, cgroup/scope identity, ordering deviations. |
| `session.terminated` | `cgroup.kill` result, pidfd/PID-init reaping result, no-live-process proof. |
| `session.termination_incomplete` | remaining process evidence, configured bound, identity hold. |
| `session.cleanup_completed` | mounts, gateway socket, brokers, grants, and ACL cleanup results. |
| `session.identity_released` | allocation record, reclamation proof reference, quarantine status. |
| `session.sealed` | final state, termination reason, seal sequence/hash. |
| `session.recovery_reconciled` / `session.orphan_detected` | discovered scope/store evidence and resulting action. |

`agentbound-audit` MUST preserve audit-loss counters and MUST make required-evidence loss observable. A launch or continued active session MUST follow the manifest's declared fail-closed behaviour when required audit evidence cannot be produced.

---

## 9. Lifecycle safety rules

### 9.1 Authority and concurrency rules

1. `agentbound-policy` MUST derive authority before `agentbound-launch` obtains an execution identity or creates a scope.
2. An approval, policy, catalogue, or runtime decision used to enter `authorized` MUST be recorded by immutable reference. A later caller MUST NOT substitute a newer or differently encoded value into an already authorized attempt.
3. At most one constructor attempt MAY own a authorization ID at a time. The ownership lease MUST be durable enough that a restart can distinguish a live owner from an abandoned attempt.
4. Only `agentbound-lifecycle` MAY advance an active session toward quiescing, termination, cleanup, or identity release. systemd supplies scope observations only; the `agentbound` CLI and revocation triggers request action through its authorized interface; `agentbound-lifecycle` alone decides, serializes, and performs the transition.
5. A failed transition MUST be fail closed with respect to new authority. In particular, a failed revocation check MUST NOT enable a new `agentbound-gateway` operation.
6. A state transition MUST preserve a causal link to the initiating request, policy decision, revocation signal, systemd event, or recovery observation that caused it.
7. An observer MAY see a lagging status replica. The status API MUST label its observation sequence. It MUST offer an authoritative record reference for privileged observers.
8. A session MUST NOT transition from `quiescing`, `terminated`, `construction-failed`, or `aborted` back to `active`. A resumed task is a new session with a new authorization ID and execution identity.
9. A session whose authority is narrowed during `continue-degraded` MUST record a replacement effective grant set. It MUST NOT retain the superseded grant merely because existing processes hold it.
10. A manual operator override MAY select a more restrictive action than the manifest, including termination. It MUST NOT select a less restrictive action unless an independently authorized policy decision is recorded.

### 9.2 Status and reason-code rules

The external status API MUST distinguish policy denial, construction failure, deliberate cancellation, incomplete termination, cleanup failure, and audit evidence loss. It MUST NOT report `terminated` merely because `cgroup.kill` was issued.

| Status condition | Minimum safe reason fields | Required operator-visible consequence |
|---|---|---|
| `rejected` | derivation input class, policy decision ID | A caller can correct a request without learning protected policy detail. |
| `construction-failed` | failing step, rollback state, retained-resource class | Operators can determine whether identity or scope remains held. |
| `degraded` | dependency/trigger, remaining operations, next re-evaluation condition | Clients know that a capability may be unavailable without mistaking it for completion. |
| `quiescing` | trigger, admission closure result, configured finishing bound | Attach/operation clients can stop retrying unavailable work. |
| `termination-incomplete` | live-process evidence, cgroup/PID-init result, identity hold | Operators know that reuse is blocked and containment needs attention. |
| `sealed` | final reason, cleanup result, seal reference | The historical result cannot be changed by ordinary lifecycle calls. |

Safe reason codes MUST be stable machine-readable values. Human-readable detail MAY be redacted according to the observer's authorization. Credentials, raw policy secrets, and unredacted gateway request bodies MUST NOT appear in reason codes.

### 9.3 Construction rollback ledger

`agentbound-launch` MUST create a rollback ledger before the first privileged construction action. Each entry MUST identify the action, resource handle, expected reverse action, completion state, and authorization ID. The ledger MAY be stored with the launch record or in a separately integrity-protected constructor journal.

| Resource class | Required rollback proof |
|---|---|
| barrier-blocked child and PID namespace | Child exit/reap and PID-namespace-init outcome. |
| systemd scope/cgroup | Scope stop result and empty `cgroup.procs`, or explicit incomplete-termination hold. |
| mounts and mount FDs | Unmount result and closure of tracked FDs. |
| gateway channel | Closure and unmount of the one permitted `SOCK_SEQPACKET` gateway socket. |
| descriptors | Closure or explicit transfer to an allowed, still-live supervised process. |
| execution identity | Transition to `reclaiming`, not speculative `free`. |
| gateway/broker grant | Issuer confirmation of revocation or a recorded unreachable dependency that blocks release. |
| audit binding and launch record | Committed failure/abort event and seal or recovery-pending marker. |

Rollback MUST proceed in reverse dependency order where doing so preserves containment. For example, it MUST stop the child before releasing its identity, and it MUST close grants before deleting evidence needed to identify them. A rollback failure is itself a lifecycle failure and MUST retain enough resource state for later recovery rather than discarding the ledger.

### 9.4 Attachment and observation

A policy-governed attachment is not an implicit lifecycle transition. `agentbound` MAY request observe, inject, approve, interrupt, or control rights, but each is separately authorized and audited. During quiescing, no new attachment MAY be admitted. Existing observers MAY remain only if the manifest allows them; existing injectors and controllers MUST lose authority to initiate new child or gateway activity.

An attachment event MUST carry the session authorization ID, attaching actor, attachment mode, trace identity, and outcome. A terminal stream or PTY descriptor MUST be treated as a capability. It MUST be closed on termination. It MUST NOT be inherited by a later session that reuses a UID.

### 9.5 Lifecycle evidence retention

The sealed launch record MUST retain the ordered lifecycle transition history, including failed/retried attempts and fault-injection markers. Sealing MAY archive detailed operational evidence according to retention policy, but it MUST preserve the identifiers and outcome needed to correlate kernel and gateway records after UID reuse.

The lifecycle specification does not claim that termination or revocation erases information already read by the session. It limits future process execution and mediated authority, then preserves accountable evidence of that decision.

### 9.6 Required implementation checks

Before reporting an implementation as conformant with this specification, the implementation MUST demonstrate the following checks for each supported transition:

| Check | Required assertion |
|---|---|
| Duplicate request | Repeating a client request with its idempotency key creates no second manifest, scope, UID allocation, or runtime. |
| Concurrent launch | Competing constructors cannot activate two scopes for one authorization ID. |
| Boundary-before-runtime | At every injected construction failure point, no runtime instruction executes before the relevant boundary and audit binding exist. |
| Grant-after-record | A gateway/broker refuses an operation until it can identify a committed launch record and active session state. |
| Descendant control | Fork, double-fork, daemonization, and orphaning do not evade the supervised cgroup, PID namespace, or termination procedure. |
| Quiesce admission | While quiescing, new child and gateway-operation attempts fail with an auditable lifecycle denial. |
| Revocation ordering | Termination proves descendant stop before ordinary grant closure, except for a recorded immediate-containment exception. |
| D-state safety | An uninterruptible task leaves the identity held and causes `termination-incomplete`, never UID reuse. |
| Recovery reconciliation | A restart finds and resolves an incomplete constructor attempt, live active scope, and orphan independently. |
| Seal immutability | An ordinary API caller cannot alter the final reason or re-open a sealed session. |

Tests SHOULD run these checks against both normal operations and fault-injected operations. The result for a failed or unsupported check MUST be recorded as a failed property or documented residual assumption; it MUST NOT be relabeled as a successful lifecycle transition.

### 9.7 Dependencies and failure boundaries

The `agentbound-lifecycle` relies on systemd scope control, cgroup v2, a PID-namespace init or subreaper, a trusted launch-record store, the identity allocator, and any configured broker or `agentbound-gateway` grant authority. Each dependency MUST expose an authenticated status interface or a durable record sufficient for recovery.

A dependency failure does not automatically authorize termination completion. For example, a missing gateway confirmation blocks identity reclamation even if the cgroup is empty; a lost audit collector triggers the manifest's audit-loss behaviour; and a missing systemd response requires corroboration from cgroup and process evidence. Implementations MUST state which source is authoritative for each check and MUST treat contradictory sources as an unsafe condition.

A configuration that omits a required dependency for a claimed property MUST mark the property unavailable before launch. It MUST NOT silently downgrade an `active` session into a mode that still advertises the absent property.

---

## 10. Open questions

Seven of the eight WP0 questions are answered in the [open-question register](open-question-register.md) (version set pinned in ADR-0003; store trust anchor and commit points in component interfaces; `continue-degraded` limited to policy-service and sub-threshold audit degradation; backpressure via `audit.loss_behaviour`, `audit_capacity`, and status counters; D-state escalation in §5; microVM state mapping in ADR-0003). One was carried to WP1 and is now answered:

- **LC-2** — does a frozen cgroup hold a `SOCK_SEQPACKET` connection open in a way that delays the gateway's zero-connection acknowledgement? **No** ([evidence](../evidence/wp1/frozen-peer.md)): the gateway closes and drains the frozen peer's connection without the peer's participation; §6 stands unchanged.
