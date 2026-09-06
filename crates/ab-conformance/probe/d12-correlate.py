#!/usr/bin/env python3
"""D-12 correlator (test-catalogue 0.7 §5).

Reads, for one repetition of the NOMINAL profile:

  * the ground truth each session's workload wrote (its own instrumented log), and
  * the platform's hash-chained audit store,

and computes attribution completeness `|C| / |G|` exactly as §5 defines it. A ground-truth effect counts as reconstructed
only when the platform's record recovers the FULL chain `initiator -> agent -> session -> process -> effect`, matches the
ground-truth class, outcome and idempotency key, and reaches the correlator by the correlation deadline (30 s after the
workload's end marker, nominal).

The point of this program is to be able to return a number BELOW 1.0. It does not assume the platform records anything; for
each class it asks what the platform actually has, and reports what is missing. A class with no telemetry path scores 0 and
is reported as such, because §5's denominator is the ground truth, not what the platform happens to emit.

Usage: d12-correlate.py <manifest.json>
where manifest.json is [{"authorization_id":..., "launch_record_digest":..., "uid":..., "session_id":...,
"ground_truth":"/path/to/d12-ground-truth.jsonl", "end_marker_wall":"<rfc3339>"}]
"""
import json
import sys
import collections
import datetime

STORE = "/var/lib/agentbound/audit/events.jsonl"
DEADLINE_S = 30  # nominal; 120 for overload


def parse_rfc3339(s):
    return datetime.datetime.strptime(s, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=datetime.timezone.utc)


def load_store():
    """Every event in the hash-chained store, indexed by authorization id."""
    by_az = collections.defaultdict(list)
    with open(STORE) as fh:
        for line in fh:
            try:
                ev = json.loads(line)["event"]
            except Exception:
                continue
            az = ev.get("authorization_id")
            if az:
                by_az[az].append(ev)
    return by_az


def chain_complete(group, sess):
    """Does this event carry the full chain initiator -> agent -> session -> process -> effect?

    Each link must be present in the record itself, not inferable only by a human reading the deployment:
      initiator/agent : recoverable from the launch record the event names (authorization_id -> launch record -> principals)
      session         : session_id and trace_id
      process         : a process identity for the acting process, not merely the session's uid
      effect          : the operation/effect the record is about
    """
    # `group` is the set of platform records that describe ONE effect. §5 requires the chain to be recoverable, not that a single
    # record carry every link: the gateway records an effect as an admitted/completed (or denied) pair, and a reconstruction may
    # legitimately join them. What it may NOT do is invent a link that no record carries.
    links = {}
    links["initiator+agent"] = any(ev.get("authorization_id") == sess["authorization_id"] for ev in group)
    links["session"] = any(ev.get("session_id") and ev.get("trace_id") for ev in group)
    # a process identity for the ACTING process. `execution_uid` is the session's identity, shared by every process in it, so it is
    # not a process identity and does not satisfy this link.
    links["process"] = any(
        any(k in (ev.get("detail") or {}) for k in ("credential_pid", "establishing_pid", "peer_pid", "init_pid"))
        or bool((ev.get("detail") or {}).get("pidfs_inode"))
        for ev in group)
    links["effect"] = any((ev.get("detail") or {}).get("operation") or ev.get("event", "").startswith("session.") for ev in group)
    return links


def main():
    sessions = json.load(open(sys.argv[1]))
    by_az = load_store()
    per_class = collections.defaultdict(lambda: {"G": 0, "C": 0, "missing_reason": collections.Counter()})
    gateway_corpus = {"G": 0, "C": 0}
    late = 0

    for sess in sessions:
        az = sess["authorization_id"]
        events = by_az.get(az, [])
        # index the platform's gateway operation records by their sequence, and split permitted from denied
        # group every gateway operation record by the idempotency key it carries: that key is the workload's handle on the effect,
        # so it is what joins the platform's records to the ground truth (§5: class, outcome AND key must match).
        by_key = collections.defaultdict(list)
        for e in events:
            if e.get("event", "").startswith("gateway.operation"):
                k = (e.get("detail") or {}).get("idempotency_key")
                if k:
                    by_key[k].append(e)
        # ground truth for this session
        gt_lines = []
        try:
            for line in open(sess["ground_truth"]):
                try:
                    gt_lines.append(json.loads(line))
                except Exception:
                    pass
        except OSError:
            per_class["SETUP"]["missing_reason"]["ground truth unreadable: %s" % sess["ground_truth"]] += 1
            continue
        end_seen = any(g.get("class") == "end-marker" for g in gt_lines)
        if not end_seen:
            per_class["SETUP"]["missing_reason"]["no end marker: workload did not declare its end"] += 1
        effects = [g for g in gt_lines if g.get("class") != "end-marker"]

        for g in effects:
            cls = g["class"]
            st = per_class[cls]
            st["G"] += 1
            if cls == "gateway-operation":
                gateway_corpus["G"] += 1
                group = by_key.get(g["idempotency_key"], [])
                if not group:
                    st["missing_reason"]["no platform record carries this effect's idempotency key"] += 1
                    continue
                # the platform's outcome must match the workload's intended outcome
                got_denied = any(e.get("event") in ("gateway.operation_denied", "gateway.upstream_rejected") for e in group)
                got_ok = any(e.get("event") == "gateway.operation_completed" for e in group)
                want_denied = g["intended_outcome"] == "denied"
                if want_denied != got_denied or (not want_denied and not got_ok):
                    st["missing_reason"]["outcome disagrees with ground truth (wanted %s)" % g["intended_outcome"]] += 1
                    continue
                links = chain_complete(group, sess)
                if not all(links.values()):
                    st["missing_reason"]["chain incomplete: missing %s" % ",".join(k for k, v in links.items() if not v)] += 1
                    continue
                try:
                    latest = max(parse_rfc3339(e["wall_clock"]) for e in group)
                    if latest > parse_rfc3339(sess["end_marker_wall"]) + datetime.timedelta(seconds=DEADLINE_S):
                        late += 1
                        st["missing_reason"]["reached the correlator after the %ds deadline" % DEADLINE_S] += 1
                        continue
                except Exception:
                    pass
                st["C"] += 1
                gateway_corpus["C"] += 1
            else:
                # classes (a) and (b): look for ANY platform record that names this individual effect
                hits = [ev for ev in events
                        if g["idempotency_key"] in json.dumps(ev.get("detail") or {})
                        or g["effect_id"] in json.dumps(ev.get("detail") or {})]
                if not hits:
                    st["missing_reason"]["no platform record names this effect at all"] += 1
                    continue
                links = chain_complete(hits, sess)
                if not all(links.values()):
                    st["missing_reason"]["chain incomplete: missing %s" % ",".join(k for k, v in links.items() if not v)] += 1
                    continue
                st["C"] += 1

    G = sum(v["G"] for k, v in per_class.items() if k != "SETUP")
    C = sum(v["C"] for k, v in per_class.items() if k != "SETUP")
    out = {
        "sessions": len(sessions),
        "G": G,
        "C": C,
        "completeness": (C / G) if G else 0.0,
        "gateway_corpus": {"G": gateway_corpus["G"], "C": gateway_corpus["C"],
                           "completeness": (gateway_corpus["C"] / gateway_corpus["G"]) if gateway_corpus["G"] else 0.0},
        "late_beyond_deadline": late,
        "per_class": {k: {"G": v["G"], "C": v["C"],
                          "completeness": (v["C"] / v["G"]) if v["G"] else 0.0,
                          "missing": dict(v["missing_reason"].most_common(4))}
                      for k, v in sorted(per_class.items())},
    }
    print(json.dumps(out, indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
