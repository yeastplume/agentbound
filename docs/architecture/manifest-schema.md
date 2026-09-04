# Agentbound Manifest Schema

**Version:** 0.7
**Status:** Frozen (WP0)
**Date:** 28 August 2026
**Applies to:** the Unix-governed reference implementation and its microVM control arm

## Revision history

| Version | Date | Change |
|---|---|---|
| 0.1 | 28 August 2026 | Initial effective-manifest draft. |
| 0.2 | 28 August 2026 | Replaces one effective-manifest object with policy-signed authorization and constructor-signed launch-binding objects. |
| 0.3 | 28 August 2026 | Gateway-free form `channel_topology: none` (the only form constructible at 1A); two distinct identifiers `authorization_id` (policy-issued) and `launch_record_digest` (pair-derived) with a per-use table in §4; correspondence check 5 extended to topology. |
| 0.4 | 28 August 2026 | Open questions disposed per the open-question register; answers written into the normative text. `mac_context` null in Profile U; invocation-profile digest recorded. |
| 0.5 | 28 August 2026 | §3.6 revocation trigger vocabulary split; `continue-degraded` restriction enforced by policy; example updated. |
| 0.6 | 28 August 2026 | Post-freeze editorial maintenance (no normative change): wording. |
| 0.7 | 28 August 2026 | WP2 correction (no normative change): the §6 illustrative pair omitted the `connection_count` class required by §3.5; added as `absent` in both objects; §3.7 `constructor` member list and the example gain `invocation_profile_digest`, which §3.3 (0.4) already required the constructor to record. The example pair now validates under the reference validator. |


---

## 1. Purpose and normative conventions

This document specifies the bounded request accepted by `agentbound-policy`, the
policy-signed authorization manifest, and the constructor-signed launch binding
consumed by `agentbound-launch`. It is a WP0 artefact required by the [Phase 1
reference implementation plan](../plans/phase-1-reference-implementation.md)
(the “plan”). It implements the launch-record and derivation requirements in
[Agents as Unix Principals](../papers/technical-report.md) and the
ownership/execution split in [ADR-0001](ADR-0001-execution-identity.md).

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**,
and **MAY** in this document are to be interpreted as described by RFC 2119.

A request is an untrusted selection of registered names. An authorization
manifest is a policy-derived decision, not an entitlement supplied by the
caller. A launch binding is the host-specific projection of precisely that
decision after identity reservation. The **effective manifest** is the verified
pair; it is not a third signed object.

`agentbound-launch` MUST accept no authority-bearing input other than a
validated, policy-signed authorization manifest. It MUST create a launch binding
only after all required checks and atomic execution-identity reservation. It
MUST NOT derive, expand, or replace policy during binding construction.

This version defines a JSON data model. All named object members are
case-sensitive ASCII strings. Identifiers are opaque values whose authority is
established only through typed validation and catalogue resolution.

### 1.1 Terms

- **Caller** means the authenticated party submitting a request to `agentbound`.
- **Catalogue** means a versioned, server-side registry of named resources,
  runtimes, adapters, mount sources, target templates, and platform bindings.
- **Authorization manifest** means the allocation-free policy decision with
  `manifest_version` equal to `agentbound.authorization-manifest.v0.1`.
- **Launch binding** means the allocation-bound host projection with
  `launch_binding_version` equal to `agentbound.launch-binding.v0.1`.
- **Launch record** means the immutable effective-manifest pair and its two
  signature envelopes.
- **Managed reclamation domain** has the meaning in ADR-0001: declared
  namespaces, mounts, registered host paths, runtime/workspace stores, grants,
  IPC state, and cgroup state examined before identity reuse.

### 1.2 Conformance boundary

`agentbound-policy` produces substrate-independent policy decisions and
catalogue-constrained intent requirements. `agentbound-launch` allocates and
records only the platform values and host projections identified as launch
binding outputs. `agentbound-gateway` enforces only named, typed operations. It
MUST NOT interpret either object as permission to proxy arbitrary traffic.

`agentbound-audit` MUST retain the sealed launch record and correlate required
keys with kernel and gateway evidence. It MUST apply the declared audit-loss
behaviour when an evidence stream is unavailable.

The Unix-governed profile does not claim dynamic information-flow propagation.
Where label-like fields are used by another profile, they are policy metadata
unless that profile's specification explicitly provides a stronger claim.

---

## 2. Untrusted session request schema

### 2.1 Envelope

The request object MUST contain exactly these members:

```text
agent_principal_id
approval_references
budget                    (optional)
initiator_credential_ref
requested_resources
requested_runtime
schema_version
task_purpose_id
```

`schema_version` MUST equal `"agentbound.session-request.v0.1"`. The request
MUST be authenticated independently of this JSON object.
`initiator_credential_ref` identifies already-authenticated evidence for
correlation; it MUST NOT contain credential material.

| Member | Type and maximum | Meaning |
|---|---|---|
| `agent_principal_id` | identifier, 1–128 UTF-8 bytes | Registered durable agent principal. |
| `task_purpose_id` | identifier, 1–128 UTF-8 bytes | Registered task or purpose. |
| `requested_resources` | array of 0–32 identifiers | Catalogue resource selections. |
| `requested_runtime` | identifier, 1–128 UTF-8 bytes | Approved runtime selection. |
| `budget` | object, optional, at most 16 members | Requested upper bounds only. |
| `initiator_credential_ref` | evidence reference, 1–256 bytes | Authenticated initiator evidence. |
| `approval_references` | array of 0–16 evidence references | Approval-object references. |

A caller MAY request fewer resources or a lower budget than policy permits. A
request MUST NOT increase authority, visibility, budget, or credential scope
merely because it includes a name.

### 2.2 Requested budget

When present, `budget` MUST be a flat object whose members are selected from:

```text
cpu_millis, memory_bytes, pids, wall_clock_seconds, disk_bytes, disk_inodes,
io_bytes, gateway_requests, gateway_bytes, connection_count,
audit_events, model_tokens, monetary_microunits, delegation_fanout
```

Each value MUST be a JSON non-negative integer exactly representable under
JCS/ECMAScript numeric rules and no greater than `9007199254740991`. The request
MUST NOT specify a rate, unlimited value, negative value, floating-point value,
or unit-bearing string. Policy MAY omit a requested class or reduce it to a
lower catalogue-supported bound.

### 2.3 Structural validation and rejected fields

The request parser MUST reject the entire request for an unknown member at any
level, including an extension member, or for duplicate object-member names. The
schema is closed. The parser MUST enforce the declared types, bounds, and the
following identifier grammars:

| Identifier class | Grammar | Additional validation |
|---|---|---|
| principal, task, resource, runtime, catalogue IDs | `[a-z][a-z0-9-]{0,31}:[a-zA-Z0-9][a-zA-Z0-9._/-]{0,127}` | MUST exist in the applicable catalogue or authority registry. |
| evidence references | `[a-z][a-z0-9-]{0,31}:[a-zA-Z0-9][a-zA-Z0-9._/-]{0,255}` | MUST resolve to authenticated evidence of the required type. |
| digest | `sha256:[0-9a-f]{64}` | MUST identify the stated algorithm and value. |
| opaque local identifier | `[A-Za-z][A-Za-z0-9._:-]{0,127}` | MUST be used only in the member class that defines it. |

No semantic inspection of opaque identifiers is performed. A component MUST NOT
infer a filesystem path, URL, address, authority, or configuration instruction
from identifier spelling. Catalogue-existence and typed-reference checks, not
string interpretation, determine whether an identifier is usable.

The schema has no members for numeric UIDs/GIDs, execution identities,
ownership projections, filesystem paths, mount sources or targets, file
descriptors, credential material, network addresses, ports, URLs, Linux
privilege controls, namespace settings, cgroup paths, systemd unit names, or
firewall rules. Such configuration is invalid because no typed member admits
it—not because an opaque string is inspected for resemblance to it.

### 2.4 Bounds and syntax

A request MUST be at most 16 KiB after UTF-8 encoding. Its maximum object or
array nesting depth is four, including the root. It MUST contain no control
characters, `NUL`, or Unicode noncharacters in identifier strings.
Implementations MUST reject invalid UTF-8, non-finite numbers, and JSON text
with trailing non-whitespace bytes.

Each array that represents a set MUST contain no duplicate value after exact
Unicode code-point comparison. The parser MUST preserve supplied bytes for
audit but MUST NOT use ordering as authorization input. `agentbound-policy`
MUST authenticate referenced identities and approvals before derivation.

---

## 3. Effective manifest schema

### 3.1 Common rules and object division

The immutable launch record has exactly two canonically encoded signed objects:

1. the **AUTHORIZATION MANIFEST**, produced and Ed25519-signed by
   `agentbound-policy`, allocation-free, with version
   `agentbound.authorization-manifest.v0.1`; and
2. the **LAUNCH BINDING**, produced and Ed25519-signed by `agentbound-launch`
   after verification and atomic identity reservation, with version
   `agentbound.launch-binding.v0.1`.

Together, after both signatures and correspondence checks succeed, they are the
effective manifest. Optional decisions use an explicitly documented `null` or
empty array; neither producer MAY silently omit a required security decision.

Every policy decision lives in the authorization manifest. Every host projection
lives in the launch binding and references its authorization-manifest counterpart
by ID. A field MUST NOT be duplicated as an unconstrained “both” field. The
constructor MUST verify this one-to-one correspondence:

1. `authorization_id` values MUST be byte-for-byte equal.
2. The binding's `authorization_manifest_digest` MUST equal the digest verified
   for the authorization manifest.
3. Every `mount_intent.mount_id` MUST have exactly one `mount_projections`
   entry with that ID, and no projection MAY name another ID.
4. Every credential-grant intent ID MUST have exactly one issued credential
   grant referencing it, and no issued grant MAY name another ID.
5. `gateway.channel_topology` MUST be honoured exactly: `none` ⇒ no gateway socket, projection, mount, or grant; `local-socket` ⇒ exactly one of each. Gateway operation IDs, typed operation content, scopes, and budgets MUST be
   unchanged; a binding MUST NOT add, drop, substitute, or widen an operation.
6. Resource classes and policy limits MUST have exactly one resource projection
   each; installed values MUST be no greater than policy values.
7. Runtime, execution-binding, audit, revocation, and retention decisions MUST
   be identical to the authorization manifest where the binding references them;
   no binding member MAY introduce additional authority.

Failure of any equality check MUST reject construction and MUST publish no
runnable session.

### 3.2 Authorization manifest

The authorization manifest MUST contain exactly these top-level members:

```text
actors
audit
credential_grant_intents
derivation
execution_binding
gateway
authorization_id
manifest_version
mount_intents
resource_limits
revocation
runtime
session_trace
task
termination_retention
agent
```

`manifest_version` MUST equal `agentbound.authorization-manifest.v0.1`.
`authorization_id`, `session_trace.session_id`, and `session_trace.trace_id` are
policy-issued. The object MUST contain no numeric execution identity, host ID,
boot ID, scope, PID namespace, resolved host object, credential handle, or raw
host path.

| Member | Required content |
|---|---|
| `agent` | `global_id` and `durable_ownership_projection`. |
| `session_trace` | Policy-issued `session_id` and `trace_id`. |
| `actors` | Initiators, approvers, scheduler, and owner. |
| `task` | `purpose_id` and approval references. |
| `derivation` | Versioned inputs, input digest, policy, and resolved resource IDs. |
| `runtime` | `catalogue_id`, `artifact_digest`, and `invocation_profile`. |
| `execution_binding` | `model`, `endpoint`, `tenant`, `adapters`, `retention_mode`, and `inference_pool`. |
| `mount_intents` | Catalogue IDs, access, and required flag; never host paths. |
| `gateway` | `channel_topology`, `operations`, and budgets. |
| `credential_grant_intents` | Grant kind, operation subset, expiry policy; never issued handles. |
| `resource_limits` | The closed class table in §3.5. |
| `audit` | Required events, correlation keys, and loss behaviour. |
| `revocation` | Required trigger-to-behaviour mapping, including reclassification. |
| `termination_retention` | Termination, retention, and reclamation-domain policy. |

`agent.global_id` is the durable agent principal. Its
`durable_ownership_projection` MUST be either an opaque storage-principal
reference or stable local ownership UID projection. It MUST NOT be the
execution UID in this profile, consistent with ADR-0001.

### 3.3 Derivation, actor, runtime, and execution-binding decisions

`actors.initiators` MUST contain one or more authenticated actor objects with
`id`, `credential_reference`, and `relationship` (`delegation`, `scheduled`,
`agent-parent`, or `service`). `actors.approvers` contains zero or more objects
with `id`, `approval_reference`, `decision`, and `expires_at`. A scheduled
request MUST contain both `actors.scheduler` and non-null `actors.owner`.

`derivation` MUST contain `agent_authority_version`, `catalogue_version`,
`derivation_relation_version`, `derivation_input_digest`, `inputs`,
`policy_version`, `requested_budget_digest`, and `resolved_resource_ids`. It
MUST record every authenticated input identity and version evaluated by
`derive(Agent, Initiators, Task, Approvals, Policy)`.

The activated authority MUST be represented only as typed named grants in
`gateway`, `mount_intents`, `credential_grant_intents`, and `resource_limits`.
Raw universal capability strings are forbidden. The result MUST be no broader
than agent authority, task/policy permission, and initiator delegation bounds.

`runtime.artifact_digest` MUST be `sha256:<lowercase-hex>`.
`runtime.invocation_profile` is a catalogue name, not a caller command-line; the catalogue entry holds the argv template and environment allowlist, and the constructor records that entry's digest in the binding's `constructor` member.
The constructor MUST resolve executable and arguments only from that profile.

`execution_binding` MUST contain `adapters`, `endpoint`, `inference_pool`,
`model`, `retention_mode`, and `tenant`. Each except `adapters` MAY be `null`
when the runtime requires none; `adapters` is a catalogue-ID array. A change to
any non-null decision is policy-controlled and auditable and MUST NOT mutate a
running launch record.

### 3.4 Mount intents, gateway, and credential-grant intents

Each `mount_intents` entry MUST contain `access` (`read-only` or `read-write`),
`catalogue_id`, `mount_id`, `required`, and `target_template_id`. It MUST NOT
contain a raw source, raw target, raw path, or host-object handle.

`gateway.channel_topology` MUST be one of exactly two values:

- **`none`** — the gateway-free form. Legal at every milestone and the only
  form constructible before milestone 1B. `gateway.operations` MUST be `[]`,
  `gateway.budgets` MUST be `{}`, `credential_grant_intents` MUST be `[]`, and
  the launch binding MUST contain no `gateway_socket` descriptor, no gateway
  mount projection, no `gateway_projection` (the member is `null`), and no
  credential grant. The session still has **no network interface and no
  reachable host socket**: `none` removes the channel, not the boundary.
  Every gateway-dependent resource class is `absent` with evidence
  (requirements R-RES-5).
- **`local-socket`** — the mediated form (milestone 1B onward), per ADR-0002.
  `gateway.operations` MUST be non-empty and the binding MUST project exactly
  one gateway socket.

The network topology is dropped from Phase 1 and deferred to a future ADR. No
authorization manifest, binding, namespace declaration, or descriptor rule may
select network topology, veth, route, interface, firewall, or direct network
traffic under either value.

Under `local-socket`, `gateway.operations` is a non-empty array of typed
operation objects. Each MUST contain `adapter_catalogue_id`, `budgets`,
`operation`, `operation_id`, and `scope`. Generic HTTP, CONNECT, arbitrary destination, arbitrary URL, and
untyped byte-stream operations are invalid. `gateway.budgets` contains the
closed budget decisions applying to the operation set.

Each `credential_grant_intents` entry MUST contain `expiry_policy`, `grant_id`,
`kind`, and `operation_subset`. It MAY contain a policy-approved audience ID.
It MUST NOT embed credential material, a broker handle, revocation handle, or
any session-visible reusable secret.

### 3.5 Resource limits

`resource_limits` is closed and MUST contain one entry for each class below.
Each entry has `status` (`enforced` or `absent`), `limit` and unit when
enforced, `enforcement_owner`, and `absence_evidence` when absent. Unknown
classes are invalid.

| Class | Minimum Phase 1 enforcement |
|---|---|
| `pids` | cgroup `pids.max` |
| `file_descriptors` | `RLIMIT_NOFILE` |
| `cpu` | cgroup CPU controller |
| `memory_bytes` | cgroup memory controller |
| `disk_bytes`, `disk_inodes` | Per-session filesystem image with fixed capacity and inode count, or tmpfs `size=`/`nr_inodes=` bounds reported as bounded volatile storage—never “project quota” on tmpfs. |
| `io_bandwidth` | cgroup I/O controller |
| `network_bandwidth`, `connection_count`, `request_rate` | Gateway limiter; absent before 1B where unavailable. |
| `audit_capacity` | Audit queue and manifest loss policy |
| `delegation_fanout` | Policy counter plus child/session limit |
| `storage_bytes`, `external_spend`, `model_tokens` | Gateway accounting; model classes absent before 1C. |
| `accelerator` | Absent unless an accelerator is exposed |

A present class MUST have an enforcement owner and test. A class MAY be absent
only if the deployment exposes no such resource; absence is evidence, not an
unlimited value.

### 3.6 Audit, revocation, and termination retention

`audit` MUST contain `correlation_keys`, `loss_behaviour`, and `required_events`.
Correlation keys MUST include authorization ID, session trace ID, agent global
ID, and, once bound, allocation ID, execution UID, host ID, and boot ID.
`loss_behaviour` MUST be `stop`, `quarantine`, or `continue-with-loss-counter`.

`revocation` MUST map every declared trigger to exactly one of `terminate`,
`quiesce`, or `continue-degraded`. It MUST include `approval_expired`,
`audit_pipeline_degraded_below_stop_threshold`, `authority_revoked`,
`catalogue_withdrawn`, `gateway_grant_withdrawn`, `gateway_unavailable`,
`initiator_disabled`, `policy_service_unavailable`, `policy_withdrawn`,
`reclassification`, and `task_cancelled`. Where execution binding or inference
access exists, their withdrawal triggers are REQUIRED. The former generic
`control_plane_unavailable` trigger is not valid: policy-service and
lifecycle-daemon outages are distinct, and `lifecycle_daemon_unavailable` is
not manifest-selectable (session lifecycle §6). `continue-degraded` is valid
**only** for `policy_service_unavailable` and
`audit_pipeline_degraded_below_stop_threshold`; `agentbound-policy` MUST
reject any other mapping to it. Where used it MUST identify disabled
operations, maximum-duration policy, and audit event.

`termination_retention` MUST contain `audit_retention_class`,
`credential_revocation_order`, `descendant_kill_order`, `reclamation_domain_id`,
`termination_triggers`, and `workspace_retention`. New grant use MUST be
disabled before termination. Descendants MUST be killed or reaped before
credential/broker release completes. Retention MUST preserve the signed pair and
UID/boot/session mapping needed to disambiguate numeric UID reuse.

### 3.7 Launch binding

The launch binding MUST contain exactly these top-level members:

```text
authorization_manifest_digest
constructor
credential_grants
descriptor_allowlist
execution_identity
gateway_projection
host_binding
launch_binding_version
authorization_id
mount_projections
namespaces
resource_projection
```

`launch_binding_version` MUST equal `agentbound.launch-binding.v0.1`.
`authorization_id` MUST equal the authorization manifest's ID.

`execution_identity` MUST contain `allocation_id`, `gids`, `mac_context`, and
`uid`. `uid` is a non-negative integer. `gids` is a non-empty unique array of
no more than 32 non-negative integers. `mac_context` MUST be `null` in Profile U (a non-null value is a construction failure in Phase 1); in other profiles it is an opaque,
policy-approved context. The identity is unique for active sessions and reusable
only after verified reclamation and quarantine under ADR-0001.

`host_binding` MUST contain `boot_id`, `host_id`, `pid_namespace_id`, and
`scope_id`; `scope_id` is the systemd scope name. `namespaces` records modes
actually applied for mount, PID, IPC, UTS, and user namespaces. The Unix process
profile MUST use private mount, PID, IPC, and UTS namespaces.

Each `mount_projections` entry MUST contain `mount_id` and exactly one resolved
source form: a handle reference or a catalogue version reference. It MUST also
record installed access and target-template projection. There MUST be one entry
per mount intent and no raw path string.

`descriptor_allowlist` is closed. Each entry MUST contain `descriptor_id`,
`kind`, and `purpose`; allowed kinds are `stdin`, `stdout`, `stderr`, `pty`, and
`gateway_socket`. Every descriptor not listed MUST be closed before exec.
Under `local-socket`, `gateway_socket` MUST appear exactly once, MUST be
`AF_UNIX SOCK_SEQPACKET`, and MUST be the descriptor described by
`gateway_projection`; under `none` it MUST NOT appear.

Under `local-socket`, `gateway_projection` MUST contain `seqpacket: true` and
`socket_mount_id`, which MUST name the single projected local-socket gateway
mount. Under `none`, `gateway_projection` MUST be `null`.

Each `credential_grants` entry MUST contain an issued non-exportable handle and
its `grant_intent_id`. It MUST reference exactly one authorization-manifest
intent and MUST NOT reveal secret material to the session.

`resource_projection` contains exactly one installed projection per policy
resource class, recording the enforcement owner and installed value. It MUST NOT
relax policy or treat an absent class as unlimited.

`constructor` MUST contain `agentbound_launch_version_digest`, `invocation_profile_digest` (the digest of the resolved catalogue invocation-profile entry, §3.3), and `key_id`.

---

## 4. Canonical encoding, digest, signature, and rollback

The canonical encoding is JSON Canonicalization Scheme (JCS), RFC 8785. Each
object digest is `SHA-256(JCS-UTF8(object))`, represented as `sha256:` plus 64
lowercase hexadecimal characters. Member ordering is JCS ordering by UTF-16 code
units; ASCII member names therefore appear in ordinary ascending byte order.

Two identifiers name a session's launch record and MUST NOT be conflated:

- **`authorization_id`** — policy-issued, opaque, unique per authorization
  decision, present in both signed objects and in every envelope. It exists
  before any host allocation and is the only identifier available while a
  request is `requested`, `authorized`, or `constructing`.
- **`launch_record_digest`** — `SHA-256(authorization_manifest_digest ||
  launch_binding_digest)` over the two 32-byte binary digest values in that
  order, serialized as `sha256:` plus 64 lowercase hexadecimal characters. It
  exists only after the launch binding is committed and is the authoritative,
  content-bound identity of the completed launch record. Exactly one
  `launch_record_digest` may ever exist for an `authorization_id`.

| Use | Identifier |
|---|---|
| Request idempotency, authorization replay rejection | `authorization_id` |
| Constructor ownership lease, retry, and rollback ledger | `authorization_id` |
| Allocator reservation record | `authorization_id` at reservation; `launch_record_digest` recorded on binding commit |
| Lifecycle state lookup | `authorization_id` (primary key); `launch_record_digest` once bound |
| Gateway grant index and per-operation authorization | `launch_record_digest` only; a connection whose allocation maps to no committed digest is refused |
| Audit correlation | both; events before binding commit carry `authorization_id` and `launch_record_digest: null` |
| Diagnostics before binding commit | `authorization_id` |
| Effect records, attribution, provenance exported beyond the host | `launch_record_digest` (plus `authorization_id` for correlation) |
| Seal | `launch_record_digest` |

The policy signature envelope MUST contain exactly `authorization_manifest_digest`,
`issued_at`, `key_id`, `authorization_id`, `signature`, and `timestamp_source`.
The constructor signature envelope MUST contain exactly `allocation_id`,
`authorization_manifest_digest`, `boot_id`, `host_id`, `issued_at`, `key_id`,
`launch_binding_digest`, `authorization_id`, and `signature`. Both use detached
Ed25519 signatures. Policy signs only the authorization object; the constructor
signs only the launch binding after reservation and correspondence checks.

`agentbound-launch` MUST reject non-canonical bytes when transport claims to
carry canonical JSON. It MAY parse and canonicalize an authenticated transport
representation only when it rejects duplicate names and verifies the signature
over the resulting canonical bytes.

After allocation but before binding commit, rollback MUST move the identity to
`reclaiming` and MUST publish no launch binding. After binding commit, a failed
construction MUST seal the pair with a failed outcome and MUST NEVER delete it.
A second binding for one authorization digest or allocation is forbidden.

---

## 5. Constructor validation rules

Before allocation or any other privileged construction operation—including
namespace creation, mount installation, cgroup creation, credential issue, audit
binding, or execution-identity installation—`agentbound-launch` MUST:

1. Parse request and authorization manifest according to §§2–4; reject
   unsupported versions, duplicates, unknown members, and non-conforming types.
2. Verify the policy detached signature, authorization digest, signing key,
   timestamp source, issuance freshness, and authorization ID binding.
3. Resolve every actor, agent, task, approval, catalogue item, resource, runtime,
   adapter, and policy reference uniquely; ambiguity MUST fail.
4. Confirm policy, derivation, runtime, catalogue, adapter, and agent-authority
   versions are current and not withdrawn, superseded where policy forbids it,
   or otherwise invalidated.
5. Confirm every approval is authentic, unexpired, unrevoked, quorum-consistent,
   and applicable to the named task and agent.
6. Recompute derivation and confirm recorded inputs, digest, relationships,
   activated grants, budgets, and bounds are consistent with the result.
7. Confirm every mount intent source and target template exists in the current
   catalogue; obtain only safe descriptor-relative references for later use.
8. Under `local-socket`, confirm gateway operations and execution-binding
   adapters exist in the current catalogue and have typed scopes and budgets;
   under `none`, confirm `operations` is `[]` and that no gateway-dependent
   projection or grant will be produced.
9. Confirm all resource classes have a valid enforcement projection plan or
   explicit deployment absence evidence; confirm no proposed value widens policy.
10. **Allocate:** atomically reserve one current, unique, non-quarantined
    execution identity bound to this authorization digest and authorization ID.
11. Create the launch binding, verify every §3.1 equality check, sign it, and
    atomically commit the allocation-to-binding association before credentials
    are installed.
12. Revalidate mutable approval, catalogue, and version facts immediately before
    every irreversible use; install only the committed binding's projections.

For mount resolution, the constructor MUST use the new mount API with a pinned
kernel: descriptor-relative `openat2` with `RESOLVE_NO_MAGICLINKS`, an explicit
`RESOLVE_NO_XDEV` policy, and `open_tree`/`move_mount` as appropriate. String
paths, symlink following, and TOCTOU-prone re-walks are not conforming
substitutes.

On any failure, `agentbound-launch` MUST create no runnable partial session,
MUST issue no usable credential or broker grant, and MUST emit an audit denial
identifying the failed input or rule. After allocation, it MUST apply §4 rollback
rather than releasing an identity directly.

---

## 6. Illustrative complete signed pair

The following compact JCS-form examples use the plan's step-zero scenario:
`engineering-agent`, Alice, `fix-issue-1234`, and `git.push-staging-ref`.
All identifiers, digests, handles, signatures, and host values are non-production
placeholders. Each fenced line is one complete JSON object with lexically sorted
ASCII keys at every object level.

### 6.1 Authorization manifest

```json
{"actors":{"approvers":[],"initiators":[{"credential_reference":"authn:alice-session-0001","id":"human:alice","relationship":"delegation"}],"owner":null,"scheduler":null},"agent":{"durable_ownership_projection":{"kind":"storage-principal","reference":"storage:engineering-agent"},"global_id":"agent:engineering-agent"},"audit":{"correlation_keys":["authorization_id","trace_id","agent_global_id","execution_allocation_id","execution_uid_boot"],"loss_behaviour":"quarantine","required_events":["launch","gateway-operation","revocation","termination"]},"authorization_id":"launchrec:fix-issue-1234-0001","credential_grant_intents":[{"expiry_policy":"expiry:session-or-2026-08-28t163000z","grant_id":"grant:git-push-0001","kind":"proof-of-possession","operation_subset":["op:git-push-staging-0001"]}],"derivation":{"agent_authority_version":"agent-authz:v2026-08-28","catalogue_version":"catalogue:v2026-08-28","derivation_input_digest":"sha256:1111111111111111111111111111111111111111111111111111111111111111","derivation_relation_version":"derive:v0.1","inputs":[{"id":"agent:engineering-agent","kind":"agent","version":"agent-authz:v2026-08-28"},{"id":"human:alice","kind":"initiator","version":"authn:alice-session-0001"},{"id":"task:fix-issue-1234","kind":"task","version":"task:v17"}],"policy_version":"policy:v2026-08-28","requested_budget_digest":"sha256:2222222222222222222222222222222222222222222222222222222222222222","resolved_resource_ids":["resource:git-service","resource:repo-worktree"]},"execution_binding":{"adapters":["adapter:git"],"endpoint":null,"inference_pool":null,"model":null,"retention_mode":"retention:ephemeral","tenant":null},"gateway":{"budgets":{"gateway_bytes":1048576,"gateway_requests":10},"channel_topology":"local-socket","operations":[{"adapter_catalogue_id":"adapter:git","budgets":{"bytes":1048576,"requests":10},"operation":"git.push-staging-ref","operation_id":"op:git-push-staging-0001","scope":"repo:protected/refs/agentbound/fix-issue-1234/*"}]},"manifest_version":"agentbound.authorization-manifest.v0.1","mount_intents":[{"access":"read-write","catalogue_id":"mount-source:repo-worktree","mount_id":"mount:workspace","required":true,"target_template_id":"mount-target:workspace"},{"access":"read-write","catalogue_id":"mount-source:gateway-socket","mount_id":"mount:gateway-socket","required":true,"target_template_id":"mount-target:gateway-socket"}],"resource_limits":{"accelerator":{"absence_evidence":"deployment:no-accelerator","enforcement_owner":"none","status":"absent"},"audit_capacity":{"enforcement_owner":"agentbound-audit","limit":10000,"status":"enforced","unit":"events"},"connection_count":{"absence_evidence":"phase1:no-network-topology","enforcement_owner":"none","status":"absent"},"cpu":{"enforcement_owner":"cgroup","limit":1000,"status":"enforced","unit":"milli-cpu"},"delegation_fanout":{"enforcement_owner":"policy","limit":0,"status":"enforced","unit":"children"},"disk_bytes":{"enforcement_owner":"session-image","limit":1073741824,"status":"enforced","unit":"bytes"},"disk_inodes":{"enforcement_owner":"session-image","limit":100000,"status":"enforced","unit":"inodes"},"external_spend":{"absence_evidence":"deployment:no-spend-adapter","enforcement_owner":"none","status":"absent"},"file_descriptors":{"enforcement_owner":"rlimit","limit":256,"status":"enforced","unit":"descriptors"},"io_bandwidth":{"enforcement_owner":"cgroup","limit":10485760,"status":"enforced","unit":"bytes-per-second"},"memory_bytes":{"enforcement_owner":"cgroup","limit":1073741824,"status":"enforced","unit":"bytes"},"model_tokens":{"absence_evidence":"runtime:no-model","enforcement_owner":"none","status":"absent"},"network_bandwidth":{"absence_evidence":"phase1:no-network-topology","enforcement_owner":"none","status":"absent"},"pids":{"enforcement_owner":"cgroup","limit":128,"status":"enforced","unit":"processes"},"request_rate":{"enforcement_owner":"gateway","limit":10,"status":"enforced","unit":"requests"},"storage_bytes":{"enforcement_owner":"gateway","limit":1048576,"status":"enforced","unit":"bytes"}},"revocation":{"approval_expired":"terminate","audit_pipeline_degraded_below_stop_threshold":"quiesce","authority_revoked":"terminate","catalogue_withdrawn":"quiesce","gateway_grant_withdrawn":"terminate","gateway_unavailable":"quiesce","initiator_disabled":"terminate","policy_service_unavailable":"continue-degraded","policy_withdrawn":"terminate","reclassification":"quiesce","task_cancelled":"terminate"},"runtime":{"artifact_digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","catalogue_id":"runtime:engineering-agent","invocation_profile":"profile:fix-issue-1234"},"session_trace":{"session_id":"session:fix-issue-1234-0001","trace_id":"trace:fix-issue-1234-0001"},"task":{"approval_references":[],"purpose_id":"task:fix-issue-1234"},"termination_retention":{"audit_retention_class":"retention:wp0","credential_revocation_order":"revoke-before-cleanup","descendant_kill_order":"children-before-parent","reclamation_domain_id":"domain:session-default","termination_triggers":["task_cancelled","approval_expired"],"workspace_retention":"discard"}}
```

### 6.2 Launch binding

```json
{"authorization_id":"launchrec:fix-issue-1234-0001","authorization_manifest_digest":"sha256:3333333333333333333333333333333333333333333333333333333333333333","constructor":{"agentbound_launch_version_digest":"sha256:4444444444444444444444444444444444444444444444444444444444444444","invocation_profile_digest":"sha256:5555555555555555555555555555555555555555555555555555555555555555","key_id":"key:launch-ed25519-01"},"credential_grants":[{"grant_intent_id":"grant:git-push-0001","issued_handle":"handle:gateway-grant-0001"}],"descriptor_allowlist":[{"descriptor_id":"fd:stdin","kind":"stdin","purpose":"harness-input"},{"descriptor_id":"fd:stdout","kind":"stdout","purpose":"harness-output"},{"descriptor_id":"fd:stderr","kind":"stderr","purpose":"harness-diagnostics"},{"descriptor_id":"fd:gateway","kind":"gateway_socket","purpose":"typed-gateway"}],"execution_identity":{"allocation_id":"allocation:host-a-0001","gids":[200001],"mac_context":null,"uid":200001},"gateway_projection":{"seqpacket":true,"socket_mount_id":"mount:gateway-socket"},"host_binding":{"boot_id":"boot:host-a-0001","host_id":"host:reference-a","pid_namespace_id":"pidns:4026533001","scope_id":"agentbound-session-0001.scope"},"launch_binding_version":"agentbound.launch-binding.v0.1","mount_projections":[{"access":"read-write","catalogue_version":"catalogue:v2026-08-28","mount_id":"mount:workspace","target_template_projection":"mount-target:workspace"},{"access":"read-write","mount_id":"mount:gateway-socket","resolved_source_handle":"handle:gateway-socket-0001","target_template_projection":"mount-target:gateway-socket"}],"namespaces":{"ipc":"private","mount":"private","pid":"private","user":"private","uts":"private"},"resource_projection":{"accelerator":{"enforcement_owner":"none","status":"absent"},"audit_capacity":{"enforcement_owner":"agentbound-audit","installed_value":10000,"unit":"events"},"connection_count":{"enforcement_owner":"none","status":"absent"},"cpu":{"enforcement_owner":"cgroup","installed_value":1000,"unit":"milli-cpu"},"delegation_fanout":{"enforcement_owner":"policy","installed_value":0,"unit":"children"},"disk_bytes":{"enforcement_owner":"session-image","installed_value":1073741824,"unit":"bytes"},"disk_inodes":{"enforcement_owner":"session-image","installed_value":100000,"unit":"inodes"},"external_spend":{"enforcement_owner":"none","status":"absent"},"file_descriptors":{"enforcement_owner":"rlimit","installed_value":256,"unit":"descriptors"},"io_bandwidth":{"enforcement_owner":"cgroup","installed_value":10485760,"unit":"bytes-per-second"},"memory_bytes":{"enforcement_owner":"cgroup","installed_value":1073741824,"unit":"bytes"},"model_tokens":{"enforcement_owner":"none","status":"absent"},"network_bandwidth":{"enforcement_owner":"none","status":"absent"},"pids":{"enforcement_owner":"cgroup","installed_value":128,"unit":"processes"},"request_rate":{"enforcement_owner":"gateway","installed_value":10,"unit":"requests"},"storage_bytes":{"enforcement_owner":"gateway","installed_value":1048576,"unit":"bytes"}}}
```

### 6.3 Policy and constructor envelopes

```json
{"constructor":{"allocation_id":"allocation:host-a-0001","authorization_id":"launchrec:fix-issue-1234-0001","authorization_manifest_digest":"sha256:3333333333333333333333333333333333333333333333333333333333333333","boot_id":"boot:host-a-0001","host_id":"host:reference-a","issued_at":"2026-08-28T15:31:00Z","key_id":"key:launch-ed25519-01","launch_binding_digest":"sha256:5555555555555555555555555555555555555555555555555555555555555555","signature":"BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"},"policy":{"authorization_id":"launchrec:fix-issue-1234-0001","authorization_manifest_digest":"sha256:3333333333333333333333333333333333333333333333333333333333333333","issued_at":"2026-08-28T15:30:00Z","key_id":"key:policy-ed25519-01","signature":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","timestamp_source":"clock:policy-utc-v0.1"}}
```

The examples expose no session-visible Git credential, host path, direct Git
address, route, veth, or other network topology. A direct push to `main` is
outside the typed operation scope and MUST fail.

---

## 7. Open questions

None. The six WP0 questions are answered in the [open-question register](open-question-register.md): JSON Schema 2020-12 normative (schemas are WP1 outputs; this prose governs until 1A); clock and freshness per component interfaces §4; `mac_context` null in Profile U; absent classes per R-RES-5; invocation profile digest recorded; retention until identity leaves quarantine and every numeric-UID reference is reconciled.
