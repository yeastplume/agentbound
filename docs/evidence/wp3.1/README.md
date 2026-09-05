# WP3.1 evidence register — conformance correction and independent test (in progress)

Hard gate before WP4 (plan 0.16 §WP3.1). This register is written incrementally; each round adds a raw run under `raw/` and updates the tables. Nothing here is a pass claim until the go/no-go section at the end is filled in.

## Round 1 — harness integrity

**Change.** `ab-conformance` now checks itself against the frozen catalogue: `crates/ab-conformance/expected-ids.txt` is generated from `docs/architecture/test-catalogue.md` 0.7 by `crates/ab-conformance/tools/gen-expected-ids.py` (every row whose Milestone column includes 1A or 1B — 118 ids — plus the three ADR-0002 rows D4, D7-8, D7-9 that the WP3 register relied on). Runner rows map to a catalogue id by prefix (`T-6.4-003.only` → `T-6.4-003`; unit-tested). The run **fails** (exit 1) if any catalogue id has no PASS row, if any row id is duplicated, if any non-fixture row maps to an id outside the catalogue, or if any assertion fails. Verdict classes are now separate: `PASS`, `WEAK` (assertion true but weaker than the row's intent), `RECORDED` (1A partial/N-A re-asserted under 1B), `FAIL`, `FIXTURE` (setup/marker — excluded from every count). Machine output records run id, repository commit, SHA-256 (16 hex) of every installed binary and in-image script, and the catalogue coverage table. WP1 `GS-*` ids used as row names were re-homed to their catalogue rows (T-6.3-002, T-6.4-011, T-6.4-012); the fixtures `PROBE-COMPLETE`, `GW-COMPLETE`, `GW-HELD`, `T-6.8-setup`, `T-6.2-005` (orphan spawn; asserted by D-07), `D-10.bundle` are now `FIXTURE`.

**Result** ([raw/run-01-harness-integrity.md](raw/run-01-harness-integrity.md), commit `207930e` binaries): 126 PASS, 3 WEAK, 4 RECORDED, 0 FAIL assertions; **catalogue coverage 85/121 PASS, 2 WEAK, 4 RECORDED, 30 NOT-EXECUTED; run verdict FAIL.** This is the honest baseline the WP3 register should have reported.

**Not executed (30), grouped by what is needed:**

| Group | Catalogue ids | Note |
|---|---|---|
| 1B rows named by the review | D-16, T-6.3-005, T-6.3-008, T-6.8-008, T-6.8-009, T-6.9-008, F-C-08, F-T-01, F-T-06, F-T-07, F-T-09 | WP3.1 item 4 |
| 1A rows that WP2 recorded from prose or never ran | D-03, D-05, T-6.1-006, T-6.1-008, T-6.1-010, T-6.1-011, T-6.1-012, T-6.5-005, T-6.5-008, T-6.6-007, T-6.7-001, T-6.9-003 | The WP2 register's "84/84" had the same defect as WP3's: rows written up without a machine row. To be implemented or recorded not-executed with reason; WP2 register to be annotated |
| Constructor fault points not injected | F-C-01, F-C-02, F-C-04, F-C-05, F-C-06 | WP2 injected F-C-03/07/09 only |
| Termination fault points not injected | F-T-05 | with F-T-06/07/09 above |

**Still to do under item 1:** per-run scoping of every assertion that reads cumulative VM state (audit greps not keyed on this run's launch records; `process_mismatch` corpus in T-6.4-009), exact `class`+`rule` on every denial, seed recording. These are folded into the false-positive repairs of item 2 because they are the same rows.
