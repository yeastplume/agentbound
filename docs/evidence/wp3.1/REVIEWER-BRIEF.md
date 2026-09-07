# WP3.1 reviewer brief

**Repository:** `https://github.com/yeastplume/agentbound` (branch `main`)
**Head at time of writing:** `e464a40`
**Plan version:** 0.17 — [`docs/plans/phase-1-reference-implementation.md`](../plans/phase-1-reference-implementation.md) §WP3.1
**Full register:** [`docs/evidence/wp3.1/README.md`](README.md) (529 lines; this brief is the 10-minute version)

This brief is written for a reviewer who has not followed the work. It states what was claimed, what was corrected, what the
current claim is, and — most importantly — **where I think the remaining weaknesses are**. Everything asserted here is traceable
to a commit and a raw run register; nothing is a summary of intent.

---

## 1. Why WP3.1 exists

WP3 (milestone 1B: unprivileged gateway, Git staging-ref adapter, correlated audit) was closed at `bd7befc` with a register
claiming "139/139" conformance. An independent review found that number counted *runner assertions*, not the frozen catalogue
population: 29 catalogue IDs were never executed, and several rows that did run were passing for reasons unrelated to the property
they named. The status was corrected at `207930e` — implementation complete, **conformance exit not met, Gate 3 not evaluated** —
and WP3.1 was added as a hard gate before WP4.

**The reviewer's original finding was correct, and the follow-through found more.** That is the honest summary of this work
package.

## 2. What the current claim is

**Verdict: no-go to WP4; narrow milestone 1B.** Recorded in plan 0.17 and [item 8 of the register](README.md).

The conformance harness is now, I believe, sound: every frozen catalogue row executes, assertions are scoped to the run that made
them, and six negative controls establish that the assertions discriminate. **But one milestone-1B requirement fails, and it
cannot be repaired inside this work package.**

| | |
|---|---|
| Suite state | 182 assertions: 175 PASS, 3 WEAK, 4 RECORDED, **0 FAIL**. Catalogue coverage: **0 NOT-EXECUTED** over all **118** frozen 1A+1B ids; dups=0, extra=0 |
| Gate condition 1–4 | met (see §4) |
| Gate condition 5 | **not met** — no independent reproduction; see §6 |
| R-AUD-2 (1B) | **fails** — attribution completeness **3.5 %** vs required ≥ 99 % |
| R-CON-8 (privileged SLOC) | 1 205 of 6 000 (pinned `tokei 13.0.0-alpha.8`) — comfortable; the previously published figure was wrong, see §5 |

**Do not read "0 FAIL" as the exit condition.** It is not, and §3 is why.

## 3. The finding that governs the verdict

**R-AUD-2 requires reconstructing `initiator → agent → session → process → effect` across three effect classes at ≥ 99 %.
Measured: 3.5 %.**

The frozen test catalogue (§5) pre-registers the metric: 8 concurrent sessions × 230 atomic effects × 10 seeded repetitions, a
30 s correlation deadline, denied operations in scope, ground truth taken from an *instrumented workload log* rather than the
platform's own record. Three ontology classes: (a) local object create/modify, (b) process lifecycle, (c) gateway operations.

- Class (c) reconstructs at **100 %** of the finite corpus, idempotency-key matched, denials included.
- Classes (a) and (b) reconstruct at **0 %**. There is no ingestion path. No audit record names an individual file a session
  created or an individual fork/exec/exit.
- 220 of every 230 effects — **95.7 % of the metric's denominator** — are therefore unattributable.

**Why WP2 and WP3 both missed it:** D-12 was scored by a presence check that asserted certain event *kinds* appeared. Those kinds
were already being emitted for other reasons, so the row passed without ever measuring completeness. It is a design gap, not a
defect — nothing was built to collect (a) and (b) — and closing it (audit-netlink shipper, per-session rule lifecycle,
reconciliation with R-AUD-3's `lost` counter) is a work package, not a repair.

**Consequence for the repository's language:** "end-to-end audit" now means *gateway-mediated effects only*, and the top-level
README says so explicitly.

Two further open findings, both real, neither repaired:

- **Lifecycle serializes blocking work, not just decisions.** The pre-registered 8-session profile cannot be admitted — 2–3 of 8
  launches succeed. `agentbound-lifecycle` serves one request at a time and waits inside that serialization: a construction held
  it **17.4 s**, a termination **61 s**, one session took **123 s** to activate. Component-interfaces §3.6 requires it to decide,
  serialize and record *transitions*; serializing the *waiting* is an implementation choice. I judge this a §3.6 conformance
  defect and it bounds every concurrency claim the platform can make.
- **The in-session gateway protocol (`agentbound.gateway.v0.1`) is specified in no frozen document.** Its request shape, replay
  and conflict semantics exist only in code. That is precisely how it came to carry **no idempotency key at all** while
  `component-wire-formats` mandates one on every component request — a retried Git push would have executed twice. Fixed in this
  work package (`9867299`), but the specification gap remains.

## 4. Gate conditions, with evidence

The plan's exit conditions, judged literally:

| # | Condition | Verdict | Evidence |
|---|---|---|---|
| 1 | All mandatory 1B rows present | MET | 0 NOT-EXECUTED over all 118 frozen 1A+1B catalogue ids (reconciled against `test-catalogue.md` 0.7 directly, not against the manifest's own header); 182 assertions incl. 48 fault-injection rows |
| 2 | No claimed-enforced resource merely copied into the binding | MET | `installed_value` read back from the kernel or class recorded `absent` (`c0adb42`) |
| 3 | Budget enforcement survives restart | MET | `T-6.9-005.budget-persist`: op_count 31→36, restored to 36 after restart, then exhausted at the same cumulative figure |
| 4 | No missing fault point permits authority or identity reuse | MET | 48 F-C/F-T rows at every construction/termination step the catalogue names (`b87773c`) |
| 5 | Independent fresh-host run reproduces the result | **NOT MET** | Not performed; and see §6 |

Stop-or-narrow triggers: frozen semantics **not** broadly changed (three narrow additive revisions); SLOC **well** inside bound;
negative controls revealed **three** non-discriminating rows — not broad in count, but one was systemic in kind (§5).

## 5. What the negative controls found (the part I would review first)

A suite reporting 0 FAIL has shown its assertions *ran*, not that they would notice if the property were false.
[`crates/ab-conformance/negative-controls.py`](../../../crates/ab-conformance/negative-controls.py) removes one enforcement at a
time, rebuilds, redeploys, re-runs, and requires the assigned rows to FAIL — then restores the tree and verifies restoration by
digest. **Three of six controls initially failed**, each exposing a false positive that four consecutive green runs had not:

1. **`T-6.4-009` does not depend on the pidfs-inode comparison.** Deleting the comparison entirely left the row passing. The
   reason is legitimate: the gateway polls the peer pidfd, so the establisher's exit closes the connection before a recycled
   process can present itself. Only unit tests cover the comparison. **No conformance row exercises it** — worth a reviewer's
   attention, because ADR-0002 D2 rests on it.
2. **The credential-count check is unreachable.** With `SO_PASSCRED` the kernel delivers exactly one `SCM_CREDENTIALS` whether the
   sender attached zero, one or several (measured on 6.12.107). The check stays in the code; no row may claim to exercise it.
   ADR-0002 → **0.10** records this. It also exposed **`T-6.4-008` as a false positive**: it attacked as *host root*, so
   `establish` refused on `uid_mismatch` before reading a packet — its own recorded evidence had been saying "DENY host-root-peer"
   for runs. The plan's item 2 had required exactly this repair and it had not been done.
3. **`T-6.9-008` was passing on other runs' history.** It counted `budget_operations` denials over the whole accumulated audit
   log; with enforcement removed it still reported 1 098. **This is the systemic one.** Any assertion whose evidence is "a record
   of kind X exists" can pass on an accumulating log. I audited all ten remaining cumulative counters — all sound (before/after
   deltas or allocation-scoped) — and the rule is now a standing obligation.

**A correction to my own earlier reporting.** Round 4 of this register published "direct privileged SLOC 2 417". That used the
wrong scope: R-CON-8 bounds `agentbound-launch` + `agentbound-lifecycle` + the gateway *authentication path*, but I reported
launch + lifecycle + all of `ab-common` and omitted the auth path, and did not use the pinned tool. Correct figures, pinned
`tokei 13.0.0-alpha.8`: **direct privileged 1 205** (launch 428, lifecycle 763, auth path 14); `ab-common` 1 011 reported
separately; gateway core 387. The bound was never close, so no decision rested on it — but the number was wrong and is corrected
rather than restated.

## 6. What I cannot establish about my own work

**Item 7's second half is structurally unavailable to me.** It requires "adversarial assertions owned and reviewed by someone
other than the implementation author." I wrote every assertion in this suite, including every negative control. A fresh-host run
would show the result is not host-specific; it would **not** supply independent ownership, and provisioning a second VM must not
be recorded as if it had. **This condition can only be closed by a reviewer who is not me.**

Beyond that, the specific things I would attack if I were reviewing:

- **Host-side oracles.** 25 rows use root joining the session's scope cgroup and then `setpriv`/`nsenter` as the session uid to
  play the "in-scope peer". That is a *stronger* adversary than a real session in some respects and a *weaker* one in others
  (it starts from root). Whether it is the right oracle for each row is a judgement call I made and would like challenged.
- **`T-6.4-009` deliberately grants the adversary CAP_SYS_ADMIN** to recycle a pid. Reasonable for the mechanism under test;
  worth confirming the row's conclusion is scoped to that.
- **Fixture rows** (7, marked `FIXTURE`) are setup/marker rows excluded from verdict counts. Confirm the exclusion is honest and
  that none carries a load-bearing assertion.
- **The remaining WEAK/RECORDED rows** (§7) are residuals I judged not achievable at 1B. Each is a place where I decided *not* to
  claim something; those decisions deserve scrutiny at least as much as the failures.
- **The freshness bounds** `MANIFEST_MAX_AGE_S = 600` / `BINDING_MAX_AGE_S = 60` and the cross-daemon bounds
  (`CROSS_DAEMON_MS = 60 s`, `BUDGET_PERSIST_MS = 2 s`) are **implementation choices absent from the frozen docs**. The asymmetry
  of the last two is load-bearing: equal bounds only *truncate* the daemon deadlock (measured 61 128 ms) rather than break it
  (2 124 ms after the fix). This reasoning should be in a spec and is not.

## 7. Residual non-PASS rows

3 WEAK — `T-6.9-006` (cooperative fan-out only), `T-6.4-012` (no TLS upstream in the harness), **`D-12`** (presence check; the
real metric fails, §3).
4 RECORDED — `D-02.1B`/`T-6.1-003.1B` (no PTY projected), `T-6.2-008.1B` (loader inventory), `D-15.1B` (no delegation operation
exists — deliberately not added to make a row pass).
1 permanent deviation — D-Bus, recorded as such.

## 8. Open work in WP3.1

- **Ten seeded suite repetitions** (item 5, second half) — not run. One transient is already known (`T-6.5-009` observed a stray
  `in-use` allocator state once), so flakiness exists and is not characterised.
- **Item 7 fresh-host reproduction** — not performed.

## 9. Recommendations put to the plan

1. **Re-scope R-AUD-2**: either move it behind a work package that builds class (a)/(b) ingestion, or restate it as
   gateway-operation attribution only — which *is* demonstrated at 100 %. It should not remain a 1B exit claim as written.
2. **Treat lifecycle's serialization of blocking work as a §3.6 conformance defect.**
3. **Specify the in-session gateway protocol**, including the now-required idempotency key and replay/conflict semantics.
4. **Adopt the negative-control harness as a release gate** — a run should not be reportable unless its controls have been
   re-established against the build under test.

## 10. Where to look

| What | Where |
|---|---|
| Full verdict and per-item narrative | [`docs/evidence/wp3.1/README.md`](README.md) |
| Raw machine registers, one per run | [`docs/evidence/wp3.1/raw/`](raw/) (`run-01` … `run-07`) |
| Negative-control harness | `crates/ab-conformance/negative-controls.py` |
| D-12 measurement harness | `crates/ab-conformance/probe/d12-{worker.sh,correlate.py,run.py}` |
| Conformance runner and all rows | `crates/ab-conformance/src/main.rs` |
| Plan, WP3.1 scope and go/no-go | [`docs/plans/phase-1-reference-implementation.md`](../plans/phase-1-reference-implementation.md) §WP3.1 |
| Frozen specifications | [`docs/architecture/README.md`](../architecture/README.md) (freeze record and versions) |

**One correctness note on the harness manifest.** `crates/ab-conformance/expected-ids.txt` is the checked-in expected-ID manifest
that item 1 added, and its header claimed "118 ids" while the file listed 121 entries — a comment that went stale as WP3.1 added
rows, citing a generator script that does not exist. I found this while checking the numbers for this brief. The manifest itself is
**correct**: reconciled directly against `test-catalogue.md` 0.7, all **118** frozen 1A/1B ids are present, and the 3 extra entries are
ADR-0002 Decision rows (`D4`, `D7-8`, `D7-9`) rather than catalogue ids. The four catalogue ids absent from it (`D-14`, `D-17`,
`T-6.4-015`, `T-6.8-010`) are all 1C/1D and correctly out of scope. The header now states this and gives the two commands that
verify it, because "the manifest is complete" is exactly the sort of claim that should not rest on a hand-maintained comment.

**Reproduction:** Debian 13, kernel 6.12.107, systemd 257; provision from `deploy/` only; `crates/build.sh build --release`;
run `ab-conformance` on the host as root. A full run is 20–40 minutes and writes its register to `conformance-run.md`.
