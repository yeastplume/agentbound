# Allocator power-loss round (machine record)

Carry-in from WP2 (register row "Allocator power-loss": RESIDUAL). Run on VM 110, 2026-09-05.

## Procedure

`/root/powerloss.sh`, started detached (`setsid nohup`):

1. Record `max(seq)` of `lifecycle.db` `alloc` (1197), audit store line count (7224), wall clock (13:33:34Z).
2. Loop: every 0.7 s launch one `runtime:probe` session as alice **and** one `local-socket` `runtime:git-worker`
   session as bob (each launch = allocation → binding commit → activation → gateway project/activate; each
   git-worker session pushes a staging ref through the gateway).
3. After 5 s: `echo b > /proc/sysrq-trigger` — immediate reboot, no sync, no unmount, no service stop.
   7 loop iterations (14 launches) were in flight or recently committed.

## Observed

| Check | Result |
|---|---|
| VM returned unattended | yes; previous boot `d0fb2875…` ends 13:33:39Z, new boot `dd21b43e…` starts 13:33:47Z |
| All four services active on the new boot | `agentbound-lifecycle`, `-audit`, `-gateway`, `-policy` active at 13:33:49Z |
| `lifecycle.db` `pragma integrity_check` / `quick_check` | `ok` / `ok` (WAL file 4.1 MiB replayed on open) |
| `alloc` rows after / max seq | 1259 / 1259 — contiguous, no gap, 62 rows committed during the 5 s window survived |
| lifecycle chain verification at open (`Store::open` → `verify_chain`) | passed (service started; a chain break exits non-zero and the unit would be `failed`) |
| Allocation states after reconciliation | `free` 96, `quarantined` 217; **no `allocated` / `in-use` / `reclaiming` left** |
| `session.recovery_reconciled` on the new boot | 16 events, all `cleanup-and-seal` (no live cgroup, no credential-holding process — the reboot killed everything) |
| Post-reconciliation path | each reconciled record: `ownership_projected` → `cleanup_completed` → `identity_released` → `sealed` (16 `session.sealed`) |
| Audit store (`events.jsonl`) | 8141 lines, every line parses, head `seq` 8141 with `prev` present; receiver started (it refuses to start on a chain break) |
| Torn tail | none: the last pre-reset audit line is complete (`Sink` writes one `write(2)` per record; ext4 `data=ordered`) |
| Gateway on the new boot | started 2 s before lifecycle's socket was bound → logged `lifecycle unreachable; starting with no projections` (harmless here: no session survived the reboot, and reconstruction only matters for live sessions). **Fixed**: reconstruct now retries lifecycle for up to 10 s. |

## Conclusion

Committed allocator transactions survived an unsynced hard reset (SQLite WAL, `synchronous=FULL`, ext4); the
identity chain and the audit chain verified on the next boot; every interrupted session was reconciled to
`sealed` with its identity quarantined — no ephemeral UID was left `in-use` or `allocated`. The one defect found
(gateway/lifecycle start ordering) does not affect durability and is fixed in the same commit.
