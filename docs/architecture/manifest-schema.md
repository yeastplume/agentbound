# Agentbound Effective Manifest Schema

**Version:** 0.1  
**Status:** Draft for WP0 review  
**Date:** 28 August 2026  
**Applies to:** the Unix-governed reference implementation and its microVM control arm

---

## 1. Purpose and normative conventions

This document specifies the bounded request accepted by `agentbound-policy` and
the policy-signed authorization manifest and constructor-signed launch binding consumed as the effective manifest by `agentbound-launch`. It is a WP0
artefact required by the [Phase 1 reference implementation plan](../plans/phase-1-reference-implementation.md)
(the “plan”). It implements the launch-record and derivation requirements in
[Agents as Unix Principals](../papers/technical-report.md) (the “technical
report”) and the ownership/execution split in
[ADR-0001](ADR-0001-execution-identity.md).

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**,
and **MAY** in this document are to be interpreted as described by RFC 2119.

A request is an untrusted selection of registered names. An effective manifest
is a policy-derived decision and is not an entitlement supplied by the caller.
`agentbound-launch` MUST accept no authority-bearing input other than a
validated, policy-signed authorization manifest; after allocation it accepts only the constructor-produced launch binding paired with that authorization digest.

This version defines a JSON data model. All object members named in this
document are case-sensitive ASCII strings. Identifiers are opaque strings;
components MUST NOT infer authority from their spelling.

### 1.1 Terms

- **Caller** means the authenticated party submitting a request to `agentbound`.
- **Catalogue** means a versioned, server-side registry of named resources,
  runtimes, adapters, mount sources, and allowed platform bindings.
- **Derivation input** means an authenticated identity, approval, policy,
  catalogue entry, or other versioned fact evaluated by the derivation relation.
- **Launch record** means the immutable signed record represented by an
  authorization manifest and launch binding plus both signature envelopes in Section 4.
- **Managed reclamation domain** has the meaning in ADR-0001: the declared
  namespaces, mounts, registered host paths, runtime/workspace stores, grants,
  IPC state, and cgroup state examined before execution-identity reuse.

### 1.2 Conformance boundary

`agentbound-policy` produces substrate-independent authorization decisions and catalogue-constrained projection requirements. `agentbound-launch` allocates and records only the platform values and substrate-specific projections explicitly identified below as launch-binding outputs; it MUST NOT derive policy. `agentbound-gateway` enforces only named typed operations. It MUST NOT
interpret a manifest as permission to proxy arbitrary network traffic.
`agentbound-audit` records the signed launch record and correlates its required
keys with kernel and gateway evidence; it MUST apply the manifest audit-loss
behaviour when its declared evidence stream is unavailable.

The Unix-governed profile does not claim dynamic information-flow propagation.
Where label-like fields are used by another profile, they are policy metadata
and do not alter that profile's conformance claim unless its own specification
says otherwise.

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

`schema_version` MUST equal the string `"agentbound.session-request.v0.1"`.
The request MUST be authenticated independently of this JSON object.
`initiator_credential_ref` identifies the already-authenticated credential or
session used for correlation; it MUST NOT contain credential material.

A caller MAY supply only the following selections:

| Member | Type and maximum | Meaning |
|---|---|---|
| `agent_principal_id` | string, 1–128 UTF-8 bytes | Registered durable agent principal ID. |
| `task_purpose_id` | string, 1–128 UTF-8 bytes | Registered task or purpose ID. |
| `requested_resources` | array of 0–32 strings, each 1–128 bytes | Named resource requests, resolved only through the catalogue. |
| `requested_runtime` | string, 1–128 bytes | Runtime name from the approved runtime catalogue. |
| `budget` | object, optional, at most 16 members | Requested upper bounds only; see Section 2.2. |
| `initiator_credential_ref` | string, 1–256 bytes | Opaque reference to authenticated initiator evidence. |
| `approval_references` | array of 0–16 strings, each 1–256 bytes | Opaque references to approval objects. |

The caller MAY request fewer resources or a lower budget than policy permits.
A request MUST NOT cause authority, visibility, budget, or credential scope to
increase merely because it includes a name.

### 2.2 Requested budget

When present, `budget` MUST be a flat object whose members are selected from:

```text
cpu_millis, memory_bytes, pids, wall_clock_seconds, disk_bytes, disk_inodes,
io_bytes, gateway_requests, gateway_bytes, connection_count,
audit_events, model_tokens, monetary_microunits, delegation_fanout
```

Each value MUST be a JSON non-negative integer that is exactly representable
under JCS/ECMAScript numeric rules and MUST be no greater than `9007199254740991`.
The request MUST NOT specify a rate, an unlimited value, a negative value, a
floating-point value, or a unit-bearing string. Policy MAY omit a requested
class or reduce it to a lower catalogue-supported bound.

### 2.3 Rejected fields and values

The request parser MUST reject the entire request if it contains an unknown
member at any level, including an extension member. It MUST reject duplicate
object member names, even if a generic JSON parser would retain one value.

In particular, the following caller-supplied fields, aliases, or semantically
equivalent values MUST be rejected if present anywhere in the request:

- numeric UIDs, GIDs, execution identities, or ownership projections;
- filesystem paths, host paths, mount targets, mount sources, file descriptors,
  device names, or labels;
- credential material, tokens, passwords, private keys, certificate bodies,
  bearer values, or broker secrets;
- network addresses, hostnames, URLs, ports, CIDRs, interface names, routes, or
  socket paths;
- Linux capabilities, securebits, seccomp, Landlock, LSM/MAC context, or other
  privilege controls;
- namespace settings, clone flags, cgroup paths, systemd unit names, or
  firewall rules.

This prohibition applies to JSON member names, nested objects, strings encoded
as JSON, and string values that purport to carry such configuration. A named
catalogue resource is permitted only as a member of `requested_resources`; it
is not a path or direct mount instruction.

### 2.4 Bounds and syntax

A request MUST be at most 16 KiB after UTF-8 encoding. Its maximum object or
array nesting depth is 4, including the root. It MUST contain no control
characters in identifier strings, no `NUL`, and no Unicode noncharacters.
Implementations MUST reject invalid UTF-8, non-finite numbers, and JSON text
with trailing non-whitespace bytes.

Each array that represents a set MUST contain no duplicate value after exact
Unicode code-point comparison. The parser MUST preserve the supplied bytes for
audit but MUST NOT use ordering as an authorization input. `agentbound-policy`
MUST authenticate referenced identities and approval objects before derivation.

---

## 3. Effective manifest schema

### 3.1 Common rules

The immutable launch record has two canonically encoded objects to avoid requiring a host allocation before policy authorization:

1. the policy-signed **authorization manifest** (`agentbound.authorization-manifest.v0.1`), containing substrate-independent decisions and catalogue references but no numeric execution identity or resolved host object; and
2. the constructor-signed **launch binding** (`agentbound.launch-binding.v0.1`), produced only after validation and atomic identity reservation, binding host allocation and substrate-specific projections to the authorization-manifest digest.

Together they are the effective manifest. Optional values use an explicitly documented `null` or empty array; neither producer may silently omit a required security decision. The launch-record identity is `SHA-256(authorization_manifest_digest || launch_binding_digest)` over the two binary digest values in that order.

“Independent” means substrate-independent and is shared unchanged with the
microVM control arm. “Specific” means substrate-specific and MUST be separately
projected and reviewed for each launcher. A field marked “both” contains an
independent policy decision plus a specific enforcement projection.

**Table 1 — required effective-manifest members**

| Member | Type / bound | Class | Record / producer | Signed |
|---|---|---|---|---|
| `manifest_version` | string | Independent | authorization / `agentbound-policy` | policy |
| `launch_binding_version` | string | Specific | launch binding / `agentbound-launch` | constructor |
| `launch_record_id` | UUID-like opaque string, ≤128 bytes | Independent | authorization / `agentbound-policy` | policy |
| `agent` | object: `global_id`, `durable_ownership_projection` | Both | authorization decision + launch projection | both, separate objects |
| `execution_identity` | object: `allocation_id`, `uid`, `gids`, `mac_context` | Specific | launch binding / `agentbound-launch` | constructor |
| `session_trace` | object: `session_id`, `trace_id`, `boot_binding` | Both | authorization decision + launch projection | both, separate objects |
| `actors` | object: initiators, approvers, scheduler, owner | Independent | authorization / `agentbound-policy` | policy |
| `task` | object: `purpose_id`, `approval_references` | Independent | authorization / `agentbound-policy` | policy |
| `derivation` | object described in Section 3.2 | Independent | authorization / `agentbound-policy` | policy |
| `runtime` | object: catalogue ID, artifact digest, invocation profile | Both | authorization decision + launch projection | both, separate objects |
| `execution_binding` | object described in Section 3.3 | Independent | authorization / `agentbound-policy` | policy |
| `namespaces` | object described in Section 3.4 | Specific | launch binding / `agentbound-launch` | constructor |
| `mounts` | array of mount specifications | Specific | authorization decision + launch projection | both, separate objects |
| `descriptor_allowlist` | array of descriptor specifications | Specific | launch binding / `agentbound-launch` | constructor |
| `resource_limits` | object of enforced limits | Both | authorization decision + launch projection | both, separate objects |
| `gateway` | object described in Section 3.6 | Both | authorization decision + launch projection | both, separate objects |
| `credential_grants` | array of broker/credential grants | Both | authorization decision + launch projection | both, separate objects |
| `audit` | object described in Section 3.7 | Independent | authorization / `agentbound-policy` | policy |
| `revocation` | object of trigger behaviours | Independent | authorization / `agentbound-policy` | policy |
| `termination_retention` | object described in Section 3.8 | Independent | authorization / `agentbound-policy` | policy |

`agent.global_id` is the durable agent principal identity. Its
`durable_ownership_projection` MUST state either an opaque storage-principal
reference or a stable local ownership UID projection. That projection MUST NOT
be the execution UID in this profile.

`execution_identity.uid` is a non-negative integer; `gids` is a non-empty array
of unique non-negative integers limited to 32 entries; `mac_context` is either
`null` or an opaque, policy-approved context string. The allocation is unique
for active sessions and is reusable only under the verified reclamation and
quarantine condition in ADR-0001. `allocation_id` MUST remain the identity's
stable reference even where the numeric UID is later reused.

### 3.2 Derivation and actor fields

`actors.initiators` MUST contain one or more authenticated actor objects with
`id`, `credential_reference`, and `relationship` (`delegation`, `scheduled`,
`agent-parent`, or `service`). `actors.approvers` contains zero or more objects
with `id`, `approval_reference`, `decision`, and `expires_at`. A scheduled
request MUST contain both `actors.scheduler` and a non-null accountable
`actors.owner`.

`derivation` MUST contain:

```text
agent_authority_version
catalogue_version
derivation_relation_version
derivation_input_digest
policy_version
requested_budget_digest
resolved_resource_ids
```

It MUST also record an ordered list of all authenticated input identities and
versions used to evaluate `derive(Agent, Initiators, Task, Approvals, Policy)`.
The activated authority MUST be represented only as typed named grants in
`gateway`, `mounts`, `credential_grants`, and resource limits; raw universal
capability strings are forbidden. The result MUST satisfy the derivation bounds
in the technical report: it is no broader than agent authority and task/policy
permission, and delegation is no broader than initiator authority.

### 3.3 Runtime and execution binding

`runtime` MUST name one approved catalogue runtime and include its immutable
artifact digest as `sha256:<lowercase-hex>`. `invocation_profile` is a catalogue
name, not a caller command-line string. The actual executable and arguments are
resolved by `agentbound-launch` from that profile.

`execution_binding` MUST contain `model`, `endpoint`, `tenant`, `adapters`, and
`retention_mode`; it MAY additionally contain `inference_pool`. Each is a
catalogue identifier or `null` where the runtime needs no model. A change to any
non-null element is a policy-controlled auditable event; it MUST NOT be made by
mutating a running manifest.

### 3.4 Namespaces, mounts, and descriptors

`namespaces` MUST declare each of `mount`, `pid`, `ipc`, `uts`, `network`, and
`user` as a catalogue-defined mode. The Unix process profile MUST use private
mount, PID, IPC, and UTS namespaces; the permitted network mode is determined
solely by `gateway.channel_topology`.

Each `mounts` entry MUST have `mount_id`, `source_catalogue_id`,
`target_template_id`, `access` (`read-only` or `read-write`), and `required`.
It MUST NOT contain raw path strings or raw host sources. `agentbound-launch`
MUST resolve both source and target descriptor-relatively from the named
catalogue entries.

Each `descriptor_allowlist` entry MUST contain `purpose`, `kind`, `direction`,
and `inheritable`. Allowed kinds are `stdin`, `stdout`, `stderr`, `pty`, and
`gateway_socket`. `gateway_socket` is legal only for the local-socket topology.
The list is closed: every descriptor not listed MUST be closed before exec.

### 3.5 Resource limits

`resource_limits` is closed and MUST contain one entry for each class below. Each entry has `status` (`enforced` or `absent`), `limit` and unit when enforced, `enforcement_owner`, and `absence_evidence` when absent. Unknown classes are invalid.

| Class | Minimum Phase 1 enforcement |
|---|---|
| `pids` | cgroup `pids.max` |
| `file_descriptors` | `RLIMIT_NOFILE` |
| `cpu` | cgroup CPU controller |
| `memory_bytes` | cgroup memory controller |
| `disk_bytes`, `disk_inodes` | project/filesystem quota or bounded disposable image |
| `io_bandwidth` | cgroup I/O controller |
| `network_bandwidth`, `connection_count`, `request_rate` | gateway/firewall limiter (absent before 1B) |
| `audit_capacity` | audit queue and manifest loss policy |
| `delegation_fanout` | policy counter plus child/session limit |
| `storage_bytes`, `external_spend`, `model_tokens` | gateway accounting (model classes absent before 1C) |
| `accelerator` | absent unless an accelerator is exposed |

A present class MUST have an enforcement owner and test. A class may be absent only if the deployment exposes no such resource; absence is evidence, not an unlimited value.

### 3.6 Gateway, channel topology, grants, and budgets

`gateway` MUST contain exactly one `channel_topology`: `network` or
`local-socket`. These values are mutually exclusive.

For `network`, `gateway` MUST identify a single veth-backed gateway binding and
MUST declare an empty local-socket grant. The session MUST have no inherited
socket descriptor. For `local-socket`, it MUST declare `network_interface: false`
and exactly one named single-purpose gateway socket mount; no other network
interface or host Unix socket is permitted.

`gateway.permitted_operations` is a non-empty array of typed operation objects.
Each object MUST contain `adapter_catalogue_id`, `operation`, `scope`, and
`budgets`. Every budget is a non-negative bounded integer. Typical Git scope is
a repository ID and an exact `refs/agentbound/<session>/...` staging-ref
pattern. A generic HTTP, CONNECT, arbitrary destination, arbitrary URL, or
untyped byte-stream operation is invalid.

A `credential_grants` entry MUST identify either a non-exportable broker grant
or a proof-of-possession credential reference. It MUST specify its audience,
operation subset, expiry, and revocation handle. It MUST NOT embed a reusable
secret in the manifest or make it visible to the session.

### 3.7 Audit and revocation

`audit` MUST contain `required_events`, `correlation_keys`, and
`loss_behaviour`. Required correlation keys include `launch_record_id`, session
trace ID, agent global ID, execution allocation ID, and execution UID plus boot
binding. `loss_behaviour` MUST be exactly one of `stop`, `quarantine`, or
`continue-with-loss-counter`.

`revocation` MUST map every declared trigger to exactly one behaviour:
`terminate`, `quiesce`, or `continue-degraded`. The manifest MUST include, at
minimum, entries for `initiator_disabled`, `approval_expired`, `authority_revoked`,
`policy_withdrawn`, `catalogue_withdrawn`, `task_cancelled`, `reclassification`,
`gateway_grant_withdrawn`, `gateway_unavailable`, and `control_plane_unavailable`.
If an execution binding or inference grant exists, entries for their withdrawal
are also REQUIRED. `continue-degraded` MUST identify the disabled operations,
maximum duration policy reference, and audit event.

### 3.8 Termination and retention

`termination_retention` MUST contain `termination_triggers`,
`descendant_kill_order`, `credential_revocation_order`, `workspace_retention`,
`audit_retention_class`, and `reclamation_domain_id`. New grant use MUST be disabled before termination begins. Descendants MUST be killed
or reaped before credential and broker-grant record release completes. The retention
policy MUST retain the signed launch record and sufficient UID/boot/session
mapping to disambiguate later numeric UID reuse.

---

## 4. Canonical encoding, digest, and signature

The canonical encoding is JSON Canonicalization Scheme (JCS), RFC 8785. JSON
is selected because the policy boundary and audit tooling need a readily
inspectable format, while JCS defines deterministic member ordering, escaping,
and number serialization. Ordinary “pretty JSON” is not canonical JSON.

Each object digest is `SHA-256(JCS-UTF8(object))`, serialized as `sha256:` plus 64 lowercase hexadecimal characters. The authorization-signature envelope carries `authorization_manifest_digest`; the launch-binding envelope carries `launch_binding_digest`, `authorization_manifest_digest`, `allocation_id`, `host_id`, and `boot_id`. Both use detached Ed25519 signatures, key ID, issuance time, and named timestamp source. Policy signs only the authorization object. After verification and atomic reservation, the constructor signs the launch binding. The gateway and `agentbound-audit` treat the pair and their combined launch-record identity as authoritative; neither object may be mutated.

If construction rolls back after allocation but before launch-binding commit, the identity enters reclamation and no launch binding is published. If failure occurs after binding commit, the pair is sealed with a failed outcome and is never deleted. A second binding for one authorization digest or allocation is forbidden.

`agentbound-launch` MUST reject non-canonical input bytes when the transport
claims to carry canonical JSON. It MAY parse and canonicalize an authenticated
transport representation only if the signature is verified over the resulting
canonical bytes and no duplicate names were accepted during parsing.

The constructor MUST NOT re-parse manifest strings as paths. It MUST resolve
catalogue-selected mounts using descriptor-relative, path-safe operations (for
example `openat2` with appropriate resolution constraints or mount file
descriptors), as required by technical-report §2.1. String paths, symlinks, and
TOCTOU-prone re-walks are not a conforming substitute.

---

## 5. Constructor validation rules

Before identity allocation or any other privileged operation—including namespace
creation, mount resolution, cgroup creation, network setup, credential issue,
or audit binding—`agentbound-launch` MUST perform all of the following:

1. Parse the request/manifest according to Sections 2–4 and confirm a supported
   schema version.
2. Verify the detached signature, signed digest, signing key identity, trusted
   timestamp source, and launch-record ID binding.
3. Resolve every actor, agent, task, approval, catalogue item, resource, runtime,
   adapter, and policy reference uniquely; ambiguity is a failure.
4. Confirm the authorization manifest requests no concrete UID/GID. Atomically reserve a current, unique, non-quarantined allocation; then create and sign the launch binding and its managed-domain declaration before installing credentials.
5. Confirm every mount source and target template is in the current catalogue,
   then obtain descriptor-relative handles before construction starts.
6. Confirm the descriptor allowlist is closed, internally consistent, and has no
   socket exception other than the one allowed by the selected topology.
7. Confirm exactly one gateway channel topology, `network` or `local-socket`, is
   selected and that all namespace and descriptor declarations match it.
8. Confirm every permitted gateway operation and every execution-binding adapter
   is present in the current adapter catalogue and has typed scope and budgets.
9. Confirm policy, derivation-relation, runtime, catalogue, adapter, and agent
   authority versions are current and not withdrawn, superseded where policy
   forbids it, or otherwise invalidated.
10. Confirm every approval is authentic, unexpired, unrevoked, quorum-consistent,
    and applicable to the named task and agent.
11. Recompute the derivation relation and confirm that all recorded derivation
    inputs, input digest, actor relationships, activated grants, and bounds are
    consistent with its result.
12. Confirm the manifest's resource and gateway budgets do not exceed policy or
    catalogue limits and that every applicable resource class has a declared
    enforcement point or is explicitly absent from the deployment.

On any failure, `agentbound-launch` MUST create no runnable partial session,
MUST issue no usable credential or broker grant, and MUST emit an audit denial
that identifies the failed input or validation rule. Revalidation MUST occur
again immediately before each irreversible step when a version, approval, or
catalogue reference can change between validation and use.

---

## 6. Illustrative complete effective manifest

The following is a compact review rendering of the **combined effective view** for the plan's step-zero scenario. In transport, policy-owned members form the authorization manifest and host-owned members form the separately signed launch binding; no UID appears in the policy-signed object.
The displayed line breaks are for review only; canonical bytes are obtained by
JCS serialization. All identifiers, hashes, UIDs, keys, and endpoints are
obviously placeholder values.

```json
{"actors":{"approvers":[],"initiators":[{"credential_reference":"authn://example.invalid/session/alice-0001","id":"human:alice@example.invalid","relationship":"delegation"}],"owner":null,"scheduler":null},"agent":{"durable_ownership_projection":{"kind":"storage-principal","reference":"storage://example.invalid/principal/engineering-agent"},"global_id":"agent:engineering-agent"},"audit":{"correlation_keys":["launch_record_id","trace_id","agent_global_id","execution_allocation_id","execution_uid_boot"],"loss_behaviour":"quarantine","required_events":["launch","gateway-operation","revocation","termination","audit-loss"]},"credential_grants":[{"audience":"agentbound-gateway","expiry":"2026-08-28T16:30:00Z","grant_id":"grant:placeholder-git-pop-0001","kind":"proof-of-possession-reference","operation_subset":["git.push-staging-ref"],"revocation_handle":"revoke:placeholder-git-0001"}],"derivation":{"agent_authority_version":"agent-authz:v2026-08-28-demo","catalogue_version":"catalogue:v2026-08-28-demo","derivation_input_digest":"sha256:1111111111111111111111111111111111111111111111111111111111111111","derivation_relation_version":"derive:v0.1","inputs":[{"id":"agent:engineering-agent","kind":"agent","version":"agent-authz:v2026-08-28-demo"},{"id":"human:alice@example.invalid","kind":"initiator","version":"authn-session:alice-0001"},{"id":"task:fix-issue-1234","kind":"task","version":"task:v17"}],"policy_version":"policy:v2026-08-28-demo","requested_budget_digest":"sha256:2222222222222222222222222222222222222222222222222222222222222222","resolved_resource_ids":["resource:repo-worktree/fix-issue-1234","resource:git-service/protected-repo"]},"descriptor_allowlist":[{"direction":"in","inheritable":true,"kind":"stdin","purpose":"harness-input"},{"direction":"out","inheritable":true,"kind":"stdout","purpose":"harness-output"},{"direction":"out","inheritable":true,"kind":"stderr","purpose":"harness-diagnostics"}],"execution_binding":{"adapters":[],"endpoint":null,"inference_pool":null,"model":null,"retention_mode":null,"tenant":null},"execution_identity":{"allocation_id":"execalloc:placeholder-7f3a","gids":[42017],"mac_context":null,"uid":42017},"gateway":{"channel_topology":"network","local_socket_grant":null,"network_binding":{"gateway_binding_catalogue_id":"gateway-bind:demo-veth-git","network_interface":true},"permitted_operations":[{"adapter_catalogue_id":"adapter:git-staging-v0.1","budgets":{"gateway_bytes":10485760,"gateway_requests":4},"operation":"git.push-staging-ref","scope":{"repository_id":"repo:protected-demo","staging_ref_pattern":"refs/agentbound/sess-placeholder-0001/*"}}]},"launch_record_id":"launchrec:placeholder-0001","manifest_version":"agentbound.effective-manifest.v0.1","mounts":[{"access":"read-write","mount_id":"mount:workspace","required":true,"source_catalogue_id":"resource:repo-worktree/fix-issue-1234","target_template_id":"target:session-workspace"},{"access":"read-only","mount_id":"mount:runtime","required":true,"source_catalogue_id":"runtimefs:python-harness-v0.1","target_template_id":"target:runtime"}],"namespaces":{"ipc":"private","mount":"private","network":"veth-gateway-only","pid":"private","user":"host-credential","uts":"private"},"resource_limits":{"audit_events":100000,"cpu_millis":2000,"disk_bytes":2147483648,"disk_inodes":100000,"gateway_bytes":10485760,"gateway_requests":4,"memory_bytes":4294967296,"pids":256,"wall_clock_seconds":3600},"revocation":{"approval_expired":"terminate","authority_revoked":"terminate","catalogue_withdrawn":"terminate","control_plane_unavailable":"quiesce","gateway_grant_withdrawn":"quiesce","gateway_unavailable":"quiesce","initiator_disabled":"terminate","policy_withdrawn":"terminate","task_cancelled":"terminate"},"runtime":{"artifact_digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","catalogue_id":"runtime:coding-harness-v0.1","invocation_profile":"profile:noninteractive-coding-harness"},"session_trace":{"boot_binding":"boot:placeholder-host-0001","session_id":"session:sess-placeholder-0001","trace_id":"trace:00000000000000000000000000000001"},"task":{"approval_references":[],"purpose_id":"task:fix-issue-1234"},"termination_retention":{"audit_retention_class":"retention:security-launch-record","credential_revocation_order":"after-descendant-kill","descendant_kill_order":"cgroup-kill-then-pidns-reap","reclamation_domain_id":"reclaim-domain:linux-session-v0.1","termination_triggers":["policy-revocation","task-cancel","explicit-owner-termination","resource-limit"],"workspace_retention":"retain-sealed-for-forensics"}}
```

The displayed envelope illustrates the policy signature; a conforming record also carries the constructor signature envelope binding its digest, allocation ID, host ID, and boot ID to this authorization digest:

```json
{"algorithm":"ed25519","authorization_manifest_digest":"sha256:3333333333333333333333333333333333333333333333333333333333333333","issued_at":"2026-08-28T15:30:00Z","key_id":"policy-key:placeholder-ed25519-01","launch_record_id":"launchrec:placeholder-0001","signature":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","timestamp_source":"time-source:policy-service-utc-v0.1"}
```

The example provides no session-visible Git credential, no path-valued mount
source, no direct Git host address, and no route other than the typed gateway
operation. A direct push to `main` is outside the operation scope and MUST fail.

---

## 7. Open questions for WP0 review

1. Which concrete schema language (JSON Schema, CDDL, or both) should be the
   machine-readable companion to this normative data model?
2. What authoritative clock, freshness tolerance, and key-rotation procedure
   should govern the policy signature envelope?
3. Does the first implementation reserve `mac_context` as `null`, or require a
   concrete MAC projection before any non-Unix-governed profile is admitted?
4. ADR-0002 has not yet selected the network or local-socket topology. Which
   catalogue fields and connection-lifetime semantics must be fixed before the
   provisional topology becomes an implementation contract?
5. Which resource classes are explicitly absent in the first deployment, and
   what evidence is required before an omitted class may be declared absent?
6. What immutable representation should record a policy-approved runtime command
   without allowing the caller to inject executable paths or arguments?
7. Which launch-record retention class is sufficient to disambiguate execution
   UID reuse while minimizing retention of potentially sensitive audit data?
