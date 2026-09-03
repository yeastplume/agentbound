# Agentbound Component Interfaces
**Version:** 0.2  
**Status:** Draft for WP0 review — skeleton; wire formats are WP1 outputs  
**Date:** 28 August 2026  
**Applies to:** Phase 1 Unix-governed reference implementation  
**Related:** [Phase 1 plan](../plans/phase-1-reference-implementation.md), [manifest schema](manifest-schema.md), [session lifecycle](session-lifecycle.md), [execution-identity lifecycle](execution-identity-lifecycle.md), [ADR-0001](ADR-0001-execution-identity.md), [ADR-0002](ADR-0002-gateway-authentication.md), and [Phase 1 requirements](phase-1-requirements.md)

## Revision history

- **0.1** — Initial WP0 skeleton.
- **0.2** — Envelope freshness values fixed; identifier terminology aligned; systemd is an observation source only.

---
## 1. Purpose and normative language
This WP0 skeleton freezes the security-relevant component boundaries, trust
relationships, persistence rules, and recovery decisions for Agentbound. It
implements the interface consequences of plan §4 and requirements R-ID-8,
R-AUD-1 through R-AUD-4, and R-LC-5. It does **not** define message schemas or
field-level wire formats.
The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**,
and **MAY** in this document are to be interpreted as described by RFC 2119.
A component MUST reject an unsupported operation, a peer that it cannot
authenticate, or security-relevant input it cannot validate. It MUST NOT infer
authority from a UID alone, a pathname alone, a caller-provided trace ID, or a
claimed component name.
### 1.1 Components and authority boundary
| Component | Role | Privilege and authority boundary |
|---|---|---|
| `agentbound` | CLI/API client that requests, observes, attaches to, and requests termination of sessions. | Unprivileged; never directly allocates identities, changes lifecycle state, or installs host controls. |
| `agentbound-policy` | Resolves authenticated selections and server-side policy into an allocation-free authorization decision. | Unprivileged service identity; sole producer of policy-signed authorization manifests. |
| `agentbound-launch` | Validates authorization, atomically reserves an execution identity, and constructs the session boundary around a `clone3` child blocked on a pipe/eventfd synchronization barrier. | Privileged, short-lived constructor; drops launch-only privilege before runtime exec. |
| `agentbound-lifecycle` | Serializes post-launch lifecycle action and recovery. | **One privileged long-running daemon**; owns the identity allocator, holds session pidfds, and is sole actor for quiesce, termination, reclamation, and restart reconciliation. |
| `agentbound-gateway` | On-host typed-operation gateway and session authenticator. | Service identity; AF_UNIX `SOCK_SEQPACKET` only for on-host component interfaces; never a generic proxy. |
| `agentbound-audit` | Receives durable security events and produces correlation evidence. | Service identity; does not grant authority or alter lifecycle state. |
`agentbound-lifecycle` replaces the former model of a lifecycle helper invoked
by systemd. systemd supplies authenticated observations and stop notifications;
it MUST NOT directly update Agentbound lifecycle state or reclaim an identity.
### 1.2 Shared terms
- **Authorization manifest** means the allocation-free object signed by
  `agentbound-policy`.
- **Launch binding** means the allocation- and host-bound object signed by
  `agentbound-launch` after identity reservation.
- **Launch record** means the immutable pair of those objects and their signed
  envelopes. Its identity is
  `SHA-256(authorization_manifest_digest || launch_binding_digest)`.
- **Active record** means a nonterminal record whose durable lifecycle state
  permits the relevant action.
- **Service identity** means the provisioned OS identity, socket ownership, and
  pinned verifier key identity assigned to a component; names alone are not
  identities.
---
## 2. Interface-wide transport and authentication rules
All on-host component pairs in this document MUST use a dedicated local
`AF_UNIX` `SOCK_SEQPACKET` socket. Stream sockets, network listeners, inherited
ambient control descriptors, and generic RPC relays are not conforming for
these pairs. Each socket MUST have an administrator-provisioned path, owner,
group, and mode such that only the named service identities can connect.
The accepting endpoint MUST obtain `SO_PEERCRED` immediately after `accept` and
MUST verify the expected effective UID, GID policy, and service identity before
processing a message. Path permissions are defense in depth and MUST NOT be
accepted as peer authentication. A service MAY additionally bind the peer PID
to a pidfd when process-lifetime evidence is needed.
Every request MUST carry an idempotency key scoped to the receiving component,
caller identity, operation, and target record. Receivers MUST durably retain
sufficient completed-operation state to return the original outcome rather than
repeat a non-idempotent action. The receiver MUST reject a reused key with
different authenticated operation inputs.
Authentication establishes the caller component, not authority to request every
operation. Each receiving component MUST enforce the authorization matrix in
Section 3 and MUST treat all caller-provided IDs, state claims, paths, UIDs,
timestamps, and trace metadata as untrusted unless independently verified.
---
## 3. Component-pair security matrix
### 3.1 CLI to policy
| Property | Contract |
|---|---|
| Transport | Dedicated local `AF_UNIX` `SOCK_SEQPACKET`; `agentbound-policy` verifies `SO_PEERCRED`. |
| Peer identity | Policy MUST accept the configured CLI client/service UID only; the CLI MUST verify the policy service UID and pinned policy verifier key identity. |
| Permitted operations | Submit bounded session request; obtain safe request/authorization status; submit policy-governed observation or attachment request; submit an approval or cancellation request only where policy permits. |
| Not trusted | Principal/task/resource names, requested budget, approval references, CLI-reported human identity, command, trace ID, idempotency outcome, and all authority-bearing host configuration. |
The CLI MUST authenticate the human or scheduler through its configured local
credential mechanism before forwarding an initiator reference. The policy
service MUST independently resolve that reference. The CLI MUST NOT receive a
policy signing private key or an authority to issue a manifest.
### 3.2 Policy to launch
| Property | Contract |
|---|---|
| Transport | Dedicated local `AF_UNIX` `SOCK_SEQPACKET`; launch verifies `SO_PEERCRED`. |
| Peer identity | Launch MUST accept only the provisioned `agentbound-policy` service identity. Policy MUST verify the constructor service identity before delivery. |
| Permitted operations | Deliver an allocation-free authorization manifest and detached signature; query a safe construction outcome for an authorized authorization ID. |
| Not trusted | Any concrete UID/GID, scope/unit name, host path, mount source, namespace configuration, credential material, gateway connection, claimed allocation, or policy assertion outside the signed canonical manifest. |
`agentbound-launch` MUST verify RFC 8785 JCS canonical bytes, the Ed25519
signature, `key_id`, verifier-key validity, freshness, revocation status, and
manifest digest before any privileged action. It MUST recompute required
policy/catalogue validity checks rather than trust a policy caller assertion.
### 3.3 Launch to lifecycle
| Property | Contract |
|---|---|
| Transport | Dedicated local `AF_UNIX` `SOCK_SEQPACKET`; lifecycle verifies `SO_PEERCRED`. |
| Peer identity | Lifecycle MUST accept only the provisioned privileged constructor identity. Launch MUST verify the lifecycle daemon identity. |
| Permitted operations | Reserve allocation through lifecycle-owned allocator; register construction ledger, pidfd, scope identity, and signed binding; report activation or construction failure; request rollback before launch exits. |
| Not trusted | Launch-reported success, child liveness, cgroup emptiness, grant revocation, identity-free state, or caller-selected allocator state sequence. |
The lifecycle daemon owns allocator mutation even though launch requests the
reservation. Launch MUST NOT write allocator state directly. Lifecycle MUST
validate that the binding matches the reservation and that a constructor can
commit it only once.
### 3.4 Lifecycle to gateway
| Property | Contract |
|---|---|
| Transport | Dedicated local `AF_UNIX` `SOCK_SEQPACKET`; gateway verifies `SO_PEERCRED`. |
| Peer identity | Gateway MUST accept only the lifecycle daemon identity for grant/state control. Lifecycle MUST verify the gateway service identity and its configured verifier key identity. |
| Permitted operations | Register active grant projection after committed record; deny new admission; revoke grant; close indexed connections; obtain durable zero-connection/revocation confirmation; recover active grant index after restart. |
| Not trusted | Gateway-reported launch state, a bare execution UID, a connection's caller-supplied trace ID, an unverified grant ID, or a claim that a remote effect completed. |
The gateway MUST check current record and grant state for every typed operation.
A lifecycle revoke commit MUST make the next operation fail. A connection or
grant confirmation failure blocks identity reclamation.
### 3.5 Launch, lifecycle, and gateway to audit
| Sender | Transport and peer identity | Allowed operations | Not trusted from sender |
|---|---|---|---|
| Launch | `AF_UNIX` `SOCK_SEQPACKET`; audit verifies constructor `SO_PEERCRED`. | Emit construction, binding, privilege-disposal, rollback, and activation events. | Event success claim, wall clock, correlation values not bound to signed record. |
| Lifecycle | `AF_UNIX` `SOCK_SEQPACKET`; audit verifies lifecycle `SO_PEERCRED`. | Emit allocation, transitions, reconciliation, termination, reclamation, and seal events. | No-live-process or revoke claim without attached evidence reference. |
| Gateway | `AF_UNIX` `SOCK_SEQPACKET`; audit verifies gateway `SO_PEERCRED`. | Emit connection, operation admission/outcome, revocation, close, and loss events. | Remote response content, client trace ID, or session mapping without gateway evidence. |
Audit MUST verify event IDs are unique and MUST deduplicate at-least-once
delivery by stable event ID. It MUST retain duplicate counters and MAY retain
the duplicate envelope for forensics. Sender retry continues until the event is
accepted or the manifest's audit-loss behavior requires stop, quarantine, or
continue-with-loss-counter.
### 3.6 systemd to lifecycle
| Property | Contract |
|---|---|
| Transport | systemd D-Bus only; lifecycle authenticates the system bus peer and validates the configured systemd manager identity. |
| Peer identity | Lifecycle MUST accept signals only from the system manager on the system bus, not from a user bus or an arbitrary D-Bus name. |
| Permitted operations | Receive `UnitRemoved` and `PropertiesChanged` subscriptions for registered session scopes; query registered scope state; receive stop/failure observations. |
| Not trusted | Unit names, cgroup paths, state transitions, PIDs, reason strings, or scope ownership unless matched to the signed binding, registered scope identity, and pidfd/cgroup evidence. |
Lifecycle MUST subscribe before an active session is exposed. It MUST retain a
pidfd-watch fallback for each session PID-namespace init. Missing, delayed, or
contradictory D-Bus observations MUST trigger reconciliation; they MUST NOT
permit an identity release. systemd supplies scope observations (signals and unit
properties) only; it never requests or performs a lifecycle action. `agentbound-lifecycle`
alone decides, serializes, and records the resulting transition.
### 3.7 Gateway to adapters
| Property | Contract |
|---|---|
| Transport | Dedicated local `AF_UNIX` `SOCK_SEQPACKET`; adapter verifies `SO_PEERCRED`; adapters MUST NOT accept a public listener. |
| Peer identity | An adapter MUST accept only the configured gateway service identity. Gateway MUST bind each adapter instance to an approved catalogue identity and artifact digest. |
| Permitted operations | Execute only a named, typed, schema-validated operation whose scope, destination, tenant, budgets, and trace binding were approved in the active manifest. |
| Not trusted | Raw request bytes as an authority grant, destination URL, arbitrary method, caller trace ID, session UID, or claimed budget consumption. |
Adapters MUST NOT expose generic HTTP, CONNECT, shell, filesystem, or arbitrary
byte-stream forwarding. They MUST return an operation outcome suitable for audit
without exposing secrets in errors.
---
## 4. Reference trust and storage profile
### 4.1 Signing and verifier trust
The Phase 1 policy signing key SHALL be file-backed. Its private-key file MUST
be owned by the `agentbound-policy` service account, mode `0600`, on a local
filesystem not writable by the CLI, session execution identities, launch,
lifecycle, gateway, or audit service identities. Its parent directory MUST be
owned by the policy service or administrator and MUST NOT be writable by any
untrusted identity. Backup, restoration, and replacement of the key MUST be a
recorded administrator procedure.
Each signature envelope MUST use detached Ed25519 over RFC 8785 JCS canonical
JSON and contain algorithm, `key_id`, issuance time, named timestamp source,
and signed-object digest. Policy signs only the authorization manifest. The
constructor signs only the launch binding after successful atomic reservation.
The pair-derived `launch_record_digest` MUST use the digest concatenation and
the identifier-use table in [the manifest schema](manifest-schema.md) §4; the
policy-issued `authorization_id` is the pre-binding key and the digest is the
post-binding authoritative identity. Gateway grant indexes MUST key on the
digest only.
Verifier keyrings MUST be distributed as integrity-protected, versioned local
configuration to launch, lifecycle, gateway, and audit. A keyring entry MUST
include `key_id`, public key, not-before time, not-after time, intended signer
role, and status. Rotation MUST provide overlapping validity: verifiers MUST
accept a valid old and new key during the recorded overlap, then reject the old
key after its expiry or revocation. A signed or administrator-authenticated
revocation list MUST be distributed with the keyring; revoked keys MUST be
rejected even within nominal validity.
The constructor signing key MUST be a distinct file-backed key held only by the
short-lived `agentbound-launch` service identity, mode `0600`, and unavailable
to policy, lifecycle, gateway, audit, CLI, and sessions. The constructor MUST
not retain this key after its construction attempt ends.
Envelope freshness is REQUIRED. Phase 1 values: `issued_at` MUST be no more
than **30 s** in the future relative to the verifier's clock; an authorization
manifest MUST be consumed by the constructor within **10 min** of `issued_at`;
a launch binding MUST be committed within **60 s** of its own `issued_at`.
A verifier that cannot read a trusted clock MUST fail closed rather than
choose an unlimited window. The reference
clock source is the host kernel realtime clock disciplined by the
administrator-configured host time service; envelopes and audit events MUST
record the named clock source, wall-clock time, and monotonic time where
available. A detected clock rollback, unacceptable skew, or unavailable trusted
clock MUST reject new authorization/launch work and follow the manifest's
existing-session degradation behavior.
### 4.2 Launch-record store
The launch-record store MUST be host-local, append-only, integrity protected,
and writable only by the configured constructor/lifecycle store authority. A
record append reaches its commit point only after the new record and its
hash-chain link are durably `fsync`ed, including directory metadata where the
storage design requires it. Each entry MUST include its predecessor digest,
monotonic store sequence, record identity, event ID, signer envelope reference,
and outcome.
A sealed record is immutable. A correction MUST be a new append-only correction
record referencing the original record ID and original entry digest, stating its
reason and authorization; it MUST NOT edit, overwrite, or remove the original.
The historical launch binding and authorization manifest remain immutable.
The durable launch-binding commit point is the append and `fsync` of the
binding, envelopes, allocation reference, and initial construction state. No
grant may become usable before this point. A committed binding followed by any
failure MUST receive a later failure/rollback/seal record; it MUST NOT be
deleted.
### 4.3 Allocator store
The lifecycle daemon owns an append-only allocator store. Allocation mutation
MUST use compare-and-set on both allocation-record ID and current state
sequence. The durable `free → allocated` append is the allocation commit point
and MUST complete before launch installs the UID/GID or any identity-dependent
resource. The allocator record MUST bind host ID, boot ID, authorization ID,
authorization digest, allocation ID, UID/GID set, scope expectation, and
managed reclamation domain.
A launch record cannot have two bindings or two active allocations. A duplicate
launch binding, allocation reuse, state-sequence conflict, or UID-to-record
conflict MUST fail closed, block implicated activation, and generate a
high-severity audit event.
### 4.4 Storage failure behavior
On launch-record-store or allocator-store outage, write failure, unavailable
commit acknowledgement, integrity-chain failure, corruption, or host-binding
mismatch, the system MUST fail closed for new authorization-dependent work:
no new session may be activated, no new identity may be allocated, no new grant
may be issued, and no existing identity may be released or reused.
Existing sessions MUST follow their signed manifest degradation policy. Lifecycle
MUST continue containment, quiesce, termination, and observation when local
evidence permits, but MUST hold identity and resources whenever durable proof
is missing. Store repair or replacement MUST be an authenticated recovery
operation that appends its decision and preserves prior evidence; it MUST NOT
silently reset sequence or hash history.
---
## 5. Replay, idempotency, and exactly-once boundaries
| Concern | Required rule |
|---|---|
| Client and component requests | Every request MUST have a scoped idempotency key. Same key plus same authenticated input returns the original result; same key plus different input is rejected. |
| Approvals | Approval objects MUST include an issuer-authenticated nonce or monotonic sequence, expiry, and subject binding. Policy MUST reject replay, stale sequence, or duplicate contradictory approval. |
| Authorization manifest | Policy MUST issue one committed authorization result per accepted request key and canonical derivation input set. Conflicting replay is rejected. |
| Launch binding | Lifecycle/launch MUST reject a second binding for an authorization digest, allocation record, or authorization ID. A retry returns the existing binding/outcome. |
| Constructor ownership | Constructor attempt ownership MUST be a durable compare-and-set lease on the authorization ID. Only one owner may execute construction. |
| Grants | Grant issue and revoke MUST have exactly-once externally visible semantics through durable launch-record state transitions. Retried issuer calls MUST converge on the original grant state. |
| Audit | Senders deliver at least once. Audit deduplicates by stable event ID and preserves loss/duplicate counters. |
Exactly-once does not assert that an external remote effect happened once. An
adapter MUST report admitted, completed, rejected, cancelled, or indeterminate
remote outcome. The gateway MUST never retry an operation merely because its
own outcome is uncertain unless that operation's manifest-approved adapter
semantics explicitly make retry safe.
---
## 6. Durability and lifecycle commit points
| Lifecycle state | Durable commit point | Required consequence |
|---|---|---|
| `requested` | Request/idempotency record is committed. | Duplicate submission returns the original request result; no identity or scope exists. |
| `authorized` | Authorization manifest, policy signature, derivation inputs, and authorization decision are appended and `fsync`ed. | No UID/GID, scope, credential, or usable grant exists. |
| `constructing` | Constructor ownership lease, rollback ledger, and allocator `allocated` record are committed; binding is committed before identity installation. | Child remains blocked on the synchronization barrier; no runtime or usable grant exists. |
| `active` | Boundary proof, binding commit, grant-issue transition, privilege-drop result, and activation outcome are durably recorded. | Runtime may execute only after required audit binding and grant admission exist. |
| `quiescing` | Quiesce/termination admission closure and trigger are committed. | No new child, attachment, or gateway operation may begin. |
| `degraded` | Trigger, reduced grant set, compensating control, and reevaluation condition are committed. | Only declared remaining authority is available. |
| `terminated` | Steps 1–5 of the [session lifecycle](session-lifecycle.md) §5 protocol (admission denial, freeze, `SIGTERM` with bounded thaw, refreeze and `cgroup.kill`, no-live-process proof including the host credential scan) and the termination outcome are committed. | Identity remains held; steps 6–11 (grant release and connection closure, broker closure, unmount, socket removal, identity to reclamation, seal) follow in order; none may precede the proof. |
| `cleaned/sealed` | Cleanup evidence, reclamation/quarantine transition, and immutable seal are appended and `fsync`ed. | No managed grant/resource remains usable; record cannot be reopened. |
| `rejected` / `construction-failed` / `aborted` | Safe failure result and rollback evidence are committed. | No runnable partial session or usable grant remains; uncertain identity stays held. |
If durability of a transition cannot be confirmed, the externally authoritative
state MUST remain at the earlier safe state or become unavailable; it MUST NOT
advance optimistically. Status replicas MAY lag but MUST identify their
observation sequence and authoritative record reference.
---
## 7. Error classes and safe diagnostics
All errors MUST use stable machine-readable codes, include the applicable
launch-record and trace identity only when the caller is authorized to know
them, and omit secrets, private keys, credential bodies, raw mount paths,
other-session identifiers, and unredacted policy or gateway payloads.
| Class | Meaning | Caller result | Required system action |
|---|---|---|---|
| `reject` | Authentication, derivation, signature, freshness, schema, or authorization failed before construction. | Safe rule/input-class code; no session capability. | Emit denial audit; retain no privileged partial side effect. |
| `construction-failed` | Required constructor step, binding, audit prerequisite, boundary, privilege drop, or exec failed. | Failing phase and rollback/hold status, redacted as needed. | Reverse rollback; retain identity on uncertainty; seal failure when safe. |
| `degraded` | A declared dependency loss leaves only policy-approved reduced authority. | Affected operations, reevaluation condition, and correlation IDs. | Enforce reduced grant set and audit the transition. |
| `unavailable` | Required service/store/clock/systemd evidence cannot be safely used. | Retry guidance only where retry cannot broaden authority. | Fail closed for new authority; apply manifest behavior to existing sessions. |
| `audit-loss` | Required audit event/evidence could not be accepted or correlated. | Loss class and counter, without event payload secrets. | Apply manifest `stop`, `quarantine`, or `continue-with-loss-counter` behavior. |
An adapter MAY expose a typed operation error nested within these classes, but
it MUST NOT reveal upstream credentials, internal authorization rules, or data
belonging to another session. `termination-incomplete` is reported as a safe
lifecycle condition, not as successful termination.
---
## 8. Lifecycle restart reconciliation
### 8.1 Algorithm outline
On daemon start, `agentbound-lifecycle` MUST block new allocation and grant
issue until it has reconciled all nonterminal records. It MUST enumerate
persisted active/nonterminal launch records, allocator records, registered
systemd scopes, retained pidfds where available, cgroup evidence, gateway
connection/grant index, and audit delivery state.
For each authorization ID, lifecycle MUST:
1. Verify launch-record hash chain, signatures, key validity/revocation, host
   and boot binding, and record seal/state sequence.
2. Match a signed binding and allocator allocation record by immutable IDs, not
   by numeric UID or scope name alone.
3. Query systemd scope state and properties; corroborate with cgroup contents,
   PID-namespace-init state, and pidfd-watch evidence.
4. Query the gateway for grant and indexed-connection state, then deny new
   admission for an orphan, contradiction, or state requiring quiesce.
5. Compare audit evidence for required committed transition/event IDs and mark
   missing evidence under the manifest audit-loss policy.
6. Resume only the state action permitted by the reconciled durable record;
   never re-exec an uncertain constructor attempt.
7. For any contradiction, preserve containment and identity hold, emit an
   orphan/contradiction event, and quiesce or terminate unless the manifest
   explicitly permits a demonstrably safe degraded mode.
### 8.2 Authoritative-source precedence
Precedence applies to a single fact, not as permission for one source to erase
contradictory evidence. The order is:
1. **Signed, fsync-committed launch-record store** for authorized identity,
   immutable manifest/binding, lifecycle intent, grants, and terminal seal.
2. **Lifecycle-owned allocator store** for UID/GID allocation ownership,
   state sequence, reclamation, quarantine, and reuse eligibility.
3. **Live kernel evidence** held/observed by lifecycle: pidfds, cgroup state,
   PID-namespace-init evidence, and registered managed-domain inspection.
4. **systemd D-Bus scope state** (`UnitRemoved`/`PropertiesChanged` and query)
   for systemd-managed scope observation.
5. **Gateway connection/grant index** for current gateway admission and
   connection closure, corroborated against record grant state.
6. **Audit pipeline state** for evidence delivery/correlation and loss policy;
   it does not authorize execution or reclamation.
A lower-precedence source may prove a dangerous live condition that blocks a
higher-precedence intended transition. For example, a live pidfd or nonempty
cgroup blocks reclamation even if a record says `terminated`; a gateway index
showing a live connection blocks release until lifecycle revokes and receives
closure evidence. Conversely, systemd reporting a removed scope does not prove
no process exists outside an expected scope.
The default on absence, mismatch, corruption, or disagreement is **fail closed**:
deny new grants and allocations, preserve identity hold, mark the session
orphaned or termination-incomplete as appropriate, and quiesce/terminate using
lifecycle authority. Only a durable, reconciled proof permits activation,
cleanup completion, quarantine, or reuse.
---
## 9. Audit correlation obligations
Every security event submitted through this interface family MUST include the
R-AUD-1 correlation set: host ID, boot ID, authorization ID, allocation-record
ID, session ID, trace ID, execution UID, monotonic and wall-clock timestamps,
actor, outcome, and stable event ID. Fields that do not exist yet MUST be
explicitly `null`, not omitted. Kernel process evidence MUST additionally carry
PID namespace identity and process start time or pidfd-derived identity where
supported.
The audit pipeline MUST make loss counters observable and preserve the signed
launch-record trust anchor, clock source, retention class, hash-chain sequence,
and correction references. For the Phase 1 effect ontology, the correlator MUST
support reconstruction of `initiator → agent → session → process → effect` as
required by R-AUD-2. Missing evidence is a recorded loss or residual assumption,
not a reconstructed effect.
---
## 10. Deferred to WP1
The following are intentionally not specified by this WP0 skeleton:
1. **Message schemas** for every request, response, event, and D-Bus mapping.
2. **Field-level wire formats**, bounds, encodings, and error payload shapes.
3. **Versioning negotiation**, compatibility rules, and upgrade/downgrade
   protocol behavior.
WP1 MUST define these without weakening the component identities, transports,
authorization boundaries, signing rules, durability points, or fail-closed
reconciliation decisions frozen here.
