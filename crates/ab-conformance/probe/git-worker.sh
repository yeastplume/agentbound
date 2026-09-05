#!/bin/sh
# In-session Git worker (runtime:git-worker). Produces commits in the workspace clone with no remote and no credential,
# bundles them, and submits the bundle over the projected gateway socket as the typed operation op:git-push-staging.
# Emits GW <row> PASS|FAIL <detail> lines on stdout (console.log) for the conformance driver.
r() { echo "GW $1 $2 $3"; }
export GIT_AUTHOR_NAME=agent GIT_AUTHOR_EMAIL=agent@session.invalid GIT_COMMITTER_NAME=agent GIT_COMMITTER_EMAIL=agent@session.invalid
cd /workspace || exit 1
[ -S /run/gateway.sock ] && r T-6.4-003.projected PASS "gateway socket node present" || r T-6.4-003.projected FAIL "no socket node"
# T-6.3-001/002: no credential anywhere the session can see
env | grep -qiE 'token|secret|passw|credential' && r T-6.3-001 FAIL "$(env | grep -iE 'token|secret|passw|credential' | cut -d= -f1)" || r T-6.3-001 PASS "no credential in environment"
[ -r /var/lib/agentbound/gateway/credential ] && r T-6.3-002 FAIL "credential readable" || r T-6.3-002 PASS "credential path absent/unreadable"
me=$(id -u); w=work-$me; rm -rf $w 2>/dev/null; git init -q $w; cd $w
# T-6.1-007.sibling: another session's clone is not modifiable by this identity
for d in /workspace/work-*; do [ "$d" = "/workspace/$w" ] && continue; touch $d/x 2>/dev/null && r T-6.1-007.sibling FAIL "wrote $d" || r T-6.1-007.sibling PASS "sibling $d not writable"; break; done
echo "fix for issue 1234 $(date +%s)" > fix.txt; git add fix.txt; git -c commit.gpgsign=false commit -q -m "fix issue 1234"
git remote | grep -q . && r GS-1 FAIL "remote present" || r GS-1 PASS "no remote"
git bundle create -q /tmp/fix.bundle HEAD 2>/dev/null || git bundle create /tmp/fix.bundle HEAD >/dev/null 2>&1
tip=$(git rev-parse HEAD); r bundle PASS "tip=$tip bytes=$(stat -c %s /tmp/fix.bundle)"
# D-10/D-13: the granted typed operation
ab-gwclient /run/gateway.sock op:git-push-staging git.push_staging "{\"expect_old\":null,\"ref_tail\":\"fix-1234\",\"repository_id\":\"repo:demo\",\"tip\":\"$tip\"}" /tmp/fix.bundle > /tmp/out 2>&1; rc=$?
cat /tmp/out; [ $rc = 0 ] && r D-10 PASS "push_staging accepted rc=0" || r D-10 FAIL "rc=$rc"
# GS-4 refusal set through the gateway: other ref names, traversal, other session namespace, force operation
for tail in "../main" "main:refs/heads/main" "+fix" "fix.lock" "a b" "" "refs/heads/main"; do
  ab-gwclient /run/gateway.sock op:git-push-staging git.push_staging "{\"expect_old\":null,\"ref_tail\":\"$tail\",\"repository_id\":\"repo:demo\",\"tip\":\"$tip\"}" /tmp/fix.bundle >/tmp/o 2>&1 && r "GS-4[$tail]" FAIL "accepted" || { rule=$(grep -o '"rule":"[^"]*"' /tmp/o | head -1); [ -n "$rule" ] && r "GS-4[$tail]" PASS "$rule" || r "GS-4[$tail]" FAIL "no gateway verdict: $(head -c 120 /tmp/o)"; }
done
ab-gwclient /run/gateway.sock op:git-push-staging git.push_staging "{\"expect_old\":null,\"ref_tail\":\"x\",\"repository_id\":\"repo:other\",\"tip\":\"$tip\"}" /tmp/fix.bundle >/tmp/o 2>&1 && r T-6.4-011 FAIL accepted || { rule=$(grep -o '"rule":"[^"]*"' /tmp/o | head -1); [ -n "$rule" ] && r T-6.4-011 PASS "$rule" || r T-6.4-011 FAIL "no verdict: $(head -c 120 /tmp/o)"; }
ab-gwclient /run/gateway.sock op:git-push-staging-force git.push_staging_force "{\"expect_old\":null,\"ref_tail\":\"fix-1234\",\"repository_id\":\"repo:demo\",\"tip\":\"$tip\"}" /tmp/fix.bundle >/tmp/o 2>&1 && r GS-8.force FAIL accepted || { rule=$(grep -o '"rule":"[^"]*"' /tmp/o | head -1); [ -n "$rule" ] && r GS-8.force PASS "$rule" || r GS-8.force FAIL "no verdict: $(head -c 120 /tmp/o)"; }
# T-6.4-006 / T-6.4-007: descriptor transfer and inherited connection
ab-gwclient /run/gateway.sock op:git-push-staging gateway.ping '{}' --scm-rights >/tmp/o 2>&1 && r T-6.4-006 FAIL accepted || { grep -q descriptor_transfer /tmp/o && r T-6.4-006 PASS "$(grep -o '"rule":"[^"]*"' /tmp/o | head -1)" || r T-6.4-006 FAIL "$(head -c 160 /tmp/o | tr '\n' ' ')"; }
ab-gwclient /run/gateway.sock op:git-push-staging gateway.ping '{}' --fork >/tmp/o 2>&1 && r T-6.4-007 FAIL accepted || { grep -q process_mismatch /tmp/o && r T-6.4-007 PASS "$(grep -o '"rule":"[^"]*"' /tmp/o | head -1)" || r T-6.4-007 FAIL "$(head -c 160 /tmp/o | tr '\n' ' ')"; }
# T-6.4-010: stream connect to the gateway path
r GW-END PASS done
while :; do sleep 1; done
