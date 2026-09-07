#!/bin/sh
# Run the full D-12 NOMINAL profile: N = 10 seeded repetitions (test-catalogue 0.7 §5), retaining EVERY attempt.
#
# Output: /var/lib/agentbound/evidence/d12/rep-<n>.json — the file the conformance runner's D-12 row computes from — plus
# rep-<n>-artifacts/ holding the eight workload ground-truth logs, the eight launch replies, and the correlator manifest, and
# rep-<n>.log holding the driver's stdout. An infrastructure abort (sessions that did not launch, did not finish inside the 300 s
# window, or a denominator that is not the profile's) produces a rep-<n>.json with "valid": false. It is NEVER re-run in place:
# the abort stays in the evidence, and the D-12 row counts only valid repetitions toward the required ten.
#
# The seed for repetition n is SHA-256("D-12" || n)[:16], per test-catalogue §4 — d12-run.py derives it, and records it in the result.
set -u
OUT=/var/lib/agentbound/evidence/d12
mkdir -p "$OUT"
i=1
while [ $i -le 10 ]; do
  if [ -e "$OUT/rep-$i.json" ]; then
    echo "rep $i: result already present, not overwritten (remove the directory to start a fresh measurement)"; i=$((i+1)); continue
  fi
  echo "=== D-12 repetition $i  $(date -u +%FT%TZ)"
  d12-run.py "$i" "$OUT/rep-$i.json" > "$OUT/rep-$i.log" 2>&1
  tail -1 "$OUT/rep-$i.log"
  i=$((i+1))
done
python3 - "$OUT" <<'PY'
import json, sys, glob
d = sys.argv[1]; reps = [json.load(open(f)) for f in sorted(glob.glob(d + "/rep-*.json"))]
valid = [r for r in reps if r.get("valid")]
G = sum(r["G"] for r in valid); C = sum(r["C"] for r in valid)
print(json.dumps({"repetitions": len(reps), "valid": len(valid), "G": G, "C": C, "completeness": (C / G) if G else None,
                  "gateway": [(r["gateway_corpus"]["C"], r["gateway_corpus"]["G"]) for r in valid],
                  "invalid": [{"rep": r["repetition"], "launched": r.get("sessions_launched"), "incomplete": r.get("sessions_incomplete"), "errors": r.get("launch_errors")} for r in reps if not r.get("valid")]}, indent=1))
PY
