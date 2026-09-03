# WP1 mechanism-verification register

Evidence for the Phase 1 plan WP1 exit condition: *every ADR-0002 Decision 7 item and every WP1 spike records pass with evidence on the pinned baseline; any fail reopens the relevant ADR before WP2 begins.*

**Pinned baseline (as run):** VM 110 `agentbound-dev` — Linux `6.12.107+deb13-cloud-amd64`, systemd `257.13-1~deb13u1`, Debian 13 genericcloud image `debian-13-genericcloud-amd64.qcow2` SHA-512 `8ea9faae810043a0b35b0149f05014f26705c2339ffb11ead308f33e844a87cc3ef46ec81d5262b38817b6a88af404874d48a5857ebe072ef6a31dfb6e371f50`, 4 vCPU / 8 GiB, nested KVM available. Each Rust spike's `Cargo.lock` is committed. Spikes are throwaway Rust under `spikes/`; raw transcripts under `raw/`.

Status values: **PASS**, **FAIL**, **FINDING** (item passed; an amendment to the owning document is recommended), **WP2** (verifiable only with implementation components; listed for completeness).

## WP1 exit status

Every WP1 item has a recorded result: 10 spikes, 103 individual checks — 102 PASS and **one FAIL** (D7-2d). Two pre-registered contingencies were exercised:

- **D7-2d failed.** The Decision 7 item 2 required result "reuse of a recycled PID is detected" does not hold when the check is the `/proc` start time alone: a PID recycled within one 10 ms tick presents an identical start time. Under Decision 7's rule, **ADR-0002 was reopened**, Decision 2 amended narrowly to key process instances on the pidfs inode (start time corroborating), and re-accepted as 0.7 (0.8 for the accounting changes below). The pidfd path of item 2 passed unchanged (F-1).
- **VM-1 took its pre-registered fallback.** Firecracker exposes a Unix-socket bridge, not a host `AF_VSOCK` peer CID, so "binding uses the VMM connection table and the ADR records the change" applies: **ADR-0003 was revised** and re-accepted as 0.9 with topology and every binding element unchanged (F-7).

Six further findings (F-2–F-6, F-8) are wording, ordering-guidance, or accounting amendments. All eight are applied as revision entries (table at the end: ADR-0002 0.8, ADR-0003 0.9, requirements 0.10, session lifecycle 0.7, identity lifecycle 0.7, test catalogue 0.7, traceability matrix 0.7, technical report 0.5-TR12). Carried to WP2–WP3 by construction: Decision 7 component items 6–9 and the typed-operation half of item 5 (need gateway and lifecycle code; ADR-0002 0.8 Decision 7), and the durable-ownership projection (needs constructor and storage broker; plan 0.13).

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
| Per-session execution identity allocation and durable-ownership projection | **allocation and execution-identity projection PASS** (allocation: [identity-store](identity-store.md); UID transition, `no_new_privs`, bounding-set disposal: [mount-construct](mount-construct.md) C7-1). **Durable-ownership projection carried to WP2**: no spike exercised a bind/ACL grant into durable-owner state, a storage-broker grant, workspace-image transfer, or proof that the durable-owner UID never executes session code | — | ADR-0001; identity lifecycle |
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

## Findings and their document amendments

| # | Finding | Owning document | Status |
|---|---|---|---|
| F-1 | `/proc` start time has 10 ms granularity; a PID recycled within one tick has an identical start time (D7-2d **FAIL**). The pidfs inode / held pidfd is the reliable instance key. | ADR-0002 Decision 2 (reopened, amended, re-accepted); test catalogue T-6.4-009 | **applied** (ADR-0002 0.7/0.8; catalogue 0.7) |
| F-2 | An inherited sysfs mount shows the host's network interfaces inside an empty netns. R-CON-2 covers `/proc` only. Add: no inherited sysfs in the session root; mount sysfs, if at all, after the netns exists. | requirements R-CON-2; session lifecycle §3 step 5; test catalogue T-6.2-009 (new), F-C-05; traceability matrix Inv 17 | **applied** (requirements 0.9; lifecycle 0.7; catalogue 0.7; matrix 0.7) |
| F-3 | `cgroup.freeze` never reaches `frozen 1` while a D-state member exists; §5 step 4 must not wait for the frozen state before `cgroup.kill`. | session lifecycle §5 step 4; test catalogue F-T-04 | **applied** (lifecycle 0.7; catalogue 0.7) |
| F-4 | PID-namespace init ignores external `SIGTERM`; `systemctl stop` stalls for `DefaultTimeoutStopSec` (90 s) unless `TimeoutStopUSec` is set at `StartTransientUnit` (cannot be set later on a scope). | session lifecycle §3 scope prerequisites, §4 | **applied** |
| F-5 | `UnitRemoved` is emitted at unit GC (~1.5 s later); `PropertiesChanged`/`ActiveState` and the held pidfd are the prompt triggers. | session lifecycle §4 (guidance only) | **applied** (session lifecycle 0.7, guidance) |
| F-6 | Pinned kernel lacks `CONFIG_AUDIT_LOGINUID_IMMUTABLE`: `loginuid` is re-settable by `CAP_AUDIT_CONTROL` (never held by sessions). Replace "write-once" with the capability-conditional statement; note the host-global `lost` counter. | R-CON-6; R-AUD-3 (host-global `lost` counter); identity lifecycle §6; technical report §5; ADR-0003 kernel row | **applied** (requirements 0.9/0.10; identity 0.7; TR12; ADR-0003 0.9) |
| F-7 | Firecracker's vsock is a Unix-socket bridge; the host sees the VMM process as peer, not a guest CID. Bind via `SO_PEERCRED`→VMM pidfd→configured `guest_cid`; daemon must own the bridge socket path. | ADR-0003 "VM identity, CID lifetime, and vsock admission" (pre-registered fallback, revised, re-accepted); ADR-0002 Decision 6 | **applied** (ADR-0003 0.9; ADR-0002 0.8) |
| F-8 | Firecracker v1.16.1 closure: direct 77 904 + 3 494; transitive 2.82 M (1.29 M C/C++/asm from AWS-LC via `aws-lc-rs`, used only for randomness). Pin tokei; state present-vs-compiled rule; generated-code allowlist; feature pins. | ADR-0003 "Trusted-code size"; requirements §12 accounting | **applied** (ADR-0003 0.9; requirements 0.10) |

## Residual assumptions and reproducibility limits

Recorded so that WP2 carries them, not so that they are forgotten:

- **Power-loss durability of the allocator store** (ID-1) was not tested; the crash model was daemon `SIGKILL`. Durability across power loss rests on SQLite's documented `synchronous=FULL` WAL behaviour. WP2 either adds a `dm-flakey`/`dm-log-writes` fault-injection test or carries this as a residual assumption in the launch record.
- **Guest kernel.** The Firecracker spike (VM-1, boot check) used the Firecracker-CI `vmlinux-6.1.128` guest kernel; ADR-0003 pins the guest to the same 6.12 release as the host. No recorded result depends on guest kernel version (the bridge model and CID rules are VMM behaviour), but the 1D pinned-configuration run MUST use the 6.12 guest kernel and re-record VM1-1..7.
- **Linux-arm SLOC stand-in.** VM-2 validated the attribution *method* over `spikes/identity-store`, not the eventual Linux-arm trusted closure; the comparative SLOC figures are a WP2–WP3 output under the ADR-0003 0.9 rules.
- **Kernel `lost` counter is host-global** (F-6): per-session loss is inferable only via the session's own rule keys; an evaluation host MUST carry no other audit-generating workload (R-AUD-3, requirements 0.10).
- **Shared host.** All spikes ran on one nested-KVM VM; timing figures (revocation latency, boot time, allocator commit latency) are indicative, not the pre-registered measurement runs.
