# WP1 evidence — `frozen-peer`

**Covers:** open-question register item **LC-2** (frozen cgroup holding a `SOCK_SEQPACKET` connection open; session lifecycle §6 quiesce); ADR-0002 Decision 7 item 5, mechanism half (next operation after committed revocation is denied; connections closed at termination before reclamation). The operation-semantics half of item 5 (typed operations, grant records) needs the gateway and is WP2.
**Baseline:** VM 110, Linux `6.12.107+deb13-cloud-amd64`, systemd `257.13-1~deb13u1`.
**Spike:** `spikes/frozen-peer/`. **Raw transcript:** `raw/frozen-peer.txt`. **Command:** `spikes/run.sh frozen-peer`.

Setup: a gateway stand-in accepts `SOCK_SEQPACKET` connections with `SO_PASSCRED`. Session peers are forked into a dedicated cgroup. Scenario 1 freezes the cgroup while a peer holds an idle connection and measures whether the gateway can still reply, close, and account zero connections. Scenario 2 commits a revocation in the gateway's live state and checks the next operation on an *existing* connection. Scenario 3 terminates the session with `cgroup.kill` and checks the gateway observes hang-up and can close before identity release.

## Results

| ID | Question | Observed | Result |
|---|---|---|---|
| LC2-1 | Can the gateway send to a frozen peer without blocking? | `send` returned in 7 µs (kernel socket buffer) | **PASS** |
| LC2-2 | Can the gateway `shutdown`+`close` while the peer is frozen? | rc 0/0 in 10 µs | **PASS** |
| LC2-3 | Does the zero-connection acknowledgement depend on the peer? | gateway holds no descriptor for the session; backlog empty; acknowledgement is the gateway's own accounting | **PASS** |
| LC2-4 | Can the frozen peer open a new connection? | none arrived in 300 ms | **PASS** |
| LC2-5 | After thaw, can the peer use the connection the gateway closed? | peer received the buffered pre-close reply, next `send` → `EPIPE`, exited 2 ms after thaw | **PASS** |
| D7-5a | Next operation after committed revocation denied on an existing connection | `op-2` denied and connection closed 2.2 ms after commit (spike's scheduling; the check itself is a state read); peer observed denial, EOF, `EPIPE` | **PASS** |
| D7-5b | Termination closes connections before identity release | `cgroup.kill` emptied the cgroup in 2 ms; gateway saw `POLLHUP|POLLRDHUP` (`0x2011`) and closed; zero connections before release | **PASS** |

## Disposition of LC-2

**A frozen peer does not delay the gateway's zero-connection acknowledgement.** Closing a Unix-domain connection is a local operation on the gateway's descriptor; the peer's scheduling state is irrelevant. The failure branch named in the register ("quiesce closes idle gateway connections before freezing and §6 is revised") is not needed. The §6 text stands: quiesce denies admission, freezes, and at bound expiry terminates; the gateway's connection close at termination (§5 step 6) succeeds regardless of freeze state.

One implementation note: a reply written *before* the close is still delivered to the thawed peer (LC2-5 code 13). If the daemon wants the peer to observe denial rather than a stale success, the gateway should send the denial then close, as D7-5a does. No document change.
