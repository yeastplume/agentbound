# D-Bus scope observation — decision (WP2 carry-in)

**Carry-in.** WP2 recorded a deviation: scope creation uses `busctl call … StartTransientUnit`, but scope
observation uses cgroup files (`cgroup.procs`, `cgroup.events`, `cgroup.freeze`) and the init pidfd, not a
`PropertiesChanged` / `UnitRemoved` D-Bus subscription as the session-lifecycle document describes.

**Options weighed in WP3.**

| Option | Cost | Assessment |
|---|---|---|
| Native binding (`zbus` or hand-written D-Bus wire protocol) | `zbus`: ≈30 k transitive SLOC into the privileged lifecycle daemon (R-CON-8 budget is 6 000 direct; the dependency itself would be the largest attack surface in the TCB). Hand-written marshalling of `AddMatch` + signal parsing: ≈400–600 SLOC of privileged code for a *confirmation-only* signal | rejected |
| `busctl monitor` child process, parse `UnitRemoved` / `PropertiesChanged` lines | ≈60 SLOC; a long-lived root child of the privileged daemon whose stdout is an unbounded text stream parsed with string matching; systemd documents `busctl monitor` output as human-oriented, not stable | rejected: adds a parser of an unstable format to the TCB for information already available from the kernel |
| Keep kernel-side observation; record the deviation permanently | 0 SLOC | **selected** |

**Why the kernel-side path is sufficient (and stronger).** The property the lifecycle needs is "no process of
the session's scope exists", and the kernel is the authority for that: `cgroup.events populated=0` and the
init pidfd's `POLLIN` are direct observations; `UnitRemoved` is systemd's *later* report of the same fact (WP1
measured ≈1.5 s after inactivity). Every §5 step that depends on emptiness (`cgroup_procs_remaining`,
`init_pidfd_exited`, reclamation) reads the kernel, so a missed or delayed D-Bus signal cannot produce a
false "empty". The power-loss round (this WP) and the T-6.5 SIGKILL rows both exercised the recovery path
without D-Bus at all.

**What is given up.** Prompt notice of systemd-side state that does not change the cgroup — e.g. a scope
that systemd marks `failed` while processes are still alive. The lifecycle's periodic tick (`session.rs`
`tick()`) polls scope state through `busctl get-property … ActiveState` only when it already needs it (quiesce
deadline, retry), which bounds the observation gap to one tick.

**Record.** Session-lifecycle §3/§5 wording "observes the scope via D-Bus" is implemented as "observes the
scope via cgroup files and pidfd; D-Bus is used for creation and on-demand property reads". This is a
recorded, permanent deviation, not a residual to be closed; the owning document gets a revision entry at
the next unfreeze (WP0 set is frozen; no normative requirement is violated — R-ISO-4 speaks of scope
emptiness, not of the signalling channel).
