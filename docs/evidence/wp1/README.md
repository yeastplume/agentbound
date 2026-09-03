# WP1 mechanism-verification register

Evidence for the Phase 1 plan WP1 exit condition: *every ADR-0002 Decision 7 item and every WP1 spike records pass with evidence on the pinned baseline; any fail reopens the relevant ADR before WP2 begins.*

**Pinned baseline (as run):** VM 110 `agentbound-dev` — Linux `6.12.107+deb13-cloud-amd64`, systemd `257.13-1~deb13u1`, Debian 13 genericcloud image SHA-512 `8ea9faae…6371f50` (full digest in the operator's home-server runbook), 4 vCPU / 8 GiB, nested KVM available. Spikes are throwaway Rust under `spikes/`; raw transcripts under `raw/`.

Status values: **PASS**, **FAIL**, **FINDING** (item passed; an amendment to the owning document is recommended), **pending**, **WP2** (verifiable only with implementation components; listed for completeness).

## ADR-0002 Decision 7

| Item | Status | Evidence | Reopens / amends on failure |
|---|---|---|---|
| 1 `SOCK_SEQPACKET` + `SO_PASSCRED` | **PASS** | [seqpacket-creds](seqpacket-creds.md) D7-1a–c | ADR-0002 Decision 1 |
| 2 pidfd from credential PID | **PASS + FINDING F-1** | [seqpacket-creds](seqpacket-creds.md) D7-2a–d | ADR-0002 Decision 2: comparison key should be the pidfs inode, start time corroborating only |
| 3 Descriptor transfer | **PASS** | [seqpacket-creds](seqpacket-creds.md) D7-3a–b, X-1 | ADR-0002 Decision 2 |
| 4 Abstract socket isolation | **PASS** | [netns-seccomp](netns-seccomp.md) D7-4a–c | ADR-0002 Decision 1; requirements R-GW |
| 5 Revocation latency | **PASS** (mechanism half); typed-operation half is WP2 | [frozen-peer](frozen-peer.md) D7-5a–b | ADR-0002 Decision 4 |
| 6 Bypass corpus | WP2 (needs gateway) | — | ADR-0002 |
| 7 TCB accounting | WP2 (needs gateway code); SLOC tool pinned under VM-2 | — | requirements §12 |
| 8 Failure behaviour | WP2 (needs components) | — | session lifecycle §6 |
| 9 Diagnostics | WP2 (needs gateway) | — | component interfaces |

## Plan WP1 spikes

| Spike | Status | Evidence | Reopens on failure |
|---|---|---|---|
| Per-session execution identity allocation and durable-ownership projection | pending | — | ADR-0001; identity lifecycle |
| systemd scope + PID-namespace init containment; `cgroup.kill`, D-state tasks | **PASS + FINDINGS F-3, F-4** | [scope-kill](scope-kill.md) A-*, B-* | session lifecycle §5 |
| Namespace/mount/procfs construction in §2.1 order; mount-descriptor resolution | **PASS** | [mount-construct](mount-construct.md) R6-*, C1–C5 | session lifecycle §3 |
| Descriptor closure and runtime launch ordering | **PASS** | [mount-construct](mount-construct.md) C6–C7 | session lifecycle §3 |
| Socket-family seccomp and abstract-socket isolation in an empty netns | **PASS + FINDING F-2** | [netns-seccomp](netns-seccomp.md) NS-*, SC-* | requirements R-CON; ADR-0002 D7-4 |
| ADR-0002 Decision 7 verification | in progress (items 1–5 done; 6–9 are WP2) | above | ADR-0002 |
| `agentbound-lifecycle` D-Bus scope-signal subscription, pidfd-watch fallback, systemd-kills-first race | **PASS + FINDINGS F-4, F-5** | [scope-kill](scope-kill.md) C-* | session lifecycle §4; component interfaces |
| Git staging-ref adapter and protected-branch behaviour | pending | — | plan §3.3 |
| `loginuid` and audit correlation, loss behaviour under load | **PASS + FINDING F-6** | [audit-loginuid](audit-loginuid.md) | requirements R-CON-6; identity lifecycle §6 |
| Minimal control-arm launcher | pending (boot check only in WP1) | — | ADR-0003 |

## Open-question register items

| Item | Status | Evidence | Reopens on failure |
|---|---|---|---|
| VM-1 vsock peer-CID reporting | pending | — | ADR-0002 Decision 6 (binding via VMM connection table) |
| VM-2 cross-arm SLOC comparability | pending | — | ADR-0003 (per-arm disclosure only) |
| LC-1 allocator/constructor implementation spike | pending | — | identity lifecycle §3 |
| LC-2 frozen cgroup holding a `SOCK_SEQPACKET` connection | **PASS** — no delay; §6 stands | [frozen-peer](frozen-peer.md) LC2-1–5 | session lifecycle §6 |
| ID-1 allocator-store crash consistency | pending | — | identity lifecycle §3 |

## Findings requiring document amendments before WP2

| # | Finding | Owning document | Status |
|---|---|---|---|
| F-1 | `/proc` start time has 10 ms granularity; a PID recycled within one tick has an identical start time. The pidfs inode / held pidfd is the reliable instance key. | ADR-0002 Decision 2 | recorded; amendment pending |
| F-2 | An inherited sysfs mount shows the host's network interfaces inside an empty netns. R-CON-2 covers `/proc` only. Add: no inherited sysfs in the session root; mount sysfs, if at all, after the netns exists. | requirements R-CON-2; session lifecycle §3 step 5; test catalogue T-6.1 | recorded; amendment pending |
| F-3 | `cgroup.freeze` never reaches `frozen 1` while a D-state member exists; §5 step 4 must not wait for the frozen state before `cgroup.kill`. | session lifecycle §5 step 4 | recorded; amendment pending |
| F-4 | PID-namespace init ignores external `SIGTERM`; `systemctl stop` stalls for `DefaultTimeoutStopSec` (90 s) unless `TimeoutStopUSec` is set at `StartTransientUnit` (cannot be set later on a scope). | session lifecycle §3 scope prerequisites, §4 | recorded; amendment pending |
| F-5 | `UnitRemoved` is emitted at unit GC (~1.5 s later); `PropertiesChanged`/`ActiveState` and the held pidfd are the prompt triggers. | session lifecycle §4 (guidance only) | recorded; no obligation change |
| F-6 | Pinned kernel lacks `CONFIG_AUDIT_LOGINUID_IMMUTABLE`: `loginuid` is re-settable by `CAP_AUDIT_CONTROL` (never held by sessions). Replace "write-once" with the capability-conditional statement; note the host-global `lost` counter. | R-CON-6; identity lifecycle §6; technical report §5; ADR-0003 kernel row | recorded; amendment pending |
