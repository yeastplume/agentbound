# WP1 mechanism-verification register

Evidence for the Phase 1 plan WP1 exit condition: *every ADR-0002 Decision 7 item and every WP1 spike records pass with evidence on the pinned baseline; any fail reopens the relevant ADR before WP2 begins.*

**Pinned baseline (as run):** VM 110 `agentbound-dev` — Linux `6.12.107+deb13-cloud-amd64`, systemd `257.13-1~deb13u1`, Debian 13 genericcloud image SHA-512 `8ea9faae…6371f50` (full digest in the operator's home-server runbook), 4 vCPU / 8 GiB, nested KVM available. Spikes are throwaway Rust under `spikes/`; raw transcripts under `raw/`.

Status values: **PASS**, **FAIL**, **FINDING** (item passed; an amendment to the owning document is recommended), **WP2** (verifiable only with implementation components; listed for completeness).

## WP1 exit status

Every WP1 item has a recorded result: 10 spikes, 103 individual checks — 102 PASS and one deliberate FAIL (D7-2d, the start-time-only reuse check that motivates F-1) — and no ADR reopened. Eight findings (F-1–F-8) were recorded; none changes a mechanism decision, all are wording, ordering-guidance, or accounting amendments to frozen documents, to be applied as revision entries before WP2 begins (table at the end). Two items remain classified WP2 by construction (D7 items 6, 8, 9 need the gateway; the typed-operation half of item 5 likewise).

| Spike | Checks | Result |
|---|---|---|
| [seqpacket-creds](seqpacket-creds.md) | D7 items 1–3, forge test, PID-reuse | PASS; F-1 |
| [netns-seccomp](netns-seccomp.md) | D7 item 4, empty netns, socket-family seccomp | PASS; F-2 |
| [scope-kill](scope-kill.md) | scope containment, freeze/kill, D-state, D-Bus, kills-first race | PASS; F-3, F-4, F-5 |
| [frozen-peer](frozen-peer.md) | LC-2, D7 item 5 mechanism half | PASS |
| [audit-loginuid](audit-loginuid.md) | loginuid semantics, audit join fields, loss counters | PASS; F-6 |
| [mount-construct](mount-construct.md) | openat2/open_tree/move_mount ordering, proc/sysfs, close_range, exec | PASS |
| [identity-store](identity-store.md) | ID-1, LC-1 allocator: CAS, chain, crash rounds, recovery | PASS |
| [git-staging](git-staging.md) | R-GW-5 ref policy, bundle import, host protection | PASS |
| [vsock-cid](vsock-cid.md) | VM-1 against Firecracker v1.16.1; boot check | PASS; F-7 |
| [sloc-arms](sloc-arms.md) | VM-2 with tokei over both arms | PASS; F-8 |

Reproduction: `spikes/run.sh <name>` from a checkout with SSH access to the baseline VM; each raw transcript in `raw/` carries host, kernel, systemd, commit, and date headers.

## ADR-0002 Decision 7

| Item | Status | Evidence | Reopens / amends on failure |
|---|---|---|---|
| 1 `SOCK_SEQPACKET` + `SO_PASSCRED` | **PASS** | [seqpacket-creds](seqpacket-creds.md) D7-1a–c | ADR-0002 Decision 1 |
| 2 pidfd from credential PID | **PASS + FINDING F-1** | [seqpacket-creds](seqpacket-creds.md) D7-2a–d | ADR-0002 Decision 2: comparison key should be the pidfs inode, start time corroborating only |
| 3 Descriptor transfer | **PASS** | [seqpacket-creds](seqpacket-creds.md) D7-3a–b, X-1 | ADR-0002 Decision 2 |
| 4 Abstract socket isolation | **PASS** | [netns-seccomp](netns-seccomp.md) D7-4a–c | ADR-0002 Decision 1; requirements R-GW |
| 5 Revocation latency | **PASS** (mechanism half); typed-operation half is WP2 | [frozen-peer](frozen-peer.md) D7-5a–b | ADR-0002 Decision 4 |
| 6 Bypass corpus | WP2 (needs gateway) | — | ADR-0002 |
| 7 TCB accounting | WP2 (needs gateway code); counting tool and accounting rules established under VM-2 | [sloc-arms](sloc-arms.md) | requirements §12 |
| 8 Failure behaviour | WP2 (needs components) | — | session lifecycle §6 |
| 9 Diagnostics | WP2 (needs gateway) | — | component interfaces |

## Plan WP1 spikes

| Spike | Status | Evidence | Reopens on failure |
|---|---|---|---|
| Per-session execution identity allocation and durable-ownership projection | **PASS** (allocation: [identity-store](identity-store.md); UID/cap projection: [mount-construct](mount-construct.md) C7-1) | — | ADR-0001; identity lifecycle |
| systemd scope + PID-namespace init containment; `cgroup.kill`, D-state tasks | **PASS + FINDINGS F-3, F-4** | [scope-kill](scope-kill.md) A-*, B-* | session lifecycle §5 |
| Namespace/mount/procfs construction in §2.1 order; mount-descriptor resolution | **PASS** | [mount-construct](mount-construct.md) R6-*, C1–C5 | session lifecycle §3 |
| Descriptor closure and runtime launch ordering | **PASS** | [mount-construct](mount-construct.md) C6–C7 | session lifecycle §3 |
| Socket-family seccomp and abstract-socket isolation in an empty netns | **PASS + FINDING F-2** | [netns-seccomp](netns-seccomp.md) NS-*, SC-* | requirements R-CON; ADR-0002 D7-4 |
| ADR-0002 Decision 7 verification | **complete for WP1** (items 1–5 PASS; 6–9 classified WP2 — they need gateway code) | above | ADR-0002 |
| `agentbound-lifecycle` D-Bus scope-signal subscription, pidfd-watch fallback, systemd-kills-first race | **PASS + FINDINGS F-4, F-5** | [scope-kill](scope-kill.md) C-* | session lifecycle §4; component interfaces |
| Git staging-ref adapter and protected-branch behaviour | **PASS** | [git-staging](git-staging.md) | plan §3.3 |
| `loginuid` and audit correlation, loss behaviour under load | **PASS + FINDING F-6** | [audit-loginuid](audit-loginuid.md) | requirements R-CON-6; identity lifecycle §6 |
| Minimal control-arm launcher | **PASS** (boot check; nested KVM available on VM 110) | [vsock-cid](vsock-cid.md) VM1-7 | ADR-0003 |

## Open-question register items

| Item | Status | Evidence | Reopens on failure |
|---|---|---|---|
| VM-1 vsock peer-CID reporting | **PASS + FINDING F-7** — no host `AF_VSOCK` endpoint with Firecracker; CID derived via VMM `SO_PEERCRED`/pidfd | [vsock-cid](vsock-cid.md) | ADR-0003 (wording) |
| VM-2 cross-arm SLOC comparability | **PASS + FINDING F-8** — attribution consistent; three accounting rules and tool pin needed | [sloc-arms](sloc-arms.md) | ADR-0003 (accounting) |
| LC-1 allocator/constructor implementation spike | **PASS** — allocator: [identity-store](identity-store.md); constructor/freeze/kill/pidfd/scope: [scope-kill](scope-kill.md), [mount-construct](mount-construct.md) | — | identity lifecycle §3 |
| LC-2 frozen cgroup holding a `SOCK_SEQPACKET` connection | **PASS** — no delay; §6 stands | [frozen-peer](frozen-peer.md) LC2-1–5 | session lifecycle §6 |
| ID-1 allocator-store crash consistency | **PASS** — candidate design holds; §3 not reopened | [identity-store](identity-store.md) ID-9–11 | identity lifecycle §3 |

## Findings requiring document amendments before WP2

| # | Finding | Owning document | Status |
|---|---|---|---|
| F-1 | `/proc` start time has 10 ms granularity; a PID recycled within one tick has an identical start time. The pidfs inode / held pidfd is the reliable instance key. | ADR-0002 Decision 2 | recorded; amendment pending |
| F-2 | An inherited sysfs mount shows the host's network interfaces inside an empty netns. R-CON-2 covers `/proc` only. Add: no inherited sysfs in the session root; mount sysfs, if at all, after the netns exists. | requirements R-CON-2; session lifecycle §3 step 5; test catalogue T-6.1 | recorded; amendment pending |
| F-3 | `cgroup.freeze` never reaches `frozen 1` while a D-state member exists; §5 step 4 must not wait for the frozen state before `cgroup.kill`. | session lifecycle §5 step 4 | recorded; amendment pending |
| F-4 | PID-namespace init ignores external `SIGTERM`; `systemctl stop` stalls for `DefaultTimeoutStopSec` (90 s) unless `TimeoutStopUSec` is set at `StartTransientUnit` (cannot be set later on a scope). | session lifecycle §3 scope prerequisites, §4 | recorded; amendment pending |
| F-5 | `UnitRemoved` is emitted at unit GC (~1.5 s later); `PropertiesChanged`/`ActiveState` and the held pidfd are the prompt triggers. | session lifecycle §4 (guidance only) | recorded; no obligation change |
| F-6 | Pinned kernel lacks `CONFIG_AUDIT_LOGINUID_IMMUTABLE`: `loginuid` is re-settable by `CAP_AUDIT_CONTROL` (never held by sessions). Replace "write-once" with the capability-conditional statement; note the host-global `lost` counter. | R-CON-6; identity lifecycle §6; technical report §5; ADR-0003 kernel row | recorded; amendment pending |
| F-7 | Firecracker's vsock is a Unix-socket bridge; the host sees the VMM process as peer, not a guest CID. Bind via `SO_PEERCRED`→VMM pidfd→configured `guest_cid`; daemon must own the bridge socket path. | ADR-0003 "VM identity, CID lifetime, and vsock admission" | recorded; wording amendment pending |
| F-8 | Firecracker v1.16.1 closure: direct 77 904 + 3 494; transitive 2.82 M (1.29 M C/C++/asm from AWS-LC via `aws-lc-rs`, used only for randomness). Pin tokei; state present-vs-compiled rule; generated-code allowlist; feature pins. | ADR-0003 "Trusted-code size"; requirements §12 accounting | recorded; accounting amendment pending |
