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
import random
import subprocess
import sys
import time

REQ = "/tmp/d12-req.json"
STORE = "/var/lib/agentbound/audit/events.jsonl"
WS = "/var/lib/agentbound/workspaces/eng"
SESSIONS = 8
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
    deadline = time.time() + 1200
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
    res["valid"] = not incomplete and res["G"] == res["expected_G"]

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
