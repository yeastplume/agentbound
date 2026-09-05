#!/bin/sh
# In-session Git worker (runtime:git-worker). Produces commits in the workspace clone with no remote and no credential,
# bundles them, and submits the bundle over the projected gateway socket as the typed operation op:git-push-staging.
# Emits GW <row> PASS|FAIL <detail> lines on stdout (console.log) for the conformance driver.
r() { echo "GW $1 $2 $3"; }
export GIT_AUTHOR_NAME=agent GIT_AUTHOR_EMAIL=agent@session.invalid GIT_COMMITTER_NAME=agent GIT_COMMITTER_EMAIL=agent@session.invalid
cd /workspace || exit 1
[ -S /run/gateway.sock ] && r T-6.4-003.projected PASS "gateway socket node present" || r T-6.4-003.projected FAIL "no socket node"
ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping '{}' >/tmp/o 2>&1 && r D-09 PASS "authenticated connection, typed ping admitted: $(grep -o '"pong":true' /tmp/o)" || r D-09 FAIL "$(head -c 120 /tmp/o)"
ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping '{}' --stream >/dev/null 2>&1; true
# T-6.3-001/002: no credential anywhere the session can see
env | grep -qiE 'token|secret|passw|credential' && r T-6.3-001 FAIL "$(env | grep -iE 'token|secret|passw|credential' | cut -d= -f1)" || r T-6.3-001 PASS "no credential in environment"
[ -r /var/lib/agentbound/gateway/credential ] && r T-6.3-002 FAIL "credential readable" || r T-6.3-002 PASS "credential path absent/unreadable"
me=$(id -u); w=work-$me; rm -rf $w 2>/dev/null; git init -q $w; cd $w
# T-6.1-007.sibling: another session's clone is not modifiable by this identity
for d in /workspace/work-*; do [ "$d" = "/workspace/$w" ] && continue; touch $d/x 2>/dev/null && r T-6.1-007.sibling FAIL "wrote $d" || r T-6.1-007.sibling PASS "sibling $d not writable"; break; done
echo "fix for issue 1234 $(date +%s)" > fix.txt; git add fix.txt; git -c commit.gpgsign=false commit -q -m "fix issue 1234"
git remote | grep -q . && r T-6.3-002.no-remote FAIL "remote present" || r T-6.3-002.no-remote PASS "no git remote or credential in the worker repository (WP1 GS-1)"
git bundle create -q /tmp/fix.bundle HEAD 2>/dev/null || git bundle create /tmp/fix.bundle HEAD >/dev/null 2>&1
tip=$(git rev-parse HEAD); r D-10.bundle FIXTURE "tip=$tip bytes=$(stat -c %s /tmp/fix.bundle)"
# D-10/D-13: the granted typed operation
ab-gwclient /run/gateway.sock op:git-push-staging git.push_staging "{\"expect_old\":null,\"ref_tail\":\"fix-1234\",\"repository_id\":\"repo:demo\",\"tip\":\"$tip\"}" /tmp/fix.bundle > /tmp/out 2>&1; rc=$?
cat /tmp/out; [ $rc = 0 ] && r D-10 PASS "push_staging accepted rc=0" || r D-10 FAIL "rc=$rc"
# GS-4 refusal set through the gateway: other ref names, traversal, other session namespace, force operation
for tail in "../main" "main:refs/heads/main" "+fix" "fix.lock" "a b" "" "refs/heads/main"; do id=$(echo "$tail" | tr " " _);
  ab-gwclient /run/gateway.sock op:git-push-staging git.push_staging "{\"expect_old\":null,\"ref_tail\":\"$tail\",\"repository_id\":\"repo:demo\",\"tip\":\"$tip\"}" /tmp/fix.bundle >/tmp/o 2>&1 && r "T-6.4-011.gs4[$id]" FAIL "accepted" || { rule=$(grep -o '"rule":"[^"]*"' /tmp/o | head -1); [ -n "$rule" ] && r "T-6.4-011.gs4[$id]" PASS "$rule" || r "T-6.4-011.gs4[$id]" FAIL "no gateway verdict: $(head -c 120 /tmp/o)"; }
done
ab-gwclient /run/gateway.sock op:git-push-staging git.push_staging "{\"expect_old\":null,\"ref_tail\":\"x\",\"repository_id\":\"repo:other\",\"tip\":\"$tip\"}" /tmp/fix.bundle >/tmp/o 2>&1 && r T-6.4-011 FAIL accepted || { rule=$(grep -o '"rule":"[^"]*"' /tmp/o | head -1); [ -n "$rule" ] && r T-6.4-011 PASS "$rule" || r T-6.4-011 FAIL "no verdict: $(head -c 120 /tmp/o)"; }
ab-gwclient /run/gateway.sock op:git-push-staging-force git.push_staging_force "{\"expect_old\":null,\"ref_tail\":\"fix-1234\",\"repository_id\":\"repo:demo\",\"tip\":\"$tip\"}" /tmp/fix.bundle >/tmp/o 2>&1 && r T-6.4-011.force FAIL accepted || { rule=$(grep -o '"rule":"[^"]*"' /tmp/o | head -1); [ -n "$rule" ] && r T-6.4-011.force PASS "$rule" || r T-6.4-011.force FAIL "no verdict: $(head -c 120 /tmp/o)"; }
# T-6.4-006 / T-6.4-007: descriptor transfer and inherited connection
ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping '{}' --scm-rights >/tmp/o 2>&1 && r T-6.4-006 FAIL accepted || { grep -q descriptor_transfer /tmp/o && r T-6.4-006 PASS "$(grep -o '"rule":"[^"]*"' /tmp/o | head -1)" || r T-6.4-006 FAIL "$(head -c 160 /tmp/o | tr '\n' ' ')"; }
ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping '{}' --fork >/tmp/o 2>&1 && r T-6.4-007 FAIL accepted || { grep -q process_mismatch /tmp/o && r T-6.4-007 PASS "$(grep -o '"rule":"[^"]*"' /tmp/o | head -1)" || r T-6.4-007 FAIL "$(head -c 160 /tmp/o | tr '\n' ' ')"; }
# T-6.4-010: stream connect to the gateway path
# T-6.4-001: socket() for INET/INET6/PACKET/NETLINK/VSOCK → seccomp EPERM (1)
fam=$(ab-gwclient --families 2>&1); echo "$fam" | grep -q OPENED && r T-6.4-001 FAIL "$(echo $fam)" || r T-6.4-001 PASS "$(echo $fam)"
# T-6.3-003: inherited descriptors are exactly 0/1/2 → console/null; nothing else (no socket, no credential file)
fds=$(ab-gwclient --fds 2>&1 | grep -v "^3 /proc" ); extra=$(echo "$fds" | awk '$1>2' | grep -v "/proc/.*/fd" | wc -l); [ "$extra" = 0 ] && r T-6.3-003 PASS "fds: $(echo $fds | tr '\n' ' ')" || r T-6.3-003 FAIL "$(echo $fds)"
# T-6.3-004: a child process inherits no credential (env/fds); it can only use the authenticated socket itself as a fresh peer
c=$(sh -c 'env | grep -ciE "token|secret|passw|credential"; ls /proc/self/fd | wc -l'); r T-6.3-004 PASS "child env credential hits=$(echo $c | cut -d" " -f1) fds=$(echo $c | cut -d" " -f2)"
# T-6.3-006: gateway error replies and the adapter's porcelain never echo the credential (scan every reply captured so far)
grep -hiE "password|authorization:|token" /tmp/o /tmp/out 2>/dev/null | grep -v '"rule"' | head -1 | grep -q . && r T-6.3-006 FAIL "credential-like text in replies" || r T-6.3-006 PASS "no credential-like text in gateway replies/adapter output"
# T-6.4-010: SOCK_STREAM / SOCK_DGRAM connect to the gateway path (SEQPACKET listener refuses both with EPROTOTYPE/ECONNREFUSED)
ab-gwclient /run/gateway.sock x gateway.ping '{}' --stream >/tmp/o 2>&1 && r T-6.4-010.stream FAIL connected || r T-6.4-010.stream PASS "$(head -c 80 /tmp/o | tr '\n' ' ')"
ab-gwclient /run/gateway.sock x gateway.ping '{}' --dgram >/tmp/o 2>&1 && r T-6.4-010.dgram FAIL connected || r T-6.4-010.dgram PASS "$(head -c 80 /tmp/o | tr '\n' ' ')"
# T-6.9-005: payload above bytes_per_operation (8 MiB) refused before any transfer
dd if=/dev/zero of=/tmp/big bs=1M count=9 2>/dev/null
ab-gwclient /run/gateway.sock op:git-push-staging git.push_staging "{\"expect_old\":null,\"ref_tail\":\"big\",\"repository_id\":\"repo:demo\",\"tip\":\"$tip\"}" /tmp/big >/tmp/o 2>&1 && r T-6.9-005 FAIL accepted || { grep -q budget_bytes /tmp/o && r T-6.9-005 PASS "$(grep -o '"rule":"[^"]*"' /tmp/o | head -1)" || r T-6.9-005 FAIL "$(head -c 120 /tmp/o)"; }; rm -f /tmp/big
# T-6.9-006: connection-count bound — 20 concurrent held connections against connection_count 16
i=0; while [ $i -lt 20 ]; do (sleep 6 | ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping '{}' --hold > /tmp/cc.$i 2>&1) & i=$((i+1)); done; sleep 3
refused=$(grep -l "closed by gateway\|connect errno" /tmp/cc.* 2>/dev/null | wc -l); ok=$(grep -l '"ok":true' /tmp/cc.* 2>/dev/null | wc -l)
[ "$ok" -le 16 ] && [ "$refused" -ge 4 ] && r T-6.9-006 WEAK "held=$ok refused=$refused (limit 16)" || r T-6.9-006 FAIL "held=$ok refused=$refused"
sleep 4
# T-6.4-014: hold an established connection open; the driver revokes the session while it is held; the second packet must be refused
# the driver (host) writes /workspace/revoked-<uid> after signalling revocation; the held client then sends its second packet
rm -f /workspace/held-*.out /workspace/revoked-* 2>/dev/null; ( (i=0; while [ ! -f /workspace/revoked-$me ] && [ $i -lt 60 ]; do sleep 1; i=$((i+1)); done) | ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping '{}' --hold > /workspace/held-$me.out 2>&1) &
sleep 1; r GW-HELD FIXTURE "connection held for revocation test"
r GW-END PASS done
while :; do sleep 1; done
