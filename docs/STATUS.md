# Agentbound: status and assessment

Original assessment of source commit `3b038cb` and the evidence committed with it. The assessment below is retained with its source scope.

**17 September repair-pass update:** with the user's authorization, the development VM was used to reproduce defects, build repairs and run tests. The launcher exposure and unsigned-timestamp issue were reproduced; repairs to these, no-gateway termination and audit event contracts pass unit tests. A live no-gateway launch/terminate/seal and central-audit smoke test passes. The full-suite attempt still has failures, including recovery delays on the accumulated store. A VM-only baseline report was recovered showing 18 earlier failures. See [repair-pass results and retained attempts](evidence/repair-pass/README.md) for current evidence; no release, independent validation or full conformance is claimed.

## Bottom line

**There is useful engineering here, but not yet a usable, validated agent-security product.** Keep the code and experiments. Pause the broad roadmap until a small repair-and-validation effort establishes whether the prototype deserves more investment.

The useful idea is straightforward: run each task with its own OS identity and restricted environment; keep credentials outside that environment; permit external actions only through a gateway that can identify the calling process. The implementation goes substantially beyond a sketch. It also introduces privileged services, persistent identity management, and cross-service failure handling that need more scrutiny than the current test record provides.

The project has not yet answered its most important practical question: **can a real agent finish a useful task under these restrictions, with an advantage over a simpler sandbox plus restricted credentials?** More specification work alone will not answer it.

## What exists

| Area | Implemented today | Limit |
|---|---|---|
| Task setup | Signed authorization manifests; root launcher; per-session UID/GID; mount, PID, IPC, UTS and network namespaces; cgroup and other limits | Linux-specific, root-constructed reference deployment; reviewed code has unresolved defects below |
| Task teardown | Process-tree termination, identity records, cleanup checks, reuse quarantine, restart reconciliation | Complex failure/retry paths; a no-gateway regression is visible in current source |
| Policy | File-backed catalogue that derives and signs allowed task configurations | Test/development policy service, not organizational IAM integration |
| External actions | Unprivileged gateway checks peer/process identity and budgets; `git.push_staging` plus a diagnostic ping | No model/inference adapter or general network proxy; demo upstream is a local bare Git repository |
| Audit | Component events and gateway operation records sent to a hash-chained receiver | No collection of individual local file effects or process lifecycle effects; some current component events also fail receiver validation |
| User interface | CLI for request, status, termination and audit queries | No demonstrated real coding-agent integration or portable install procedure |
| Tests | Rust tests, root-only adversarial driver, fault injection, mutation controls, recorded mechanism experiments | Historical results are not a passing verdict on the current source |

Code entry points: [launcher](../crates/agentbound-launch/src/construct.rs), [lifecycle](../crates/agentbound-lifecycle/src/session.rs), [policy](../crates/agentbound-policy/src/main.rs), [gateway adapters](../crates/agentbound-gateway/src/adapters.rs), [CLI](../crates/agentbound/src/main.rs).

Not demonstrated: production operation, a real model-backed agent task, full file/process attribution, cross-host portability, or a comparative microVM result. Labelled memory, information-flow control and trusted release belong to the broader proposal, not the implemented baseline.

## What the evidence supports

“Recorded” below means present in this repository, not independently reproduced in this review.

| Evidence | Recorded result | What it does not establish |
|---|---|---|
| [Latest committed full-suite report](evidence/wp3.1/raw/run-07-negative-controls.md), 6 September 2026 | 175 PASS assertions, 3 WEAK, 4 RECORDED, 0 FAIL. Coverage: 115 PASS, 2 WEAK, 4 RECORDED across 121 expected IDs (118 catalogue IDs plus 3 architecture-decision rows) | All requirements passed, or current HEAD passed. The report labels the source `207930e`; later source changes have no matching committed full-suite report |
| [Ten audit-test repetitions](evidence/wp3.1/raw/d12/README.md), 7 September | Each reports 80/80 gateway effects attributed and 80/1,840 effects overall. Total: **800/800 gateway; 800/18,400 overall = 4.35%** | Full attribution, general reliability, or “4.35% secure.” This is an attribution metric for a finite workload |
| Same audit results, by missing class | 0/16,000 local-object effects; 0/1,600 process-lifecycle effects attributed | A zero audit-loss counter does not compensate for telemetry that was never collected |
| [Correction history](evidence/wp3.1/README.md) | Tests exposed missing coverage, false positives, budget-reset bugs and daemon liveness problems; mutation controls reportedly caught additional weak assertions | Independent assurance. The committed clean-suite report is not the underlying mutation-run evidence |
| Fresh-host and independent test ownership | Not established by the checked-in record | Ten repetitions on the development VM do not close either requirement |

### The audit scope changed; the old summaries did not

The older README and plan quoted 3.5% and said the audit requirement failed milestone 1B. The newer [requirements, R-AUD-2](architecture/phase-1-requirements.md) and [test catalogue](architecture/test-catalogue.md) split the obligation:

- **1B:** attribute 100% of the finite gateway-operation corpus and report the full-workload fraction.
- **1C:** reach at least 99% across file, process and gateway effects. The full-attribution Gate 3 condition remains.

The retained audit results report meeting the narrower gateway metric. They do not meet the original broad claim. Moving the missing collection work to 1C changes the schedule, not the implementation's capabilities. The plan's older no-go explanation needs reconciliation; this review does not declare a milestone passed or alter security requirements.

Ten **audit-test** repetitions are committed. Ten seeded **whole-suite** repetitions are a separate obligation, still unestablished. Avoid saying simply “the ten repetitions are done” or “not done.”

### Limits of the retained audit evidence

This review parsed all ten result files and all 80 workload logs: the 18,400-effect denominator, class counts and 80 end markers agree with the summaries. The reported successful matches cannot be independently reconstructed from those files: the platform audit events and per-effect matched chains were not included.

The files named `d12-manifest.json` contain session identities and paths, not correlation decisions. Build provenance is stated in the evidence README rather than attached to each result. The measured workloads lasted about 97–99 seconds; the catalogue's 300-second duration is treated by the driver as a completion window. This is not evidence of 300 seconds of sustained load.

## Findings to resolve before deployment

These are findings from reading current source. No exploit was executed and no privileged service was started. They are not an exhaustive security audit. Line references below refer to the assessed source commit.

### 1. The sudo launch path exposes privileged configuration — deployment blocker

[Provisioning](../deploy/provision.sh), lines 62–63, lets members of `agentbound` invoke the launcher as root without restricting its arguments. [The launcher](../crates/agentbound-launch/src/main.rs), lines 18–25, accepts caller-selected keyrings, signing keys, catalogues, policy UID, lifecycle socket, image base and output paths.

That lets a caller choose inputs that are supposed to be trusted configuration for privileged execution. It undermines the policy/launcher boundary if these users are not already trusted as root. Root-compromise impact requires reproduction; the configuration exposure itself is visible in the source.

**Required outcome:** unprivileged operators can request an authorized launch but cannot select trust roots, privileged paths, service endpoints or fault controls. Test through the actual sudo entry point, not only by invoking the default configuration as root.

### 2. Sessions without a gateway cannot finish normal termination — lifecycle regression

[Termination](../crates/agentbound-lifecycle/src/session.rs), lines 152–176, requires a successful gateway admission-closure response regardless of session topology. [The gateway](../crates/agentbound-gateway/src/main.rs), line 183, rejects a record it has never projected. A `none`-topology session therefore remains `termination-incomplete` on this path, even after its processes exit; its identity stays held. Cleanup later in the file already has a topology-aware exception, but this earlier check prevents reaching it.

**Required outcome:** reproduce and test teardown through `cleaned/sealed` for both no-gateway and gateway sessions, including gateway failures. Fixing the no-gateway case must not bypass admission closure for gateway sessions.

### 3. Audit producers and receiver disagree — evidence integrity defect

[Lifecycle cleanup](../crates/agentbound-lifecycle/src/session.rs), line 268, emits `acl_removal_failures`. [The receiver's closed schema](../crates/agentbound-audit/src/events.rs), lines 15 and 43–44, does not allow it, so that cleanup event is rejected. Other producer/schema mismatches warrant the same review.

[The audit sink](../crates/ab-common/src/audit.rs), lines 59–66, can keep a rejected event in a local spool without incrementing its `lost` counter. Local durability is not evidence that the central audit chain contains the event.

**Required outcome:** contract-test actual producer payloads against the receiver, and verify that required events reach the central store. Account separately for local-only, rejected, forwarded and irretrievably lost events.

### 4. Signature freshness is not bound to the signed content — security review item

[Envelope code](../crates/ab-common/src/envelope.rs), lines 20–26 and 41–48, signs the manifest/binding but checks `issued_at` from the surrounding, unsigned envelope. Changing that timestamp does not itself invalidate the signature. File ownership and persistent authorization state restrict who can introduce a modified handoff, so this is not a demonstrated arbitrary authorization forgery.

**Required outcome:** review the envelope contract and add tampering/replay tests. The freshness claim needs authenticated issuance data or another documented, enforced source of freshness.

## Is it worth continuing?

### Work worth retaining

- **The isolation and gateway composition.** Per-task identities, process-bound socket authentication, restricted Git publication, and credential separation form a concrete experiment.
- **Failure-path work.** UID reuse, cleanup, resource read-back, restart budgets, PID reuse and retry storms are real engineering problems; the repository contains code and observations about them.
- **The test corrections.** Discovering that a green assertion tested the wrong property is useful. Keep the failed runs and the tests that exposed them.

These are valuable as a research prototype and as components to evaluate. They are not yet evidence of a competitive product or a generally superior architecture.

### Reasons not to continue the present roadmap automatically

- The first real agent workload is still ahead, after substantial architecture and conformance work.
- Lifecycle, gateway and audit interactions are already costly to reason about. A small reported source-line count does not show that the trusted system is simple.
- Production adoption would require installation, upgrades, operational diagnostics, credential/IAM integration, recovery procedures and external security review in addition to the missing features.
- There is no comparative result showing which benefits require this custom system rather than an existing sandbox with a restricted publishing service.

**Recommendation:** continue only as a bounded experiment with an owner, time budget and stop decision agreed in advance. Do not spend the next increment on more broad papers, information-flow profiles or a larger feature list.

## Next decision: a small sequence, not another programme

1. **Repair and test the current trust boundaries.** Start with the four findings above. Have a reviewer who did not implement the affected paths own the regression assertions.
2. **Make one candidate build reproducible.** Use an explicitly authorized disposable host, remove hard-coded developer-machine assumptions, and record source/tree state, installed binary digests and environment. Retain raw audit inputs, failed attempts and mutation outputs.
3. **Validate that candidate.** Run the full suite and negative controls, distinguish mandatory passes from recorded absences, repeat the fault/bypass tests, and explain nondeterminism. A fresh-host run and independent review are separate deliverables. Do not call this complete based on the old green report.
4. **Choose the product claim before extending it.** If full file/process attribution is essential, estimate and approve the missing collection work. If the useful product is a restricted execution/Git service, document that narrower claim and reconcile the plan without presenting it as full conformance.
5. **Only after those decisions, try one real task.** An existing coding agent should edit a test repository and submit a staging change through controlled model and Git access. Compare setup effort, task success, cleanup, resource cost and audit usefulness with a simpler sandbox deployment. The required adapter/integration work is an experiment to approve, not something already present.

Continue beyond that only if an identified user needs the extra controls and the demonstration justifies their operational cost. Stop or extract the reusable components if useful work needs permissions that defeat the boundary, the repair effort exceeds the agreed budget, or the simpler deployment meets the same need.

## Documentation cleanup

Before this pass, the tracked Markdown contained roughly **134,000 whitespace-delimited words** across 50 files. Length alone is not the problem: status, requirements, aspirations and debugging history were repeatedly presented together as current truth.

This pass replaces the root introduction, adds this assessment and a [reading guide](README.md), and marks stale status entry points. Raw run outputs, security requirements and historical findings remain intact. It changes documentation, not implementation or milestone gates.

For future edits:

- Keep current status here. Link to it instead of copying a paragraph of work-package numbers into every document.
- Start with the user-visible capability and limitation. Put requirement IDs beside supporting evidence, not in the first sentence.
- Label each claim **implemented**, **recorded result**, **reproduced result**, or **proposed**. Name the build for a result.
- Preserve historical failures, but move development chronology out of the reader's main path.
- Keep specifications precise. Shorten summaries without weakening requirements or rewriting old test output.
- Finish propagating the audit milestone split through the plan in a reviewed scope change; do not hide it as an editorial correction.

### Checks performed in this review

Read the implementation and deployment paths cited above, cross-checked the latest requirements against evidence, parsed the ten audit results and workload logs, and checked Python source syntax without executing privileged tools. The catalogue generator and checked-in expected-ID file name the same 121 IDs, but the generator omits the newer required-verdict column; it is not safe to regenerate that file blindly.

Rust compilation/unit tests and all deployment/conformance tests were **not run**: no local Rust toolchain was available. No remote VM was contacted. The separate read-only reviews used for this assessment do not satisfy the project's independent runtime-validation requirement.
