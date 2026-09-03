# WP1 evidence — `vsock-cid`

**Covers:** open-question register item **VM-1** (vsock peer-CID reporting on the pinned host kernel); ADR-0003 "VM identity, CID lifetime, and vsock admission"; plan WP1 "minimal control-arm launcher — boot check only".
**Baseline:** VM 110 as host (Linux `6.12.107+deb13-cloud-amd64`, `vhost_vsock` loaded, nested KVM API 12 available inside the Proxmox guest); Firecracker **v1.16.1** (release tarball SHA-256 `382a02a8…c242e6`, binary `2fd01713…`); guest kernel Firecracker-CI `vmlinux-6.1.128` (SHA-256 `27a8310b…`) — the pinned 6.12 guest kernel is a WP2 build item, and no result here depends on the guest kernel version.
**Spike:** `spikes/vsock-cid/` (`run.sh`, `guest-init.c`, `host.py`). **Raw transcript:** `raw/vsock-cid.txt`. **Command:** `spikes/run.sh vsock-cid`.

Setup: Firecracker started with `--no-api` and a config exposing **only** a vsock device (`guest_cid=42`, `uds_path=…/v.sock`), no drives, no network — the ADR-0003 device set. The guest init (static C, 760 KB initrd) reports `IOCTL_VM_SOCKETS_GET_LOCAL_CID`, connects to host port 5000 (offered) and 5001 (not offered), tries to bind a forged CID, opens a second connection, and reboots.

## Results

| ID | Question | Observed | Result |
|---|---|---|---|
| VM1-1 | What does the host endpoint see as the peer? | `SO_PEERCRED` on the accepted connection is the **firecracker process** (`pid` = VMM pid, `uid 0`); no guest credential | **PASS** — confirms ADR-0003 "no `SCM_CREDENTIALS` counterpart" |
| VM1-2 | Does the guest's CID equal the configured `guest_cid`? | guest reports 42; config 42 | **PASS** |
| VM1-3 | Is an unoffered port refused? | connect to 5001 → `ECONNRESET` (no `v.sock_5001` on the host) | **PASS** — the "exactly one service" rule is enforced by the host offering exactly one listener |
| VM1-4 | Can the guest forge a CID? | `bind(svm_cid=999)` → `EADDRNOTAVAIL` | **PASS** |
| VM1-5 | Is a second connection attributable to the same VM instance? | same VMM pid on the second accepted connection | **PASS** |
| VM1-6 | Is VMM exit observable for CID-lifetime invalidation? | exit code 0, "exiting successfully" | **PASS** |
| VM1-7 | Boot check (ADR-0003 control arm) | 625 ms wall-clock: VMM start, guest boot, vsock attempts, reboot, VMM exit | **PASS** |

## Disposition of VM-1 — resolved, with a wording correction to ADR-0003

The register asked whether "the host `AF_VSOCK` endpoint reports the guest CID for each accepted connection". **With Firecracker there is no host `AF_VSOCK` endpoint.** Firecracker's vsock device is a Unix-socket bridge: a guest `connect(CID 2, port P)` surfaces on the host as a `AF_UNIX` connection from the *VMM process* to `<uds_path>_P`. The guest CID is not carried on that connection; it is a property of the VMM instance the daemon configured. The "host-observed guest CID" ADR-0003 binds is therefore derived as:

`accepted AF_UNIX connection` → `SO_PEERCRED` pid (+ pidfd/start-time check, as ADR-0002 Decision 2) → **this VMM instance** → its configured `guest_cid`, instance token, jailer identity, config digest.

This is exactly the register's *failure branch* ("binding uses the VMM connection table and the ADR records the change") — but it is not a failure of the design: every binding element ADR-0003 lists is still available, and the VMM pidfd it already requires is the anchor. The daemon must additionally ensure the bridge socket directory is writable only by the daemon and that the `uds_path` is per-instance, so no other process can connect to `<uds_path>_P` and impersonate the VMM (the same D7-item-4 abstract-socket concern in Unix-path form; the jailer's chroot provides this).

**Finding F-7 (ADR-0003, wording).** Replace "the host-observed guest CID" with "the CID configured for the VMM instance that owns the accepted bridge connection, established via `SO_PEERCRED` on that connection and the held VMM pidfd"; state that Firecracker's vsock is a Unix-socket bridge and that the daemon MUST own the bridge socket path. No obligation is weakened; the register item closes with the ADR amended rather than the design changed.

## Boot check

Nested KVM inside VM 110 works (`KVM_CREATE_VM` succeeds; Firecracker boots a guest in well under a second). The control-arm launcher can therefore be developed and its tests run on this VM without a bare-metal host. Firecracker's `--no-api` + config-file mode gives the daemon a fully declarative, one-shot launch with no API socket to protect — attractive for the TCB accounting in ADR-0003.
