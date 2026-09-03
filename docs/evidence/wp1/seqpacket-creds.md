# WP1 evidence — `seqpacket-creds`

**Covers:** ADR-0002 Decision 7 items 1–3 (`SOCK_SEQPACKET` + `SO_PASSCRED`; pidfd from credential PID; descriptor transfer).
**Baseline:** VM 110 `agentbound-dev`, Linux `6.12.107+deb13-cloud-amd64`, systemd `257.13-1~deb13u1`, image digest recorded in the home-server runbook.
**Spike:** `spikes/seqpacket-creds/` (Rust, `libc` only). **Raw transcript:** `raw/seqpacket-creds.txt`.
**Command:** `spikes/run.sh seqpacket-creds` (syncs the tree, `cargo build --release`, runs as root on the VM).

## Results

| ID | Required result (Decision 7) | Observed | Result |
|---|---|---|---|
| D7-1a | Exactly one `SCM_CREDENTIALS` per `recvmsg` for one `sendmsg` | 1 control message, type `SCM_CREDENTIALS`, PID equals `SO_PEERCRED` PID | **PASS** |
| D7-1b | Second packet carries its own credential | second `recvmsg` returned the second payload with exactly one `SCM_CREDENTIALS` | **PASS** |
| D7-1c | Oversize packets truncate rather than split | 8192-byte packet into a 1024-byte buffer: `n=1024`, `MSG_TRUNC` set; next `recvmsg` returned the following packet intact (remainder discarded, boundary preserved) | **PASS** |
| D7-2a | `pidfd_open` succeeds for the live peer | `pidfd_open(cred.pid)` → fd, pidfs inode readable via `fstat` | **PASS** |
| D7-2b | Start time and PID-namespace reads succeed via pidfd | PID namespace via `ioctl(pidfd, PIDFD_GET_PID_NAMESPACE, 0)` equals `/proc/<pid>/ns/pid` inode; start time read from `/proc/<pid>/stat` then pidfd liveness re-checked (race-free ordering: pidfd first, `/proc` read, liveness recheck) | **PASS** |
| D7-2c | Reuse of a recycled PID is detected | peer exited; `ns_last_pid` forced so a new process received the *same PID*; held pidfd reports exit (`poll` readable; `pidfd_send_signal` → `ESRCH`); new process has a different pidfs inode (1658 → 1659) | **PASS** |
| D7-3a | `SCM_RIGHTS` rejected | packet carrying `SCM_RIGHTS` arrives with 2 control messages (`SCM_RIGHTS` + kernel `SCM_CREDENTIALS`); receiver detects it, closes the received fds and the connection | **PASS** |
| D7-3b | Inherited descriptor fails the establishing-PID check and closes the connection | parent's packet PID = establishing PID; forked child's packet on the inherited descriptor carries the child's PID (kernel-supplied, differs); gateway closed the connection; child's next `sendmsg` → `EPIPE` | **PASS** |
| X-1 (extra) | Unprivileged sender cannot forge `SCM_CREDENTIALS` | after `setuid(65534)`, `sendmsg` with `SCM_CREDENTIALS{pid:1,uid:0,gid:0}` → `EPERM`; the kernel-attached credential on the next packet shows the real PID and uid 65534 | **PASS** |

## Finding F-1 — start time is not a sufficient PID-reuse key (flags ADR-0002 Decision 2)

Extra check D7-2d: with the PID recycled inside one scheduler tick, the recycled process's `/proc/<pid>/stat` start time (`starttime`, field 22, in clock ticks; `CLK_TCK=100` → 10 ms) was **identical** to the exited peer's (44598 = 44598). Start time therefore cannot distinguish two process instances that share a PID and were created within 10 ms of each other. The pidfs inode (`fstat(pidfd).st_ino`, unique per process instance on 6.9+) did distinguish them, and the held pidfd itself reported the exit.

**Consequence for the design.** ADR-0002 Decision 2 binds `(pidfd, start time, UID, GID, PID namespace, session scope, boot ID)` at establishment and requires each packet's credential PID to "resolve to the same pidfd/start time". The mechanism is sound *because the gateway holds the pidfd*, but the text names start time as the comparison key. Recommended amendment, for the ADR owner: the per-packet check MUST compare the pidfs inode of `pidfd_open(cred.pid)` with the establishment pidfd's inode (or use `pidfd_send_signal(pidfd, 0)`/`poll` on the held pidfd for liveness); start time is corroborating, not authoritative. This is a key-selection clarification within the accepted mechanism, not a topology or mechanism change. **Decision 7 item 2 is PASS as written; the amendment is recorded in the WP1 register as an ADR-0002 revision to make before WP2.**

Also confirmed: `PIDFD_GET_PID_NAMESPACE` requires the ioctl argument to be 0 (`EINVAL` otherwise) — an implementation note for the gateway, not a finding.

## Not covered here

Decision 7 items 4–9 (abstract-socket isolation, revocation latency, bypass corpus, TCB accounting, failure behaviour, diagnostics) are separate spikes or WP2+ items; see the register.
