# ADR-0002: Gateway channel topology and session authentication

**Status:** Proposed for WP0 review; mechanism selection deferred to WP1 spike  
**Date:** 28 August 2026  
**Applies to:** Unix-governed profile, milestones 1B–1C  
**Related:** [Phase 1 plan](../plans/phase-1-reference-implementation.md) §3.3, §4.2, §6.3–6.4; [technical report](../papers/technical-report.md) §3.2; [manifest schema](manifest-schema.md) §3.6; [ADR-0001](ADR-0001-execution-identity.md); [ADR-0003](ADR-0003-control-substrate.md)

## Context

Phase 1 requires every remote effect to cross `agentbound-gateway`, which exposes typed operations and propagates the session trace identity. A network namespace, path-protected socket, bearer token, or workload identity alone is insufficient to prove which local session initiated an operation:

- a network namespace creates a network stack but does not authenticate a process;
- a pathname-protected Unix socket controls who can open the pathname but not which process is on an established connection;
- bearer credentials are copyable inside the session;
- a veth peer identifies a network attachment, not necessarily the calling process;
- `SO_PEERCRED` reports credentials captured when an `AF_UNIX` connection is established, but connection lifetime, descriptor passing, PID reuse, process exit, and revocation still require explicit rules;
- a credential broker can reduce exportability while remaining drivable by any hostile process authorized to use it.

The design also permits two mutually exclusive channel topologies in the Linux arm. Mixing their assumptions would create bypasses and make Gate 3 unfalsifiable. WP0 therefore freezes the candidate set, required evidence, lifetime rules, and selection criteria. WP1 selects one topology and authentication mechanism after focused spikes.

## Decision 1: mutually exclusive Linux-arm topologies

Every manifest MUST select exactly one of these topologies. `agentbound-launch` MUST fail closed if fields from both appear.

### Candidate N — network topology

```text
session network namespace
  └── one veth
        └── host nftables/eBPF policy
              └── agentbound-gateway address and port only
```

Requirements:

1. The session has one veth and no other interface except loopback. The default route leads only to the host-side enforcement point.
2. Host nftables or eBPF policy permits only the gateway address and port and, if the manifest requires DNS, one constructor-operated resolver. It drops host, bridge, sibling, link-local, metadata, multicast, and all other destinations for IPv4 and IPv6.
3. The session has no reachable host Unix socket, no inherited socket, and no `CAP_NET_RAW` or `CAP_NET_ADMIN`.
4. Seccomp blocks `AF_PACKET`, `AF_VSOCK`, raw sockets, and every socket family the runtime does not need. UDP is blocked unless the approved resolver requires it; QUIC is not permitted.
5. Authentication is one of N1 or N2 below.

**N1 — per-session mTLS / proof-of-possession.** `agentbound-launch` provisions a non-exportable or broker-held private key after every boundary is installed. The certificate or signed workload document binds global agent ID, session ID, execution-identity allocation record ID, launch-record ID, audience (`agentbound-gateway`), and expiry. `agentbound-gateway` verifies the proof on every connection and checks that the launch record is active and not revoked.

**N2 — host-side connection broker.** A privileged host component accepts traffic only from the session's veth peer and maps the peer interface and network namespace to one active execution-identity allocation and launch record. It establishes or authenticates the upstream gateway connection. The mapping is an explicit TCB component and MUST be immutable for the session lifetime. Source IP or network identity alone is insufficient evidence.

### Candidate L — local-socket topology

```text
session (no network interface)
  └── one explicitly mounted AF_UNIX stream socket
        └── agentbound-gateway
```

Requirements:

1. The session has no non-loopback network interface and no route. Seccomp blocks all network socket families except the single `AF_UNIX` stream use required by the adapter.
2. Exactly one gateway socket is bind-mounted into the session at the manifest path. No other host Unix socket is visible. The socket is a single-purpose operation endpoint, not a general relay.
3. The socket pathname and filesystem permissions are defense in depth only. Connection establishment evidence is `SO_PEERCRED`, collected immediately after `accept`, plus a peer pidfd obtained from the reported PID where supported. For **each operation**, the gateway MUST enable `SO_PASSCRED` and require kernel-supplied `SCM_CREDENTIALS` on the operation message, then bind its PID to a pidfd or PID+start-time and verify that process remains in the authenticated session's allocation, PID namespace, and scope. This per-message evidence—not caller metadata or the connection's original peer alone—is authoritative for process attribution. The gateway binds `(operation pidfd or PID+start-time, UID, GID, PID namespace, scope, boot ID)` to the active allocation and launch record.
4. If a race prevents acquisition or verification of the pidfd/process start time, authentication fails. Numeric UID alone is insufficient because UIDs are reclaimed.
5. `SCM_RIGHTS` on the gateway protocol is forbidden. The gateway MUST reject ancillary descriptors. For Phase 1 process attribution, an established gateway descriptor MUST NOT be transferred between processes. The gateway MUST nevertheless require kernel-backed per-operation process evidence; descriptor-passing, creator exit, PID/UID reuse, or forged process identity MUST fail closed. A topology unable to provide an equivalent operation witness reports session-level attribution only and MUST NOT claim full Invariant 13 conformance.

## Decision 2: caller and operation authority

Authentication identifies the session, not the semantic intent of a process. By default, any process under the session execution identity that can reach the gateway channel may invoke operations granted to that session. The gateway MUST assume a hostile process can drive every granted operation.

A manifest MAY narrow an operation to an executable digest, process role, or child-session identity only when the selected authentication mechanism can prove that property without trusting caller-supplied metadata. Phase 1 does not require such process-level narrowing.

For every request, `agentbound-gateway` MUST:

1. authenticate the connection under the chosen mechanism;
2. resolve it to exactly one active launch record;
3. verify the typed operation appears in that manifest;
4. verify arguments, destination, tenant, body schema, response-size limit, and resource budget;
5. attach the immutable session trace identity, authenticated operation-process identity, and operation identifier to the upstream request and audit event;
6. reject if the session is quiescing, terminated, revoked, expired, or audit policy requires a stop.

## Decision 3: connection lifetime and revocation

Connection authentication is not permanent authority.

1. The gateway MUST re-check active launch-record and grant status on **every typed operation**, not merely at connection establishment.
2. A connection established before a revocation MUST fail its next operation after the revocation record is committed. An operation already admitted is handled according to its manifest policy: complete, cancel where the adapter supports safe cancellation, or record an indeterminate remote outcome. The policy MUST be predeclared per operation.
3. New connections after revocation MUST fail authentication.
4. Exit of the process that established an `AF_UNIX` connection does not transfer authority. Another process holding the descriptor is independently identified by per-operation `SCM_CREDENTIALS`; its operation is accepted only if it maps to the same active session and grant. A peer pidfd exit event is logged and MAY close the connection under a stricter manifest policy.
5. At session termination, the lifecycle helper MUST first mark the launch record terminating and disable/revoke admission for all gateway grants, then close indexed connections, terminate descendants, and only then release credential/grant records. The gateway MUST acknowledge zero remaining connections before identity reclamation proceeds. “Revoke” means deny use immediately; “release” means final lifecycle cleanup after descendant death.
6. A stale connection whose launch record is sealed, missing, has a different boot binding, or no longer maps to the execution-identity allocation MUST be closed without processing a request.
7. Gateway restart MUST reconstruct only active grants from the signed launch-record store. It MUST NOT preserve unauthenticated transport sessions across restart.

## Decision 4: trace and audit binding

Every connection record MUST include:

- selected topology and authentication mechanism;
- launch-record ID and manifest digest;
- global agent ID, session ID, and trace ID;
- execution UID, allocation-record ID, host ID, and boot ID;
- for Candidate L: peer UID/GID, PID, process start time, and pidfd acquisition result;
- for Candidate N1: certificate/workload-document digest, key ID, issuer, audience, and expiry;
- for Candidate N2: veth/interface identity, network namespace identity, broker mapping record, and broker identity;
- establishment, last-operation, revocation, and close events;
- close reason and any indeterminate operation.

The trace identity is correlation metadata, not authentication evidence. Caller-supplied trace IDs MUST be ignored unless they match the authenticated launch record.

## Decision 5: WP1 selection criteria

WP1 MUST implement focused spikes for Candidate L and at least one Candidate N mechanism. It MUST record:

| Criterion | Required result |
|---|---|
| Connection/process-to-session binding | Every operation resolves through kernel evidence to one process and one active launch record; descriptor transfer and stale UID/PID reuse tests fail closed. A network candidate without per-operation process evidence fails Invariant 13 and cannot be selected for the conforming arm. |
| Exportability | Whether a hostile session process can copy the authenticator; bearer-only mechanisms fail this criterion |
| Revocation | Next operation after committed revocation is denied; all indexed connections close at termination |
| Descriptor passing | Cross-session transfer is prevented by isolation; same-session transfer does not bypass per-operation checks |
| Gateway bypass | Plan §6.4 corpus passes for the topology |
| TCB size | Authentication and mapping code included in the privileged/trusted code accounting |
| Failure behavior | Gateway, broker, policy, and audit loss follow manifest-declared stop/quarantine behavior |
| Portability | Required kernel APIs and distribution constraints recorded, never silently degraded |
| Diagnostics | Denial names the policy/requirement and correlation IDs without leaking another session's data |

The final ADR-0002 revision MUST select one Linux-arm topology and mechanism before milestone 1B implementation. The unselected topology MAY remain as a documented alternative but MUST NOT appear in the effective manifest of the evaluation arm.

## Decision 6: microVM control-arm mapping

ADR-0003 uses a single veth for the network topology and a single vsock service for the non-network control-arm topology. `AF_VSOCK` is **not** `AF_UNIX` and peer credentials do not transfer across this boundary. The vsock path is therefore a pre-registered **substrate-equivalent**, not an identical implementation of Candidate L.

For the vsock realization:

- Firecracker's VM identity (VMM process, jailer, VM configuration digest, guest CID, and launch-record binding) is the authoritative peer evidence;
- the host-side endpoint MUST map the guest CID and VMM/jailer identity to exactly one active launch record;
- a guest-supplied CID or trace ID is not sufficient;
- operation-time grant checks and termination invalidation are identical to Decisions 2–4;
- ADR-0003's test-equivalence table governs comparison and explicitly records the changed attack mechanics.

This clarification does not add a third Linux-arm topology.

## Consequences

- Gate 3 remains provisional until WP1 finalizes this ADR.
- The gateway authentication path and any host broker are in the trusted computing base and line-count review.
- Candidate L has a smaller network attack surface but introduces process-credential and descriptor-lifetime subtleties.
- Candidate N composes naturally with remote gateways but requires strong proof-of-possession or a trusted host broker.
- Neither candidate prevents a hostile process inside an authorized session from invoking an authorized operation; typed adapters and narrow grants remain the semantic boundary.
- Every operation checks live authority, so connection pooling cannot bypass revocation.

## Alternatives considered

### Generic HTTP/CONNECT proxy

Rejected. Destination allowlists do not constrain method, payload semantics, tenant, redirects, SSRF behavior, or cumulative effects.

### Exportable bearer token in the session

Rejected as the primary mechanism. A hostile process can copy and replay it outside the session until expiry or revocation.

### Pathname permissions alone on an `AF_UNIX` socket

Rejected. They control opening the path but do not provide durable session identity for an established or passed descriptor.

### Source IP or veth address alone

Rejected. It identifies a network attachment and is susceptible to mapping error or reuse; it does not prove the local process or active launch record without broker state.

### Both veth and Unix socket for defense in depth

Rejected. Two effect paths enlarge the bypass surface and violate the single-channel property. Defense in depth belongs inside one path (peer evidence plus live grant checks), not in a second path.

## Open questions for WP0 review

1. Should Candidate L be the default because its no-network session makes bypass testing simpler, or Candidate N1 because it more closely resembles production remote-gateway deployment?
2. Is peer pidfd acquisition sufficiently portable across the Phase 1 kernel baseline, or should Candidate L require a small host broker with privileged process inspection?
3. Should same-session descriptor passing of an established gateway connection be prohibited outright, even though session authority is unchanged, to simplify causal attribution to a process?
4. Which operation-admission policy is safe for a Git push already in progress when revocation arrives: complete-and-record or abort-and-mark-remote-state-unknown?
5. Does the inference adapter require a separate connection pool per execution binding, or are per-operation binding checks sufficient?
