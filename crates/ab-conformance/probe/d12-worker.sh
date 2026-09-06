#!/bin/sh
# D-12 attribution-completeness workload (test-catalogue 0.7 §5, NOMINAL profile).
#
# Per session this emits exactly 230 in-scope atomic effects in the three ontology classes:
#   (a) 200 local object create/modify within the session's world
#   (b)  20 process lifecycle events (fork/exec/exit)
#   (c)  10 gateway operations — 8 permitted, 2 denied
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
  printf '{"class":"%s","effect_id":"effect:%s-%06d","idempotency_key":"%s","intended_outcome":"%s","detail":"%s","pid":%d,"seq":%d,"uid":%s}\n' \
    "$1" "$me" "$seq_no" "$2" "$3" "$4" "$$" "$seq_no" "$me" >> "$GT"
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

# ---- class (c): 10 gateway operations, 8 permitted and 2 denied ----
# Permitted: gateway.ping, which the task holds and which has no side effect on the upstream repository.
i=0
while [ $i -lt 8 ]; do
  i=$((i + 1))
  out=$(ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping '{}' --idem "d12-ping-$me-$i" 2>&1)
  case "$out" in
    *pong*) oc=ok ;;
    *) oc=error ;;
  esac
  gt gateway-operation "d12-ping-$me-$i" "$oc" "gateway.ping"
done
# Denied: an operation the task does not hold. The denial is an in-scope effect and must be reconstructed too.
i=0
while [ $i -lt 2 ]; do
  i=$((i + 1))
  ab-gwclient /run/gateway.sock op:git-push-staging-force git.push_staging '{}' --idem "d12-denied-$me-$i" >/dev/null 2>&1
  gt gateway-operation "d12-denied-$me-$i" "denied" "op:git-push-staging-force (not held by this task)"
done

# The declared end marker: §5 excludes effects after it, and the correlation deadline is measured from it.
printf '{"class":"end-marker","effect_id":"effect:%s-end","seq":%d,"uid":%s}\n' "$me" "$((seq_no + 1))" "$me" >> "$GT"
echo "D12 END uid=$me effects=$seq_no"
# stay alive so the host can read the workspace and terminate us deliberately
while :; do sleep 1; done
