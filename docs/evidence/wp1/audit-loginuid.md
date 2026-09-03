# WP1 evidence — `audit-loginuid`

**Covers:** plan WP1 spike "`loginuid` and audit correlation, including loss behaviour under load"; R-CON-6; identity lifecycle §6; manifest `audit.loss_behaviour` precondition (kernel-side loss must be observable).
**Baseline:** VM 110, Linux `6.12.107+deb13-cloud-amd64` (Debian config: `CONFIG_AUDIT=y`, `CONFIG_AUDITSYSCALL=y`, **no** `CONFIG_AUDIT_LOGINUID_IMMUTABLE`), systemd `257.13-1~deb13u1`, auditd active with `backlog_limit 8192`, `backlog_wait_time 60000`.
**Spike:** `spikes/audit-loginuid/`. **Raw transcript:** `raw/audit-loginuid.txt`. **Command:** `spikes/run.sh audit-loginuid` (re-executes itself under `systemd-run` so the constructor stand-in has an *unset* `loginuid`, as a real service does; an SSH shell has a PAM-set value).

## Results

| ID | Required result | Observed | Result |
|---|---|---|---|
| LU-1 | Barrier-blocked child inherits the constructor's `loginuid` (unset for a service) | child `4294967295` before release | **PASS** |
| LU-2 | Constructor (`CAP_AUDIT_CONTROL`) sets `loginuid` in the child before exec | write succeeds; `sessionid` allocated alongside | **PASS** |
| LU-3 | Write-once semantics on the baseline | **second privileged write also succeeds** (`200042` → `200043`); see F-6 | measured |
| LU-4 | Session (after `setuid` to the execution UID) cannot set `loginuid` | `EACCES` | **PASS** |
| LU-5 | Value inherited across `fork`+`exec` | exec'd grandchild reads the set `loginuid`/`sessionid` | **PASS** |
| AC-1 | Audit rule keyed on the execution UID emits records | 4 SYSCALL records (3 unset + 1 set) | **PASS** |
| AC-2 | Set `loginuid` appears as `auid`, with `ses` | `auid=200042 ses=125 uid=200042` | **PASS** |
| AC-3 | Unset `loginuid` appears as `auid=4294967295 ses=4294967295` | as required; attribution falls to `uid`+`pid` | **PASS** |
| AC-4 | Record fields available for the join | `uid auid ses pid ppid comm exe`; **no** PID-namespace id or start time | **PASS** (confirms §6's join design) |
| AL-1 | Kernel-side loss is observable | with `backlog_limit 64`, `backlog_wait_time 0`: 20 000 audited syscalls in 87 ms → `lost` advanced by 24 413 | **PASS** |
| AL-2 | `backlog_wait_time` trades loss for producer stall | with `backlog_wait_time 200`: 2 000 calls took 207 ms and lost 0 | **PASS** |

## Finding F-6 — `loginuid` is not write-once on the pinned kernel

Debian's 6.12 config omits `CONFIG_AUDIT_LOGINUID_IMMUTABLE`, and the corresponding sysctl does not exist. Consequently a `CAP_AUDIT_CONTROL` holder can **rewrite** an already-set `loginuid` (LU-3). The technical report §5, identity lifecycle §6 and R-CON-6 describe `loginuid` as "write-once". On this baseline that is true only for processes without `CAP_AUDIT_CONTROL` — which is every session process (LU-4), so the *security* argument stands (the session cannot alter its own `loginuid`) and `loginuid` remains corroborating-only as the specs already say. But R-CON-6's outcome vocabulary (`set`, `immutable`, `already-set`, `denied`) should not imply the kernel guarantees immutability: recommended wording change "write-once" → "cannot be changed by a process lacking `CAP_AUDIT_CONTROL`; kernel-level immutability depends on `CONFIG_AUDIT_LOGINUID_IMMUTABLE`, absent from the pinned baseline". Also worth recording in ADR-0003's pinned-set kernel row. No mechanism change.

Related implementation point: since the constructor holds `CAP_AUDIT_CONTROL` and the child inherits a *set* value whenever the constructor itself has one (LU-1), the constructor MUST run with `loginuid` unset (service context, no PAM) or the `already-set` branch is taken on every launch.

## Loss behaviour — what `loss_behaviour` can observe

`auditctl -s lost` (kernel counter, `AUDIT_STATUS`) is the observable trigger for `stop`/`quarantine`. Two regimes exist, chosen by host audit policy, not by Agentbound:

- **drop regime** (`backlog_wait_time 0` or exhausted): records are dropped, `lost` advances, producers continue at full speed (20 000 audited syscalls in 87 ms);
- **stall regime** (`backlog_wait_time > 0`, the Debian default 60 000 jiffies): producers block until the backlog drains; nothing is lost while the wait budget holds (2 000 calls, 207 ms, 0 lost).

`agentbound-audit` therefore needs both a `lost`-counter watch (for the drop regime) and the daemon-side queue depth (its own loss) to implement `loss_behaviour` deterministically. The requirement text already anticipates this ("loss counters are exposed"); the spike confirms the kernel exposes the counter and that it moves. Note the counter is global to the host, not per session, so a session's `stop` decision under `loss_behaviour = stop` is triggered by *any* host audit loss in its window — acceptable for the evaluation arm, worth stating in the technical report's audit section.
