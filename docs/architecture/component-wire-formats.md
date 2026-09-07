# Agentbound Component Wire Formats
**Version:** 0.2  
**Status:** Draft (WP2) — defines the items deferred by [component interfaces](component-interfaces.md) §10  
**Date:** 28 August 2026  
**Applies to:** Phase 1 Unix-governed reference implementation, milestones 1A and 1B  
**Related:** [component interfaces](component-interfaces.md), [manifest schema](manifest-schema.md), [session lifecycle](session-lifecycle.md), [execution-identity lifecycle](execution-identity-lifecycle.md)

## Revision history

- **0.2** — WP3.1 (independent validation findings 4 and 6): adds §10, the in-session gateway protocol `agentbound.gateway.v0.1`, which was implemented in WP3 but specified in no frozen document — the omission that let it ship without the idempotency key component-interfaces §5 mandates. Specifies the key, the input digest that defines "same authenticated input", the replay and conflict rules, and the requirement that completed outcomes be durable across a gateway restart. Adds §11: cross-daemon calls MUST be time-bounded, and a call that can close a cycle MUST be bounded asymmetrically. Adds the `outcomes` member of `record_budget` to §3. No 0.1 member list changes.
- **0.1** — Initial WP2 definition: message envelope, per-operation schemas for the 1A component pairs, error payload, event schema, versioning rules, and the 1A policy-to-launch delivery shape.

---

## 1. Purpose and normative language

Component interfaces §10 deferred (1) message schemas, (2) field-level wire formats and error payload shapes, and (3) versioning negotiation. This document defines them for milestone 1A. It MUST NOT weaken any component identity, transport, authorization boundary, signing rule, durability point, or fail-closed reconciliation decision frozen there; where this document is silent, that document governs.

The key words **MUST**, **MUST NOT**, **SHOULD**, and **MAY** are to be interpreted as described by RFC 2119. The reference implementation is `crates/ab-common/src/wire.rs` and the per-component handlers; the closed member lists below are enforced by the same closed-object validator used for manifests (unknown or duplicate member ⇒ reject).

## 2. Transport framing and envelope

Every message is exactly one `SOCK_SEQPACKET` datagram containing one RFC 8785 canonical JSON object of at most **65 536 bytes**. A receiver MUST reject a truncated datagram (`MSG_TRUNC`), a non-canonical encoding, a non-object, or a parse failure under the manifest-schema §2.4 rules, closing the connection without processing.

### 2.1 Request

```text
{"body":{...},"idempotency_key":"<1–128 bytes>","op":"<operation>","v":"agentbound.wire.v0.1"}
```

Exactly these four members. `idempotency_key` is scoped by the receiver to (caller UID, `op`, target record); a repeated key with an identical canonical `body` returns the original reply; a repeated key with a different `body` returns class `conflict`.

### 2.2 Reply

```text
{"body":{...},"class":"ok"|<error class>,"ok":true|false,"v":"agentbound.wire.v0.1"}
```

On `ok:false`, `body` is the error payload of §6. A reply MUST be sent for every accepted request; a receiver that cannot produce a reply closes the connection, which the caller treats as `unavailable`.

### 2.3 Descriptor-carrying messages

Only `register_session` (launch → lifecycle) carries `SCM_RIGHTS`. The ancillary data MUST contain exactly the descriptors the body's `descriptors` array names, in order. A receiver MUST close every descriptor received on any other operation and reject the message.

## 3. Lifecycle operations (launch → lifecycle; CLI → lifecycle)

Peer rule: `reserve_identity`, `commit_binding`, `register_session`, `report_activation`, `report_construction_failed` are accepted only from UID 0 (the constructor identity). `status`, `list`, `terminate`, `quiesce`, `revocation_signal` are accepted from UID 0 and the configured CLI UID(s); the lifecycle daemon enforces the manifest's actor rules before acting.

| `op` | Request `body` (closed) | Reply `body` on success |
|---|---|---|
| `reserve_identity` | `authorization_id`, `authorization_manifest_digest`, `agent_global_id`, `session_id`, `trace_id`, `reclamation_domain_id` | `allocation_id`, `uid`, `gids` (array), `state_seq` |
| `commit_binding` | `allocation_id`, `launch_binding` (object, §3.7), `envelope` (§4 signature envelope), `authorization_manifest` (object), `manifest_envelope` | `launch_record_digest`, `store_seq` |
| `register_session` | `allocation_id`, `launch_record_digest`, `scope_id`, `pid_namespace_id`, `init_pid`, `session_dir` (host path of the per-session runtime directory, one of the closed managed-domain paths), `descriptors`: array of `{"kind":"init_pidfd"|"cgroup_dir"|"rootfs_mount","index":n}` | `registered:true` |
| `report_activation` | `allocation_id`, `launch_record_digest`, `runtime_artifact_digest`, `privilege_disposal` (object: `uid`, `gids`, `no_new_privs`, `cap_bounding_empty`, `seccomp`) | `state:"active"` |
| `report_construction_failed` | `allocation_id`, `launch_record_digest` (nullable), `failed_step` (0–9), `rule`, `ledger` (array of `{"resource","action","result"}`) | `state:"construction-failed"`, `identity_state` |
| `status` | `authorization_id` **or** `launch_record_digest` | `state`, `identity_state`, `reason` (nullable), `observation_seq`, `record_ref` |
| `list` | `{}` | `sessions`: array of `{authorization_id, launch_record_digest, state}` |
| `terminate` | `launch_record_digest`, `reason` (one of `client_request`, `operator_override`, `revocation`, `quiesce_bound_expired`, `recovery`), `bound_s` (optional, ≤ manifest bound) | `state` (`terminated` or `termination-incomplete`), `evidence` (§5 step results) |
| `quiesce` | `launch_record_digest`, `reason`, `bound_s` | `state:"quiescing"` |
| `revocation_signal` | `launch_record_digest`, `trigger` (a manifest-schema §3.6 trigger name), `source` | `behaviour` (`terminate`/`quiesce`/`continue-degraded`), `state` |

Server-side rules: `commit_binding` verifies the constructor envelope, the manifest envelope, both schemas, the §3.1 correspondence checks, and that `allocation_id` is in state `allocated` for the same `authorization_id` and digest; it appends the launch record and allocation reference in one fsynced transaction and rejects a second binding for the same allocation, authorization ID, or manifest digest (`conflict`). `register_session` is accepted only after a committed binding. `report_activation` moves the identity to `in-use`. Any `terminate` reply of `terminated` MUST be backed by the recorded no-live-process proof.

## 4. Policy operations (CLI → policy)

| `op` | Request `body` | Reply `body` |
|---|---|---|
| `submit_request` | `request` (manifest-schema §2 object), `initiator_credential_ref` MUST equal `request.initiator_credential_ref` | `authorization_id`, `authorization_manifest` (object), `envelope`, `state:"authorized"` |
| `request_status` | `authorization_id` | `state` (`authorized`/`rejected`), `reason` (nullable) |

Rejection returns `ok:false`, class `reject`, `rule` = the failing schema rule or derivation input class (`unknown_principal`, `unknown_task`, `unknown_runtime`, `unknown_resource`, `initiator_unauthenticated`, `approval_expired`, `approval_replayed`, `approval_missing`, `scheduled_without_owner`, `authority_exceeded`, `budget_exceeds_policy`, `continue_degraded_not_permitted`).

## 5. Policy-to-launch delivery at 1A

Component interfaces §3.2 requires the constructor to accept the manifest from the policy service identity. In the 1A reference deployment the constructor is a short-lived process without a listener; delivery is by **file handoff under the policy service's ownership**: policy writes `<spool>/<authorization_id>.manifest.json` (the signed pair `{"authorization_manifest":{...},"envelope":{...}}`, canonical) with mode 0640, owner `agentbound-policy`, group `agentbound-launch`, into a directory writable only by policy. The constructor opens the file `O_NOFOLLOW`, verifies via `fstat` that the owner is the configured policy UID and the mode grants no other-write, then applies every §3.2 verification (canonical bytes, signature, key, freshness, digest). File ownership is the peer-identity check for this edge; nothing in the file is trusted before the signature verifies. The CLI passes only the `authorization_id`, never manifest bytes.

## 6. Error payload

```text
{"detail":"<redacted human text>","rule":"<stable code>"}
```

optionally with `launch_record_digest` and `trace_id` when the caller is authorised to know them. Classes: `invalid` (envelope/schema), `unauthenticated` (peer UID not permitted), `unauthorized` (peer permitted but operation not permitted for this record), `reject`, `construction-failed`, `conflict` (idempotency or CAS), `unavailable`, `audit-loss`, `internal`. No other-session identifier, path, key, or credential appears in `detail`.

## 7. Audit event schema (all components → audit)

`op` = `emit`; `body` = one event object with exactly the R-AUD-1 members produced by `ab_common::audit::event` plus `event_id` (`sha256:` digest of the canonical event without `event_id`). Audit deduplicates by `event_id`, appends `{"event":..., "prev":<digest>, "seq":n}` to the hash-chained store, and replies `{"accepted":true,"seq":n}` or `{"accepted":true,"duplicate":true}`. Event `detail` member lists are closed per event kind in the reference implementation (`crates/agentbound-audit/src/events.rs`).

## 8. Versioning

`v` is a single string; there is no negotiation. A receiver supporting a different version replies `invalid` with rule `unsupported_version` and closes. A wire version change is a new document revision; a change to any closed member list is a new wire version. Components of one deployment MUST be upgraded together; mixed versions fail closed rather than degrade.

## 9. Store record formats (informative)

Allocator store (SQLite WAL, `synchronous=FULL`): table `alloc(seq INTEGER PRIMARY KEY, allocation_id, uid, gid, state, state_seq, authorization_id, manifest_digest, agent_global_id, session_id, trace_id, host_id, boot_id, scope_id, pidns_id, domain_id, actor, wall_clock, monotonic_ns, evidence, prev_hash, hash)`; every state change is a new row; `hash = SHA-256(prev_hash || canonical row)`. Launch-record store: same shape with `kind ∈ {binding, event, seal, correction}` and the canonical signed pair stored once by digest. Both are readable only by the lifecycle daemon.

## 10. In-session gateway protocol (`agentbound.gateway.v0.1`, 1B)

Spoken by a session workload over the projected socket `/run/gateway.sock`; distinct from the component protocol of §2 because the
peer is untrusted and is authenticated per connection and per packet (ADR-0002). One JSON line per request, ≤ 64 KiB, followed by
`payload_len` raw bytes when non-zero.

```text
{"args":<object>,"idempotency_key":"<1–128 bytes>","operation":"<name>","operation_id":"<op:…>","payload_len":<n>,"payload_sha256":"sha256:<hex>","v":"agentbound.gateway.v0.1"}
```

Exactly these seven members. `payload_sha256` is required when `payload_len > 0` and MUST match the payload bytes. `operation_id`
MUST NOT contain a space. The reply is the §2.2 reply envelope; a payload-bearing request first receives
`{"awaiting_payload":<n>,"operation_seq":<k>}` and then the final reply.

**Idempotency (component-interfaces §5).** The key is scoped to `(allocation, operation_id, idempotency_key)`. The *authenticated
input* is the canonical JSON of the request object with `idempotency_key` removed (so: `args`, `operation`, `operation_id`,
`payload_len`, `payload_sha256`, `v`); its SHA-256 is the **input digest**.

- Same key, same input digest → the gateway MUST return the **original reply** (same `operation_seq`, same `result`), MUST NOT
  execute the adapter again, MUST NOT consume budget, and MUST emit `gateway.operation_replayed`.
- Same key, different input digest, or same key used for a different `operation` → `conflict` / `idempotency_conflict`.
- A key is recorded only when the adapter reports **completion**; a denied or failed operation records nothing, so a retry after a
  denial is a fresh attempt (and a fresh in-scope effect, per test-catalogue §5).
- Completed outcomes MUST be **durable before the reply is released** and MUST survive a gateway restart: they are appended to the
  session's hash-chained lifecycle record as `outcomes` (via `record_budget`, §3) and restored on activate/reconstruct. If the
  outcome cannot be recorded the reply is `unavailable` / `outcome_persist` and admission closes — the same fail-closed rule as
  budget consumption. Rationale: a session survives a gateway restart, opens a new connection and retries; without durable outcomes
  a non-idempotent adapter operation (a staging push) would execute twice. This was a real defect at WP3 (WP3.1 finding 6).

Budget consumption is checked and made durable *before* the adapter runs (deny rules `budget_operations`, `budget_bytes`,
`budget_objects`); outcome persistence happens *after*. Both use the same `record_budget` call and the lifecycle store refuses a
regression of either (`budget_regression`, `outcome_rewrite`).

**`record_budget` (gateway → lifecycle), added to §3.** `body` = `{"launch_record_digest", "budget": {<operation_id>: {"bytes","operations"}},
"outcomes"?: {"<operation_id> <idempotency_key>": {"input_digest","operation","operation_seq","reply"}}}`. Both maps are complete
snapshots; the store appends each as its own record kind and rejects any decrease (`budget`) or change to an existing key (`outcomes`).

## 11. Cross-daemon call bounds

Every call one daemon makes to another MUST be bounded in time. A call that can close a cycle between two daemons — at 1B,
gateway → lifecycle `record_budget` while lifecycle → gateway `deny_admission`/`release` may be in flight — MUST be bounded
**asymmetrically**: the side whose failure is cheap to fail closed (the gateway: refuse the one operation, close admission) takes
the short bound (`BUDGET_PERSIST_MS`, 2 s); the side whose failure would abandon a transition takes the long one (`CROSS_DAEMON_MS`,
60 s, which MUST exceed the peer's slowest legitimate service time). Equal bounds do not break the cycle; they merely truncate the
deadlock to the bound (measured at WP3.1: 61 128 ms with equal 60 s bounds, 2 124 ms after the asymmetry). These constants live in
`ab_common::wire` and any change to them is a revision of this document.
