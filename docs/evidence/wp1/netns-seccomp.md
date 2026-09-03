# WP1 evidence — `netns-seccomp`

**Covers:** ADR-0002 Decision 7 item 4 (abstract socket isolation); plan WP1 spike "socket-family seccomp and abstract-socket isolation in an empty network namespace"; R-GW-2; R-CON-2 (`pivot_root`, private propagation); catalogue rows T-6.1-004, T-6.1-012, T-6.4-004 (mechanism projection).
**Baseline:** VM 110, Linux `6.12.107+deb13-cloud-amd64`, systemd `257.13-1~deb13u1`.
**Spike:** `spikes/netns-seccomp/`. **Raw transcript:** `raw/netns-seccomp.txt`. **Command:** `spikes/run.sh netns-seccomp`.

Setup: the parent holds abstract socket `@ab-host` and pathname socket `<dir>/gateway.sock`. Two sessions are forked; each does `unshare(CLONE_NEWNET|CLONE_NEWNS|CLONE_NEWUTS)`, marks `/` recursively private, builds a tmpfs root holding only a read-only, `nosuid,nodev,noexec` bind of the gateway socket, and enters it with `pivot_root` (the old root is detached). Session A binds `@ab-sibling`; session B runs the checks, then installs a seccomp filter (`no_new_privs`, `SECCOMP_FILTER_FLAG_TSYNC`) that returns `EPERM` from `socket(2)` for every family except `AF_UNIX`.

## Results

| ID | Required result | Observed | Result |
|---|---|---|---|
| NS-1 | New netns is empty | `RTM_GETLINK` dump: one interface (`lo`), down | **PASS** |
| D7-4a | Host abstract sockets unreachable from the session | `connect(@ab-host)` → `ECONNREFUSED` | **PASS** |
| D7-4b | Sibling-session abstract sockets unreachable | `connect(@ab-sibling)` → `ECONNREFUSED` | **PASS** |
| D7-4c | Abstract namespace is per-netns | session binds `@ab-host` successfully while the host holds the same name | **PASS** |
| NS-2 | Gateway socket reachable only through the projection | `connect(/gateway.sock)` → 0; `connect(<original host path>)` → `ENOENT` after `pivot_root` | **PASS** |
| NS-3 | Pathname sockets cross netns (mount namespace is the isolation boundary for them) | projected pathname socket connects across the netns boundary | **PASS** (design confirmation) |
| SC-1 | Seccomp filter installs with `TSYNC` under `no_new_privs` | rc 0 | **PASS** |
| SC-2 | Every non-`AF_UNIX` family denied | `AF_INET`, `AF_INET6`, `AF_NETLINK`, `AF_PACKET`, `AF_VSOCK`, `AF_BLUETOOTH`, `AF_ALG` → `EPERM` | **PASS** |
| SC-3 | `AF_UNIX` still permitted | `SOCK_SEQPACKET` and `SOCK_STREAM` succeed | **PASS** |
| SC-4 | Filter applies to all threads | thread's `socket(AF_INET)` → `EPERM`; second `TSYNC` install with a live thread rc 0 | **PASS** |
| SC-5 | `socketpair(2)` is not a bypass | `socketpair(AF_INET)` → `EOPNOTSUPP` (kernel; only `AF_UNIX` supports it, and an `AF_UNIX` pair reaches nothing) | **PASS** |
| NS-4 | Host unaffected | host still reaches `@ab-host`; no session mount propagated | **PASS** |

## Finding F-2 — inherited sysfs shows the host's network interfaces (gap in R-CON-2)

NS-0: inside the new netns and mount namespace, *before* `pivot_root`, `/sys/class/net` still listed the host's two interfaces (`eth0`, `lo`). sysfs network views are bound to the netns of the mount, not of the reader; a sysfs mount inherited from the constructor's namespace exposes host network topology (interface names, MACs, statistics, and via `/sys/class/net/*/` anything the host exposes) even though the session's netns is empty. `RTM_GETLINK` inside the session correctly reported `lo` only.

R-CON-2 forbids host `/proc` visibility and requires `proc` to be mounted after the PID namespace exists. It says nothing about `/sys`. The spike's `pivot_root` into a tmpfs root removes the inherited sysfs entirely, which is the correct outcome, but the requirement should state it: **the session root MUST NOT contain a sysfs mount inherited from the constructor; if a sysfs is needed it MUST be mounted after the network namespace exists** (a fresh sysfs mount in the new netns shows only `lo`). Recommended amendment to R-CON-2 and session lifecycle §3 step 5, and a catalogue row under T-6.1 (host `/sys` not visible). No mechanism or decision changes; this is a completeness gap in the construction ordering text.

## Notes for the constructor

- Abstract-socket isolation is a property of the netns alone; it holds with no interfaces, no iptables, and no LSM. This is what ADR-0002 Decision 1 relies on.
- Pathname sockets are not isolated by the netns; the mount namespace plus `pivot_root` is what confines the session to the single projected gateway socket. R-GW-2's "every Unix socket other than the manifest's gateway socket … MUST be unreachable" therefore depends on R-CON-2, not on the network namespace.
- The seccomp rule needs only `socket(2)` argument 0; `socketpair(2)` needs no rule because non-`AF_UNIX` families reject it.
