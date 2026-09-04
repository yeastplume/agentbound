#!/bin/sh
# In-session probe (busybox sh). Every line: PROBE <id> PASS|FAIL <detail>. PASS means the boundary held.
r() { echo "PROBE $1 $2 $3"; }
ok() { [ "$2" -ne 0 ] && r "$1" PASS "denied rc=$2 $3" || r "$1" FAIL "succeeded $3"; }
# T-6.1-001 / T-6.2-006: /proc shows only our namespace; no host pids
n=$(ls /proc | grep -c '^[0-9]'); [ "$n" -le 4 ] && r T-6.1-001 PASS "pids_visible=$n" || r T-6.1-001 FAIL "pids_visible=$n"
[ "$$" -le 4 ] && r T-6.2-006.pidns PASS "pid=$$" || r T-6.2-006.pidns FAIL "pid=$$"
cat /proc/1/environ >/dev/null 2>&1; ok T-6.1-001.init-environ $? "(init environ)"
# T-6.1-002 / T-6.1-010: signal host init (pid 1 outside ns is unreachable; inside ns pid 1 is our init — signal it must be denied for a *host* target: use pid 2 kthreadd absent)
kill -0 2 2>/dev/null; ok T-6.1-002 $? "kill -0 2"
# T-6.1-004 / T-6.1-012: abstract & pathname sockets: only AF_UNIX allowed; no host sockets visible
ls /run/agentbound 2>/dev/null; ok T-6.1-004 $? "ls /run/agentbound"
# T-6.1-005: no host IPC (private ipcns) — ipcs absent in busybox; check /dev/shm absence
[ -d /dev/shm ] && r T-6.1-005 FAIL "/dev/shm present" || r T-6.1-005 PASS "no /dev/shm; ipc ns private"
# T-6.1-007: private paths
ls /var/lib/agentbound 2>/dev/null; ok T-6.1-007 $? "ls /var/lib/agentbound"
ls /etc/agentbound 2>/dev/null; ok T-6.1-007.etc $? "ls /etc/agentbound"
# T-6.1-009: descriptors at start
fds=$(ls /proc/$$/fd | tr '\n' ' '); [ "$(ls /proc/$$/fd | wc -l)" -le 4 ] && r T-6.1-009 PASS "fds=$fds" || r T-6.1-009 FAIL "fds=$fds"
# T-6.2-001: cgroup migration
echo $$ > /sys/fs/cgroup/cgroup.procs 2>/dev/null; ok T-6.2-001 $? "write cgroup.procs"
[ -d /sys/fs/cgroup ] && r T-6.2-001.sysfs FAIL "cgroupfs visible" || r T-6.2-001.sysfs PASS "no cgroupfs"
# T-6.2-002: namespace manipulation
mount -t tmpfs none /tmp/x 2>/dev/null; ok T-6.2-002.mount $? "mount tmpfs"
mkdir -p /tmp/x; mount -t proc proc /tmp/x 2>/dev/null; ok T-6.2-006.proc $? "mount proc"
# T-6.2-003/004: setuid, caps
cp /bin/busybox /tmp/bb 2>/dev/null && chmod u+s /tmp/bb 2>/dev/null; /tmp/bb id -u 2>/dev/null | grep -q '^0$'; ok T-6.2-003 $? "setuid copy (nosuid tmpfs)"
grep -q 'CapEff:.0000000000000000' /proc/$$/status && r T-6.2-004 PASS "CapEff=0" || r T-6.2-004 FAIL "$(grep CapEff /proc/$$/status)"
grep -q 'NoNewPrivs:.1' /proc/$$/status && r T-6.2-004.nnp PASS nnp=1 || r T-6.2-004.nnp FAIL
# T-6.2-007: persistence outside workspace
touch /image/persist 2>/dev/null; ok T-6.2-007.image $? "write image (ro)"
touch /persist 2>/dev/null; ok T-6.2-007.root $? "write root tmpfs"
touch /workspace/probe-was-here 2>/dev/null && r T-6.2-007.workspace PASS "workspace writable rc=0" || r T-6.2-007.workspace FAIL "workspace not writable"
# T-6.2-009: sysfs
ls /sys/class/net 2>/dev/null; ok T-6.2-009 $? "ls /sys/class/net"
# network: seccomp forbids non-AF_UNIX sockets; netns private anyway
busybox nc -l -p 1 2>/dev/null & sleep 0.2; kill %1 2>/dev/null; wait 2>/dev/null
cat /proc/net/dev 2>/dev/null | grep -qv '^ *lo\|Inter\|face' ; ok T-6.2-002.netdev $? "interfaces other than lo"
# T-6.2-005 / D-07: double-fork orphan (reaped by init later; lifecycle proves at termination)
(sleep 1000 &) ; r T-6.2-005 PASS "orphan spawned"
# T-6.9-001: pid fan-out bound (TasksMax from manifest)
i=0; while [ $i -lt 400 ]; do sleep 1000 & i=$((i+1)); done 2>/dev/null; live=$(ls /proc | grep -c '^[0-9]'); [ "$live" -lt 400 ] && r T-6.9-001 PASS "procs=$live" || r T-6.9-001 FAIL "procs=$live"
kill $(jobs -p) 2>/dev/null
# T-6.9-002: fd bound
i=0; while [ $i -lt 2000 ]; do eval "exec $((i+10))</dev/null" 2>/dev/null || break; i=$((i+1)); done; [ $i -lt 2000 ] && r T-6.9-002 PASS "fds_opened=$i" || r T-6.9-002 FAIL "fds_opened=$i"
# T-6.9-004: disk bound (root tmpfs 16m)
dd if=/dev/zero of=/tmp/big bs=1M count=100 2>/dev/null; ok T-6.9-004 $? "dd 100M into tmpfs"
r PROBE-END PASS done
sync
while :; do sleep 1; done
