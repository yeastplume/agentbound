#!/bin/sh
# D-12 attribution-completeness workload (test-catalogue 0.7 §5, NOMINAL profile).
#
# Per session this emits exactly 230 in-scope atomic effects in the three ontology classes:
#   (a) 200 local object create/modify within the session's world
#   (b)  20 process lifecycle events (fork/exec/exit)
#   (c)  10 gateway operations — 8 permitted, 2 denied
#
# Pacing: the profile is 20 effects/s AGGREGATE over 8 sessions, i.e. 2.5 effects/s per session — one effect every 400 ms.
# Each effect below is followed by a 0.4 s sleep, so a session's 230 effects take ~92 s and the eight sessions together emit at
# the specified aggregate rate. (The earlier worker ran unpaced, which the independent WP3.1 validation correctly called out.)
# The profile's 300 s duration is the run window the host-side driver enforces (d12-run.py); effects after the end marker are
# excluded by §5 regardless.
PACE=${D12_PACE:-0.4}
#
# It writes its OWN ground-truth log, because §5 requires the ground truth to be the instrumented workload log and not the
# platform's own record: using the platform's events as both ground truth and reconstruction would make the metric vacuous.
# Each line carries: atomic effect id, class, sequence number, idempotency key, process identity, session/trace id (as the
# session can see them), and the intended outcome. Denied operations are in-scope and MUST be present (§5).
#
# The log is written into the workspace, which the correlator on the host can read. That is a deliberate part of the fixture:
# the workload is trusted to report what it intended to do, and nothing else.

me=$(id -u)
GT=/workspace/d12-gt-$me.jsonl
: > "$GT"
seq_no=0

# Emit one ground-truth line. $1 class, $2 idempotency key, $3 intended outcome, $4 detail
gt() {
  seq_no=$((seq_no + 1))
  printf '{"class":"%s","effect_id":"effect:%s-%06d","idempotency_key":"%s","intended_outcome":"%s","detail":"%s","pid":%d,"seq":%d,"uid":%s,"t":%s}\n' \
    "$1" "$me" "$seq_no" "$2" "$3" "$4" "$$" "$seq_no" "$me" "$(date +%s)" >> "$GT"
  sleep "$PACE"
}

echo "D12 START uid=$me pid=$$"

# ---- class (a): 200 local object create/modify inside the session's world ----
# 150 creates and 50 modifies, so the class exercises both shapes.
i=0
while [ $i -lt 150 ]; do
  i=$((i + 1))
  f="/workspace/d12-$me/obj-$i"
  mkdir -p "/workspace/d12-$me"
  printf 'create %d\n' "$i" > "$f" 2>/dev/null
  gt local-object "d12-create-$me-$i" "$([ -f "$f" ] && echo ok || echo error)" "create $f"
done
i=0
while [ $i -lt 50 ]; do
  i=$((i + 1))
  f="/workspace/d12-$me/obj-$i"
  printf 'modify %d\n' "$i" >> "$f" 2>/dev/null
  gt local-object "d12-modify-$me-$i" "$([ -f "$f" ] && echo ok || echo error)" "modify $f"
done

# ---- class (b): 20 process lifecycle events ----
# Each iteration is one fork+exec+exit of a distinct short-lived child; the child's own pid is recorded, since the chain
# `initiator → agent → session → process → effect` must reach the individual process and not merely the session.
i=0
while [ $i -lt 20 ]; do
  i=$((i + 1))
  cpid=$( (exec /bin/sh -c 'echo $$') 2>/dev/null )
  gt process-lifecycle "d12-proc-$me-$i" "$([ -n "$cpid" ] && echo ok || echo error)" "fork+exec+exit child=$cpid"
done

# ---- class (c): 10 Git gateway operations — 8 `push-staging-ref` permitted, 2 denied (the profile's exact mix) ----
# Each permitted operation pushes a DISTINCT commit to a distinct staging ref tail, so the eight are eight real upstream effects and
# not one effect repeated. The commit is made in a scratch repository with no remote and no credential, bundled, and submitted as
# the typed operation op:git-push-staging over the projected socket — the same path the 1B git-worker uses.
export GIT_AUTHOR_NAME=agent GIT_AUTHOR_EMAIL=agent@session.invalid GIT_COMMITTER_NAME=agent GIT_COMMITTER_EMAIL=agent@session.invalid
# One INDEPENDENT single-commit repository per push. A single growing history would make each bundle carry all earlier objects
# (3, 6, 9 …), and the granted per-operation `objects` budget is 8 — so pushes 3-8 would be denied `budget_objects` and the run would
# measure the fixture's own budget rather than attribution. Eight separate one-commit repositories are eight real, distinct upstream
# effects, each 3 objects, well inside the reviewed grant.
i=0
while [ $i -lt 8 ]; do
  i=$((i + 1))
  w=/workspace/d12-$me/r-$i; rm -rf "$w"; git init -q "$w" 2>/dev/null
  ( cd "$w" && echo "d12 $me $i $(date +%s)" > f.txt && git add f.txt && git -c commit.gpgsign=false commit -q -m "d12 $me $i" ) 2>/dev/null
  tip=$(git -C "$w" rev-parse HEAD 2>/dev/null)
  git -C "$w" bundle create -q /tmp/d12-$i.bundle HEAD 2>/dev/null
  out=$(ab-gwclient /run/gateway.sock op:git-push-staging git.push_staging "{\"expect_old\":null,\"ref_tail\":\"d12-$me-$i\",\"repository_id\":\"repo:demo\",\"tip\":\"$tip\"}" /tmp/d12-$i.bundle --idem "d12-push-$me-$i" 2>&1)
  case "$out" in
    *'"ok":true'*) oc=ok ;;
    *) oc=error ;;
  esac
  gt gateway-operation "d12-push-$me-$i" "$oc" "git.push_staging refs/staging/d12-$me-$i tip=$tip"
done
# Denied: a push to a repository outside the operation's scope (rule scope), and a force-push operation the task does not hold
# (rule authority). Both denials are in-scope effects and must be reconstructed with their outcome.
tip=$(git -C /workspace/d12-$me/r-8 rev-parse HEAD 2>/dev/null)
ab-gwclient /run/gateway.sock op:git-push-staging git.push_staging "{\"expect_old\":null,\"ref_tail\":\"d12-$me-x\",\"repository_id\":\"repo:other\",\"tip\":\"$tip\"}" /tmp/d12-8.bundle --idem "d12-denied-$me-1" >/dev/null 2>&1
gt gateway-operation "d12-denied-$me-1" "denied" "git.push_staging repo:other (outside operation scope)"
ab-gwclient /run/gateway.sock op:git-push-staging-force git.push_staging_force "{\"expect_old\":null,\"ref_tail\":\"d12-$me-x\",\"repository_id\":\"repo:demo\",\"tip\":\"$tip\"}" /tmp/d12-8.bundle --idem "d12-denied-$me-2" >/dev/null 2>&1
gt gateway-operation "d12-denied-$me-2" "denied" "op:git-push-staging-force (not held by this task)"

# The declared end marker: §5 excludes effects after it, and the correlation deadline is measured from it.
printf '{"class":"end-marker","effect_id":"effect:%s-end","seq":%d,"uid":%s}\n' "$me" "$((seq_no + 1))" "$me" >> "$GT"
echo "D12 END uid=$me effects=$seq_no"
# stay alive so the host can read the workspace and terminate us deliberately
while :; do sleep 1; done
