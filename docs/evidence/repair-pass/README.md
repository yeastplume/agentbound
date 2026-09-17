# Development VM repair pass

In progress. This pass follows the assessment of source `3b038cb`. The user authorized use of the existing development VM, `agentbound-dev` at `10.20.44.12`.

This is same-host implementation validation, not independent or fresh-host validation. Existing documents from the assessment remain uncommitted; candidate source changes will be identified by content digests as well as base commit.

## Preservation

Before deployment changes, preserve the installed build provenance, current session state, and existing VM-only full-suite reports. Keep a stopped-service backup of configuration, persistent state, installed binaries and existing build sources on the VM under `/root/agentbound-repair-backup-20260917/`. This backup includes secrets and is not copied into the repository.

Initial inspection: services active; no session-UID processes in the process listing; many sessions `termination-incomplete` or `terminated`; lifecycle spool approximately 5 GB and SQLite database approximately 3.5 GB. Installed binaries report source `584b6657c452b9fdc78b02d1c8af50811458f4d4`, `dirty=false`.

Backup completed before changes: `state.tgz` SHA-256 `d2286956c19da727b50a64c3f3b8bb8230931809832abf0beea310683fc6cb22` (about 1 GB compressed). A newer [VM-only baseline report](baseline-vm-conformance.md) was recovered: 7 September, source `584b665`, 160 PASS / 2 WEAK / 4 RECORDED / **18 FAIL** assertions. This supersedes the assumption that the last available report was the committed 6 September run; it does not validate source `3b038cb`.

## Reproduced findings and candidate checks

- [Unmodified source unit tests](baseline-unit-tests.log) pass. They did not cover the reported defects.
- [Old launcher regression](baseline-launcher-regression.log): `alice` invoked the sudo launcher with a caller-chosen audit path. It created a root-owned 0600 marker under the root-only backup directory before rejecting a nonexistent authorization. This demonstrates privileged file mutation, not a full root-shell exploit.
- [Old freshness regression](baseline-freshness-regression.log): a new assertion fails against old code because changing `issued_at` refreshed the stale signature.
- [Candidate build and tests](candidate-build-tests.log): workspace tests and release build passed. [Lifecycle follow-up](candidate-lifecycle-followup.log) also passed. The candidate is deliberately labelled `dirty=true`; it is not a clean release commit.
- [Live smoke test](candidate-smoke-segment2.log): forbidden sudo configuration/environment/fault arguments refused; launcher pre-I/O guard refused even with `SUDO_UID=0`; a normal operator launched a no-gateway task, terminated it to `cleaned/sealed`, and retrieved its cleanup and seal events from the central receiver.
- [Targeted negative controls](unit-controls/results.json): removing each of the sudo argument guard, no-gateway waiver, and required cleanup-schema member caused its assigned test to fail. These controls ran in a temporary source copy and **never deployed mutants**. They do not replace the full live negative-control campaign.

## Repairs

The launcher now validates privilege, arguments, authorization path components and environment before configuration I/O. Provisioning restricts the actual sudo command arguments and disables environment overrides. Root fault injection remains available directly to the root test driver, not through the operator CLI's sudo path.

Termination waives gateway confirmation only for the explicit no-gateway topology. Gateway sessions still need an explicit closed-admission acknowledgement. Cleanup events and two gateway diagnostic events now match the receiver's closed schema; construction-failure audit payloads use null for unreported diagnostics rather than claiming successful rollback.

Signature transcripts now authenticate the object and envelope metadata under role-separated v0.2 domains. This is a **compatibility break**, documented in manifest-schema 0.9 and component-interfaces 0.6. All relevant components were rebuilt together; pending old handoffs require reauthorization. Historical records were not rewritten. New tests cover timestamp tampering, metadata tampering and legacy-signature rejection.

## Operational failures retained, not hidden

The first candidate deployment attempt stopped after discovering `ab-gwclient` has no `--provenance` option; [the log](candidate-deploy.log) is retained. The corrected deployment completed ([retry log](candidate-deploy-retry.log)).

The first smoke launch failed because lifecycle was still replaying the accumulated store ([log](candidate-smoke.log)). The next failed because the audit receiver had reached exactly **1,000,000 events**, its configured capacity ([log](candidate-smoke-ready.log), [head and loss counter](exhausted-audit-head.json)). Admission was refused rather than bypassing the audit reservation.

All services were stopped and the exhausted audit segment moved, intact, to `/root/agentbound-repair-backup-20260917/exhausted-events.jsonl`. Its SHA-256 is `936034c4754409c91c7c74faff1cc59f876dbd8f598e484098d3ac5e2815c180`. A new empty audit segment was started for this validation. This is an explicit segment boundary, **not** uninterrupted loss-free audit history; the old counter recorded 110 lost events since the receiver restart. The identity database, keys and catalogues were not reset. [Transition log](audit-segment-transition.log) records lifecycle readiness taking another 52 seconds.

## Full-suite result: failed

The [candidate report](candidate-conformance-report.md) records **107 PASS, 1 WEAK, 2 RECORDED and 42 FAIL assertions**, with **12 expected IDs not executed**. The [console log](candidate-conformance.log) retains failures without clipping. These counts are not a like-for-like regression score against the older 18-failure run: the workload fixtures, recovered host state, audit segment and startup timing differ. They are a failed candidate-validation result.

Concrete findings from this run:

1. The saturated no-gateway probe reached an empty cgroup and an exited pidfd, but the credential scan still saw its init PID. Termination held the identity. This is consistent with unreaped process bookkeeping, but the run did not retain `State`/`PPid` for that instance, so a zombie diagnosis is not established. The simple no-gateway smoke case passes; full saturated teardown does not.
2. Lifecycle restart took longer than the suite's recovery wait. Dependent tests proceeded without valid sessions, producing missing PIDs and cascading launch failures. [Recovery diagnostics](conformance-recovery-diagnostic.log) show the daemon still consuming CPU during recovery. A systemd `active` state is not readiness.
3. D-12 rejected all ten **old** result files with `NonIntegerNumber`; the current strict JSON reader cannot consume that older evidence format. The old files were not rewritten to make this pass. No fresh D-12 corpus was produced for this candidate.
4. The gateway-unavailable row expects the old misleading admission text. The implementation now reports `closure-unconfirmed` rather than `denied-no-gateway` when a gateway session cannot confirm closure. The test needs a reviewed semantic assertion, not just a replacement string.
5. Fault/retry and gateway-budget rows remain failed or unevaluated. The driver also reports a failed setup marker (`GW-COMPLETE`) as outside the catalogue. A failed fixture should stop dependent tests rather than produce plausible assertions from empty setup values.
6. Dirty-source provenance independently prevents a release PASS, by design. That is additional to the real assertion and coverage failures, not their explanation.

No timeout was relaxed, state table reset, or required verdict weakened to hide these outcomes. Full lifecycle/gateway conformance remains unestablished.

## Evidence identity and remaining work

[Source digests](candidate-source-sha256.json) and [VM comparison](candidate-source-verification.log) identify the candidate's code/configuration content independently of its dirty base-commit label. The deployment/transition logs record installed binary hashes. Later test-only helpers may be recorded separately; a matching base commit alone is insufficient provenance.

The [retained candidate audit segment](candidate-audit-events.jsonl) contains 1,426 records through the full-suite end. Its sequence and hash chain were independently recalculated from the JSON records; the head matches `sha256:e7cfe96d66ea81de0c002b059d635012f1ba357b3a397b2af8afaeb2c730d4ab` in [post-suite status](post-suite-state.log). This validates internal chain consistency, not every event's semantic correctness or full attribution.

A [post-suite smoke rerun](post-suite-smoke.log) again passed all three focused checks. All four services remained active and the post-suite session list contained only sealed sessions. No identity-database reset was used.

Still required: investigate the full-suite failures, review lifecycle readiness/credential-scan behaviour, and complete validation on a clean pinned candidate. The existing full live negative-control harness requires clean committed sources and has not been bypassed. Fresh-host reproduction, independently owned tests and complete file/process telemetry remain out of scope for this same-host repair pass.
