# D-12 raw evidence — ten seeded repetitions of the finite audit workload

> **Evidence limits:** result files report the figures below; this is not a fresh independent reproduction. Workload logs support the denominator, but platform audit events and per-effect matched chains are not retained here, so the successful-match numerator cannot be independently recalculated from this bundle. Recorded workload durations are about 97–99 seconds; the driver treats 300 seconds as a completion window, not a sustained-load duration. See [the assessment](../../../../STATUS.md).

The original run record names VM 110 (Debian 13, kernel 6.12.107, systemd 257.13) and binaries reporting
`commit=d44254e7f1f2ea65db57a4f9ffc9fcdb345379e7 dirty=false`. That provenance is recorded here in prose, not attached to each result with installed binary digests. The bundle retains workload logs, launch replies, session manifests and result summaries; it does not retain every input needed to repeat the correlation.

## Result

| | |
|---|---|
| Repetitions marked valid by the driver | **10 of 10** |
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
`profile` block (declared profile parameters), the `timing` block (launch wall time, workload wall time, end-marker
and correlator wall clocks, and the host-global audit `lost` counter observed at correlation time), per-class `C`/`G` with a
`missing` breakdown naming *why* each unreconstructed effect failed, `sessions_incomplete`, `launch_errors`, and `valid`.

The driver's `valid` flag checks that all 8 sessions launched, every one declared its end marker inside the 300 s window,
and `G` equals the expected 1 840. A repetition failing these completion checks is retained as invalid and not scored. These checks do not independently validate every catalogue-profile obligation. `d12-all.sh` refuses to overwrite an existing result; operators must also retain failed attempts when preparing later runs.

`rep-<n>.log` — the driver's own stdout for that repetition.

`d12-rep-<n>-artifacts/` — the raw inputs:

- `d12-gt-<uid>.jsonl` — the **ground truth**, written by the workload itself (`d12-worker.sh`) inside the session, one line per
  atomic effect with its class, `effect_id`, idempotency key and sequence. §5 requires the denominator to come from an instrumented
  workload log rather than from the platform's own records, so that a platform that fails to record an effect cannot also shrink the
  denominator it is measured against. 231 lines per session: 230 effects plus the end marker.
- `d12-launch-<i>.json` — the full authorization envelope and launch reply for each of the 8 sessions.
- `d12-manifest.json` — the correlator's session list: authorization and launch-record IDs, UID, session ID, ground-truth path and end-marker time. It is **not** a per-effect decision map. The underlying platform audit events and successful-match chains are not included.

## Reproducing

This requires an explicitly authorized disposable deployment, not an ordinary checkout. The [driver](../../../../../crates/ab-conformance/probe/d12-all.sh) launches sessions and writes host evidence; the conformance suite also changes live host state. Review the scripts and provisioning first.

Archive existing evidence before preparing a new run; do not delete failed or inconvenient attempts. Record the candidate source and installed binary digests, run the D-12 driver and full suite as separate tests, and retain the platform audit records as well as the files listed above. A repeat run on the development VM is not independent fresh-host validation.

The seed for repetition *n* is `SHA-256("D-12" ‖ n)` truncated to 16 hex characters and is recorded in each result, so the workload's
random choices are reproducible.

## What a reviewer should attack first

1. **The correlator could be matching too generously.** Review the correlator code and retain platform records plus per-effect match decisions in the next evidence bundle. The existing session manifest cannot establish those matches. Both denied operations per session belong in the corpus and must match as denials, not successes.
2. **The gateway corpus could be 100 % because it is small.** It is 10 effects per session, 80 per repetition, 800 in total, and it is
   finite by construction (the §5 corpus). That is exactly why the frozen threshold for it is 100 % rather than 99 %.
3. **The 0 % classes could be a harness failure rather than a platform gap.** Workload logs record 200 local-object effects and 20 process-lifecycle effects per session. The correlator reports no matching platform records; the implementation lacks those collection paths. Independently rechecking the reported matches or absences requires retained platform audit records.
4. **The profile could be unfaithful in a way the table above hides.** `profile` in each result states the parameters used; compare
   them against test-catalogue §5 directly. The earlier harness was unfaithful on six counts, all listed in the register's item 5.
