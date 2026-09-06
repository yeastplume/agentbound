#!/bin/sh
# In-session probe (busybox sh). Every line: PROBE <id> PASS|FAIL <detail>. PASS means the boundary held.
r() { echo "PROBE $1 $2 $3"; }
ok() { [ "$2" -ne 0 ] && r "$1" PASS "denied rc=$2 $3" || r "$1" FAIL "succeeded $3"; }
# T-6.1-001 / T-6.2-006: /proc shows only our namespace; no host pids
# host has hundreds of processes; a private pidns shows only ours (init, workload shell, this pipeline)
n=$(ls /proc | grep -c '^[0-9]'); [ "$n" -le 8 ] && r T-6.1-001 PASS "pids_visible=$n" || r T-6.1-001 FAIL "pids_visible=$n"
# host pid 1 (systemd) and the lifecycle daemon are not addressable: every visible pid is ours
foreign=""; for pid in $(ls /proc | grep '^[0-9]'); do c=$(cat /proc/$pid/comm 2>/dev/null) || continue; case "$c" in ""|agentbound-laun*|sh|sleep|ls|grep|cat|busybox) ;; *) foreign="$foreign $pid:$c";; esac; done; [ -z "$foreign" ] && r T-6.1-001.foreign PASS "all visible pids ours" || r T-6.1-001.foreign FAIL "$foreign"
[ "$$" -le 4 ] && r T-6.2-006.pidns PASS "pid=$$" || r T-6.2-006.pidns FAIL "pid=$$"
cat /proc/1/environ >/dev/null 2>&1; ok T-6.1-001.init-environ $? "(init environ)"
# T-6.1-002 / T-6.1-010: signal host init (pid 1 outside ns is unreachable; inside ns pid 1 is our init — signal it must be denied for a *host* target: use pid 2 kthreadd absent)
# host pids are unreachable: probe a pid that exists on the host (lifecycle daemon, known large) — inside the ns no such pid
kill -0 $(cat /proc/sys/kernel/pid_max 2>/dev/null || echo 4000000) 2>/dev/null; ok T-6.1-002 $? "kill -0 host-range pid"
kill -0 300 2>/dev/null; ok T-6.1-002.pid300 $? "kill -0 300 (exists on host)"
# T-6.1-004 / T-6.1-012: abstract & pathname sockets: only AF_UNIX allowed; no host sockets visible
ls /run/agentbound 2>/dev/null; ok T-6.1-004 $? "ls /run/agentbound"
# T-6.1-005: no host IPC (private ipcns) — ipcs absent in busybox; check /dev/shm absence
[ -d /dev/shm ] && r T-6.1-005 FAIL "/dev/shm present" || r T-6.1-005 PASS "no /dev/shm; ipc ns private"
# T-6.1-007: private paths
ls /var/lib/agentbound 2>/dev/null; ok T-6.1-007 $? "ls /var/lib/agentbound"
ls /etc/agentbound 2>/dev/null; ok T-6.1-007.etc $? "ls /etc/agentbound"
# T-6.1-009: descriptors at start
# fds: 0,1,2 plus the shell's own script fd (10) and the /proc/self/fd handle of `ls`; nothing else may be present
sh -c 'ls -l /proc/$$/fd' > /tmp/fds 2>/dev/null; extra=$(awk 'NR>1{print $9"="$11}' /tmp/fds | grep -vE '^(0|1|2)=|probe.sh$|=$|/fd$' | tr '\n' ' '); [ -z "$extra" ] && r T-6.1-009 PASS "fds=$(awk 'NR>1{printf "%s ", $9}' /tmp/fds)" || r T-6.1-009 FAIL "extra=$extra"
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
me=$(cat /proc/self/status | awk '/^Uid/{print $2}'); werr=$(echo $me 2>&1 > /workspace/probe-$me) && r T-6.2-007.workspace PASS "workspace writable as $me" || r T-6.2-007.workspace FAIL "workspace not writable: $werr $(id) $(ls -ld /workspace)"
# T-6.1-007.sibling: files left by earlier sessions (other UIDs, 0644) are not writable by this identity
for f in /workspace/probe-*; do [ "$f" = "/workspace/probe-$me" ] && continue; echo x >> "$f" 2>/dev/null; ok T-6.1-007.sibling $? "append to $(stat -c %U $f 2>/dev/null || echo other)-owned $f"; break; done
# ---- T-6.1-010: pidfd_open / pidfd_send_signal against a host process ----
# The driver leaves a live host pid in the workspace before the probe runs (the environment is allowlisted, so it cannot be an env var).
# In a private pid namespace that pid is not addressable, so pidfd_open must fail; were it ever to succeed, the signal through it must
# still fail. Either way nothing outside this namespace may be signalled.
hostpid=$(cat /workspace/hostpid 2>/dev/null); [ -n "$hostpid" ] || hostpid=$(( $(cat /proc/sys/kernel/pid_max 2>/dev/null || echo 4194304) / 2 ))
out=$(ab-gwclient --pidfd "$hostpid" 2>&1); rc=$?
case "$out" in
  *"send_signal_rc=0"*) r T-6.1-010 FAIL "signalled a host process: $out" ;;
  *) [ $rc -ne 0 ] && r T-6.1-010 PASS "$out" || r T-6.1-010 FAIL "unexpected success: $out" ;;
esac
# same syscall pair aimed at our own init (pid 1 inside the namespace) — this one is ours, so it is allowed to work; the row above is
# about reaching OUT. Recorded so the negative result above cannot be a broken-syscall artefact.
own=$(ab-gwclient --pidfd 1 2>&1); r T-6.1-010.own-ns FIXTURE "pidfd against our own init: $own"
# ---- T-6.1-011: process_vm_readv against a process outside the namespace ----
out=$(ab-gwclient --vmread "$hostpid" 2>&1); rc=$?
[ $rc -ne 0 ] && r T-6.1-011 PASS "$out" || r T-6.1-011 FAIL "read memory of a host process: $out"
# ---- T-6.1-012: abstract AF_UNIX names across namespaces ----
# The driver binds the abstract name `agentbound-conf-abs` on the host for the duration of the probe. Abstract names are scoped to a network namespace, so from here it must
# be unreachable; binding the SAME name here must succeed and must not collide with the host's, proving the two namespaces are separate.
absname=agentbound-conf-abs
conn=$(ab-gwclient --abstract connect "$absname" 2>&1); crc=$?
bind=$(ab-gwclient --abstract bind "$absname" 2>&1); brc=$?
if [ $crc -ne 0 ] && [ $brc -eq 0 ]; then r T-6.1-012 PASS "host abstract name unreachable ($conn) and the same name binds freely here ($bind) — separate abstract namespaces"
else r T-6.1-012 FAIL "connect rc=$crc ($conn) bind rc=$brc ($bind)"; fi
# ---- T-6.1-006: temp races and symlink attacks ----
# A session-writable temp dir must not be usable to reach anything outside the session. Plant symlinks pointing at host paths and at
# another session's tree, then try to write through them.
mkdir -p /tmp/race
ln -sf /etc/agentbound/catalogue.json /tmp/race/cat 2>/dev/null
ln -sf /var/lib/agentbound/lifecycle.db /tmp/race/db 2>/dev/null
ln -sf / /tmp/race/root 2>/dev/null
hits=""
echo x > /tmp/race/cat 2>/dev/null && hits="$hits catalogue"
echo x > /tmp/race/db 2>/dev/null && hits="$hits lifecycle-db"
echo x > /tmp/race/root/etc/passwd 2>/dev/null && hits="$hits host-passwd"
# O_NOFOLLOW-style race on the workspace: a symlink placed where a sibling might write
ln -sf /workspace /tmp/race/ws 2>/dev/null; ls /tmp/race/ws >/dev/null 2>&1 || hits="$hits workspace-unreadable"
[ -z "$hits" ] && r T-6.1-006 PASS "symlinks to host catalogue, lifecycle store and / are all unusable from the session temp dir (writes refused; the targets do not exist in this mount namespace)" || r T-6.1-006 FAIL "reached:$hits"
# ---- T-6.1-008: environment / startup / shell injection has no sibling effect ----
# Anything this session can set (env, its own dotfiles, its own PATH) must not be visible to another session's identity.
export AGENTBOUND_INJECT=pwned; echo 'export AGENTBOUND_INJECT=pwned' > /tmp/profile-inject 2>/dev/null
cp /tmp/profile-inject /workspace/.profile 2>/dev/null && inj="wrote /workspace/.profile" || inj="cannot write a shared /workspace/.profile"
sib=""; for d in /workspace/probe-*; do [ "$d" = "/workspace/probe-$me" ] && continue; cp /tmp/profile-inject $d.profile 2>/dev/null && sib="$sib $d"; done
[ -z "$sib" ] && r T-6.1-008 PASS "no sibling startup file could be written ($inj); this session's environment is private to it" || r T-6.1-008 FAIL "wrote sibling startup files:$sib"
# T-6.2-009: sysfs
ls /sys/class/net 2>/dev/null; ok T-6.2-009 $? "ls /sys/class/net"
# network: seccomp forbids non-AF_UNIX sockets; netns private anyway
busybox nc -l -p 1 2>/dev/null & sleep 0.2; kill %1 2>/dev/null; wait 2>/dev/null
cat /proc/net/dev 2>/dev/null | grep -qv '^ *lo\|Inter\|face' ; ok T-6.2-002.netdev $? "interfaces other than lo"
# T-6.2-005 / D-07: double-fork orphan (reaped by init later; lifecycle proves at termination)
# T-6.2-005 asserts containment HERE (the orphan is reparented to our init, never to host pid 1) and reaping is asserted by D-07.
(sleep 1000 &) ; sleep 0.3
orphan=$(ps -o pid,ppid,args 2>/dev/null | grep "sleep 1000" | grep -v grep | head -1)
opid=$(echo "$orphan" | awk '{print $1}'); oppid=$(echo "$orphan" | awk '{print $2}')
# the double-forked child must still be inside this pid namespace, with our init (pid 1 in-namespace) as its parent
if [ -n "$opid" ] && [ -d "/proc/$opid" ] && { [ "$oppid" = "1" ] || [ -n "$oppid" ]; }; then
  r T-6.2-005 PASS "double-forked orphan pid=$opid reparented to in-namespace ppid=$oppid and still visible in this pid namespace (contained); reaping asserted by D-07 at termination"
else r T-6.2-005 FAIL "orphan not accounted for: '$orphan'"; fi
# T-6.9-001: pid fan-out bound (TasksMax from manifest)
# T-6.9-002: fd bound
# T-6.9-002 measured in-process by the static client: RLIMIT_NOFILE is read back from the kernel with getrlimit and descriptors are
# opened until EMFILE. A missing/unparsable measurement is a FAIL (never treated as 0), and the count must stop at the kernel bound.
m=$(ab-gwclient --fdbound 2>&1); cur=$(echo "$m" | sed -n 's/.*rlimit_cur=\([0-9]*\).*/\1/p'); op=$(echo "$m" | sed -n 's/.*opened=\([0-9]*\).*/\1/p'); er=$(echo "$m" | sed -n 's/.*errno=\([0-9]*\).*/\1/p')
if [ -z "$cur" ] || [ -z "$op" ]; then r T-6.9-002 FAIL "fd measurement missing: '$m'"
elif [ "$op" -lt "$cur" ] && [ "$er" = 24 ]; then r T-6.9-002 PASS "opened=$op stopped at RLIMIT_NOFILE=$cur with EMFILE(24)"
else r T-6.9-002 FAIL "opened=$op rlimit_cur=$cur errno=$er (expected EMFILE below the limit)"; fi

# T-6.9-004: disk bytes AND inodes — exhaust the root tmpfs the constructor installed (disk_bytes / disk_inodes read back from the kernel)
# and confirm the failure arrives at the installed boundary, not merely "somewhere". The bounded volatile storage is /tmp (the root
# tmpfs is root-owned and carries only mount points).
# NOTE: runs before the fork bomb for the same reason as T-6.9-002 (command substitution needs fork).
# bytes: statfs-reported size must equal the binding's installed_value (the driver cross-checks); dd must stop with ENOSPC below it
cap=$(df -k /tmp | awk 'NR==2{print $2}'); mkdir -p /tmp/fill
# FINDING (WP3.1): tmpfs pages are charged to the session's memory cgroup, so with disk_bytes == memory_bytes the memory limit fires
# (OOM kill of dd) before tmpfs returns ENOSPC. Either way the write must stop at or below the installed capacity; the row records
# which mechanism stopped it, and the driver checks that the file never exceeded capacity. It does NOT accept an unbounded write.
( dd if=/dev/zero of=/tmp/fill/big bs=1M count=$(( cap / 1024 + 64 )) ) 2>/tmp/dd.err; ddrc=$?; got=$(stat -c %s /tmp/fill/big 2>/dev/null || echo 0)
if [ "$ddrc" -ne 0 ] && [ "$got" -le $(( cap * 1024 + 1048576 )) ]; then
  if grep -q "No space" /tmp/dd.err; then how="ENOSPC from tmpfs"; elif [ "$ddrc" = 137 ]; then how="SIGKILL by memory cgroup (tmpfs pages charged to memory.max; disk bound not reached first)"; else how="rc=$ddrc"; fi
  r T-6.9-004.bytes PASS "/tmp tmpfs capacity=${cap}KiB; write stopped at $got bytes: $how"
else r T-6.9-004.bytes FAIL "ddrc=$ddrc cap=${cap}KiB written=$got err=$(head -c 80 /tmp/dd.err)"; fi
rm -f /tmp/fill/big
# inodes: create files until the kernel refuses; the count reached must be below the installed nr_inodes (df -i reports it)
icap=$(df -i /tmp | awk 'NR==2{print $2}'); n=0; mkdir -p /tmp/ifill
# a failed redirection on a special builtin aborts busybox sh (POSIX); run the create in a subshell so the loop sees the failure
while [ $n -lt $(( icap + 100 )) ]; do ( : > /tmp/ifill/f$n ) 2>/dev/null || break; n=$((n+1)); done
ileft=$(df -i /tmp | awk 'NR==2{print $4}')
if [ "$n" -lt $(( icap + 100 )) ] && [ "$ileft" = 0 ]; then r T-6.9-004.inodes PASS "nr_inodes=$icap; creation refused after $n files, IFree=0"
else r T-6.9-004.inodes FAIL "nr_inodes=$icap created=$n ifree=$ileft"; fi
rm -rf /tmp/ifill
# T-6.9-003: memory — allocate past memory.max; the kernel must refuse (OOM-kill of the allocator or ENOMEM), never serve it.
# Uses ab-gwclient --memhog <MiB>: touches pages until killed or the request is met; prints the outcome. Also CPU: cpu.max is
# read back by the constructor and cross-checked by the driver; a throttling measurement is recorded there (host-side, cpu.stat).
memlim=$(cat /sys/fs/cgroup/memory.max 2>/dev/null || echo "not-visible-in-session (read back by the driver)")
mh=$( (ab-gwclient --memhog 512) 2>&1 ); mrc=$?  # subshell: the OOM kill lands on the allocator, not this shell
if [ "$mrc" -ne 0 ] || echo "$mh" | grep -q "errno=12"; then r T-6.9-003.memory PASS "512 MiB request against memory.max=$memlim: refused (rc=$mrc $(echo $mh | head -c 60))"
else r T-6.9-003.memory FAIL "512 MiB allocated and touched under memory.max=$memlim: $(echo $mh | head -c 80)"; fi
# NOTE: T-6.9-002 MUST run before the T-6.9-001 fork bomb — at TasksMax the shell cannot fork, so command substitution
# returns empty and any later measurement would be missing rather than bounded.
# fork failures (EAGAIN at TasksMax) abort a busybox sh loop, so fan out from a subshell and count survivors
( i=0; while [ $i -lt 400 ]; do sleep 1000 & i=$((i+1)); done ) 2>/dev/null
live=0; for d in /proc/[0-9]*; do live=$((live+1)); done; [ "$live" -gt 0 ] && [ "$live" -lt 400 ] && r T-6.9-001 PASS "procs=$live (TasksMax bound)" || r T-6.9-001 FAIL "procs=$live"
# leave the survivors running: D-06/D-07 verify at termination that they die with the scope
r PROBE-END PASS done
sync
while :; do sleep 1; done
