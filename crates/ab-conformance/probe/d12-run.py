#!/usr/bin/env python3
"""Run one repetition of the D-12 NOMINAL profile: 8 concurrent sessions, 230 effects each, then correlate.

test-catalogue 0.7 §5 pre-registers this shape, and §4 requires the seed for a repetition to be
`SHA-256(test_id ‖ repetition_index)` truncated to 64 bits. The seed is recorded and used for the only thing that is
legitimately variable here — the order in which the sessions are started — so a repetition is reproducible.

Usage: d12-run.py <repetition_index> [out.json]
"""
import datetime
import hashlib
import json
import os
import shutil
import random
import subprocess
import sys
import time

REQ = "/tmp/d12-req.json"
STORE = "/var/lib/agentbound/audit/events.jsonl"
WS = "/var/lib/agentbound/workspaces/eng"
SESSIONS = 8
DURATION_S = 300          # test-catalogue §5 NOMINAL profile duration
CORRELATION_DEADLINE_S = 30   # §5 nominal correlation deadline, measured from the workload end marker
EFFECTS_PER_SESSION = 230


def sh(cmd):
    return subprocess.run(["sh", "-c", cmd], capture_output=True, text=True).stdout


def lifecycle(op, body):
    """One lifecycle call, using the same wire envelope as every other component."""
    import socket
    s = socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET)
    s.settimeout(20)
    s.connect("/run/agentbound/lifecycle.sock")
    s.send(json.dumps({"body": body, "idempotency_key": "d12-%d" % time.time_ns(), "op": op,
                       "v": "agentbound.wire.v0.1"}, separators=(",", ":"), sort_keys=True).encode())
    return json.loads(s.recv(1 << 20))


def main():
    t0 = time.time()
    rep = int(sys.argv[1])
    out_path = sys.argv[2] if len(sys.argv) > 2 else "/tmp/d12-rep-%d.json" % rep
    seed_hex = hashlib.sha256(("D-12" + str(rep)).encode()).hexdigest()[:16]
    rng = random.Random(int(seed_hex, 16))

    sh("rm -f %s/d12-gt-*.jsonl; rm -rf %s/d12-2*" % (WS, WS))
    order = list(range(SESSIONS))
    rng.shuffle(order)

    # The profile requires eight CONCURRENT sessions. `agentbound request` returns only after construction and activation, so a
    # sequential loop would produce eight sessions that barely overlap — measuring something the profile does not describe. All eight
    # are started at once, in the seeded order, and each writes its reply to its own file.
    procs = []
    for slot in order:
        out = "/tmp/d12-launch-%d.json" % slot
        procs.append((slot, out, subprocess.Popen(
            ["sh", "-c", "su -s /bin/sh bob -c 'agentbound request %s' </dev/null > %s 2>&1" % (REQ, out)])))
    for _, _, pr in procs:
        try:
            pr.wait(timeout=600)
        except subprocess.TimeoutExpired:
            pr.kill()
    launched = []
    for slot, out, _ in procs:
        line = ""
        try:
            line = [l for l in open(out).read().splitlines() if l.strip()][-1]
        except Exception:
            pass
        try:
            v = json.loads(line)
        except Exception:
            launched.append({"error": ("slot %d: " % slot) + line[:180]})
            continue
        if "launch_record_digest" not in v:
            launched.append({"error": ("slot %d: " % slot) + line[:180]})
            continue
        launched.append(v)

    # Wait for every session's workload to declare its end marker. The bound is generous because the gateway serves one request at a
    # time by design, so eight concurrent sessions' gateway phases serialize: measured at ~200 s for 72 operations. A run that hits
    # this bound is reported as incomplete (sessions_incomplete) rather than scored, because reading a workload's log while it is
    # still writing would put effects in the denominator whose deadline has not expired — §5 counts only effects whose deadlines do.
    # The profile's duration is 300 s. At 20 effects/s aggregate the 1 840 effects take ~92 s per session, so a run that has not
    # declared every end marker by 300 s after the last launch is outside the profile and is recorded as such (not scored as loss).
    t_launched = time.time()
    deadline = t_launched + DURATION_S
    while time.time() < deadline:
        done = 0
        for v in launched:
            if "uid" not in v:
                continue
            gt = "%s/d12-gt-%s.jsonl" % (WS, v["uid"])
            try:
                if any(json.loads(l).get("class") == "end-marker" for l in open(gt)):
                    done += 1
            except OSError:
                pass
        if done == len([v for v in launched if "uid" in v]):
            break
        time.sleep(2)
    incomplete = []
    for v in launched:
        if "uid" not in v:
            continue
        gt = "%s/d12-gt-%s.jsonl" % (WS, v["uid"])
        try:
            if not any(json.loads(l).get("class") == "end-marker" for l in open(gt)):
                incomplete.append(v["uid"])
        except OSError:
            incomplete.append(v["uid"])
    end_marker_wall = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    # §5: a reconstruction counts only if it reaches the correlator by the correlation deadline (30 s after the end marker). The
    # audit receiver appends asynchronously, so the correlator runs exactly at the deadline — neither early (which would under-count)
    # nor late (which would let records that missed the deadline count). Recorded so the evidence shows the deadline was honoured.
    time.sleep(CORRELATION_DEADLINE_S)
    correlator_wall = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    # audit-loss snapshot (R-AUD-3): any `lost` counter present at correlation time is part of the evidence, never silently absent
    loss_snapshot = sh("grep -h '\"kind\":\"audit\\.' %s 2>/dev/null | grep -c lost || true" % STORE).strip()

    # resolve each session's authorization id from the store
    az_by_lrd = {}
    sid_by_lrd = {}
    with open(STORE) as fh:
        for line in fh:
            try:
                e = json.loads(line)["event"]
            except Exception:
                continue
            lrd = e.get("launch_record_digest")
            if lrd:
                az_by_lrd[lrd] = e.get("authorization_id")
                if e.get("session_id"):
                    sid_by_lrd[lrd] = e.get("session_id")

    manifest = []
    for v in launched:
        if "launch_record_digest" not in v:
            continue
        lrd = v["launch_record_digest"]
        manifest.append({"authorization_id": az_by_lrd.get(lrd), "launch_record_digest": lrd,
                         "uid": v["uid"], "session_id": sid_by_lrd.get(lrd),
                         "ground_truth": "%s/d12-gt-%s.jsonl" % (WS, v["uid"]),
                         "end_marker_wall": end_marker_wall})
    json.dump(manifest, open("/tmp/d12-manifest.json", "w"))

    res = json.loads(sh("d12-correlate.py /tmp/d12-manifest.json"))
    res["repetition"] = rep
    res["seed"] = seed_hex
    res["start_order"] = order
    res["sessions_launched"] = len(manifest)
    res["launch_errors"] = [v.get("error") for v in launched if "error" in v]
    res["expected_G"] = SESSIONS * EFFECTS_PER_SESSION
    # a session still writing its log when the correlator ran has not reached its end marker, so its effects' deadlines have not
    # expired and it must not be scored (§5). Recording this makes an under-run visible instead of looking like attribution loss.
    res["sessions_incomplete"] = incomplete
    res["profile"] = {"sessions": SESSIONS, "effects_per_session": EFFECTS_PER_SESSION, "aggregate_rate_per_s": 20, "duration_s": DURATION_S,
                      "correlation_deadline_s": CORRELATION_DEADLINE_S, "mix": "200 local create/modify, 20 process lifecycle, 8 push-staging-ref permitted + 2 denied"}
    # integer milliseconds, not rounded seconds: this file must stay readable by the canonical JSON parser, which rejects floats
    res["timing"] = {"launch_wall_ms": int((t_launched - t0) * 1000), "workload_wall_ms": int((time.time() - CORRELATION_DEADLINE_S - t_launched) * 1000),
                     "end_marker_wall": end_marker_wall, "correlator_wall": correlator_wall, "audit_lost_records_at_correlation": loss_snapshot}
    # `valid` means the run realised the profile: all 8 sessions launched, all 8 declared their end within the 300 s window, and the
    # denominator is exactly the profile's. An invalid repetition is an infrastructure abort — retained as evidence, never scored, and
    # never quietly replaced by a rerun without the abort being kept alongside it.
    res["valid"] = len(manifest) == SESSIONS and not incomplete and res["G"] == res["expected_G"]
    # retain every input of the measurement next to the result so the number is reproducible from the bundle alone
    keep = os.path.join(os.path.dirname(os.path.abspath(out_path)), "d12-rep-%d-artifacts" % rep)
    os.makedirs(keep, exist_ok=True)
    for m in manifest:
        try:
            shutil.copy(m["ground_truth"], keep)
        except OSError:
            pass
    shutil.copy("/tmp/d12-manifest.json", keep)
    for slot in range(SESSIONS):
        try:
            shutil.copy("/tmp/d12-launch-%d.json" % slot, keep)
        except OSError:
            pass
    res["artifacts_dir"] = keep

    for v in launched:
        if "launch_record_digest" in v:
            try:
                lifecycle("terminate", {"launch_record_digest": v["launch_record_digest"], "reason": "d12"})
            except Exception:
                pass

    json.dump(res, open(out_path, "w"), indent=1, sort_keys=True)
    print(json.dumps({k: res[k] for k in ("repetition", "seed", "sessions_launched", "sessions_incomplete", "valid",
                                          "G", "C", "completeness", "gateway_corpus", "expected_G")}, sort_keys=True))


if __name__ == "__main__":
    main()
