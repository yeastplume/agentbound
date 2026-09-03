# WP1 evidence — `identity-store`

**Covers:** open-question register item **ID-1** (allocator-store implementation: append-only, compare-and-set, crash consistency); implementation spike **LC-1** allocator half (identity lifecycle §3.1–§3.3, §4 state machine; R-ID-*); plan WP1 spike "per-session execution identity allocation" (allocation half — durable-ownership projection is exercised by `mount-construct` C7-1 and lifecycle reclamation is WP2).
**Baseline:** VM 110, Linux `6.12.107+deb13-cloud-amd64`; SQLite 3.46 (bundled via `rusqlite 0.32`), `journal_mode=WAL`, `synchronous=FULL`, single-writer `BEGIN IMMEDIATE` transactions, ext4 on virtio-scsi.
**Spike:** `spikes/identity-store/`. **Raw transcript:** `raw/identity-store.txt`. **Command:** `spikes/run.sh identity-store`.

Store shape (the register's candidate): one `records` table keyed by a monotonic `seq`, each row carrying `record_id`, `uid`, `state`, `authz_id`, actor, timestamp, `prev_hash`, and `hash = SHA-256(prev_hash ‖ fields)`. `BEFORE UPDATE`/`BEFORE DELETE` triggers raise on any mutation. A partial unique index on `authz_id WHERE state='allocated'` prevents one launch record binding two UIDs. Every transition is a compare-and-set: the caller presents the `(record_id, seq)` it observed for the UID; the append succeeds only if that is still the UID's tail and the transition is legal in the §4 machine.

## Results

| ID | Required result | Observed | Result |
|---|---|---|---|
| ID-0 | Range disjoint from local accounts (§3.1) | 27 `passwd` entries, none in 200000–299999 | **PASS** |
| ID-2 | No transition skips `reclaiming` or `quarantined` | `allocated→free` and `allocated→quarantined` rejected | **PASS** |
| ID-3 | CAS rejects a stale sequence | rejected with the observed vs. actual tail named | **PASS** |
| ID-4 | Second allocation of an in-use UID fails closed | rejected → `identity.double_allocation_detected` | **PASS** |
| ID-5 | One launch record bound to two UIDs fails closed | `UNIQUE` violation from the partial index | **PASS** |
| ID-6 | Full cycle `free→…→free→allocated`; reuse gets a new record ID | seq 1–7 monotonic; new `record_id` on reuse | **PASS** |
| ID-7 | Append-only enforced inside the store | `UPDATE`/`DELETE` raise `append-only` | **PASS** |
| ID-8 | Tamper-evident chain | in-place edit of seq 1 (after dropping the trigger) → `hash mismatch at seq 1` | **PASS** |
| ID-9 | Concurrent allocators, exactly one winner per UID | 8 processes racing for the same 50 UIDs: 50 records over 50 distinct UIDs; losers saw CAS errors only; chain verified. (The lock holder won all 50 because 50 commits take ≈20 ms — the property tested is "never two records", which held.) | **PASS** |
| ID-10 | Crash consistency: writer `SIGKILL`ed mid-stream | 150 rounds, kill after 1–37 acknowledged commits; all 2 815 acknowledged transitions present after reopen; chain verified every round; 0 anomalies | **PASS** |
| ID-11 | Recovery: orphaned `allocated`/`in-use` identities go to `reclaiming`, never `free` | 331 orphans; `→free` rejected 331×, `→reclaiming` appended 331× | **PASS** (§4 crash rule) |
| ID-12 | Session UID cannot read the store | 0700 store dir: open as uid 200042 fails; owner reads | **PASS** |
| — | Allocation latency (durable commit, `synchronous=FULL`) | ≈ 400 µs per transition | measured |

## Disposition of ID-1

The candidate design (single-writer SQLite WAL, hash-chained append-only table, CAS on `(record_id, seq)`) satisfies every §3.2/§3.3/§4 obligation tested and is crash-consistent under process death. **Identity lifecycle §3 is not reopened.** Two scope statements:

- **Power loss not modelled.** The crash model is `SIGKILL` of the daemon; durability across power loss rests on SQLite's documented `synchronous=FULL` WAL behaviour, which the spike did not independently test. This should be stated as a residual assumption in the WP2 implementation, or a fault-injection test using a dm-flakey/`dm-log-writes` device added to the F-C/F-T set.
- **Compaction** (§3.2, separately authenticated maintenance) is not prototyped; the chain design supports it by recording a compaction record whose `prev_hash` is the last compacted hash.

One note for the implementer: `busy_timeout` makes losers wait for the lock and then fail CAS cleanly; with a larger fleet of concurrent `agentbound-launch` requests the daemon should serialise allocations in-process rather than rely on SQLite's lock, since the allocator "lives in the daemon" (§3.2) — the multi-process race here is a stress of the store, not the intended deployment shape.
