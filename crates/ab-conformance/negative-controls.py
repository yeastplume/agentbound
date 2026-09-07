#!/usr/bin/env python3
"""Negative controls for the conformance suite (WP3.1 plan item 6).

A suite that reports 0 FAIL has shown that its assertions ran. It has NOT shown that they would notice if the property they
assert were false. This program establishes that second thing, which is the only thing that makes the first one evidence.

For each control it: applies one surgical mutation to the implementation source that removes exactly one enforcement, rebuilds,
deploys, runs the conformance suite, and checks that the rows assigned to that mutation **FAIL**. A mutation whose rows still
pass is itself a finding — it means those rows do not depend on the enforcement they claim to test — and is reported as
`CONTROL-FAILED`. The source tree is restored after every control, and the restoration is verified by a digest over the
working-tree CONTENT of every tracked file (not the git index, which does not change when a file is edited).

Verdicts: CONTROL-OK (the assigned rows failed), CONTROL-FAILED (they did not), CONTROL-ERROR (the control could not be run),
NOT-DISCRIMINATING-UNREACHABLE (the branch cannot be reached on the pinned kernel; nothing failing is consistent with that, and
this verdict is explicitly NOT a discriminating control). The process exits 1 unless every control is CONTROL-OK or
NOT-DISCRIMINATING-UNREACHABLE, the tree was restored, and the clean build was redeployed.

Evidence for every control is retained under $NC_EVIDENCE (default /tmp/negative-controls/<control>/): mutation.diff, build.log,
deploy.log, installed-digests-mutant.txt, suite.log, conformance-run.md, failed-rows.txt, unit-tests.log; plus clean-tree.txt and
installed-digests-{before,after}.txt at the top. Copy the whole directory into docs/evidence/.../raw/ when recording a run.

The script REFUSES to start unless crates/ and deploy/ are clean and committed, so the mutation is the only thing that differs from
the reviewed commit, and restores by writing the original bytes back (never `git checkout`, which would discard an operator's edits).

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
    {
        "name": "idempotency-outcomes-not-restored",
        "why": "component-interfaces §5: same key + same input returns the ORIGINAL result. If the gateway does not restore completed "
               "outcomes from the lifecycle store on activate/reconstruct, a repeated key after a gateway restart re-executes the "
               "operation (independent WP3.1 validation, finding 6). D4.7-idempotency-persist must see a second execution.",
        "file": "crates/agentbound-gateway/src/main.rs",
        "old": "pr.record = Some(b.clone()); pr.idem = outcomes; pr.used = used.clone();",
        "new": "pr.record = Some(b.clone()); pr.used = used.clone();",
        "expect_fail": ["D4.7-idempotency-persist"],
        "also_ok": [],
    },
]


def sh(cmd, cwd=None, timeout=3600):
    r = subprocess.run(["bash", "-c", cmd], capture_output=True, text=True, cwd=cwd, timeout=timeout)
    return r.returncode, r.stdout, r.stderr


def digest_tree():
    """SHA-256 over the CONTENT of every tracked file under crates/ and deploy/ (path + bytes), plus the list of untracked files.

    The earlier version hashed `git ls-files -s`, i.e. the INDEX entries — which do not change when a working-tree file is edited, so
    it could not detect a failed restoration at all (independent WP3.1 validation, finding 5). This one reads the working tree."""
    import hashlib
    h = hashlib.sha256()
    _, files, _ = sh("git ls-files -z crates deploy")
    for f in sorted(x for x in files.split("\0") if x):
        h.update(f.encode() + b"\0")
        with open(f, "rb") as fh:
            h.update(hashlib.sha256(fh.read()).digest())
    _, untracked, _ = sh("git ls-files -z --others --exclude-standard crates deploy")
    h.update(b"untracked:" + untracked.encode())
    return h.hexdigest()


def tree_is_clean():
    rc, out, _ = sh("git diff --exit-code --quiet -- crates deploy && git diff --cached --exit-code --quiet -- crates deploy; echo $?")
    _, untracked, _ = sh("git ls-files --others --exclude-standard crates deploy")
    return out.strip().endswith("0") and not untracked.strip()


def apply_mutation(c):
    """Returns (ok, message, original_bytes). The original bytes are what restoration writes back — never `git checkout`, which would
    also discard any pre-existing edit the operator had in that file."""
    path = c["file"]
    src = open(path).read()
    if c["old"] not in src:
        return False, "mutation target not found in %s (source has changed; the control must be updated, not skipped)" % path, None
    if src.count(c["old"]) != 1:
        return False, "mutation target appears %d times in %s" % (src.count(c["old"]), path), None
    open(path, "w").write(src.replace(c["old"], c["new"], 1))
    return True, "applied", src


def restore(c, original):
    if original is not None:
        open(c["file"], "w").write(original)


EVIDENCE = os.environ.get("NC_EVIDENCE", "/tmp/negative-controls")


def save(control, name, text):
    d = os.path.join(EVIDENCE, control)
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, name), "w") as f:
        f.write(text if text is not None else "")


def run_unit_tests(control):
    """Returns the set of unit tests that FAILED, or None if the build itself failed. Full output is retained."""
    rc, out, err = sh("crates/build.sh test 2>&1")
    blob = out + err
    save(control, "unit-tests.log", blob)
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


def installed_digests():
    _, out, _ = sh("%s 'cd /usr/local/bin && sha256sum agentbound-gateway agentbound-lifecycle agentbound-audit ab-conformance; "
                   "for b in agentbound-gateway agentbound-lifecycle agentbound-audit; do echo -n \"$b: \"; timeout 5 ./$b --provenance; done' 2>/dev/null" % SSH)
    return out


def deploy_mutant():
    """Deploy the mutated daemons. Every step's status is enforced: a control whose broken build never reached the host would
    otherwise report the clean binaries' behaviour as the mutant's."""
    cmd = ("%s 'set -e; systemctl stop agentbound-gateway agentbound-lifecycle agentbound-audit; "
           "install -m 0755 /root/wp2/target/release/agentbound-gateway /root/wp2/target/release/agentbound-lifecycle /root/wp2/target/release/agentbound-audit /usr/local/bin/; "
           "rm -f /run/agentbound/gateway.sock; systemctl start agentbound-audit agentbound-lifecycle agentbound-gateway; sleep 3; "
           "systemctl is-active agentbound-audit agentbound-lifecycle agentbound-gateway'" % SSH)
    rc, out, err = sh(cmd)
    return rc == 0 and out.split() == ["active"] * 3, out + err


def run_suite(control):
    rc, out, err = sh("crates/build.sh build --release 2>&1")
    save(control, "build.log", out + err)
    if "error[E" in out + err or "could not compile" in out + err:
        return None, "build failed (see build.log)"
    ok, log = deploy_mutant()
    save(control, "deploy.log", log)
    if not ok:
        return None, "deploy failed or a daemon did not come up (see deploy.log)"
    save(control, "installed-digests-mutant.txt", installed_digests())
    rc, out, err = sh("%s 'timeout 3000 ab-conformance > /root/wp2/run.log 2>&1; echo \"exit=$?\"; cat /root/wp2/run.log'" % SSH, timeout=3600)
    save(control, "suite.log", out + err)
    _, reg, _ = sh("%s 'cat /root/wp2/conformance-run.md'" % SSH)
    save(control, "conformance-run.md", reg)
    return out, None


def failed_rows(suite_out):
    rows = []
    for line in (suite_out or "").splitlines():
        if line.startswith("FAIL "):
            rows.append(line.split()[1])
    return rows


def main():
    wanted = sys.argv[1:] or [c["name"] for c in CONTROLS]
    os.makedirs(EVIDENCE, exist_ok=True)
    if not tree_is_clean():
        print("REFUSING: crates/ or deploy/ has uncommitted or untracked changes. Negative controls must start from a clean, committed "
              "tree so the mutation — and only the mutation — is what differs from the reviewed commit.", flush=True)
        sys.exit(2)
    _, head, _ = sh("git rev-parse HEAD")
    clean = digest_tree()
    save("", "clean-tree.txt", "commit=%s\ncontent_digest=%s\n" % (head.strip(), clean))
    save("", "installed-digests-before.txt", installed_digests())
    results = []
    any_failed = False
    for c in CONTROLS:
        if c["name"] not in wanted:
            continue
        print("=== control: %s" % c["name"], flush=True)
        ok, msg, original = apply_mutation(c)
        if not ok:
            results.append({"control": c["name"], "verdict": "CONTROL-ERROR", "detail": msg})
            print("  CONTROL-ERROR %s" % msg, flush=True)
            any_failed = True
            continue
        _, diff, _ = sh("git diff -- %s" % c["file"])
        save(c["name"], "mutation.diff", diff)
        try:
            if not c["expect_fail"] and c.get("expect_unit_fail"):
                unit_failed, unit_err = run_unit_tests(c["name"])
                want_unit = c["expect_unit_fail"]
                missing_unit = [t for t in want_unit if not any(t in u for u in (unit_failed or []))]
                verdict = "CONTROL-OK" if unit_failed is not None and not missing_unit else "CONTROL-FAILED"
                results.append({"control": c["name"], "verdict": verdict, "why": c["why"], "note": c.get("note"),
                                "expected_to_fail": [], "actually_failed": [],
                                "expected_unit_to_fail": want_unit, "unit_actually_failed": unit_failed,
                                "did_not_fail": missing_unit, "collateral": []})
                print("  %s unit expected=%s got=%s" % (verdict, want_unit, unit_failed), flush=True)
                continue
            suite_out, err = run_suite(c["name"])
            if err:
                results.append({"control": c["name"], "verdict": "CONTROL-ERROR", "detail": err})
                print("  CONTROL-ERROR %s" % err, flush=True)
                continue
            fails = failed_rows(suite_out)
            save(c["name"], "failed-rows.txt", "\n".join(l for l in suite_out.splitlines() if l.startswith("FAIL ")))
            unit_failed, unit_err = run_unit_tests(c["name"])
            want_unit = c.get("expect_unit_fail", [])
            missing = [r for r in c["expect_fail"] if r not in fails]
            missing_unit = [t for t in want_unit if not any(t in u for u in (unit_failed or []))]
            unexpected = [r for r in fails if r not in c["expect_fail"] and r not in c.get("also_ok", [])]
            if c.get("expect_unreachable"):
                # NOT a discriminating negative control and never labelled as one: the claim is that no assertion depends on this
                # branch because the kernel makes it unreachable. "Nothing failed" is consistent with that claim; it does not prove it.
                verdict = "NOT-DISCRIMINATING-UNREACHABLE" if not fails else "CONTROL-FAILED"
            else:
                verdict = "CONTROL-OK" if not missing and not missing_unit else "CONTROL-FAILED"
            results.append({"control": c["name"], "verdict": verdict, "why": c["why"],
                            "note": c.get("note"),
                            "expected_to_fail": c["expect_fail"], "actually_failed": fails,
                            "expected_unit_to_fail": want_unit, "unit_actually_failed": unit_failed,
                            "did_not_fail": missing + missing_unit, "collateral": unexpected,
                            "evidence_dir": os.path.join(EVIDENCE, c["name"]),
                            "summary": [l for l in (suite_out or "").splitlines() if l.startswith("assertions") or l.startswith("exit=")]})
            print("  %s rows expected=%s got=%s | unit expected=%s got=%s"
                  % (verdict, c["expect_fail"], fails, want_unit, unit_failed), flush=True)
        finally:
            restore(c, original)
            back = digest_tree()
            if back != clean:
                results.append({"control": c["name"], "verdict": "CONTROL-ERROR",
                                "detail": "source tree not restored (content digest %s != %s)" % (back[:12], clean[:12])})
                print("  CONTROL-ERROR source not restored", flush=True)
                any_failed = True
                break
    # rebuild and redeploy the clean tree so the host is not left running a mutant
    sh("crates/build.sh build --release")
    ok, log = deploy_mutant()
    save("", "redeploy-clean.log", log)
    save("", "installed-digests-after.txt", installed_digests())
    restored = digest_tree() == clean
    for r in results:
        if r["verdict"] in ("CONTROL-FAILED", "CONTROL-ERROR"):
            any_failed = True
    out = {"controls": results, "commit": head.strip(), "clean_tree_content_digest": clean, "restored": restored,
           "clean_redeployed": ok, "evidence_dir": EVIDENCE,
           "verdict": "FAIL" if (any_failed or not restored or not ok) else "PASS"}
    print(json.dumps(out, indent=1))
    open(os.path.join(EVIDENCE, "negative-controls.json"), "w").write(json.dumps(out, indent=1, sort_keys=True))
    sys.exit(0 if out["verdict"] == "PASS" else 1)


if __name__ == "__main__":
    main()
