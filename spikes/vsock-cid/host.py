# WP1 spike VM-1 host side. Firecracker's vsock device is a Unix socket bridge: a
# guest connect(CID 2, port P) becomes a Unix connection from the firecracker
# process to <uds_path>_P. What the host endpoint can observe about the *guest*
# is therefore what this script measures. Not TCB; not SLOC-counted.
import socket, os, struct, sys, json, subprocess, time, re
uds = sys.argv[1]; cfg = sys.argv[2]; guest_cid = json.load(open(cfg))["vsock"]["guest_cid"]
def res(item, ok, detail): print(f"RESULT {item} {'PASS' if ok else 'FAIL'} {detail}", flush=True)
for p in (5000, 5001):
    try: os.unlink(f"{uds}_{p}")
    except FileNotFoundError: pass
ls = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM); ls.bind(f"{uds}_5000"); ls.listen(4); ls.settimeout(30)
# deliberately NO listener on _5001: the single-service rule is enforced by what the host does not offer
fc = subprocess.Popen(["firecracker", "--no-api", "--config-file", cfg], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
c, _ = ls.accept()
pid, uid, gid = struct.unpack("3i", c.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
lines = []
def rd():
    d = c.recv(512).decode().strip(); c.send(b"ack\n"); lines.append(d); return d
first = rd()
m = re.search(r"local_cid=(\d+)", first); gcid = int(m.group(1)) if m else -1
res("VM1-1.unix-peer-is-vmm-not-guest", pid == fc.pid and uid == 0, f"SO_PEERCRED on the accepted bridge socket: pid={pid} (firecracker pid {fc.pid}) uid={uid}; no guest credential is present on the host socket, as ADR-0003 states")
res("VM1-2.guest-local-cid-equals-configured-guest_cid", gcid == guest_cid, f"guest reports IOCTL_VM_SOCKETS_GET_LOCAL_CID={gcid}; VMM config guest_cid={guest_cid}")
second = rd()
res("VM1-3.unoffered-port-refused", "rc=-1" in second, f"guest connect to host port 5001 (no host listener): {second} (ECONNRESET=104: firecracker found no {os.path.basename(uds)}_5001)")
forged = rd()
res("VM1-4.guest-cannot-bind-forged-cid", "rc=-1" in forged, f"guest bind(svm_cid=999): {forged} (EADDRNOTAVAIL=99: guest may only use its own CID or ANY)")
# second connection from the same guest arrives on the same listener: same VMM pid → same VM instance
c2, _ = ls.accept(); pid2, _, _ = struct.unpack("3i", c2.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12)); d2 = c2.recv(512).decode().strip(); c2.send(b"ack\n")
res("VM1-5.second-connection-attributable-to-same-vmm-instance", pid2 == fc.pid, f"second connection peer pid={pid2}; {d2}")
rd()
fc.wait(timeout=30); out = fc.stdout.read()
res("VM1-6.vmm-exit-observable", fc.returncode == 0 and "exiting successfully" in out, f"firecracker exit code {fc.returncode}; the CID mapping can be invalidated only after this (ADR-0003 CID lifetime)")
print("--- disposition ---")
print("The host AF_VSOCK endpoint is NOT what Firecracker exposes: the host sees a per-VMM Unix socket bridge.")
print("Host-observed guest CID therefore = the CID the daemon configured for the VMM whose process owns the bridge connection (SO_PEERCRED pid → VMM → guest_cid), not a value carried by the connection.")
for l in lines: print("  guest:", l)
