#!/usr/bin/env python3
"""Negative controls for the conformance suite (WP3.1 plan item 6).

A suite that reports 0 FAIL has shown that its assertions ran. It has NOT shown that they would notice if the property they
assert were false. This program establishes that second thing, which is the only thing that makes the first one evidence.

For each control it: applies one surgical mutation to the implementation source that removes exactly one enforcement, rebuilds,
deploys, runs the conformance suite, and checks that the rows assigned to that mutation **FAIL**. A mutation whose rows still
pass is itself a finding — it means those rows do not depend on the enforcement they claim to test — and is reported as
`CONTROL-FAILED`. The source tree is restored after every control, and the restoration is verified by digest.

Usage: negative-controls.py [control-name ...]     (default: all)
Run from the repository root, with the VM reachable as the conformance runs use it.
"""
import hashlib
import json
import os
import subprocess
import sys
import time

SSH = ("ssh -i /root/.ssh/id_ed25519_agentbound_dev -o UserKnownHostsFile=/tmp/kh "
       "-o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 root@10.20.44.12")

# Each control: the file, an exact literal to replace, its replacement, and the rows that MUST fail as a result.
# `expect_fail` is the discriminating claim. `also_ok` lists rows that may legitimately also fail as collateral (a mutation that
# disables a check often breaks neighbouring assertions); anything failing outside expect_fail ∪ also_ok is reported but does not
# by itself invalidate the control.
CONTROLS = [
    {
        "name": "inode-comparison-disabled",
        "why": "ADR-0002 Decision 2 / WP1 F-1: the pidfs inode is what distinguishes a recycled pid from the establishing "
               "process instance. With the comparison removed, a packet from a different process that reused the pid must be "
               "accepted — and T-6.4-009 must notice.",
        "file": "crates/agentbound-gateway/src/session.rs",
        "old": "    if now.pidfs_ino != est.pidfs_ino { return Some(format!(\"pidfs inode {} vs establishing {} (pid {} recycled to another process instance)\", now.pidfs_ino, est.pidfs_ino, est.pid)); }\n"
               "    if now.pidns != est.pidns { return Some(format!(\"pidns {} vs establishing {}\", now.pidns, est.pidns)); }\n"
               "    if now.start_time != est.start_time { return Some(format!(\"start time {} vs establishing {} (inode equal — kernel invariant violated)\", now.start_time, est.start_time)); }\n",
        "new": "    let _ = (est, now); // NEGATIVE CONTROL: process-instance comparison removed\n",
        "expect_fail": [],
        "expect_unit_fail": ["session::tests::recycled_pid_rejected", "session::tests::pidns_change_rejected",
                             "session::tests::start_time_change_rejected"],
        "also_ok": [],
        "note": "T-6.4-009 deliberately does NOT depend on this comparison: the gateway polls each connection's peer pidfd, so the "
                "establisher's exit closes the connection before the recycled process exists. The live row asserts that composite "
                "defence; the comparison itself is asserted by the unit tests named here. A control that expected T-6.4-009 to fail "
                "was wrong about which assertion covers this enforcement — see the register.",
    },
    {
        "name": "credential-count-disabled",
        "why": "ADR-0002 Decision 2: exactly one SCM_CREDENTIALS per packet. With the count check removed, a packet carrying "
               "zero or several credential structures must be accepted — T-6.4-008 must notice.",
        "file": "crates/agentbound-gateway/src/session.rs",
        "old": "    if pk.creds.len() != 1 { return Err((wire::CLASS_INVALID, \"credential_count\", format!(\"{} SCM_CREDENTIALS\", pk.creds.len()), true)); }",
        "new": "    if pk.creds.is_empty() { return Err((wire::CLASS_INVALID, \"credential_count\", \"none\".to_string(), true)); } // NEGATIVE CONTROL: multiple credentials no longer refused",
        # Measured outcome: NOTHING fails, and that is the correct result. With SO_PASSCRED the kernel delivers exactly one
        # credential message whether the sender attached zero, one or several (verified on 6.12.107), so this branch cannot be
        # reached from any peer. The control is what established that; ADR-0002 0.10 now records it.
        "expect_fail": [],
        "expect_unreachable": True,
        "also_ok": [],
        "note": "This enforcement is unobservable on the pinned baseline: the kernel normalises the credential count under "
                "SO_PASSCRED. The check stays in the implementation (a transport that did not normalise would make it reachable), "
                "but no conformance row may claim to exercise it. T-6.4-008 was a false positive until item 6 — it ran as host root, "
                "so establish() refused every case on uid before any packet was read; it now uses an in-scope session-UID peer and "
                "asserts the credential-PID/process-instance comparison, which is what the per-packet path actually enforces.",
    },
    {
        "name": "scope-check-disabled",
        "why": "R-GW-3: the establishing process must live in the allocation's scope cgroup, so a process that shares the "
               "session's namespaces and uid but not its scope is not the session. With the check removed, T-6.4-005 must notice.",
        "file": "crates/agentbound-gateway/src/auth.rs",
        "old": "    if !inst.cgroup.contains(&format!(\"agentbound-{suffix}.scope\")) { return Err((\"scope_mismatch\", format!(\"cgroup {}\", inst.cgroup))); }",
        "new": "    let _ = suffix; // NEGATIVE CONTROL: scope-cgroup check removed",
        "expect_fail": ["T-6.4-005"],
        "also_ok": ["T-6.3-008"],
    },
    {
        "name": "budget-check-disabled",
        "why": "R-GW-7: the operations budget is checked before the operation is counted. With the check removed a session may "
               "exceed its declared operation count — T-6.9-008 and the T-6.8-011 budget rows must notice.",
        "file": "crates/agentbound-gateway/src/session.rs",
        "old": "    if used_ops >= max_ops { return Err((wire::CLASS_UNAUTHORIZED, \"budget_operations\", format!(\"{used_ops} of {max_ops} used\"), false)); }",
        "new": "    let _ = (used_ops, max_ops); // NEGATIVE CONTROL: operations budget no longer enforced",
        "expect_fail": ["T-6.9-008"],
        "also_ok": ["T-6.8-011", "D-14", "T-6.9-004"],
    },
    {
        "name": "uid-check-disabled",
        "why": "R-GW-3: a connection is authenticated to exactly one execution identity. With the peer-uid comparison removed, "
               "another session's uid may connect to this session's socket — T-6.3-008 must notice.",
        "file": "crates/agentbound-gateway/src/auth.rs",
        "old": "    if c.peer.uid != p.uid { return Err((\"uid_mismatch\", format!(\"peer uid {} allocation uid {}\", c.peer.uid, p.uid))); }",
        "new": "    let _ = &p.uid; // NEGATIVE CONTROL: peer-uid check removed",
        "expect_fail": ["T-6.3-008"],
        "also_ok": ["T-6.4-005", "T-6.4-001", "T-6.4-002"],
    },
    {
        "name": "idempotency-key-not-recorded",
        "why": "test-catalogue §5 makes the workload's idempotency key part of the correctness condition for a reconstruction. "
               "With the key omitted from the gateway's audit records, D-12's gateway corpus must drop from 100%.",
        "file": "crates/agentbound-gateway/src/session.rs",
        "old": "(\"idempotency_key\", Value::s(&idem)), (\"operation\", Value::s(&name)), (\"operation_seq\", Value::Int(op_seq)), (\"result\", body.clone())",
        "new": "(\"operation\", Value::s(&name)), (\"operation_seq\", Value::Int(op_seq)), (\"result\", body.clone())",
        # Measured: D-12 does discriminate on this. The correlator matches a reconstruction to ground truth by idempotency key, so
        # removing the key from the gateway's records drops the gateway corpus below 100% and D-12 fails.
        "expect_fail": ["D-12"],
        "also_ok": [],
    },
]


def sh(cmd, cwd=None, timeout=3600):
    r = subprocess.run(["bash", "-c", cmd], capture_output=True, text=True, cwd=cwd, timeout=timeout)
    return r.returncode, r.stdout, r.stderr


def digest_tree():
    """Digest of every tracked source file, so a restoration can be proven complete rather than assumed."""
    _, out, _ = sh("git ls-files -s crates deploy | sha256sum")
    return out.split()[0]


def apply_mutation(c):
    path = c["file"]
    src = open(path).read()
    if c["old"] not in src:
        return False, "mutation target not found in %s (source has changed; the control must be updated, not skipped)" % path
    if src.count(c["old"]) != 1:
        return False, "mutation target appears %d times in %s" % (src.count(c["old"]), path)
    open(path, "w").write(src.replace(c["old"], c["new"], 1))
    return True, "applied"


def run_unit_tests():
    """Returns the set of unit tests that FAILED, or None if the build itself failed."""
    rc, out, err = sh("crates/build.sh test 2>&1 | tail -80")
    blob = out + err
    if "error[E" in blob or "could not compile" in blob:
        return None, "build failed"
    failed = []
    for line in blob.splitlines():
        line = line.strip()
        if line.startswith("test ") and " ... FAILED" in line:
            failed.append(line.split()[1])
        if line.startswith("---- ") and " stdout ----" in line:
            failed.append(line.split()[1])
    return sorted(set(failed)), None


def run_suite():
    rc, out, err = sh("crates/build.sh build --release 2>&1 | grep -E '^error' -A6 | head -20")
    if "error" in out:
        return None, "build failed: " + out[:400]
    deploy = ("%s 'install -m 0755 /root/wp2/target/release/{agentbound-gateway,agentbound-lifecycle,agentbound-audit} "
              "/usr/local/bin/ && systemctl restart agentbound-audit agentbound-lifecycle agentbound-gateway && sleep 3'" % SSH)
    sh(deploy)
    rc, out, err = sh("%s 'timeout 3000 ab-conformance > /root/wp2/run.log 2>&1; "
                      "grep -E \"^(FAIL|assertions)\" /root/wp2/run.log'" % SSH, timeout=3600)
    return out, None


def failed_rows(suite_out):
    rows = []
    for line in (suite_out or "").splitlines():
        if line.startswith("FAIL "):
            rows.append(line.split()[1])
    return rows


def main():
    wanted = sys.argv[1:] or [c["name"] for c in CONTROLS]
    clean = digest_tree()
    results = []
    for c in CONTROLS:
        if c["name"] not in wanted:
            continue
        print("=== control: %s" % c["name"], flush=True)
        ok, msg = apply_mutation(c)
        if not ok:
            results.append({"control": c["name"], "verdict": "CONTROL-ERROR", "detail": msg})
            print("  CONTROL-ERROR %s" % msg, flush=True)
            continue
        try:
            if not c["expect_fail"] and c.get("expect_unit_fail"):
                # the claim is entirely about unit tests: build and test only
                unit_failed, unit_err = run_unit_tests()
                want_unit = c["expect_unit_fail"]
                missing_unit = [t for t in want_unit if not any(t in u for u in (unit_failed or []))]
                verdict = "CONTROL-OK" if unit_failed is not None and not missing_unit else "CONTROL-FAILED"
                results.append({"control": c["name"], "verdict": verdict, "why": c["why"], "note": c.get("note"),
                                "expected_to_fail": [], "actually_failed": [],
                                "expected_unit_to_fail": want_unit, "unit_actually_failed": unit_failed,
                                "did_not_fail": missing_unit, "collateral": []})
                print("  %s unit expected=%s got=%s" % (verdict, want_unit, unit_failed), flush=True)
                continue
            suite_out, err = run_suite()
            if err:
                results.append({"control": c["name"], "verdict": "CONTROL-ERROR", "detail": err})
                print("  CONTROL-ERROR %s" % err, flush=True)
                continue
            fails = failed_rows(suite_out)
            unit_failed, unit_err = run_unit_tests()
            want_unit = c.get("expect_unit_fail", [])
            missing = [r for r in c["expect_fail"] if r not in fails]
            missing_unit = [t for t in want_unit if not any(t in u for u in (unit_failed or []))]
            unexpected = [r for r in fails if r not in c["expect_fail"] and r not in c.get("also_ok", [])]
            if c.get("expect_unreachable"):
                # the claim is that no assertion depends on this branch because it cannot be reached; nothing failing proves it
                verdict = "CONTROL-OK-UNREACHABLE" if not fails else "CONTROL-FAILED"
            else:
                verdict = "CONTROL-OK" if not missing and not missing_unit else "CONTROL-FAILED"
            results.append({"control": c["name"], "verdict": verdict, "why": c["why"],
                            "note": c.get("note"),
                            "expected_to_fail": c["expect_fail"], "actually_failed": fails,
                            "expected_unit_to_fail": want_unit, "unit_actually_failed": unit_failed,
                            "did_not_fail": missing + missing_unit, "collateral": unexpected,
                            "assertions": [l for l in (suite_out or "").splitlines() if l.startswith("assertions")]})
            print("  %s rows expected=%s got=%s | unit expected=%s got=%s"
                  % (verdict, c["expect_fail"], fails, want_unit, unit_failed), flush=True)
        finally:
            sh("git checkout -- %s" % c["file"])
            back = digest_tree()
            if back != clean:
                results.append({"control": c["name"], "verdict": "CONTROL-ERROR",
                                "detail": "source tree not restored (digest %s != %s)" % (back[:12], clean[:12])})
                print("  CONTROL-ERROR source not restored", flush=True)
                break
    # rebuild and redeploy the clean tree so the host is not left mutated
    sh("crates/build.sh build --release")
    sh("%s 'install -m 0755 /root/wp2/target/release/{agentbound-gateway,agentbound-lifecycle,agentbound-audit} /usr/local/bin/ "
       "&& systemctl restart agentbound-audit agentbound-lifecycle agentbound-gateway'" % SSH)
    out = {"controls": results,
           "clean_tree_digest": clean,
           "restored": digest_tree() == clean}
    print(json.dumps(out, indent=1))
    open("/tmp/negative-controls.json", "w").write(json.dumps(out, indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
