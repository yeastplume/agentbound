# ADR-0002: Gateway channel topology and session authentication

**Status:** Accepted for Phase 1 (topology and mechanism selected); WP1 spike verifies kernel-baseline assumptions listed in Decision 7  
**Version:** 0.5  
**Date:** 28 August 2026  
**Applies to:** Unix-governed profile, milestones 1B–1C; microVM projection per ADR-0003  
**Related:** [Phase 1 plan](../plans/phase-1-reference-implementation.md) §3.3, §4.2, §6.3–6.4; [technical report](../papers/technical-report.md) §3.2, §5; [manifest schema](manifest-schema.md); [component interfaces](component-interfaces.md); [ADR-0001](ADR-0001-execution-identity.md); [ADR-0003](ADR-0003-control-substrate.md)

## Revision history

- **0.1** — Proposed two mutually exclusive Linux-arm topologies (network Candidate N with mTLS/broker; local-socket Candidate L) and deferred selection to WP1.
- **0.2** — Selected Candidate L for Phase 1. Candidate N withdrawn from Phase 1 because it cannot satisfy the per-operation process leg of Invariant 13; deferred to a future multi-host ADR. Socket type fixed as `SOCK_SEQPACKET`; per-packet `SCM_CREDENTIALS`; descriptor transfer prohibited; `SO_PEERCRED` wording corrected; vsock CID lifetime rules added; WP1 scope narrowed to kernel-baseline verification.
- **0.3** — One connection per process: every packet's credential PID must equal the establishing PID; inherited or passed descriptors close the connection. Identifier terminology aligned with manifest schema §4 (`authorization_id` / `launch_record_digest`).
- **0.4** — Decision 1 heading names both legal topology values; attribution policy `required` for the evaluation arm; open questions closed via the register.
- **0.5** — Post-freeze editorial maintenance (no normative change): Decision 1 heading; per-packet and one-connection rules merged into one check; "session scope" used consistently.


## Context

Phase 1 requires every remote effect to cross `agentbound-gateway`, which exposes typed operations and propagates the session trace identity. Invariant 13 claims the attribution chain `initiator → agent → session → process → effect`. The gateway must therefore identify not only the session but the **process** that issued each operation, using evidence the kernel supplies rather than evidence the caller asserts.

No single conventional mechanism does this:

- a network namespace creates a network stack but does not authenticate a process;
- a pathname-protected Unix socket controls who can open the pathname but not which process is on an established connection;
- bearer credentials are copyable inside the session;
- a veth peer identifies a network attachment, not the calling process;
- mTLS or proof-of-possession proves that *some* process in the session holds the credential, not *which* process sent a given request;
- `SO_PEERCRED` is a kernel-supplied credential snapshot taken at connection establishment. It is authoritative for the connecting process but says nothing about later writers on a passed descriptor, and any PID it reports must be resolved to a live process with reuse protection;
- a credential broker reduces exportability while remaining drivable by any hostile process authorized to use it.

Version 0.1 of this ADR kept a network topology (Candidate N) open alongside the local-socket topology (Candidate L). Review showed that N's authentication mechanisms (per-session mTLS, veth-mapping broker) can reach session-level attribution only. Keeping N while requiring the process leg made the ADR's own selection criterion decide the question implicitly. This revision decides it explicitly.

## Decision 1: Phase 1 permits only `none` and `local-socket`

```text
session (no network interface, including loopback)
  └── exactly one bind-mounted AF_UNIX SOCK_SEQPACKET socket
        └── agentbound-gateway (on-host)
```

1. The session network namespace MUST contain no interface, including loopback, and no route. Seccomp MUST deny every socket family except `AF_UNIX`, and MUST deny `AF_UNIX` `SOCK_STREAM`/`SOCK_DGRAM` `connect` unless the runtime profile explicitly requires local IPC inside the session (recorded in the manifest).
2. Exactly one gateway socket is bind-mounted into the session at the manifest's `gateway_socket` mount. No other host Unix socket is visible in the mount namespace. The abstract socket namespace is isolated by the session's network namespace.
3. The socket type MUST be `SOCK_SEQPACKET`. One complete typed operation MUST be carried in exactly one `sendmsg`; the gateway MUST reject any packet that does not parse as exactly one operation and MUST NOT reassemble operations across packets. `SOCK_STREAM` is not permitted because ancillary data attaches to stream segments, not to application messages.
4. The manifest's `gateway.channel_topology` has exactly two legal values in Phase 1: `none` (gateway-free form, the only form constructible at milestone 1A; no channel at all, same no-network boundary) and `local-socket` (this ADR, from 1B). `agentbound-launch` MUST fail closed on any other value or on any network-topology field.

The network topology is **withdrawn from Phase 1**, not rejected in principle. It is the natural mechanism for off-host gateways in later multi-host profiles and will be the subject of a separate ADR whose first obligation is to state honestly which leg of the attribution chain it can and cannot prove.

## Decision 2: per-operation process evidence

Authentication of the connection and attribution of each operation are separate steps with separate evidence.

**Connection establishment.** Immediately after `accept`, the gateway MUST read `SO_PEERCRED` and MUST resolve the reported PID to a pidfd (`pidfd_open`) and read its start time and PID-namespace identity. If the process has exited or the start time does not match, the connection MUST be closed unauthenticated. The gateway binds `(pidfd, start time, UID, GID, PID namespace, session scope, boot ID)` to exactly one active execution-identity allocation and launch record via `agentbound-lifecycle`'s authoritative index. A UID that maps to no active allocation, or to an allocation in `reclaiming` or `quarantined`, MUST be refused.

**Every operation.** The gateway MUST enable `SO_PASSCRED` before reading any data. Each packet MUST carry exactly one kernel-supplied `SCM_CREDENTIALS` control message; packets with zero or more than one MUST be rejected. The credential PID MUST equal the connection's establishing PID and MUST resolve to the same pidfd/start time, UID, PID namespace, and session scope as the connection's bound allocation. Any mismatch denies the operation, closes the connection, and emits `gateway.process_mismatch`. The per-packet credential is the process identity recorded against the effect.

**One connection per process.** The rule above means one connection serves exactly one process. A descriptor inherited across `fork`, passed, or leaked fails the PID check on its first packet; the gateway need not know how the descriptor was acquired. `SCM_RIGHTS` on the gateway protocol MUST be rejected. Children and other processes in the session open their own connections, each authenticated at establishment. Cross-allocation use fails the allocation check and is a conformance failure if accepted.

**pidfd unavailability.** If the pinned kernel cannot supply pidfds or process start time for the credential PID, the constructor MUST record the condition as a residual assumption in the launch record and the gateway MUST refuse the connection when the manifest's attribution policy is `required`. It MUST NOT fall back to PID alone silently. For every Linux evaluation-arm run the attribution policy is `required`, so Invariant 13 is measured without this residual assumption; a production manifest choosing `best-effort` records the assumption.

## Decision 3: caller and operation authority

Authentication identifies a session and a process, not the semantic intent of that process. By default, any process under the session execution identity that can reach the gateway socket may invoke operations granted to that session. The gateway MUST assume a hostile process can drive every granted operation.

A manifest MAY narrow an operation to an executable digest or process role only when the pinned kernel can prove that property from the pidfd (for example via `/proc/<pid>/exe` resolved through the pidfd) without trusting caller-supplied metadata. Phase 1 does not require such narrowing.

For every operation, `agentbound-gateway` MUST:

1. verify the per-packet credential per Decision 2;
2. resolve it to exactly one active launch record;
3. verify the typed operation appears in that record's `gateway.operations`;
4. verify arguments, destination, tenant, body schema, response-size limit, and resource budget;
5. attach the immutable session trace identity, the authenticated operation-process identity, and the operation identifier to the upstream request and the audit event;
6. reject if the session is quiescing, terminating, revoked, expired, or the manifest's audit-loss policy requires a stop.

## Decision 4: connection lifetime and revocation

Connection authentication is not permanent authority.

1. The gateway MUST re-check active launch-record and grant status on **every operation**.
2. A connection established before a revocation MUST fail its next operation after the revocation record is committed. An operation already admitted is handled per its predeclared policy: complete-and-record, cancel where the adapter supports safe cancellation, or record an indeterminate remote outcome. For the Git staging-ref adapter the policy is **complete-and-record**: a push already accepted by the Git host is not retracted by the gateway; the staging ref remains under branch-protection control and the audit record marks the operation as admitted-before-revocation.
3. New connections after revocation MUST fail authentication.
4. Exit of the establishing process ends the connection's authority: the gateway MUST close the connection on the peer pidfd exit event, and any packet arriving from another PID is rejected per Decision 2.
5. On entry to termination, `agentbound-lifecycle` MUST first instruct the gateway to **deny admission** of new operations for the launch record (grant records are not yet released), then run the termination protocol in [session lifecycle](session-lifecycle.md) §5, then close indexed connections and release grant records. The gateway MUST acknowledge zero remaining connections before identity reclamation proceeds.
6. A connection whose launch record is sealed, missing, has a different boot binding, or no longer maps to a live allocation MUST be closed without processing a request.
7. Gateway restart MUST reconstruct active grants only from the signed launch-record store and `agentbound-lifecycle`'s allocation index. It MUST NOT preserve transport connections across restart.
8. The inference adapter (1C) MUST perform execution-binding checks per operation; a separate connection pool per binding is not required because per-operation checks are authoritative.

## Decision 5: trace and audit binding

Every connection record and every operation record MUST include:

- topology (`local-socket`) and socket type;
- authorization ID, authorization-manifest digest, launch-binding digest, and launch-record digest;
- global agent ID, session ID, and trace ID;
- execution UID, allocation-record ID, host ID, boot ID, PID namespace, scope;
- establishing peer: UID/GID, PID, start time, pidfd acquisition result;
- per operation: credential PID, start time, pidfd acquisition result, and process-mismatch result if any;
- establishment, last-operation, denial, revocation, and close events with reason;
- any admitted-before-revocation or indeterminate operation.

The trace identity is correlation metadata, not authentication evidence. Caller-supplied trace IDs MUST be ignored unless they equal the authenticated launch record's trace ID.

## Decision 6: microVM control-arm projection

ADR-0003 exposes exactly one vsock path from the guest to the host gateway endpoint. `AF_VSOCK` is not `AF_UNIX`; kernel peer credentials do not cross the VM boundary. The vsock path is therefore a pre-registered **substrate-equivalent** projection of the single-channel property, not an implementation of Decision 2.

1. The host endpoint MUST bind the host-observed peer CID to a **non-reusable VM instance token** issued at launch, the VMM process pidfd and start time, the jailer identity, and the active launch record. A guest-supplied CID, token, or trace ID is not sufficient.
2. CIDs are reusable after VM teardown. All mappings and connections for a CID MUST be invalidated, and the gateway MUST acknowledge zero connections, **before** `agentbound-lifecycle` permits the CID to be reassigned.
3. The VM arm claims **session-level** attribution for the process leg of Invariant 13 unless a guest-side trusted witness supplies per-operation process evidence. This is a pre-registered difference between arms and is recorded in the ADR-0003 per-test classification and the traceability matrix; it MUST NOT be silently equated with the Linux arm's per-process evidence.
4. Operation-time grant checks, revocation, and termination ordering are identical to Decisions 3–4.

## Decision 7: WP1 verification scope

WP1 no longer selects a topology. It MUST verify, on the pinned kernel and systemd baseline, and record as pass/fail with evidence:

| Item | Required result |
|---|---|
| `SOCK_SEQPACKET` + `SO_PASSCRED` | Exactly one `SCM_CREDENTIALS` per `recvmsg` for one `sendmsg`; oversize packets truncate rather than split |
| pidfd from credential PID | `pidfd_open` succeeds for live peer; start-time and PID-namespace reads succeed via pidfd; reuse of a recycled PID is detected |
| Descriptor transfer | `SCM_RIGHTS` rejected; a connected descriptor inherited by a child or passed to another process fails the establishing-PID check and closes the connection |
| Abstract socket isolation | Abstract-namespace sockets of the host and sibling sessions are unreachable from the session's network namespace |
| Revocation latency | Next operation after committed revocation is denied; connections closed at termination before reclamation |
| Bypass corpus | Plan §6.4 corpus, local-socket realization, passes |
| TCB accounting | Gateway authentication and mapping code counted under the SLOC rules in `phase-1-requirements.md` §12 |
| Failure behaviour | Gateway, lifecycle, policy, and audit loss follow manifest-declared stop/quarantine behaviour |
| Diagnostics | Denial names the requirement ID, authorization ID, launch-record digest, and trace ID without leaking another session's identifiers |

If any item fails on the pinned baseline, this ADR is reopened; the constructor MUST NOT be built against an unverified assumption.

## Consequences

- Gate 3's mechanism is fixed; its provisional status is removed. Gate 3 can now fail only on evidence, not on an unselected mechanism.
- No network namespace configuration, nftables/eBPF rules, mTLS PKI, or host connection broker is in the Phase 1 TCB. The gateway authentication path (`SO_PEERCRED`, `SO_PASSCRED`, pidfd resolution, allocation index lookup) is in the TCB and the SLOC accounting.
- The gateway MUST be on-host. Remote services are reached by the gateway's adapters, not by the session.
- No credential is exportable from the session: kernel credentials cannot be copied.
- Neither this decision nor any other prevents a hostile process inside an authorized session from invoking an authorized operation; typed adapters and narrow grants remain the semantic boundary.
- Every operation checks live authority, so connection pooling cannot bypass revocation.
- Multi-host deployment is deferred; a future ADR must address the process-attribution gap of network transports explicitly.

## Alternatives considered

### Candidate N — network topology (veth + mTLS/PoP or host broker)

Withdrawn from Phase 1. Both authentication variants identify the session, not the operation-issuing process, so the Invariant 13 chain stops at the session leg. A host-side per-process broker could close the gap but would add a privileged component larger than the mechanism it replaces. Retained as the intended mechanism for a later multi-host ADR.

### Generic HTTP/CONNECT proxy

Rejected. Destination allowlists do not constrain method, payload semantics, tenant, redirects, SSRF behaviour, or cumulative effects.

### Exportable bearer token in the session

Rejected. A hostile process can copy and replay it outside the session until expiry or revocation.

### `AF_UNIX SOCK_STREAM` with per-message credentials

Rejected. Credentials attach to stream segments; an application message can straddle a credential boundary and a passed descriptor permits interleaved writers.

### Pathname permissions alone on an `AF_UNIX` socket

Rejected. They control opening the path but provide no durable process identity for an established or passed descriptor.

### Both veth and Unix socket for defence in depth

Rejected. Two effect paths enlarge the bypass surface and violate the single-channel property.

## Open questions

None. Both WP0 questions are answered in the [open-question register](open-question-register.md) (attribution policy `required` for the evaluation arm; no guest-side witness in Phase 1). Kernel-baseline items are in Decision 7.
