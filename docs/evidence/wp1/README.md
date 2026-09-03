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
| 4 Abstract socket isolation | pending | — | ADR-0002 Decision 1; requirements R-GW |
| 5 Revocation latency | pending (mechanism portion); operation denial is WP2 | — | ADR-0002 Decision 4 |
| 6 Bypass corpus | WP2 (needs gateway) | — | ADR-0002 |
| 7 TCB accounting | WP2 (needs gateway code); SLOC tool pinned under VM-2 | — | requirements §12 |
| 8 Failure behaviour | WP2 (needs components) | — | session lifecycle §6 |
| 9 Diagnostics | WP2 (needs gateway) | — | component interfaces |

## Plan WP1 spikes

| Spike | Status | Evidence | Reopens on failure |
|---|---|---|---|
| Per-session execution identity allocation and durable-ownership projection | pending | — | ADR-0001; identity lifecycle |
| systemd scope + PID-namespace init containment; `cgroup.kill`, D-state tasks | pending | — | session lifecycle §5 |
| Namespace/mount/procfs construction in §2.1 order; mount-descriptor resolution | pending | — | session lifecycle §3 |
| Descriptor closure and runtime launch ordering | pending | — | session lifecycle §3 |
| Socket-family seccomp and abstract-socket isolation in an empty netns | pending | — | requirements R-CON; ADR-0002 D7-4 |
| ADR-0002 Decision 7 verification | in progress (items 1–3 done) | above | ADR-0002 |
| `agentbound-lifecycle` D-Bus scope-signal subscription, pidfd-watch fallback, systemd-kills-first race | pending | — | session lifecycle §4; component interfaces |
| Git staging-ref adapter and protected-branch behaviour | pending | — | plan §3.3 |
| `loginuid` and audit correlation, loss behaviour under load | pending | — | requirements R-CON-6; identity lifecycle §6 |
| Minimal control-arm launcher | pending (boot check only in WP1) | — | ADR-0003 |

## Open-question register items

| Item | Status | Evidence | Reopens on failure |
|---|---|---|---|
| VM-1 vsock peer-CID reporting | pending | — | ADR-0002 Decision 6 (binding via VMM connection table) |
| VM-2 cross-arm SLOC comparability | pending | — | ADR-0003 (per-arm disclosure only) |
| LC-1 allocator/constructor implementation spike | pending | — | identity lifecycle §3 |
| LC-2 frozen cgroup holding a `SOCK_SEQPACKET` connection | pending | — | session lifecycle §6 |
| ID-1 allocator-store crash consistency | pending | — | identity lifecycle §3 |

## Findings requiring document amendments before WP2

| # | Finding | Owning document | Status |
|---|---|---|---|
| F-1 | `/proc` start time has 10 ms granularity; a PID recycled within one tick has an identical start time. The pidfs inode / held pidfd is the reliable instance key. | ADR-0002 Decision 2 | recorded; amendment pending |
