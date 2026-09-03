# WP1 evidence — `mount-construct`

**Covers:** plan WP1 spikes "namespace, mount, and procfs construction in the required ordering with mount-descriptor resolution" and "descriptor closure and runtime launch ordering"; R-REQ-6, R-CON-2, R-CON-3, R-CON-4; fault points F-C-01..07 (mechanism projection, not the fault-injection tests themselves); catalogue T-6.1-009 (mechanism projection).
**Baseline:** VM 110, Linux `6.12.107+deb13-cloud-amd64`, systemd `257.13-1~deb13u1`; `busybox-static` used as the runtime image binary.
**Spike:** `spikes/mount-construct/`. **Raw transcript:** `raw/mount-construct.txt`. **Command:** `spikes/run.sh mount-construct`.

Setup: a trusted base holds a workspace, a runtime image, and a "secret" directory outside the workspace. The workspace (session-controlled content) contains three planted symlinks: relative escape (`../../secret`), absolute (`/etc`), and magic (`/proc/self/root/etc`). The constructor resolves sources with `openat2(RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS)` from an `O_PATH` base descriptor, converts them into detached mount trees (`open_tree(OPEN_TREE_CLONE)` + `mount_setattr`), then `clone3`s the session init into new mount/PID/UTS/IPC/net namespaces behind a barrier. The child builds a tmpfs root with the new mount API (`fsopen`/`fsconfig`/`fsmount`), attaches the trees by descriptor (`move_mount(MOVE_MOUNT_F_EMPTY_PATH)`), `pivot_root`s, mounts `proc` and `sysfs` after the namespaces exist, closes descriptors with `close_range`, drops to the execution UID with `no_new_privs` and an empty bounding set, and execs the image binary.

## Results

| ID | Required result | Observed | Result |
|---|---|---|---|
| R6-1 | Legitimate path resolves | `openat2(base, ws1/src)` → fd | **PASS** |
| R6-2 | Relative symlink escape rejected | `ELOOP` | **PASS** |
| R6-3 | Absolute symlink rejected | `ELOOP` | **PASS** |
| R6-4 | Magic link (`/proc/self/root/…`) rejected | `ELOOP` | **PASS** |
| R6-5 | `..` escape rejected under `RESOLVE_BENEATH` | `EXDEV` | **PASS** |
| R6-6 | Held descriptor immune to a directory→symlink swap after validation | `openat(held_fd, hello.txt)` still returns the workspace file; a string re-walk of the same path now returns `SECRET` | **PASS** — demonstrates the race R-REQ-6 forbids |
| R6-7 | Detached mount trees from held descriptors | `open_tree(OPEN_TREE_CLONE)` + `mount_setattr` (ro/nosuid/nodev) without any path string | **PASS** |
| C1-1 | Barrier: child exists in new namespaces but has changed nothing | child mountinfo identical to host before release; pidfd held for rollback | **PASS** (F-C-01 projection) |
| C2-1 | `/` made `rprivate` first | rc 0 | **PASS** (F-C-02) |
| C4-1 | Root and trees attached by descriptor | tmpfs root via `fsmount`; both trees via `move_mount` | **PASS** (F-C-03/04) |
| C4-2 | `pivot_root` + detach old root | rc 0 / 0 | **PASS** |
| C5-1 | `proc` mounted after PID namespace shows only the session | `pid1=1`, one process visible | **PASS** (F-C-05) |
| C5-2 | `sysfs` mounted after netns shows only `lo` | 1 interface | **PASS** (closes the F-2 mitigation loop) |
| C4-3 | Every mount `nosuid,nodev`; no cgroupfs; no host root; no systemd/D-Bus socket; secret unreachable | 5 mounts, all conditions true | **PASS** (R-CON-4) |
| C4-4 | Workspace and image content correct; image executable | as required | **PASS** |
| C6-1 | `close_range` leaves only the allowlist | 16 fds → `{0,1,2,1000}` | **PASS** (F-C-06) |
| C6-2 | No reintroduction via `/proc/self/fd`, memfd, or `SCM_RIGHTS` carrier; no host PID visible | `ENOENT` for both procfd reopen attempts; socketpair end closed; PID 2 invisible | **PASS** (T-6.1-009 projection) |
| C7-1 | `no_new_privs`, empty bounding set, UID transition, then exec | `uid=200042 nnp=1 CAP_SYS_ADMIN not in bounding set`; `busybox true` exit 0 | **PASS** (F-C-07/09 projection) |
| C2-2 | Nothing propagated to the host | host mountinfo line count unchanged; `/mnt/workspace` absent on host | **PASS** |

## Notes

- **No findings.** The ordering in lifecycle §3 and the R-REQ-6 primitives work as written on the pinned kernel. `openat2`'s `RESOLVE_NO_MAGICLINKS` is required in addition to `RESOLVE_NO_SYMLINKS` — a plain `RESOLVE_NO_SYMLINKS` also rejects magic links, but naming both makes the intent explicit; R-REQ-6 lists only `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS`, which is sufficient (tested: `ELOOP` is produced by `RESOLVE_NO_SYMLINKS` alone since magic links are symlinks to the resolver). No text change required.
- `close_range` with `CLOSE_RANGE_CLOEXEC` is an alternative that marks rather than closes; the spike closes outright, which is what step 6 asks for. The reporting pipe is `dup3`'d to a fixed high number and closed before exec so that the exec'd runtime inherits exactly `{0,1,2}`.
- The image must be self-contained (static binary or a complete image tree); a dynamically linked `/bin/sh` would fail to exec in the constructed root — a runtime-image packaging concern for WP2, not a mechanism gap.
