# WP1 mechanism spikes

Throwaway prototypes that verify kernel and systemd assumptions on the pinned
baseline (VM 110 `agentbound-dev`: Linux 6.12.107+deb13, systemd 257.13). They
are **not** part of the reference implementation or the TCB and are not held to
the SLOC rules. Each spike is a standalone Cargo crate with no dependencies
beyond `libc`; each prints one `RESULT <item> PASS|FAIL <detail>` line per
required result so the evidence file can be checked mechanically.

`run.sh <spike>` syncs this tree to the VM, builds, runs, and writes the raw
transcript to `docs/evidence/wp1/raw/<spike>.txt`. The evidence register is
`docs/evidence/wp1/README.md`.
