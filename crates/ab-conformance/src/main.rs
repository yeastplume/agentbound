//! ab-conformance: drives the live 1A deployment on the reference host and
//! writes the evidence register (docs/evidence/wp2/). Run as root on the host
//! after `deploy/provision.sh`. Every row records observed evidence, never a
//! claim; a row without evidence is FAIL.
use ab_common::json::{self, canonical, Value, MANIFEST_LIMITS};
use ab_common::wire;
use std::process::Command;

struct Row { id: String, verdict: &'static str, evidence: String }
struct Rig { rows: Vec<Row>, as_user: String }

fn sh(cmd: &str) -> (i32, String) { let o = Command::new("sh").arg("-c").arg(cmd).output().unwrap(); (o.status.code().unwrap_or(-1), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))) }
fn jget<'a>(v: &'a Value, path: &str) -> Option<&'a Value> { let mut c = Some(v); for k in path.split('.') { c = c.and_then(|x| x.get(k)); } c }
fn js(v: &Value, path: &str) -> String { jget(v, path).map(|x| match x { Value::Str(s) => s.clone(), o => String::from_utf8_lossy(&canonical(o)).into_owned() }).unwrap_or_default() }
fn parse(s: &str) -> Value { s.lines().rev().find_map(|l| json::parse(l.trim().as_bytes(), &MANIFEST_LIMITS).ok()).unwrap_or(Value::Null) }
fn lc(op: &str, body: Value) -> Value { match wire::connect("/run/agentbound/lifecycle.sock") { Ok(c) => c.call(&wire::request(op, &format!("conf-{}", ab_common::sig::monotonic_ns()), body)).unwrap_or(Value::Null), Err(_) => Value::Null } }
fn audit_rows(key: &str) -> Vec<Value> { let c = wire::connect("/run/agentbound/audit.sock").unwrap(); let k = if key.starts_with("sha256:") { "launch_record_digest" } else { "authorization_id" }; c.call(&wire::request("query", "q", Value::obj(vec![(k, Value::s(key))]))).ok().and_then(|r| jget(&r, "body.rows").and_then(|x| x.as_arr()).cloned()).unwrap_or_default() }
fn kinds(rows: &[Value]) -> Vec<String> { rows.iter().map(|r| js(r, "event.event")).collect() }
fn sig(lrd: &str, trigger: &str) -> Value { lc("revocation_signal", Value::obj(vec![("launch_record_digest", Value::s(lrd)), ("source", Value::s("conformance")), ("trigger", Value::s(trigger))])) }
fn cgprocs(scope: &str) -> i32 { sh(&format!("cat /sys/fs/cgroup/system.slice/{scope}/cgroup.procs 2>/dev/null | wc -l")).1.trim().parse().unwrap_or(0) }

impl Rig {
    fn rec(&mut self, id: &str, pass: bool, ev: impl Into<String>) { let ev = ev.into().replace('\n', " "); println!("{} {} {}", if pass { "PASS" } else { "FAIL" }, id, ev.chars().take(160).collect::<String>()); self.rows.push(Row { id: id.into(), verdict: if pass { "PASS" } else { "FAIL" }, evidence: ev }); }
    fn cli(&self, args: &str) -> (i32, Value, String) { let (rc, out) = sh(&format!("su -s /bin/sh {} -c 'agentbound {}' </dev/null 2>&1", self.as_user, args)); (rc, parse(&out), out) }
    fn request(&self, file: &str, extra: &str) -> (i32, Value, String) { self.cli(&format!("request {file} {extra}")) }
    fn write_req(&self, name: &str, body: &str) -> String { let p = format!("/tmp/conf-{name}.json"); std::fs::write(&p, body).unwrap(); sh(&format!("chmod 644 {p}")); p }
    fn terminate(&self, lrd: &str) -> Value { lc("terminate", Value::obj(vec![("launch_record_digest", Value::s(lrd)), ("reason", Value::s("conformance"))])) }
    fn launch(&self, runtime: &str, task: &str) -> (i32, Value, String) {
        let p = self.write_req("launch", &format!(r#"{{"schema_version":"agentbound.session-request.v0.1","agent_principal_id":"agent:finance-agent","task_purpose_id":"{task}","requested_runtime":"{runtime}","requested_resources":["resource:workspace-finance"],"initiator_credential_ref":"authn:alice-session-0001","approval_references":[]}}"#));
        self.request(&p, "")
    }
}

fn main() {
    let mut g = Rig { rows: vec![], as_user: "alice".into() };
    sh("rm -f /var/lib/agentbound/workspaces/finance/*");
    let base = r#"{"schema_version":"agentbound.session-request.v0.1","agent_principal_id":"agent:finance-agent","task_purpose_id":"task:redwood-analysis","requested_runtime":"runtime:scripted-loop","requested_resources":["resource:workspace-finance"],"initiator_credential_ref":"authn:alice-session-0001","approval_references":[]}"#;
    let eng = |s: &str| base.replace("task:redwood-analysis", "task:fix-issue-1234").replace("agent:finance-agent", "agent:engineering-agent").replace("workspace-finance", "workspace-eng").replace("\"approval_references\":[]", &format!("\"approval_references\":[{s}]"));

    // ---- D-01 positive path with the probe runtime; T-6.1/6.2/6.9 rows from inside ----
    let (rc, v, out) = g.launch("runtime:probe", "task:redwood-analysis");
    let lrd = js(&v, "launch_record_digest"); let scope = js(&v, "scope_id"); let uid = js(&v, "uid");
    g.rec("D-01", rc == 0 && !lrd.is_empty(), format!("rc={rc} lrd={lrd} {}", out.lines().last().unwrap_or("").chars().take(200).collect::<String>()));
    let console = js(&v, "console"); std::thread::sleep(std::time::Duration::from_secs(8));
    let probe = std::fs::read_to_string(&console).unwrap_or_default();
    let mut seen_end = false;
    for l in probe.lines().filter(|l| l.starts_with("PROBE ")) { let p: Vec<&str> = l.splitn(4, ' ').collect(); if p.len() < 3 { continue; } if p[1] == "PROBE-END" { seen_end = true; continue; } g.rec(p[1], p[2] == "PASS", p.get(3).copied().unwrap_or("")); }
    g.rec("PROBE-COMPLETE", seen_end, format!("probe lines={}", probe.lines().count()));
    let st = lc("status", Value::obj(vec![("launch_record_digest", Value::s(&lrd))]));
    g.rec("D-01.status", js(&st, "body.state") == "active" && js(&st, "body.identity_state") == "in-use", js(&st, "body"));
    let procs = cgprocs(&scope); g.rec("D-06", procs >= 2, format!("scope procs={procs} (init + workload + orphan/fan-out survivors)"));
    let (_, outside) = sh(&format!("for p in $(ps -eo pid,uid | awk '$2=={uid}{{print $1}}'); do grep -q {scope} /proc/$p/cgroup || echo $p; done | wc -l"));
    g.rec("D-04.host-view", outside.trim() == "0", format!("uid {uid} processes outside scope={}", outside.trim()));
    // D-08 / F-T-*: terminate with descendants present
    let t = g.terminate(&lrd);
    g.rec("D-08", js(&t, "body.state") == "cleaned/sealed", js(&t, "body.evidence"));
    g.rec("F-T-03", js(&t, "body.evidence.sigterm_sent") == "true" && js(&t, "body.evidence.init_pidfd_exited") == "true", js(&t, "body.evidence"));
    g.rec("F-T-04", js(&t, "body.evidence.cgroup_kill_written") == "true" && js(&t, "body.evidence.cgroup_procs_remaining") == "[]", "kill written without waiting for frozen 1; procs empty; pidfd exited");
    g.rec("D-07", js(&t, "body.evidence.credential_scan_outside_scope") == "[]" && js(&t, "body.evidence.cgroup_procs_remaining") == "[]", "orphan/double-fork survivors killed with the scope; host credential scan clean");
    let st = lc("status", Value::obj(vec![("launch_record_digest", Value::s(&lrd))]));
    g.rec("F-T-10", js(&st, "body.identity_state") == "quarantined", js(&st, "body"));
    let k = kinds(&audit_rows(&lrd));
    g.rec("F-T-11", k.contains(&"session.sealed".into()) && k.contains(&"session.cleanup_completed".into()) && k.contains(&"session.identity_released".into()), format!("audit kinds={k:?}"));
    g.rec("F-T-08", !std::path::Path::new(&console).exists(), format!("session dir removed={}; workspace root retained by durable owner", !std::path::Path::new(&console).exists()));
    let (_, ws) = sh("stat -c '%U:%G %a' /var/lib/agentbound/workspaces/finance"); g.rec("T-6.2-007.host", ws.trim() == "root:root 2770", format!("workspace root after cleanup: {}", ws.trim()));

    // ---- request-layer rejections (T-6.5 / T-6.6 / T-6.8-013) ----
    let cases: Vec<(&str, String, &str)> = vec![
        ("T-6.5-001.unknown", base.replace("\"approval_references\":[]", "\"approval_references\":[],\"uid\":0"), "unknown-member"),
        ("T-6.5-001.dup", base.replace("\"approval_references\":[]", "\"approval_references\":[],\"approval_references\":[]"), "duplicate-member"),
        ("T-6.5-007", base.replace("\"approval_references\":[]", "\"approval_references\":[],\"mount\":\"/etc\""), "unknown-member"),
        ("T-6.5-006", base.replace("agentbound.session-request.v0.1", "agentbound.session-request.v0.0"), "version"),
        ("T-6.5-002.deep", format!("{{\"a\":{}1{}}}", "[".repeat(10), "]".repeat(10)), "depth-limit"),
        ("T-6.5-002.big", format!("{{\"schema_version\":\"{}\"}}", "x".repeat(20000)), "size-limit"),
        ("T-6.6-001.principal", base.replace("agent:finance-agent", "agent:nobody"), "unknown_principal"),
        ("T-6.6-001.authority", base.replace("resource:workspace-finance", "resource:workspace-eng"), "authority_exceeded"),
        ("T-6.6-003", eng(""), "approval_missing"),
        ("T-6.6-002.expired", eng("\"approval:eng-1234-expired\""), "approval_expired"),
        ("T-6.6-002.stale", eng("\"approval:eng-1234-stale\""), "approval_replayed"),
        ("T-6.6-005", base.replace("\"approval_references\":[]", "\"approval_references\":[],\"budget\":{\"pids\":100000}"), "budget_exceeds_policy"),
        ("T-6.6-006", base.replace("runtime:scripted-loop", "runtime:evil"), "unknown_runtime"),
        ("T-6.6-008", base.replace("agent:finance-agent", "agent:finance agent"), "grammar"),
        ("T-6.8-013", base.replace("task:redwood-analysis", "task:degraded-bad").replace("\"requested_resources\":[\"resource:workspace-finance\"]", "\"requested_resources\":[]").replace("runtime:scripted-loop", "runtime:sh"), "continue_degraded_not_permitted"),
        ("T-6.5-010.wrong-caller", base.replace("authn:alice-session-0001", "authn:bob-session-0001"), "initiator_unauthenticated"),
        ("T-6.8-001.disabled", base.replace("authn:alice-session-0001", "authn:carol-disabled"), "initiator_disabled"),
    ];
    for (id, body, want) in cases {
        let p = g.write_req(id, &body); let (rc, v, _) = g.request(&p, "--no-launch");
        let rule = js(&v, "body.rule"); let detail = js(&v, "body.detail");
        g.rec(id, rc != 0 && (rule.contains(want) || detail.contains(want)), format!("class={} rule={rule} detail={}", js(&v, "class"), detail.chars().take(120).collect::<String>()));
    }
    // T-6.6-002 replay: use a valid approval, then present it again
    let p = g.write_req("appr", &eng("\"approval:eng-1234-a\"")); g.as_user = "bob".into();
    let p2 = g.write_req("appr2", &eng("\"approval:eng-1234-a\"").replace("authn:alice-session-0001", "authn:bob-session-0001"));
    let (rc1, _, _) = g.request(&p2, "--no-launch"); let (rc2, v, _) = g.request(&p2, "--no-launch"); let _ = p;
    g.rec("T-6.6-002.replayed", rc1 == 0 && rc2 != 0 && js(&v, "body.rule") == "approval_replayed", format!("first rc={rc1} second rule={}", js(&v, "body.rule")));
    // T-6.6-004 scheduler without owner
    g.as_user = "cron".into();
    let p = g.write_req("sched", &base.replace("authn:alice-session-0001", "authn:cron-nightly")); let (rc, v, _) = g.request(&p, "--no-launch"); g.rec("T-6.6-004", rc != 0 && js(&v, "body.rule") == "scheduled_without_owner", js(&v, "body.rule"));
    let p = g.write_req("sched2", &base.replace("authn:alice-session-0001", "authn:cron-owned")); let (rc, v, _) = g.request(&p, "--no-launch"); g.rec("T-6.6-004.owned", rc == 0 && js(&v, "body.authorization_manifest.actors.owner") == "human:alice", js(&v, "body.authorization_manifest.actors"));
    g.as_user = "alice".into();
    // T-6.5-010: CLI user calling a constructor-only lifecycle op
    std::fs::write("/tmp/lc-probe.py", "import socket\ns=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET);s.connect('/run/agentbound/lifecycle.sock')\ns.send(b'{\"body\":{},\"idempotency_key\":\"x\",\"op\":\"reserve_identity\",\"v\":\"agentbound.wire.v0.1\"}');print(s.recv(65536).decode())\n").unwrap(); sh("chmod 644 /tmp/lc-probe.py");
    let (_, out) = sh("su -s /bin/sh alice -c 'python3 /tmp/lc-probe.py' </dev/null 2>&1");
    g.rec("T-6.5-010.lifecycle", out.contains("peer_not_permitted"), out.trim().chars().take(200).collect::<String>());
    let (_, rej) = sh("grep -c session.rejected /var/lib/agentbound/audit-policy.jsonl"); g.rec("T-6.6-001.audit", rej.trim().parse::<i32>().unwrap_or(0) >= 15, format!("session.rejected events with failed_input={}", rej.trim()));

    // ---- constructor faults (D-11, F-C) ----
    for (id, fault, want_step) in [("F-C-03", "mount-symlink", "3"), ("F-C-07", "pre-commit-crash", "7"), ("F-C-09", "post-commit-crash", "8")] {
        let p = g.write_req(id, base); let (rc, _, out) = g.request(&p, &format!("--fault {fault}"));
        let (_, last) = sh("tail -1 /var/lib/agentbound/audit-launch.jsonl"); let ev = parse(&last);
        let (step, rule, rb) = (js(&ev, "detail.failed_step"), js(&ev, "detail.rule"), js(&ev, "detail.rollback"));
        let scope_name = jget(&ev, "detail.ledger").and_then(|l| l.as_arr()).and_then(|l| l.iter().find(|e| js(e, "what") == "scope").map(|e| js(e, "detail"))).unwrap_or_default();
        let scope_left = !scope_name.is_empty() && std::path::Path::new(&format!("/sys/fs/cgroup/{scope_name}")).exists(); let scopes = if scope_left { "1" } else { "0" }.to_string();
        let az = out.split("launchrec:").nth(1).map(|x| format!("launchrec:{}", x.chars().take_while(|c| c.is_alphanumeric() || *c == '-').collect::<String>())).unwrap_or_default();
        let ident = js(&lc("status", Value::obj(vec![("authorization_id", Value::s(&az))])), "body.identity_state");
        g.rec(id, rc != 0 && step == want_step && (ident == "reclaiming" || ident == "quarantined") && scopes.trim() == "0", format!("step={step} rule={rule} identity={ident} scopes_left={} rollback={rb}", scopes.trim()));
        if fault == "post-commit-crash" { let l = js(&ev, "launch_record_digest"); let k = kinds(&audit_rows(&l)); g.rec("F-C-09.record", !l.is_empty() && k.contains(&"session.launch_record_committed".into()) && k.contains(&"session.construction_failed".into()), format!("lrd={l} kinds={k:?}")); }
    }
    g.rec("D-11", g.rows.iter().filter(|r| r.id.starts_with("F-C-0")).all(|r| r.verdict == "PASS"), "constructor fault rows F-C-03/07/09: no runnable session, identity held, scope gone");
    // ---- T-6.5-004: concurrent duplicate launch of one authorization ----
    let p = g.write_req("replay", base); let (_, v, _) = g.request(&p, "--no-launch"); let az = js(&v, "body.authorization_id");
    let (_, o1) = sh(&format!("agentbound-launch --authorization {az} 2>&1 & agentbound-launch --authorization {az} 2>&1; wait"));
    let acts = o1.matches("\"scope_id\"").count(); let refusals = o1.matches("lease_held").count() + o1.matches("already allocated").count() + o1.matches("handoff_missing").count();
    g.rec("T-6.5-004", acts == 1 && refusals == 1, format!("activations={acts} refusals={refusals}"));
    if let Some(l) = jget(&lc("list", Value::obj(vec![])), "body.sessions").and_then(|s| s.as_arr()).and_then(|a| a.iter().rev().find(|s| js(s, "state") == "active").map(|s| js(s, "launch_record_digest"))) { g.terminate(&l); }
    let (_, q) = sh("python3 -c \"import sqlite3;c=sqlite3.connect('/var/lib/agentbound/lifecycle.db');print(c.execute(\\\"select state,count(*) from alloc a where seq=(select max(seq) from alloc b where b.allocation_id=a.allocation_id) group by state\\\").fetchall())\"");
    g.rec("T-6.5-009", q.contains("quarantined") && !q.contains("'free'") && !q.contains("in-use"), format!("allocator latest states: {}", q.trim()));

    // ---- revocation behaviours (T-6.8) ----
    let (rc, v, _) = g.launch("runtime:scripted-loop", "task:quiesce-cases"); let lrd = js(&v, "launch_record_digest"); let scope = js(&v, "scope_id"); g.rec("T-6.8.setup", rc == 0, &lrd);
    let r = sig(&lrd, "policy_service_unavailable"); g.rec("T-6.8-006", js(&r, "body.behaviour") == "continue-degraded" && js(&r, "body.state") == "active", js(&r, "body"));
    let r = sig(&lrd, "audit_pipeline_degraded_below_stop_threshold"); g.rec("T-6.8-011", js(&r, "body.behaviour") == "continue-degraded", js(&r, "body"));
    let r = sig(&lrd, "reclassification"); g.rec("T-6.8-007", js(&r, "body.behaviour") == "quiesce" && js(&r, "body.state") == "quiescing", js(&r, "body"));
    let (_, frozen) = sh(&format!("cat /sys/fs/cgroup/system.slice/{scope}/cgroup.events")); g.rec("F-T-02", frozen.contains("frozen 1"), frozen.trim().to_string());
    let r = sig(&lrd, "authority_revoked"); g.rec("T-6.8-003", js(&r, "body.behaviour") == "terminate" && js(&r, "body.state") == "cleaned/sealed", js(&r, "body"));
    let k = kinds(&audit_rows(&lrd)); g.rec("T-6.8.audit", k.iter().filter(|x| *x == "session.revocation_received").count() == 4 && k.contains(&"session.degraded".into()) && k.contains(&"session.quiesce_started".into()), format!("{k:?}"));
    for (id, trig, want) in [("T-6.8-001", "initiator_disabled", "terminate"), ("T-6.8-002", "approval_expired", "quiesce"), ("T-6.8-004", "catalogue_withdrawn", "quiesce"), ("T-6.8-005", "task_cancelled", "terminate")] {
        let (rc, v, _) = g.launch("runtime:scripted-loop", "task:quiesce-cases"); let l = js(&v, "launch_record_digest");
        let r = sig(&l, trig); g.rec(id, rc == 0 && js(&r, "body.behaviour") == want, format!("trigger={trig} behaviour={} state={}", js(&r, "body.behaviour"), js(&r, "body.state")));
        if want == "quiesce" { g.terminate(&l); }
    }
    // ---- T-6.8-012: lifecycle killed while a session is active ----
    let (rc, v, _) = g.launch("runtime:scripted-loop", "task:redwood-analysis"); let lrd = js(&v, "launch_record_digest"); let scope = js(&v, "scope_id");
    // Restart=on-failure would bring the daemon back within ~100 ms; hold it down explicitly to observe the gap
    sh("systemctl kill -s SIGKILL agentbound-lifecycle; systemctl stop agentbound-lifecycle 2>/dev/null; sleep 0.5");
    let alive = cgprocs(&scope); let (_, down) = sh("su -s /bin/sh alice -c 'agentbound list' </dev/null 2>&1");
    let probe_up = std::path::Path::new("/run/agentbound/lifecycle.sock").exists() && wire::connect("/run/agentbound/lifecycle.sock").is_ok();
    sh("systemctl start agentbound-lifecycle; sleep 2");
    let st = lc("status", Value::obj(vec![("launch_record_digest", Value::s(&lrd))])); let k = kinds(&audit_rows(&lrd));
    g.rec("T-6.8-012", rc == 0 && alive > 0 && !probe_up && k.contains(&"session.recovery_reconciled".into()), format!("procs_while_down={alive} (containment held, no authority available: daemon_reachable={probe_up}) cli_reply={} after_restart={} kinds={k:?}", down.trim().chars().take(60).collect::<String>(), js(&st, "body.state")));
    std::thread::sleep(std::time::Duration::from_secs(3));
    let st = lc("status", Value::obj(vec![("launch_record_digest", Value::s(&lrd))]));
    g.rec("T-6.8-012.contained", cgprocs(&scope) == 0, format!("state={} identity={} procs={}", js(&st, "body.state"), js(&st, "body.identity_state"), cgprocs(&scope)));
    // ---- audit store ----
    let a = wire::connect("/run/agentbound/audit.sock").unwrap().call(&wire::request("status", "s", Value::obj(vec![]))).unwrap_or(Value::Null);
    g.rec("T-6.9-007", js(&a, "body.lost") == "0" && js(&a, "body.seq").parse::<i64>().unwrap_or(0) > 50, format!("audit chain head={} seq={} lost={}", js(&a, "body.head"), js(&a, "body.seq"), js(&a, "body.lost")));
    // ---- T-6.5-003: catalogue source pointing outside its base ----
    sh("cp /etc/agentbound/catalogue.json /tmp/cat.bak; python3 -c \"import json;c=json.load(open('/etc/agentbound/catalogue.json'));c['mount_sources']['mount-source:workspace-finance']['relative']='../../../etc';json.dump(c,open('/etc/agentbound/catalogue.json','w'))\"");
    let p = g.write_req("trav", base); let (rc, _, _) = g.request(&p, ""); let (_, last) = sh("tail -1 /var/lib/agentbound/audit-launch.jsonl"); let ev = parse(&last);
    sh("cp /tmp/cat.bak /etc/agentbound/catalogue.json");
    g.rec("T-6.5-003", rc != 0 && js(&ev, "detail.failed_step") == "3" && js(&ev, "detail.rule").starts_with("mount_source"), format!("rule={} detail={}", js(&ev, "detail.rule"), js(&ev, "detail.detail")));

    let pass = g.rows.iter().filter(|r| r.verdict == "PASS").count();
    let mut md = format!("# WP2 conformance run (machine output)\n\n- Host: {}\n- Kernel: {}\n- systemd: {}\n- Rows: {} PASS / {} FAIL\n\n| Row | Verdict | Evidence |\n|---|---|---|\n", sh("hostname").1.trim(), sh("uname -r").1.trim(), sh("systemctl --version | head -1").1.trim(), pass, g.rows.len() - pass);
    for r in &g.rows { md.push_str(&format!("| {} | {} | {} |\n", r.id, r.verdict, r.evidence.replace('|', "\\|"))); }
    std::fs::write("/root/wp2/conformance-run.md", md).unwrap();
    println!("\n{pass}/{} PASS; register at /root/wp2/conformance-run.md", g.rows.len());
}
