#!/usr/bin/env bash
# WP1 spike VM-1 (+ ADR-0003 boot check): boot Firecracker v1.16.1 with one vsock device and
# measure what the host endpoint can observe about the guest CID.
set -euo pipefail
cd "$(dirname "$0")"
FC=/usr/local/bin/firecracker; KERNEL=/root/fc/vmlinux-6.1.128
echo "firecracker: $($FC --version | head -1); sha256 $(sha256sum $FC | cut -c1-16)…"
echo "guest kernel: $(basename $KERNEL) sha256 $(sha256sum $KERNEL | cut -c1-16)… (Firecracker CI artefact; the pinned 6.12 guest kernel is a WP2 build item)"
echo "nested KVM: $(python3 -c 'import fcntl,os; fd=os.open("/dev/kvm",os.O_RDWR); print("api", fcntl.ioctl(fd,0xAE00))')"
gcc -static -O2 -o guest-init guest-init.c 2>/dev/null
W=$(mktemp -d); cp guest-init "$W/init"; (cd "$W" && echo init | cpio -o -H newc --quiet > initrd.cpio)
cat > "$W/vm.json" <<JSON
{"boot-source":{"kernel_image_path":"$KERNEL","initrd_path":"$W/initrd.cpio","boot_args":"console=ttyS0 reboot=k panic=1 pci=off rdinit=/init quiet"},
 "drives":[],"machine-config":{"vcpu_count":1,"mem_size_mib":128},
 "vsock":{"guest_cid":42,"uds_path":"$W/v.sock"}}
JSON
echo "config: single vsock device guest_cid=42, no drives, no network (ADR-0003 device set)"
python3 host.py "$W/v.sock" "$W/vm.json"
# boot-time measurement
rm -f "$W"/v.sock*
T0=$(date +%s%N); OUT=$(timeout 20 $FC --no-api --config-file "$W/vm.json" 2>&1 || true); T1=$(date +%s%N)
if echo "$OUT" | grep -q "exiting successfully"; then R=PASS; else R=FAIL; fi
echo "RESULT VM1-7.microvm-boot-to-exit-wallclock $R $(( (T1-T0)/1000000 )) ms wall-clock for VMM start, guest boot, vsock attempts (no listener → fast fail), guest reboot, VMM exit"
rm -rf "$W"
