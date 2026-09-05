# Milestone 1B implementation brief (WP3) — gateway and mediated effect

Companion to `DESIGN.md` (1A). Normative sources: ADR-0002 0.8 Decisions 1–5, requirements R-GW-1..7, manifest-schema §3.4 / `descriptor_allowlist` / `gateway_projection`, session-lifecycle §5 steps 1, 3, 6, 9, test-catalogue 1B rows. This file is the working brief; when it and a frozen document disagree, the frozen document wins and this file is corrected.

## New component

| Crate | Role | Privilege |
|---|---|---|
| `agentbound-gateway` | on-host daemon; one per-session listening socket; connection + per-operation authentication (ADR-0002 D2); operation authorization (D3); revocation/lifetime (D4); Decision 5 events; R-GW-7 budgets; adapters | unprivileged user `agentbound-gateway`, group `agentbound`; holds the Git credential the session must never see |

Sockets: control `/run/agentbound/gateway.sock` (0660 root:agentbound; lifecycle and launch call it), per-session `/run/agentbound/gw/<allocation suffix>.sock` (mode 0666 inside a 0770 gateway-only directory; the unprivileged gateway cannot chown to the session UID — reachability is by bind mount into exactly one mount namespace, and establishment refuses any peer UID other than the allocation's; created by the gateway on `project`, bind-mounted by the constructor at the manifest's `gateway_socket` target).

## Message flow (1B additions)

```
policy: manifest.gateway.channel_topology = local-socket, operations[≥1], credential_grant_intents[≥1]
launch step 3: `project` → gateway creates listening SEQPACKET socket for (authorization_id, allocation_id), returns path; launch bind-mounts it at target (ro bind, nosuid, nodev, noexec)
launch step 8: after commit_binding, `activate` → gateway loads grant records from the committed launch record (by digest), admission = open
session process: connect() → gateway: SO_PEERCRED → pidfd_open → pidfs inode + start time + pidns + cgroup ⇒ must map to exactly one active record; else close
session process: sendmsg(one operation JSON [+ bundle bytes]) with kernel SCM_CREDENTIALS → gateway: exactly one SCM_CREDENTIALS, no SCM_RIGHTS, pid == establishing pid, same pidfs inode → authorize → adapter → reply
lifecycle terminate step 1: `deny_admission(lrd)`; step 6: `release(lrd)` → gateway closes connections, replies remaining=0; lifecycle removes mount (step 9) and unlinks socket
gateway restart: reconstructs grants only from lifecycle `list` + launch-record store; no connection survives
```

## Gateway control ops (wire v0.1, ab-common wire; op names)

`project {authorization_id, allocation_id, uid, gid}` → `{socket_path}`; `activate {launch_record_digest}`; `deny_admission {launch_record_digest}`; `release {launch_record_digest}` → `{connections_closed, remaining}`; `status {launch_record_digest}`. Callers: root (launch, lifecycle). Idempotent by lrd.

## Session-side protocol (per-session socket)

One `sendmsg` = one operation: canonical JSON `{"v":"agentbound.gateway.v0.1","operation_id":..., "operation":"git.push_staging", "args":{...}, "payload_sha256":..., "payload_len":n}` **Verified on VM 110:** SEQPACKET messages of 256 KiB pass, 1 MiB fails `EMSGSIZE` (`wmem_max` 212992; buffer raised by the unprivileged peer only up to that). Therefore: the operation packet is JSON only; the payload is carried in following `payload_chunk` packets (each ≤ 128 KiB, each carrying its own kernel `SCM_CREDENTIALS`, same process instance) up to `payload_len` bytes, then the gateway verifies `payload_sha256` and executes. Per-connection reassembly bound = `bytes_per_operation` (default 8 MiB); an operation is admitted only once the whole payload is present and verified — every chunk is attributed to the same live process. Reply: one packet, canonical JSON `{ok, class, body|rule, operation_seq, trace_id}`. Caller-supplied `trace_id` is ignored unless equal to the record's.

Reject classes on the session socket: `unauthenticated` (credential/instance mismatch → close), `unauthorized` (operation not granted / session not active), `invalid` (parse, multiple/zero SCM_CREDENTIALS, SCM_RIGHTS present → close), `budget`, `upstream_rejected`, `admitted-before-revocation` (recorded on the operation, not a reject).

## Git staging-ref adapter (`git.push_staging`)

Args: `repository_id` (catalogue), `ref_tail` (validated: no `..`, no `:`, `+`, `.lock`, whitespace, control chars; `git check-ref-format --branch` second filter), `expect_old` (sha or `null` = create-only), `force` NOT a field: `git.push_staging_force` is a separate operation id that the reference catalogue never grants. Payload: `git bundle` bytes. Steps: write bundle to quarantine dir (gateway-owned, 0700), `git bundle verify`, `git fetch <bundle> <tip>` into a quarantine repo, `git fsck --connectivity-only`, object count/size limits from budgets, then `git push <repo-url> <tip>:refs/agentbound/<session_id>/<ref_tail>` (non-force) with the gateway credential (`GIT_ASKPASS`/credential helper file 0600 gateway-owned; for the reference deployment the "remote" is a bare repo on the host under `/var/lib/agentbound/git/<repo>.git` with a pre-receive hook enforcing protected branches — GS-6 composition). Trace propagation: `-c push.pushOption=agentbound-trace=<trace_id>` plus the ref namespace itself carrying `session_id`. Events: `gateway.operation_admitted`, `gateway.operation_completed{remote_ref, old, new, bytes, objects}`, `gateway.operation_denied{rule}`, `gateway.upstream_rejected`.

## Constructor changes

**FINDING (manifest-schema, to record as a revision entry):** `descriptor_allowlist` models `gateway_socket` as an inherited descriptor ("every descriptor not listed MUST be closed before exec"), while ADR-0002 D2 requires each session process to establish its own authenticated connection, which a single inherited connected descriptor cannot provide. Implementation: the binding lists `gateway_socket` with `descriptor_id: mount:gateway_socket` and realises it as the read-only bind mount of the socket node; the session's fd set at exec stays `0,1,2`. Proposed amendment: `gateway_socket` kind denotes the projected socket node; the binding schema check `gateway_projection ⇔ gateway_socket entry` is unchanged.

- topology `local-socket` accepted when the manifest's `gateway.operations` non-empty and `credential_grant_intents` non-empty; `none` path unchanged.
- step 3: call gateway `project`; `open_tree` the socket path; mount at `gateway_socket` target inside the new root (bind, ro, nosuid, nodev, noexec); record `gateway_projection {socket_path_digest, target, type:"AF_UNIX/SOCK_SEQPACKET"}` in the binding; `descriptor_allowlist` unchanged (the socket is a mount, not an inherited fd — the session `connect()`s).
- step 8 (after commit): `activate`. Rollback: `release` + unlink on any failure.
- seccomp unchanged (AF_UNIX allowed; SOCK_STREAM/DGRAM connect denial per ADR-0002 D1.1 is a manifest-profile option — implement as a second filter arm keyed on the profile's `local_ipc: false`; record).

## Lifecycle changes

- grant records: `binding` record already carries `credential_grant_intents`; lifecycle stores `grant` rows `{grant_id, lrd, state: issued|released}` on `activate` (gateway reports) and `release`.
- §5 step 1: `deny_admission`; step 6: `release` and require `remaining == 0` before step 10 (identity release) — otherwise `termination-incomplete` with `gateway_connections_remaining` in evidence.
- states: `quiescing` blocks admission (gateway re-checks `status` per operation → lifecycle answers from `Sessions`); gateway caches nothing across operations.
- reconciliation: on start, `release` every non-sealed record's connections (there can be none — gateway restart also drops them) and re-`activate` active ones.

## Policy / catalogue additions

`repositories: {repo:demo → {url, protected: [refs/heads/main]}}`, `adapter_catalogue: {adapter:git-staging}`, task `task:fix-issue-1235` (approvals 0; `task:fix-issue-1234` stays the WP2 approval fixture) gets `operations: [op:git-push-staging]`, `grants: [grant:git-staging]`; `operations: {op:git-push-staging → {adapter_catalogue_id, operation:"git.push_staging", scope:{repository_id}, budgets:{bytes_per_operation, operations, objects}}}`; profile `profile:git-worker` argv `/bin/sh /image/git-worker.sh` (image now needs `git` — use the host's static-ish git via a bind of `/usr/bin/git` + libs? No: build the image with `git` and its shared libraries copied by `ldd`; record image digest).

## Conformance additions (1B rows)

D-09/T-6.4-001/002/003/004/010: from inside — no interface, `socket(AF_INET)` EPERM, host socket paths ENOENT, SOCK_STREAM connect to gateway path → EPROTOTYPE. T-6.4-005/T-6.3-001..004/006: no credential in env/files/fds/children/console. T-6.4-006: SCM_RIGHTS packet → closed + event. T-6.4-007: connected fd inherited by child → first packet PID mismatch → closed + `gateway.process_mismatch`; fd passed to another session (needs a host-side helper since sessions can't reach each other — pass via the driver: open in session A, `pidfd_getfd` from host into B's process… record as host-assisted). T-6.4-008: zero/multiple SCM_CREDENTIALS, forged pid (needs CAP_SYS_ADMIN → from host as root into the socket: reject). T-6.4-009: PID reuse within a tick — host-side test with a pid-wrapping spinner against a fake establishing pid (WP1 D7-2d harness). T-6.4-011/012/013: alternate repo id, other session's lrd/trace in args, refs outside namespace. T-6.4-014/T-6.3-007: revoke then operate on existing connection → denied; new connect refused; termination closes. D-10/D-13: full push, ref present on bare repo, `main` unchanged, audit chain from request to `gateway.operation_completed`. D-12: completeness metric = every required event kind present per record. T-6.9-005/006: bytes and connection-count bounds. Re-run partial 1A rows: D-02 (attach: still no PTY — remains partial), D-15 (delegation: no child-session API at 1B either → stays residual unless a `delegate` operation is added; do not add), T-6.1-003, T-6.1-013 (broker socket reuse: sibling's projected socket path not present in this mount ns; connect to own socket with another allocation's identity fields → deny), T-6.2-008 (git-worker image has `git` + `sh` only).

## Carry-ins in this WP

1. Storage-principal projection: at seal, `chown` files created by the ephemeral UID under the workspace root to the manifest's `durable_ownership_projection.reference` mapped to a host user via catalogue (`storage:finance-agent → uid`), recorded as `session.ownership_projected {files, bytes}`. 2. Power-loss: `virsh`/qm reset of VM 110 is not available from here without Servers/43 → run `echo b > /proc/sysrq-trigger` on the VM mid-transaction under a spinner workload, then verify chain on reboot; record. 3. D-Bus: try `zbus`? adds ~30k transitive SLOC to privileged process — instead subscribe via `busctl monitor` child process parsing `PropertiesChanged`/`UnitRemoved` lines (no new deps) and treat as confirmation-only; record decision. 4. D7 items 6–9: read ADR-0002 D7 list at that step.

## 1B limits (recorded, not hidden)

Only one adapter; one bare repo on-host stands in for the Git host (protected-branch hook = GS-6); no TLS upstream (T-6.4-012 exercised with a redirecting URL and a wrong-repo path, not a TLS identity); no inference adapter (1C); gateway restart reconstruction tested by SIGKILL, not power loss.
