# WP3 evidence register — milestone 1B (Mediated effect)

Evidence for the Phase 1 plan WP3 exit condition: *a session produces a remote effect (a Git staging ref) only through the gateway, every 1B row of the test catalogue runs as an automated conformance row with PASS/FAIL and observed evidence, the effect is attributable end to end in the audit chain, and the five R-CON-8 SLOC figures are republished including the gateway authentication path and the gateway core figure.* Gate 3 (ADR-0002 Decision 7 items 6–9 verified against the gateway) is recorded here.

**Pinned baseline (as run):** VM 110 `agentbound-dev` — Linux `6.12.107+deb13-cloud-amd64`, systemd `257.13-1~deb13u1`, Debian 13, git 2.47.3, cargo 1.98.1, tokei `13.0.0-alpha.8`. Final run 2026-09-05T17:37Z at the commit that adds this file; the suite binary and every component binary were built from the same tree (`crates/build.sh build --release` on the VM; source digests checked equal to the repository). WP3 spans commits `a1d3786`..HEAD (13 increments, each built, run and committed separately).

**Reading this register.** §2 lists every row with its verdict and a summary of observed evidence; [raw/conformance-run.md](raw/conformance-run.md) is the unedited machine output of the final run. Rows marked **PASS (weak)** pass their assertion but the assertion is weaker than the catalogue's intent; the weakness is stated. Rows marked **PASS (recorded)** are 1A partial/N-A rows re-run under `local-socket` whose 1A justification still holds; the driver asserts the property that would have changed. The row assertions and the code under test were written by the same author; independent review of the assertions is the next step (see §7).

## WP3 exit status

**139 rows, 139 PASS, 0 FAIL** in the final machine run. Of the 55 rows added in WP3: 45 PASS, 5 PASS (weak), 5 PASS (recorded). Gate 3: **pass (provisional until independent review)** — ADR-0002 D7 items 6–9 each have a row with evidence (§4). The remote effect is attributable end to end (§3). All four WP2 carry-ins closed (§5). Recorded limits and residuals: §6.

## 1. Components delivered

| Crate / artefact | Role | Privilege | SLOC (tokei, code) |
|---|---|---|---|
| `agentbound-gateway` (new) | ADR-0002 D1–D5: one bind-mounted `AF_UNIX` `SOCK_SEQPACKET` socket per session under a gateway-only directory; connection authentication (`auth.rs`): `SO_PEERCRED` → session UID → `pidfd_open` → pidfs inode + start time + pidns + scope cgroup must name the allocation's `agentbound-<alloc>.scope`; `SO_PASSCRED` with exactly one `SCM_CREDENTIALS` per packet matching the establishing process instance (pid **and** pidfs inode); `SCM_RIGHTS` → close; one connection per process; close on peer pidfd exit; per-operation grant re-check; `deny_admission` / `release` control ops for lifecycle (root-only control socket); restart reconstruction from lifecycle's record store with listeners recovered from the systemd fd store; D5 event set; per-session byte budget and connection count; audit-loss behaviour per manifest (D7-8); requirement-naming denials (D7-9). Adapters: `gateway.ping`, `git.push_staging` | unprivileged (`agentbound-gateway` user), holds the upstream credential the session never sees | 317 (auth path 14 + 51 in `ab-common::wire` + 42 per-packet check in `session.rs`) |
| `git.push_staging` adapter (`git.rs`) | session sends a bundle (no remote, no credential) as chunked payload with SHA-256; gateway imports into a per-operation quarantine repo (`bundle verify` → fetch tip → `fsck --connectivity-only` → object budget) and pushes only `refs/agentbound/<session>/<tail>` with WP1 GS-4 tail refusals, `--force-with-lease=<ref>:<expect_old>`, trace and session as push options; closed argument set (`expect_old`, `ref_tail`, `repository_id`, `tip`); force is `git.push_staging_force`, present in the catalogue and never granted | part of gateway | 54 |
| `ab-gwclient` (new) | static session-side client used by the in-session rows (one op per invocation; `--hold`, `--fork`, `--scm-rights`, `--stream`, `--dgram`, `--families`, `--fds` probes) | in image, session UID | 54 |
| `agentbound-launch` | `local-socket`: `mount:gateway-socket` intent resolved to the gateway's projected node and bind-mounted read-only at `/run/gateway.sock` (file mountpoint); binding `gateway_projection {seqpacket, socket_mount_id}` and `credential_grants[]`; calls gateway `project` then lifecycle commit then gateway `activate` (§3 step 8) | privileged | 400 |
| `agentbound-lifecycle` | §5 step 1 `deny_admission` on quiesce/terminate entry; step 6 `release` requires the gateway's zero-connections acknowledgement (else hold); grants recorded in `session.cleanup_completed`; accepts `revocation_signal` from the gateway UID; storage-principal ownership projection at seal (`session.ownership_projected`); recovery retry for termination-incomplete sessions | privileged | 725 |
| `agentbound-policy` | adds the `mount:gateway-socket` intent when the task's runtime declares `local-socket`; derives `gateway.operations` and grant intents from agent ∩ task | unprivileged | 158 |
| `agentbound-audit` | closed detail schemas for every `gateway.*` kind and `session.ownership_projected`; unknown or missing members rejected | unprivileged | 103 |
| `deploy/` | `agentbound-gateway.service` (`Type=notify`, fd store); catalogue: `repo:demo`, ops/grants, `task:fix-issue-1235` (git-worker), `storage_principals`; bare repo with `pre-receive` hook standing in for the Git host (refuses everything outside `refs/agentbound/*`, logs push options); image gets `git` + `ab-gwclient` | — | 61 lines units/hook |

## 2. Row register

### 2.1 In-session rows (`git-worker.sh`, session UID, inside the constructed environment)

| Row | Verdict | Evidence (summary) |
|---|---|---|
| T-6.4-003.projected | PASS | `/run/gateway.sock` is a socket node; the only one in the session's mount namespace |
| D-09 | PASS | authenticated typed `gateway.ping` returns `pong` with `trace_id`; `gateway.connection_established` carries pidfs inode, start time, pidns, cgroup |
| T-6.3-001 / -002 | PASS | no credential in environment, `/proc/self/fd`, image, workspace; the gateway credential file is outside the mount namespace |
| T-6.3-003 | PASS | inherited descriptors are exactly 0 (`/dev/null`), 1, 2 (console pipe) |
| T-6.3-004 | PASS | child inherits no credential material; a child must establish its own authenticated connection |
| T-6.3-006 | PASS | no credential-like text in any gateway reply or adapter output captured during the run |
| T-6.1-007.sibling | PASS | sibling session's socket path absent (ENOENT) |
| GS-1 / bundle / D-10 | PASS | session builds a bundle offline; `git.push_staging` → `{new, objects, remote_ref: refs/agentbound/<sid>/<tail>}` |
| GS-4[7 tails] | PASS | `../main`, `main:refs/heads/main`, `+fix`, `fix.lock`, `a b`, empty, `refs/heads/main` each refused with the named rule before any git process runs |
| T-6.4-011 | PASS | `repository_id` outside the grant → `scope_repository` |
| GS-8.force | PASS | `git.push_staging_force` → `operation_not_granted`; force is never in a grant |
| T-6.4-006 | PASS | packet with `SCM_RIGHTS` → connection closed, `gateway.descriptor_transfer_rejected` |
| T-6.4-007 | PASS | connected fd inherited by a forked child → first packet's `SCM_CREDENTIALS` pid ≠ establishing pid → closed, `gateway.process_mismatch` |
| T-6.4-001 | PASS | `socket()` for INET, INET6, PACKET, NETLINK, VSOCK → `EPERM` (seccomp) |
| T-6.4-010.stream / .dgram | PASS | `SOCK_STREAM` / `SOCK_DGRAM` connect to the socket path → `EPROTOTYPE` |
| T-6.9-005 | PASS | 9 MiB payload → `budget_bytes` (8 MiB per-session budget) |
| T-6.9-006 | PASS (weak) | 20 held connections: 16 established, 4 refused `connection_limit`. Weak: exercised only under the worker's own fan-out, not with a hostile process pattern |
| GW-HELD / GW-COMPLETE | — | fixtures for T-6.4-014 and the driver |

### 2.2 Host-side rows (driver as root; "in-scope peer" = driver moves itself into the session's scope cgroup, then `nsenter` into its namespaces as the session UID — see §7 note)

| Row | Verdict | Evidence (summary) |
|---|---|---|
| D-13 | PASS | staging ref present on the bare repo at the recorded tip; `main` unchanged |
| D-13.trace | PASS | host `pre-receive` log has the ref update with `agentbound-trace=<trace>` and `agentbound-session=<sid>` push options equal to the manifest's `session_trace` |
| GS-6 | PASS | clone + push to `main` refused by the host hook (`protected`) |
| T-6.4-002 | PASS | no interface other than `lo` in the session netns |
| T-6.4-003 / .only | PASS | `/run/agentbound/*` ENOENT from inside; exactly one socket node in the session |
| T-6.4-004 | PASS | abstract-namespace sockets of the host unreachable from the session netns |
| T-6.4-005 | PASS | a process with the session UID **outside** the scope cgroup (host `nsenter` without joining the scope) → `connection_refused scope_mismatch` |
| T-6.4-008 | PASS | host root forging `SCM_CREDENTIALS` (zero / two / wrong pid) into the session socket → rejected |
| T-6.4-009 | PASS (weak) | the per-operation check is keyed on the pidfs inode (WP1 F-1 amendment); evidence is the corpus of `process_mismatch` denials whose detail names both instances. Weak: a same-tick PID recycle is not reproduced on demand; the argument is that an inode-keyed check is tick-independent |
| T-6.4-012 | PASS (weak) | caller-supplied `url` → `args_schema` (closed argument set; the upstream is resolved from the catalogue only). Weak: no TLS upstream at 1B, so "TLS identity mismatch" is not exercised (recorded limit, §6) |
| T-6.4-013 | PASS | caller-supplied `session_id`/`trace_id` in args → refused; no ref appears under the other session's namespace |
| T-6.4-014 | PASS | quiesce → gateway `admission=false`; a new connection while quiesced refused; the held connection's next packet → `admission_closed`; termination closes it; `status` after seal → `unknown_record` |
| T-6.3-007 / .socket | PASS | post-termination: operation on the stale path fails; socket node removed at §5 step 9 |
| T-6.3-008 | PASS (recorded) | covered by T-6.4-013 (foreign identifiers in args) + T-6.4-008 (forged credentials) + T-6.4-005 (foreign scope): a replay from another session cannot satisfy the peer-credential binding; no dedicated row |
| D-12 | PASS | completeness metric: all 15 required event kinds present on one launch record (request → manifest → launch → connection → operation → effect → termination → seal) |
| D4.7-reconstruct | PASS | `systemctl restart agentbound-gateway` with a live session: `gateway.reconstructed projections=1`, listener recovered from the fd store, the session's next ping succeeds |
| D7-9.diagnostics | PASS | denial body carries `requirement_id=R-GW-4`, the denied session's `authorization_id`, `launch_record_digest`, `trace_id`; a concurrent foreign session's digest and trace absent |
| D7-8.audit-loss | PASS | receiver stopped + gateway spool made unwritable → first lost event closes admission and sends `revocation_signal audit_pipeline_degraded_below_stop_threshold`; lifecycle applies the manifest map (`terminate`); state `terminated` |
| D-06.storage-principal | PASS | after seal every workspace file created by the ephemeral UID is owned by `storage-engineering` (26 files); `session.ownership_projected failed=0` |
| D-02.1B / T-6.1-003.1B | PASS (recorded) | descriptor allowlist is 4 entries (stdin/stdout/stderr + socket mount); still no attach/PTY path to deny — partial stays as recorded in WP2 |
| T-6.1-013 | PASS | sealed session's socket node gone from the host; connect to the stale path → `ENOENT` |
| T-6.2-008.1B | PASS (recorded) | git-worker image has `sh` + `git` + client only; no interpreter or package loader |
| D-15.1B | PASS (recorded) | no delegation operation exists in the catalogue and no component socket is reachable from a session — residual stays recorded (nothing to narrow) |

### 2.3 1A rows

All 84 WP2 rows re-ran in the same suite and passed; two assertions changed in WP3 and are noted: T-6.5-009 now asserts "no identity freed before its 24 h floor" instead of "no identity freed" (the floor legitimately elapsed for WP2-era identities); T-6.6-* use the restored `task:fix-issue-1234` fixture after the gateway task was split into `task:fix-issue-1235`.

## 3. End-to-end attribution (goal 11 thin integrity slice)

One `git.push_staging` in the final run is traceable through three independently written stores by a single `launch_record_digest` / `trace_id`:

1. `agentbound-audit` chain (`events.jsonl`, hash-linked, receiver refuses to start on a break): `session.requested` → `session.manifest_verified` → `session.launch_record_committed` → `gateway.projected` → `gateway.grants_loaded` → `session.activated` → `gateway.connection_established {pidfs_inode, cgroup}` → `gateway.operation_admitted {operation_seq, payload_bytes}` → `gateway.operation_completed {remote_ref, objects}` → `session.revocation_received` → `session.termination_started` → `gateway.admission_denied` → `session.terminated` → `gateway.released {connections_closed}` → `session.ownership_projected` → `session.cleanup_completed {grants}` → `session.identity_released` → `session.sealed`. D-12's required set is the 15 kinds from `launch_record_committed` to `sealed` (both gateway and lifecycle kinds).
2. Lifecycle record store (`lifecycle.db`, chained): the committed binding names `gateway_projection` and `credential_grants`; the seal names the reclamation proof.
3. The Git host: `refs/agentbound/<sid>/<tail>` at the tip the session named, and the hook log carrying the trace and session as push options.

Integrity slice: (1) is hash-chained and verified at every daemon start and after the power-loss round; (2) is chained and verified at open; component-side spools (`audit-*.jsonl`) are the per-component buffer with an `unforwarded` marker per event the receiver rejected or missed. What is *not* in the slice: signing of chain heads, cross-store commitments, or an external anchor — recorded as 1C/1D work.

## 4. Gate 3 — ADR-0002 Decision 7 items 6–9

| Item | Result | Rows |
|---|---|---|
| 6 Bypass corpus | pass | T-6.4-001..014 (§2) |
| 7 TCB accounting | pass | §5 figures: authentication path 107 SLOC, gateway core 317 |
| 8 Failure behaviour | pass | D7-8; WP2 T-6.8-006/011 for policy/audit degradation; gateway loss → `gateway_unavailable` trigger with lifecycle holding cleanup until zero-connections ack |
| 9 Diagnostics | pass | D7-9 |

Recorded in [ADR-0002 0.9](../../architecture/ADR-0002-gateway-authentication.md). **Gate 3 verdict: pass, provisional until the independent review of §7.**

## 5. R-CON-8 SLOC report (Gate 1, republished)

Tool: `tokei 13.0.0-alpha.8`, code lines only, `-t Rust`; counted on the VM at the register commit. Methodology as WP2 §3.

| Figure | Value | Note |
|---|---|---|
| 1. Direct privileged SLOC (`agentbound-launch` 400 + `agentbound-lifecycle` 725 + `ab-common` 999) | **2 124** | ≤ 6 000: **PASS** (WP2: 2 166; a net decrease despite §5 grants, ownership projection and recovery retry, from consolidation). `ab-common` counted in full although only json/sig/schema/wire/audit are linked privileged |
| 1a. Gateway authentication path (TCB per ADR-0002 Consequences) | **107** | `auth.rs` 14 + `ab-common::wire` credential/pidfd functions (`accept`, `peercred`, `Packet`/`recv_packet`, `set_passcred`, `ProcInstance`/`proc_instance`) 51 + `session.rs::handle` per-packet credential check 42 |
| 2. Generated SLOC | 0 | unchanged |
| 3. Transitive dependency SLOC, privileged closure | ≈880 000 over 54 resolved crates (C 349 582, Rust 518 831; tests/benches/examples excluded) | WP2's ≈1 200 000 counted every crate in the lockfile; this figure is the runtime closure of the two privileged binaries only. Gateway closure: ≈433 000 (42 crates, Rust only — no SQLite) |
| 4. Configuration/rule SLOC | 61 | four systemd units (11+11+12+15) + 12-line `pre-receive` hook; one sudoers line, one tmpfiles line; seccomp filter is 20 BPF instructions in code |
| 5. Memory-unsafe-language SLOC | ≈350 000 | bundled SQLite amalgamation in `libsqlite3-sys 0.30.1` (tokei `C` code lines, excluding headers); WP2's ≈520 000 included headers and a second downloaded version. Alternative (hand-written store) still deferred |
| 6. Gateway core (unbounded, reported) | **317** | whole `agentbound-gateway` crate incl. adapters (git 54) |

**Honesty note on figure 1 (unchanged from WP2):** 976 of 2 879 lines across the four TCB crates exceed 100 columns; a `rustfmt`-normalised count would be roughly 1.6–2× higher (≈3 400–4 300 for figure 1), still under the bound.

## 6. Carry-ins, limits and residuals

| Item | Disposition |
|---|---|
| Storage-principal ownership projection | **Closed.** At seal, every file under a workspace root owned by the ephemeral identity is `lchown`ed to the manifest's `durable_ownership_projection.reference` mapped through catalogue `storage_principals` to a host user (`storage-engineering`, `storage-finance`); `session.ownership_projected {files, bytes, failed}`; an unmapped principal or any failed chown holds cleanup (not released). Row D-06.storage-principal |
| Allocator power-loss | **Closed.** [raw/power-loss-round.md](raw/power-loss-round.md): `sysrq b` with 14 launches in flight; `lifecycle.db` integrity ok, allocation chain contiguous 1197→1259, audit chain 8 141 records intact, 16 interrupted sessions reconciled to `sealed`, no identity left `in-use`. Defect found: gateway started before lifecycle's socket at boot — fixed (10 s retry) |
| D-Bus scope observation | **Recorded, permanent deviation.** [raw/dbus-observation-decision.md](raw/dbus-observation-decision.md): kernel-side observation (cgroup files + pidfd) kept; `zbus` and `busctl monitor` both rejected on TCB grounds; session-lifecycle wording to be revised at next unfreeze |
| ADR-0002 D7 items 6–9 | **Closed** (§4) |
| No TLS upstream | **Limit.** The Git host is a bare repository on the VM; T-6.4-012 exercises the closed argument set, not a TLS identity check. 1C must add an HTTPS upstream with pinned identity to the adapter |
| Socket projected as a mount, not a descriptor | **Finding.** Manifest-schema's `gateway_socket` element is realised as `mount:gateway-socket` (file bind mount) because the constructor's descriptor allowlist is stdin/stdout/stderr; binding `gateway_projection.socket_mount_id` names it. Revision entry in manifest-schema |
| Gateway socket mode | **Finding.** The unprivileged gateway cannot `chown` the node to the session UID; reachability is by mount namespace (0770 gateway-only directory, one bind mount per session) plus the UID/scope check at establishment. Documented in `crates/DESIGN-1B.md` |
| D-02 / T-6.1-003 (attach) | partial, unchanged: no attach/PTY interface exists at 1B |
| D-15 (delegation) | residual, unchanged: no delegation path exists; a `delegate` operation was deliberately not added |
| Same-tick PID reuse (T-6.4-009) | weak evidence, see §2.2 |

## 7. Defects found by the suite and fixed, and review notes

Defects: (1) all `gateway.*` events rejected by the receiver until schemas existed (unforwarded markers made this visible); (2) `deny_admission` was not consulted by the held-connection path until the per-packet admission check was added; (3) recovered termination-incomplete sessions never completed (retry needed a cgroup fd — rewritten to kill by path and rescan); (4) gateway restart lost every projection (fd store added); (5) gateway ignored `Sink.lost` (D7-8); (6) denials named only the rule (D7-9); (7) gateway/lifecycle boot ordering race (power-loss round); (8) T-6.5-009 asserted a property that stops being true after 24 h.

**For the independent reviewer.** (a) Row assertions and code under test have the same author; the PASS (weak) and PASS (recorded) rows above are the ones whose assertion the author judges weaker than the catalogue row. (b) Several host-side rows use root `nsenter` after joining the session's scope cgroup as the "legitimate in-scope peer" oracle; the reviewer should decide whether that oracle is acceptable or whether "host root can join the scope" is itself a threat-model observation. (c) The trust boundary to read is `crates/agentbound-gateway/src/auth.rs`, `session.rs` (`handle`), and `crates/ab-common/src/wire.rs` (`recv_packet`, `proc_instance`) — 107 SLOC — against ADR-0002 D2–D4.

## 8. Raw

- [raw/conformance-run.md](raw/conformance-run.md) — final machine run, 139 rows
- [raw/power-loss-round.md](raw/power-loss-round.md)
- [raw/dbus-observation-decision.md](raw/dbus-observation-decision.md)
