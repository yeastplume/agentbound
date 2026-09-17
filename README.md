# Agentbound

**An experimental Linux sandbox and restricted Git gateway for agent tasks.**

Agentbound gives each task a separate Unix user ID, filesystem view, process tree, and resource limits. Task processes have no direct network access. A gateway checks the calling process and allows only named operations; the useful operation implemented today submits Git changes to a staging ref rather than a protected branch.

The aim is to enforce a task's permissions outside the agent framework, even when the agent runs arbitrary shell commands. This is a security prototype, not an agent framework or a production sandbox.

## Where it stands

**Repair-pass update:** the development VM is now available. The prioritized launcher, no-gateway termination, audit-schema and signature-freshness changes build and pass focused tests. A live smoke test passes, but the full suite still exposes failures. Read the [repair-pass evidence](docs/evidence/repair-pass/README.md) before treating the earlier static findings as the current code status.

- **Built:** a Rust launcher, identity allocator and lifecycle service, file-backed policy service, audit receiver, operator CLI, Git gateway, and adversarial test tools.
- **Recorded demonstrations:** isolated shell workloads and Git staging on one Debian 13 development VM. No real coding-agent/model integration or comparative microVM evaluation yet.
- **Audit result:** ten retained repetitions report all **800 gateway operations** attributed. They attribute only **800 of 18,400 effects overall (4.35%)**: local file effects and individual process events are not collected. The raw audit records needed to independently recheck the successful matches are not retained with those results.
- **Validation gap:** the latest committed full-suite report predates the current code. Fresh-host reproduction and independently owned adversarial tests are not established.
- **Safety:** this review found a privileged-launcher configuration exposure, a no-gateway cleanup regression, and audit schema mismatches. These are static findings, not exploit demonstrations. Do not deploy for untrusted users or sensitive workloads.

**Recommendation: retain the prototype, pause feature expansion, and do a bounded repair-and-validation pass before deciding whether to continue.** See [the assessment and next decision](docs/STATUS.md) for evidence, priorities, and stop conditions.

## Start here

| Question | Read |
|---|---|
| What works, what is missing, and is it worth continuing? | [Status and assessment](docs/STATUS.md) |
| Where are the useful documents? | [Documentation guide](docs/README.md) |
| How are the components arranged? | [Launcher/lifecycle design](crates/DESIGN.md), [gateway design](crates/DESIGN-1B.md) — implementation notes, not proof of correctness |
| What is the intended security contract? | [Architecture specifications](docs/architecture/README.md) |
| What was actually measured? | [Audit-test results](docs/evidence/wp3.1/raw/d12/README.md), [historical full-suite output](docs/evidence/wp3.1/raw/run-07-negative-controls.md) |

The papers titled *Agents as Unix Principals* describe the broader proposal. They are not a list of shipped capabilities.

## Code and deployment

`crates/` contains the nine-crate Rust workspace and test tools. `deploy/` contains the sample catalogue, systemd units, and development-host provisioning. `spikes/` contains earlier mechanism experiments. `docs/evidence/` preserves development results, including failures and superseded claims.

For source-level tests on a Linux machine with a Rust toolchain:

```sh
cargo test --workspace --locked
```

This does **not** run the privileged conformance suite or prove isolation. The initial documentation review had no local Rust toolchain. The subsequent repair pass ran workspace tests and a release build on the development VM; those passed, but full conformance has not.

**Do not run the deployment helpers casually.** `crates/build.sh` syncs to a hard-coded remote VM as root. `deploy/provision.sh` creates users, installs sudo rules, copies binaries, and starts services. The negative-control harness mutates code and redeploys it. These are development-lab tools, not a portable installation procedure.
