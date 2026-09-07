# D-12 raw evidence — ten seeded repetitions of the test-catalogue §5 NOMINAL profile

Produced by `d12-all.sh` on VM 110 (Debian 13, kernel 6.12.107, systemd 257.13) against binaries reporting
`commit=d44254e7f1f2ea65db57a4f9ffc9fcdb345379e7 dirty=false`, i.e. attributable to a reviewed commit. This is the evidence the
independent WP3.1 validation asked for: not a number quoted in prose, but every input of the measurement retained beside its result.

## Result

| | |
|---|---|
| Valid repetitions | **10 of 10** |
| Gateway-operation corpus | **800 / 800 = 100.0 %** — the 1B bar (phase-1-requirements 0.11, catalogue 0.8) |
| Whole ontology | **800 / 18 400 = 4.35 %** — reported; owed by D-12.full at 1C |
| Denominator per repetition | exactly **1 840** = 8 sessions × 230 effects, verified per repetition rather than assumed |
| Local-object class | 0 / 16 000 — `no platform record names this effect at all` |
| Process-lifecycle class | 0 / 1 600 — `no platform record names this effect at all` |
| Audit records lost at correlation | 0 in every repetition |

Every repetition returned the identical figure across ten different seeds, which is the point of the repetition count: the 4.35 % is
not variance, it is two of three effect classes having no ingestion path at all.

## What each file is

`rep-<n>.json` — the machine-readable result the `D-12` conformance row *computes* its verdict from. It carries the seed, the
`profile` block (the frozen parameters the repetition was run at), the `timing` block (launch wall time, workload wall time, end-marker
and correlator wall clocks, and the host-global audit `lost` counter observed at correlation time), per-class `C`/`G` with a
`missing` breakdown naming *why* each unreconstructed effect failed, `sessions_incomplete`, `launch_errors`, and `valid`.

`valid` is the honesty gate: it is true only when all 8 sessions launched, every one declared its end marker inside the 300 s window,
and `G` equals the profile's expected 1 840. A repetition that did not realise the profile is **retained as invalid and never
scored** — `d12-all.sh` refuses to overwrite an existing result, so an inconvenient attempt cannot be quietly re-run in place.

`rep-<n>.log` — the driver's own stdout for that repetition.

`d12-rep-<n>-artifacts/` — the raw inputs:

- `d12-gt-<uid>.jsonl` — the **ground truth**, written by the workload itself (`d12-worker.sh`) inside the session, one line per
  atomic effect with its class, `effect_id`, idempotency key and sequence. §5 requires the denominator to come from an instrumented
  workload log rather than from the platform's own records, so that a platform that fails to record an effect cannot also shrink the
  denominator it is measured against. 231 lines per session: 230 effects plus the end marker.
- `d12-launch-<i>.json` — the full authorization envelope and launch reply for each of the 8 sessions.
- `d12-manifest.json` — the correlator's per-effect decision: which platform record was matched to which ground-truth effect, and for
  every unmatched one, the reason.

## Reproducing

```sh
# on the host, as root, with no other sessions live
agentbound terminate <lrd>            # for any leftover manual session
rm -rf /var/lib/agentbound/evidence/d12
d12-all.sh                            # ~70 min: 10 × (8 launches + 300 s window + 30 s correlation deadline)
ab-conformance                        # the D-12 row reads the files above and computes its own verdict
```

The seed for repetition *n* is `SHA-256("D-12" ‖ n)` truncated to 16 hex characters and is recorded in each result, so the workload's
random choices are reproducible.

## What a reviewer should attack first

1. **The correlator could be matching too generously.** Read `d12-manifest.json` against a `d12-gt-*.jsonl`: a match requires class,
   outcome **and** idempotency key to agree. The 2 denied gateway operations per session are in scope and are matched as denials — a
   correlator that ignored outcome would score them as successes.
2. **The gateway corpus could be 100 % because it is small.** It is 10 effects per session, 80 per repetition, 800 in total, and it is
   finite by construction (the §5 corpus). That is exactly why the frozen threshold for it is 100 % rather than 99 %.
3. **The 0 % classes could be a harness failure rather than a platform gap.** The ground-truth logs prove the effects happened (200
   files created and modified, 20 fork/exec/exit per session). Then search the audit store for any record naming one of those
   `effect_id`s, or naming an individual file, or an individual fork: there are none. That is the finding.
4. **The profile could be unfaithful in a way the table above hides.** `profile` in each result states the parameters used; compare
   them against test-catalogue §5 directly. The earlier harness was unfaithful on six counts, all listed in the register's item 5.
