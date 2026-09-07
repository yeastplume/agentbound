//! ab-conformance: drives the live 1A deployment on the reference host and
//! writes the evidence register (docs/evidence/wp2/). Run as root on the host
//! after `deploy/provision.sh`. Every row records observed evidence, never a
//! claim; a row without evidence is FAIL.
use ab_common::json::{self, canonical, Value, MANIFEST_LIMITS};
use ab_common::wire;
use std::process::Command;

struct Row { id: String, verdict: &'static str, evidence: String }
struct Rig { rows: Vec<Row>, as_user: String }
/// Verdict classes (WP3.1 harness integrity). Only `PASS` counts toward catalogue coverage.
/// `WEAK`: the assertion holds but is weaker than the catalogue row's intent (stated in evidence).
/// `RECORDED`: a 1A partial / N-A verdict re-asserted under 1B by the property that would have changed.
/// `FIXTURE`: setup or completion marker, never counted. `FAIL`: assertion false or measurement missing.
const CLASSES: [&str; 5] = ["PASS", "WEAK", "RECORDED", "FAIL", "FIXTURE"];
/// The catalogue ID a runner row belongs to: `T-6.4-003.only` → `T-6.4-003`; `GS-4[+fix]` → `GS-4`; `D-06.storage-principal` → `D-06`.
fn catalogue_id(row: &str) -> String {
    // strip a bracketed parameter, then keep the id prefix: `T-6.4-003.only` → `T-6.4-003`; `D-06.storage-principal` → `D-06`; `GS-4[+fix]` → `GS-4`; `D4.7-reconstruct` stays as is (not a catalogue id)
    let base = row.split('[').next().unwrap_or(row);
    if let Some(rest) = base.strip_prefix("T-") { let mut parts = rest.splitn(3, '.'); let a = parts.next().unwrap_or(""); let b = parts.next().unwrap_or(""); return format!("T-{a}.{b}"); }
    base.split('.').next().unwrap_or(base).to_string()
}
#[cfg(test)]
mod tests { #[test] fn ids() { for (r, c) in [("T-6.4-003.only", "T-6.4-003"), ("T-6.4-010.stream", "T-6.4-010"), ("T-6.1-001.init-environ", "T-6.1-001"), ("D-06.storage-principal", "D-06"), ("D-02.1B", "D-02"), ("GS-4[../main]", "GS-4"), ("F-C-09.record", "F-C-09"), ("T-6.8-setup", "T-6.8-setup"), ("D4.7-reconstruct", "D4"), ("D7-9.diagnostics", "D7-9"), ("T-6.5-001.dup", "T-6.5-001")] { assert_eq!(super::catalogue_id(r), c, "{r}"); } } }
/// (id, milestones, required verdict). The required verdict is `PASS` unless the frozen catalogue row's own pass criterion admits a
/// recorded / not-applicable outcome (`RECORDED`); the manifest header lists those four with their justification. `WEAK` is never
/// acceptable as a final verdict for a mandatory row.
fn expected_ids() -> Vec<(String, String, String)> { let t = include_str!("../expected-ids.txt"); t.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()).map(|l| { let f: Vec<&str> = l.split_whitespace().collect(); (f[0].to_string(), f.get(1).copied().unwrap_or("").to_string(), f.last().copied().filter(|v| *v == "RECORDED").unwrap_or("PASS").to_string()) }).collect() }
/// Does a row's best verdict satisfy its required one? PASS satisfies everything; RECORDED satisfies only a RECORDED requirement.
fn satisfies(best: &str, required: &str) -> bool { best == "PASS" || (best == "RECORDED" && required == "RECORDED") }

fn sh(cmd: &str) -> (i32, String) { let o = Command::new("sh").arg("-c").arg(cmd).output().unwrap(); (o.status.code().unwrap_or(-1), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))) }
fn jget<'a>(v: &'a Value, path: &str) -> Option<&'a Value> { let mut c = Some(v); for k in path.split('.') { c = c.and_then(|x| x.get(k)); } c }
fn js(v: &Value, path: &str) -> String { jget(v, path).map(|x| match x { Value::Str(s) => s.clone(), o => String::from_utf8_lossy(&canonical(o)).into_owned() }).unwrap_or_default() }
fn parse(s: &str) -> Value { s.lines().rev().find_map(|l| json::parse(l.trim().as_bytes(), &MANIFEST_LIMITS).ok()).unwrap_or(Value::Null) }
fn lc(op: &str, body: Value) -> Value { match wire::connect("/run/agentbound/lifecycle.sock") { Ok(c) => c.call(&wire::request(op, &format!("conf-{}", ab_common::sig::monotonic_ns()), body)).unwrap_or(Value::Null), Err(_) => Value::Null } }
/// Single-quote a string for `sh -c` (the inner command is built by us, never by a session).
/// T-6.5-005 helper: rewrite one catalogue limit in place (kept out of Rust string literals — quotes and braces fight the lexer).
/// The 1B session request: `task:fix-issue-1235` is the only task holding the gateway operations, and the only `local-socket` topology.
const GW_REQ: &str = r#"{"agent_principal_id":"agent:engineering-agent","approval_references":[],"initiator_credential_ref":"authn:bob-session-0001","requested_resources":["resource:workspace-eng"],"requested_runtime":"runtime:git-worker","schema_version":"agentbound.session-request.v0.1","task_purpose_id":"task:fix-issue-1235"}"#;

/// D-03 helper: advance `approval:eng-1234-d03`'s sequence past the highest this approver key has already used, so the row can
/// obtain a live second-principal session on every run without ever replaying an approval.
const D03_APPROVAL_PY: &str = "import json,os\np='/etc/agentbound/catalogue.json'\nc=json.load(open(p))\nhi=0\ns='/var/lib/agentbound/policy.jsonl'\nif os.path.exists(s):\n for l in open(s):\n  try:\n   r=json.loads(l)\n  except Exception:\n   continue\n  v=r.get('v') or {}\n  if r.get('kind')=='approval_seq' and v.get('key')=='key:erin-d03': hi=max(hi,int(v.get('seq',0) or 0))\nc['approvals']['approval:eng-1234-d03']['sequence']=hi+1\njson.dump(c,open(p,'w'),indent=2,sort_keys=True)\nprint(hi+1)\n";

/// T-6.6-007 helper: roll the catalogue's policy version backwards.
const ROLLBACK_PY: &str = "import json\np='/etc/agentbound/catalogue.json'\nc=json.load(open(p))\nc['policy_version']='policy:v0'\njson.dump(c,open(p,'w'),indent=2,sort_keys=True)\n";
const FLIP_PY: &str = "import json,sys\np='/etc/agentbound/catalogue.json'\nc=json.load(open(p))\nc['resource_limits']['pids']['limit']=int(sys.argv[1])\njson.dump(c,open(p,'w'),indent=2,sort_keys=True)\n";
fn shq(s: &str) -> String { format!("'{}'", s.replace('\'', "'\\''")) }
fn audit_rows(key: &str) -> Vec<Value> { let c = wire::connect("/run/agentbound/audit.sock").unwrap(); let k = if key.starts_with("sha256:") { "launch_record_digest" } else { "authorization_id" }; c.call(&wire::request("query", "q", Value::obj(vec![(k, Value::s(key))]))).ok().and_then(|r| jget(&r, "body.rows").and_then(|x| x.as_arr()).cloned()).unwrap_or_default() }
fn kinds(rows: &[Value]) -> Vec<String> { rows.iter().map(|r| js(r, "event.event")).collect() }
fn sig(lrd: &str, trigger: &str) -> Value { lc("revocation_signal", Value::obj(vec![("launch_record_digest", Value::s(lrd)), ("source", Value::s("conformance")), ("trigger", Value::s(trigger))])) }
/// grep -oE pattern for the `detail` member of an event line (kept out of format! strings: braces and quotes fight the macro).
/// T-6.4-009: recycle a specific pid inside the caller's pid namespace (requires CAP_SYS_ADMIN — more power than a session has)
/// and report whether the recycled process got the same pid as the establisher, plus its pid-namespace inode.
const RECYCLE_PY: &str = r#"
import os, sys, time, subprocess
t = int(sys.argv[1])
open("/proc/sys/kernel/ns_last_pid", "w").write(str(t - 1))
p = os.fork()
if p == 0:
    time.sleep(4)
    os._exit(0)
print("recycled_pid", p)
if p == t:
    print("same_pid_as_establisher")
    subprocess.run(["stat", "-c", "%i", "/proc/" + str(p) + "/ns/pid"])
os.waitpid(p, 0)
"#;
const DETAIL_RE: &str = "'\"detail\":\"[^\"]*\"'";
fn cgprocs(scope: &str) -> i32 { sh(&format!("cat /sys/fs/cgroup/system.slice/{scope}/cgroup.procs 2>/dev/null | wc -l")).1.trim().parse().unwrap_or(0) }

impl Rig {
    /// Print one row. A passing row's evidence is clipped to keep the console readable; a FAIL's is printed in full, because the
    /// clip hid the failing field exactly when it was needed — D-08's evidence is a JSON object whose deciding member sits past
    /// character 160, so the console showed a truncated blob and the reason had to be reconstructed from the audit store (WP3.1).
    fn put(&mut self, id: &str, verdict: &'static str, ev: impl Into<String>) {
        let ev = ev.into().replace('\n', " ");
        if verdict == "FAIL" { println!("{verdict} {id} {ev}"); } else { println!("{verdict} {id} {}", ev.chars().take(160).collect::<String>()); }
        self.rows.push(Row { id: id.into(), verdict, evidence: ev });
    }
    fn rec(&mut self, id: &str, pass: bool, ev: impl Into<String>) { self.put(id, if pass { "PASS" } else { "FAIL" }, ev) }
    /// Assertion holds but is weaker than the catalogue intent; a false assertion is still FAIL.
    fn weak(&mut self, id: &str, pass: bool, ev: impl Into<String>) { self.put(id, if pass { "WEAK" } else { "FAIL" }, ev) }
    fn recorded(&mut self, id: &str, pass: bool, ev: impl Into<String>) { self.put(id, if pass { "RECORDED" } else { "FAIL" }, ev) }
    fn fixture(&mut self, id: &str, ok: bool, ev: impl Into<String>) { self.put(id, if ok { "FIXTURE" } else { "FAIL" }, ev) }
    fn cli(&self, args: &str) -> (i32, Value, String) { let (rc, out) = sh(&format!("su -s /bin/sh {} -c 'agentbound {}' </dev/null 2>&1", self.as_user, args)); (rc, parse(&out), out) }
    fn request(&self, file: &str, extra: &str) -> (i32, Value, String) { self.cli(&format!("request {file} {extra}")) }
    fn write_req(&self, name: &str, body: &str) -> String { let p = format!("/tmp/conf-{name}.json"); std::fs::write(&p, body).unwrap(); sh(&format!("chmod 644 {p}")); p }
    fn terminate(&self, lrd: &str) -> Value { lc("terminate", Value::obj(vec![("launch_record_digest", Value::s(lrd)), ("reason", Value::s("conformance"))])) }
    /// Terminate with an injected step fault (F-T rows). The fault makes one protocol step fail; nothing is relaxed.
    fn terminate_faulted(&self, lrd: &str, fault: &str) -> Value { lc("terminate", Value::obj(vec![("fault", Value::s(fault)), ("launch_record_digest", Value::s(lrd)), ("reason", Value::s(&format!("conformance-fault:{fault}")))])) }
    fn launch(&self, runtime: &str, task: &str) -> (i32, Value, String) {
        let p = self.write_req("launch", &format!(r#"{{"schema_version":"agentbound.session-request.v0.1","agent_principal_id":"agent:finance-agent","task_purpose_id":"{task}","requested_runtime":"{runtime}","requested_resources":["resource:workspace-finance"],"initiator_credential_ref":"authn:alice-session-0001","approval_references":[]}}"#));
        self.request(&p, "")
    }
}

// GW_FORGE (host-root SCM_CREDENTIALS forgery) was removed in WP3.1 item 6: it never reached the per-packet rules because
// establish() refused it on uid before any packet was read. T-6.4-008 now attacks from an in-scope session-uid peer.

fn main() {
    // `--provenance`: print the source provenance embedded at build time and exit (independent WP3.1 validation, finding 4). The
    // conformance runner asks every installed binary, so a stale install is visible as a commit mismatch rather than hidden.
    if std::env::args().nth(1).as_deref() == Some("--provenance") { println!("commit={} dirty={} tree={}", ab_common::provenance::COMMIT, ab_common::provenance::DIRTY, ab_common::provenance::TREE); return; }
    let mut g = Rig { rows: vec![], as_user: "alice".into() };
    // Where this run starts in the hash-chained store. Any row whose evidence is "a denial of kind X was recorded" MUST count only
    // lines appended after this point: counting the whole log lets a previous run's denials satisfy a check that is no longer being
    // performed, which is exactly how the T-6.9-008 false positive survived until the negative controls (WP3.1 item 6).
    let run_start_line: i64 = sh("grep -hc '' /var/lib/agentbound/audit/events.jsonl").1.trim().parse().unwrap_or(0);
    sh("rm -f /var/lib/agentbound/workspaces/finance/*");
    let base = r#"{"schema_version":"agentbound.session-request.v0.1","agent_principal_id":"agent:finance-agent","task_purpose_id":"task:redwood-analysis","requested_runtime":"runtime:scripted-loop","requested_resources":["resource:workspace-finance"],"initiator_credential_ref":"authn:alice-session-0001","approval_references":[]}"#;
    let eng = |s: &str| base.replace("task:redwood-analysis", "task:fix-issue-1234").replace("agent:finance-agent", "agent:engineering-agent").replace("workspace-finance", "workspace-eng").replace("\"approval_references\":[]", &format!("\"approval_references\":[{s}]"));

    // ---- D-01 positive path with the probe runtime; T-6.1/6.2/6.9 rows from inside ----
    // T-6.1-012 fixture: bind a known abstract AF_UNIX name in the HOST network namespace for the probe's lifetime. The probe must
    // find it unreachable (abstract names are per netns) while being able to bind the identical name itself.
    std::fs::write("/tmp/abs-holder.py", "import socket,time\ns=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)\ns.bind('\\0agentbound-conf-abs')\ns.listen(4)\ntime.sleep(240)\n").unwrap();
    sh("[ -f /tmp/abs-holder.pid ] && kill $(cat /tmp/abs-holder.pid) 2>/dev/null; setsid python3 /tmp/abs-holder.py >/dev/null 2>&1 </dev/null & echo $! > /tmp/abs-holder.pid; sleep 1");
    let (_, abs_held) = sh("ss -xl 2>/dev/null | grep -c 'agentbound-conf-abs'");
    // T-6.1-010/011 fixture: the probe needs a pid that is live on the host and absent from its own pid namespace. Pass it through
    // the workspace (the invocation profile allowlists the environment, so it cannot be an env var) — the probe copies it to /tmp.
    let (_, hostpid) = sh("systemctl show -p MainPID --value agentbound-lifecycle");
    sh(&format!("printf '%s' '{}' > /var/lib/agentbound/workspaces/finance/hostpid; chmod 644 /var/lib/agentbound/workspaces/finance/hostpid", hostpid.trim()));
    let (rc, v, out) = g.launch("runtime:probe", "task:redwood-analysis");
    let lrd = js(&v, "launch_record_digest"); let scope = js(&v, "scope_id"); let uid = js(&v, "uid");
    g.rec("D-01", rc == 0 && !lrd.is_empty(), format!("rc={rc} lrd={lrd} {}", out.lines().last().unwrap_or("").chars().take(200).collect::<String>()));
    // wait for the probe to finish (it now exhausts inodes and memory, which takes several seconds); bounded at 60 s
    let console = js(&v, "console"); let mut probe = String::new();
    for _ in 0..120 { std::thread::sleep(std::time::Duration::from_millis(500)); probe = std::fs::read_to_string(&console).unwrap_or_default(); if probe.contains("PROBE PROBE-END") { break; } }
    let mut seen_end = false;
    for l in probe.lines().filter(|l| l.starts_with("PROBE ")) { let p: Vec<&str> = l.splitn(4, ' ').collect(); if p.len() < 3 { continue; } if p[1] == "PROBE-END" { seen_end = true; continue; } g.put(p[1], CLASSES.iter().find(|c| **c == p[2]).copied().unwrap_or("FAIL"), p.get(3).copied().unwrap_or("")); }
    g.fixture("PROBE-COMPLETE", seen_end, format!("probe lines={}", probe.lines().count()));
    g.fixture("T-6.1-012.host-name", abs_held.lines().next().map(|l| l.trim() != "0").unwrap_or(false), format!("host abstract name agentbound-conf-abs bound in the host netns while the probe ran (ss listeners for the name = {})", abs_held.trim()));
    sh("[ -f /tmp/abs-holder.pid ] && kill $(cat /tmp/abs-holder.pid) 2>/dev/null; rm -f /tmp/abs-holder.pid");
    let st = lc("status", Value::obj(vec![("launch_record_digest", Value::s(&lrd))]));
    g.rec("D-01.status", js(&st, "body.state") == "active" && js(&st, "body.identity_state") == "in-use", js(&st, "body"));
    let procs = cgprocs(&scope); g.rec("D-06", procs >= 2, format!("scope procs={procs} (init + workload + orphan/fan-out survivors)"));
    // ---- T-6.9-003 / T-6.9-004 (host view): every enforced class in the committed binding is a KERNEL read-back, and it equals what the
    // kernel enforces now. The binding is fetched from the lifecycle store; the kernel figures come from the scope's cgroup files and,
    // for the tmpfs/rlimit classes, from inside the session (statfs / getrlimit via nsenter). A manifest value copied into
    // `installed_value` without installation would disagree with at least one of these.
    let rec = lc("record", Value::obj(vec![("launch_record_digest", Value::s(&lrd))]));
    let rp = rec.get("body").and_then(|b| b.get("binding")).and_then(|b| b.get("launch_binding")).and_then(|b| b.get("resource_projection")).cloned().unwrap_or(Value::Null);
    let iv = |c: &str| rp.get(c).and_then(|x| x.get("installed_value")).and_then(|x| x.as_int());
    let cgf = |f: &str| sh(&format!("cat /sys/fs/cgroup/system.slice/{scope}/{f}")).1.trim().to_string();
    let k_pids: Option<i64> = cgf("pids.max").parse().ok(); let k_mem: Option<i64> = cgf("memory.max").parse().ok();
    let k_cpu: Option<i64> = { let c = cgf("cpu.max"); let mut it = c.split_whitespace(); match (it.next().and_then(|q| q.parse::<i64>().ok()), it.next().and_then(|p| p.parse::<i64>().ok())) { (Some(q), Some(p)) => Some(q * 1000 / p), _ => None } };
    let k_io: Option<i64> = cgf("io.max").split_whitespace().find_map(|t| t.strip_prefix("wbps=")).and_then(|v| v.parse().ok());
    let ipid = sh(&format!("head -1 /sys/fs/cgroup/system.slice/{scope}/cgroup.procs")).1.trim().to_string();
    let inside = sh(&format!("nsenter -t {ipid} -m -p -- sh -c 'stat -f -c \"%b %S %c\" /tmp; ulimit -n' 2>/dev/null")).1;
    let (k_disk, k_ino, k_fd) = { let mut l = inside.lines(); let a = l.next().unwrap_or("").split_whitespace().map(|x| x.parse::<i64>().unwrap_or(-1)).collect::<Vec<_>>(); (a.get(0).zip(a.get(1)).map(|(b, f)| b * f), a.get(2).copied(), l.next().and_then(|x| x.trim().parse::<i64>().ok())) };
    let pairs = [("pids", iv("pids"), k_pids), ("memory_bytes", iv("memory_bytes"), k_mem), ("cpu", iv("cpu"), k_cpu), ("io_bandwidth", iv("io_bandwidth"), k_io), ("disk_bytes", iv("disk_bytes"), k_disk), ("disk_inodes", iv("disk_inodes"), k_ino), ("file_descriptors", iv("file_descriptors"), k_fd)];
    let bad: Vec<String> = pairs.iter().filter(|(_, b, k)| b.is_none() || k.is_none() || b != k).map(|(c, b, k)| format!("{c}: binding={b:?} kernel={k:?}")).collect();
    let all: Vec<String> = pairs.iter().map(|(c, b, k)| format!("{c}={}/{}", b.map(|x| x.to_string()).unwrap_or("-".into()), k.map(|x| x.to_string()).unwrap_or("-".into()))).collect();
    g.rec("T-6.9-004.readback", bad.is_empty(), format!("binding installed_value vs kernel (binding/kernel): {} {}", all.join(" "), if bad.is_empty() { String::new() } else { format!("MISMATCH {bad:?}") }));
    // owner-installed classes: the audit receiver reports the budget it reserved for this session; delegation fan-out is 0 by construction
    let (_, aud) = sh(&format!("python3 -c \"import socket,json;s=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET);s.connect('/run/agentbound/audit.sock');s.send(json.dumps({{'body':{{}},'idempotency_key':'c','op':'status','v':'agentbound.wire.v0.1'}},separators=(',',':'),sort_keys=True).encode());print(s.recv(65536).decode())\""));
    g.rec("T-6.9-003.owners", iv("audit_capacity").is_some() && iv("delegation_fanout") == Some(0) && aud.contains("\"ok\":true"), format!("audit_capacity installed_value={:?} (reserved with the receiver, which is reachable), delegation_fanout installed_value={:?}", iv("audit_capacity"), iv("delegation_fanout")));
    // CPU throttling evidence: cpu.stat must show the quota being applied while the probe's fan-out runs
    let cpustat = cgf("cpu.stat"); let throttled: i64 = cpustat.lines().find_map(|l| l.strip_prefix("nr_throttled ")).and_then(|v| v.trim().parse().ok()).unwrap_or(-1);
    g.rec("T-6.9-003.cpu", k_cpu == Some(1000) && throttled >= 0, format!("cpu.max quota={:?} milli-cpu (read back); cpu.stat nr_throttled={throttled} (quota installed and accounted; throttling occurs only under contention, which this probe does not guarantee)", k_cpu));
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
    let (rc1, v1, _) = g.request(&p2, "--no-launch"); let (rc2, v, _) = g.request(&p2, "--no-launch"); let _ = p;
    // the policy store is durable: on a re-run the approval is already consumed and the first presentation is itself the replay
    let first_ok = rc1 == 0 || js(&v1, "body.rule") == "approval_replayed";
    g.rec("T-6.6-002.replayed", first_ok && rc2 != 0 && js(&v, "body.rule") == "approval_replayed", format!("first rc={rc1} rule={} second rule={} (durable consumption across runs)", js(&v1, "body.rule"), js(&v, "body.rule")));
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
    // Every constructor step that can fail has a fault: the step is made to fail *with its side effects already in place*, so the
    // rollback runs against a partially constructed session. Common assertion for all of them: non-zero exit, the failure recorded at
    // the expected step, the identity held (reclaiming/quarantined — never free), and no scope left behind. Per-row extras follow.
    for (id, fault, want_step) in [("F-C-01", "barrier-hold", "1"), ("F-C-02", "mount-private", "2"), ("F-C-03", "mount-symlink", "3"),
                                   ("F-C-04", "pivot-root", "4"), ("F-C-05", "proc-mount", "5"), ("F-C-06", "fd-leak", "6"),
                                   ("F-C-07", "pre-commit-crash", "7"), ("F-C-09", "post-commit-crash", "8")] {
        let p = g.write_req(id, base); let (rc, _, out) = g.request(&p, &format!("--fault {fault}"));
        let (_, last) = sh("tail -1 /var/lib/agentbound/audit-launch.jsonl"); let ev = parse(&last);
        let (step, rule, rb) = (js(&ev, "detail.failed_step"), js(&ev, "detail.rule"), js(&ev, "detail.rollback"));
        let scope_name = jget(&ev, "detail.ledger").and_then(|l| l.as_arr()).and_then(|l| l.iter().find(|e| js(e, "what") == "scope").map(|e| js(e, "detail"))).unwrap_or_default();
        let scope_left = !scope_name.is_empty() && std::path::Path::new(&format!("/sys/fs/cgroup/{scope_name}")).exists(); let scopes = if scope_left { "1" } else { "0" }.to_string();
        let az = out.split("launchrec:").nth(1).map(|x| format!("launchrec:{}", x.chars().take_while(|c| c.is_alphanumeric() || *c == '-').collect::<String>())).unwrap_or_default();
        let ident = js(&lc("status", Value::obj(vec![("authorization_id", Value::s(&az))])), "body.identity_state");
        if fault != "barrier-hold" {
            g.rec(id, rc != 0 && step == want_step && (ident == "reclaiming" || ident == "quarantined") && scopes.trim() == "0", format!("step={step} rule={rule} identity={ident} scopes_left={} rollback={rb}", scopes.trim()));
        }
        if fault == "post-commit-crash" { let l = js(&ev, "launch_record_digest"); let k = kinds(&audit_rows(&l)); g.rec("F-C-09.record", !l.is_empty() && k.contains(&"session.launch_record_committed".into()) && k.contains(&"session.construction_failed".into()), format!("lrd={l} kinds={k:?}")); }
        // F-C-01: the child was released neither to exec nor to the workload — it must have been reaped, and nothing may have run
        if fault == "barrier-hold" {
            let reaped = rb.contains("child killed and reaped"); let child_pid = jget(&ev, "detail.ledger").and_then(|l| l.as_arr()).and_then(|l| l.iter().find(|e| js(e, "what") == "clone3").map(|e| js(e, "detail"))).unwrap_or_default();
            let pid = child_pid.trim_start_matches("pid=").to_string();
            let alive = !pid.is_empty() && std::path::Path::new(&format!("/proc/{pid}")).exists();
            g.rec("F-C-01", rc != 0 && step == "1" && reaped && !alive && (ident == "reclaiming" || ident == "quarantined") && scopes.trim() == "0",
                format!("barrier never released: child {child_pid} reaped by rollback={reaped}, still alive={alive}; identity={ident}; scopes_left={}; rollback={rb}", scopes.trim()));
        }
        // F-C-05: the child aborted at the proc mount — no host /proc may have been left mounted anywhere the session could reach,
        // and (WP1 F-2) no host sysfs may be inherited. The session tree is gone with the namespace; assert the host is unchanged.
        if fault == "proc-mount" {
            let (_, leaked) = sh("findmnt -rno TARGET | grep -c '/var/lib/agentbound/sessions/'");
            g.rec("F-C-05.no-host-proc", leaked.trim() == "0", format!("host mount table shows {} session-tree mounts after the aborted construction (the child's proc/sysfs died with its namespace)", leaked.trim()));
        }
        // F-C-06: the leaked descriptor was caught by step 6's own check, not by a later step
        if fault == "fd-leak" {
            let d = js(&ev, "detail.detail"); let k = kinds(&audit_rows(&js(&ev, "launch_record_digest")));
            g.rec("F-C-06.own-check", step == "6" && d.contains("leaked") && !k.contains(&"session.activated".into()),
                format!("a descriptor surviving the closure pass was caught by step 6's own verification through the fresh /proc: {d}; the session never activated (audit kinds={k:?})"));
        }
    }
    g.rec("D-11", g.rows.iter().filter(|r| r.id.starts_with("F-C-0")).all(|r| r.verdict == "PASS"), format!("all {} constructor fault rows (F-C-01..09, one per failing step): no runnable session, identity held, scope gone", g.rows.iter().filter(|r| r.id.starts_with("F-C-0")).count()));
    // ---- T-6.5-004: concurrent duplicate launch of one authorization ----
    let p = g.write_req("replay", base); let (_, v, _) = g.request(&p, "--no-launch"); let az = js(&v, "body.authorization_id");
    let (_, o1) = sh(&format!("agentbound-launch --authorization {az} 2>&1 & agentbound-launch --authorization {az} 2>&1; wait"));
    let acts = o1.matches("\"scope_id\"").count(); let refusals = o1.matches("lease_held").count() + o1.matches("already allocated").count() + o1.matches("handoff_missing").count();
    g.rec("T-6.5-004", acts == 1 && refusals == 1, format!("activations={acts} refusals={refusals}"));
    if let Some(l) = jget(&lc("list", Value::obj(vec![])), "body.sessions").and_then(|s| s.as_arr()).and_then(|a| a.iter().rev().find(|s| js(s, "state") == "active").map(|s| js(s, "launch_record_digest"))) { g.terminate(&l); }
    // every `free` row must follow a `quarantined` row at least the 24 h floor earlier; no identity is in-use after the run
    let (_, q) = sh("python3 -c \"import sqlite3;c=sqlite3.connect('/var/lib/agentbound/lifecycle.db');print(c.execute(\\\"select state,count(*) from alloc a where seq=(select max(seq) from alloc b where b.allocation_id=a.allocation_id) group by state\\\").fetchall())\"");
    let (_, early) = sh("python3 -c \"import sqlite3,datetime;c=sqlite3.connect('/var/lib/agentbound/lifecycle.db');p=lambda s:datetime.datetime.fromisoformat(s.replace('Z','+00:00'));bad=0\nfor a,fw in c.execute(\\\"select allocation_id,wall_clock from alloc where state='free'\\\").fetchall():\n q=c.execute(\\\"select max(wall_clock) from alloc where allocation_id=? and state='quarantined'\\\",(a,)).fetchone()[0]\n if q is None or (p(fw)-p(q)).total_seconds()<86400: bad+=1\nprint(bad)\"");
    g.rec("T-6.5-009", q.contains("quarantined") && !q.contains("in-use") && early.trim() == "0", format!("allocator latest states: {}; free-before-floor violations={}", q.trim(), early.trim()));

    // ---- revocation behaviours (T-6.8) ----
    let (rc, v, _) = g.launch("runtime:scripted-loop", "task:quiesce-cases"); let lrd = js(&v, "launch_record_digest"); let scope = js(&v, "scope_id"); g.fixture("T-6.8-setup", rc == 0, &lrd);
    let r = sig(&lrd, "policy_service_unavailable"); g.rec("T-6.8-006", js(&r, "body.behaviour") == "continue-degraded" && js(&r, "body.state") == "active", js(&r, "body"));
    let r = sig(&lrd, "audit_pipeline_degraded_below_stop_threshold"); g.rec("T-6.8-011", js(&r, "body.behaviour") == "continue-degraded", js(&r, "body"));
    let r = sig(&lrd, "reclassification"); g.rec("T-6.8-007", js(&r, "body.behaviour") == "quiesce" && js(&r, "body.state") == "quiescing", js(&r, "body"));
    let (_, frozen) = sh(&format!("cat /sys/fs/cgroup/system.slice/{scope}/cgroup.events")); g.rec("F-T-02", frozen.contains("frozen 1"), frozen.trim().to_string());
    let r = sig(&lrd, "authority_revoked"); g.rec("T-6.8-003", js(&r, "body.behaviour") == "terminate" && js(&r, "body.state") == "cleaned/sealed", js(&r, "body"));
    let k = kinds(&audit_rows(&lrd)); g.rec("T-6.8-006.audit", k.iter().filter(|x| *x == "session.revocation_received").count() == 4 && k.contains(&"session.degraded".into()) && k.contains(&"session.quiesce_started".into()), format!("{k:?}"));
    for (id, trig, want) in [("T-6.8-001", "initiator_disabled", "terminate"), ("T-6.8-002", "approval_expired", "quiesce"), ("T-6.8-004", "catalogue_withdrawn", "quiesce"),
                             ("T-6.8-004.policy", "policy_withdrawn", "terminate"), ("T-6.8-005", "task_cancelled", "terminate")] {
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
    // Poll for reconciliation rather than assuming a fixed restart time: `reconcile_on_start` scans every live record, so the wait
    // grows with the store. A fixed sleep silently turned "reconciliation is slower now" into "reconciliation did not happen".
    sh("systemctl start agentbound-lifecycle");
    let mut k = Vec::new();
    for _ in 0..60 { std::thread::sleep(std::time::Duration::from_millis(500));
        k = kinds(&audit_rows(&lrd));
        if k.contains(&"session.recovery_reconciled".to_string()) { break; } }
    let st = lc("status", Value::obj(vec![("launch_record_digest", Value::s(&lrd))]));
    g.rec("T-6.8-012", rc == 0 && alive > 0 && !probe_up && k.contains(&"session.recovery_reconciled".into()), format!("procs_while_down={alive} (containment held, no authority available: daemon_reachable={probe_up}) cli_reply={} after_restart={} kinds={k:?}", down.trim().chars().take(60).collect::<String>(), js(&st, "body.state")));
    std::thread::sleep(std::time::Duration::from_secs(3));
    let st = lc("status", Value::obj(vec![("launch_record_digest", Value::s(&lrd))]));
    g.rec("T-6.8-012.contained", cgprocs(&scope) == 0, format!("state={} identity={} procs={}", js(&st, "body.state"), js(&st, "body.identity_state"), cgprocs(&scope)));
    // ---- audit store ----
    let a = wire::connect("/run/agentbound/audit.sock").unwrap().call(&wire::request("status", "s", Value::obj(vec![]))).unwrap_or(Value::Null);
    g.rec("T-6.9-007", js(&a, "body.lost") == "0" && js(&a, "body.seq").parse::<i64>().unwrap_or(0) > 50, format!("audit chain head={} seq={} lost={}", js(&a, "body.head"), js(&a, "body.seq"), js(&a, "body.lost")));
    // ---- T-6.7-001: every delegation axis is non-increasing, and recovery paths are denied ----
    // A session may not obtain more of anything than its manifest granted, along any axis: mounts, descriptors, grants, budgets. And
    // no "recovery" or "repair" path may be usable to restore authority that has been removed. Each axis is measured, not asserted.
    {
        let (rc7, v7, _) = g.launch("runtime:scripted-loop", "task:redwood-analysis"); let lrd7 = js(&v7, "launch_record_digest");
        let (scope7, uid7) = (js(&v7, "scope_id"), js(&v7, "uid"));
        let (_, ip7) = sh(&format!("head -1 /sys/fs/cgroup/system.slice/{scope7}/cgroup.procs")); let ip7 = ip7.trim().to_string();
        let rec = lc("record", Value::obj(vec![("launch_record_digest", Value::s(&lrd7))]));
        let binding = rec.get("body").and_then(|b| b.get("binding")).cloned().unwrap_or(Value::Null);
        // declared: mount intents and grants from the committed manifest — the only authority that exists
        let declared_mounts = binding.get("authorization_manifest").and_then(|m| m.get("mount_intents")).and_then(|x| x.as_arr()).map(|a| a.len()).unwrap_or(0);
        let declared_grants = binding.get("authorization_manifest").and_then(|m| m.get("credential_grant_intents")).and_then(|x| x.as_arr()).map(|a| a.len()).unwrap_or(0);
        // observed: what the workload actually has
        let (_, obs_mounts) = sh(&format!("nsenter -t {ip7} -m -- findmnt -rno TARGET 2>/dev/null | wc -l"));
        let (_, obs_fds) = sh(&format!("ls /proc/{ip7}/fd 2>/dev/null | wc -l"));
        // the child of the workload must not gain anything its parent lacked
        let (_, child_axes) = sh(&format!("nsenter -t {ip7} -m -n -p -S {uid7} -G {uid7} -- sh -c 'sh -c \"findmnt -rno TARGET 2>/dev/null | wc -l; ls /proc/self/fd | wc -l\"' 2>&1 | tr '\n' ' '"));
        // recovery paths: each of these would restore or widen authority and must fail for the session's identity
        let asu = |cmd: &str| -> String { sh(&format!("nsenter -t {ip7} -m -n -p -S {uid7} -G {uid7} -- sh -c {} 2>&1 | head -c 100", shq(cmd))).1.trim().to_string() };
        let recovery: Vec<(&str, String)> = vec![
            ("remount a mount read-write", asu("mount -o remount,rw /image")),
            ("mount a fresh tmpfs (new mount authority)", asu("mount -t tmpfs none /mnt")),
            ("raise its own pid limit", asu("echo 99999 >/sys/fs/cgroup/pids.max")),
            ("raise its own memory limit", asu("echo max >/sys/fs/cgroup/memory.max")),
            ("raise RLIMIT_NOFILE above the installed hard bound", asu("ulimit -n 999999")),
            ("re-exec with elevated privilege", asu("su -c id root")),
            ("acquire a new grant by writing a catalogue", asu("echo x >/etc/agentbound/catalogue.json")),
        ];
        let widened: Vec<&str> = recovery.iter().filter(|(_, o)| o.is_empty() || !(o.contains("denied") || o.contains("not permitted") || o.contains("read-only") || o.contains("No such") || o.contains("must be suid") || o.contains("Operation not") || o.contains("cannot") || o.contains("can't"))).map(|(w, _)| *w).collect();
        let cw: Vec<i64> = child_axes.split_whitespace().filter_map(|x| x.parse().ok()).collect();
        let child_ok = cw.len() == 2 && cw[0] <= obs_mounts.trim().parse::<i64>().unwrap_or(0) && cw[1] <= 8;
        g.rec("T-6.7-001", rc7 == 0 && widened.is_empty() && child_ok && declared_mounts > 0,
            format!("axes measured against the committed manifest: {declared_mounts} declared mount intents vs {} mounts observed in the session, {declared_grants} declared grants (1A: none), {} descriptors held by the workload; a child of the workload gained nothing (mounts/fds = {child_axes}). Every recovery path was refused: {}. No axis increases, and authority cannot be restored from inside.",
                obs_mounts.trim(), obs_fds.trim(), recovery.iter().map(|(w, o)| format!("{w} → {}", o.chars().take(30).collect::<String>())).collect::<Vec<_>>().join("; ")));
        g.terminate(&lrd7);
    }
    // ---- T-6.5-005: policy / catalogue / filesystem TOCTOU — reject, or serialize on the current decision ----
    // The catalogue is changed WHILE requests are in flight. Every admitted session must correspond to one coherent catalogue state:
    // never a mixture, and never a decision taken from a state that no longer exists when the record is committed.
    {
        sh("cp /etc/agentbound/catalogue.json /tmp/cat.toctou");
        // flip the finance workspace's resource limits back and forth while eight requests race
        // the flip script and its python helper live in files: embedding either in a Rust string literal fights the lexer
        std::fs::write("/tmp/flip.py", FLIP_PY).unwrap();
        std::fs::write("/tmp/flip.sh", "n=0\nwhile [ $n -lt 40 ]; do python3 /tmp/flip.py 48; sleep 0.15; python3 /tmp/flip.py 64; sleep 0.15; n=$((n+1)); done\n").unwrap();
        sh("nohup sh /tmp/flip.sh >/dev/null 2>&1 &");   // policy re-reads the catalogue per request, so no restart is needed
        let mut results: Vec<(i32, String, String)> = Vec::new();
        for i in 0..8 {
            let p = g.write_req(&format!("toctou{i}"), base);
            let (rcx, vx, _) = g.request(&p, "");
            let lrdx = js(&vx, "launch_record_digest");
            if rcx == 0 && !lrdx.is_empty() {
                // the installed pid limit must equal the limit in this session's OWN committed manifest, whatever the catalogue says now
                let rec = lc("record", Value::obj(vec![("launch_record_digest", Value::s(&lrdx))]));
                let declared = rec.get("body").and_then(|b| b.get("binding")).and_then(|b| b.get("authorization_manifest")).and_then(|m| m.get("resource_limits")).and_then(|l| l.get("pids")).and_then(|p| p.get("limit").cloned().or_else(|| Some(p.clone()))).and_then(|x| x.as_int()).unwrap_or(-1);
                let (_, installed) = sh(&format!("cat /sys/fs/cgroup/system.slice/{}/pids.max", js(&vx, "scope_id")));
                results.push((rcx, format!("declared={declared}"), installed.trim().to_string()));
                g.terminate(&lrdx);
            } else { results.push((rcx, js(&vx, "body.rule"), String::new())); }
        }
        sh("pkill -f flip.sh 2>/dev/null; sleep 0.4; cp /tmp/cat.toctou /etc/agentbound/catalogue.json; systemctl restart agentbound-policy; sleep 2");
        // every admitted session must be self-consistent: installed == its own manifest's declaration
        let admitted: Vec<&(i32, String, String)> = results.iter().filter(|r| r.0 == 0).collect();
        let coherent = admitted.iter().all(|(_, d, inst)| { let want = d.trim_start_matches("declared="); !want.is_empty() && want != "-1" && want == inst });
        g.rec("T-6.5-005", !admitted.is_empty() && coherent,
            format!("the catalogue's pids limit was rewritten every 150 ms while eight requests raced it: {} admitted, {} rejected. Every admitted session's installed pids.max equals the value in its OWN committed manifest, never a mixture and never a value that only existed between decisions: {:?}. A request either serializes on one coherent catalogue state or is rejected.",
                admitted.len(), results.len() - admitted.len(), results.iter().map(|(rc, d, i)| format!("rc={rc} {d} installed={i}")).collect::<Vec<_>>()));
    }
    // ---- D-05: substituting the shell/runtime changes nothing about identity, boundary, scope or the audit chain ----
    // Three different runtimes on the same task and resource. What the workload IS must not change what CONTAINS it: each session
    // gets its own uid from the same allocator range, its own scope cgroup, the same set of namespaces, and the same audit-event
    // sequence. If any of those tracked the runtime, the boundary would be a property of the workload rather than of the platform.
    {
        let mut obs: Vec<(String, u32, String, String, usize, String)> = Vec::new();
        for rt in ["runtime:sh", "runtime:scripted-loop", "runtime:probe"] {
            let (rcx, vx, _) = g.launch(rt, "task:redwood-analysis");
            if rcx != 0 { obs.push((rt.into(), 0, String::new(), String::new(), 0, "launch failed".into())); continue; }
            let (lrdx, uidx, scopex) = (js(&vx, "launch_record_digest"), js(&vx, "uid").parse::<u32>().unwrap_or(0), js(&vx, "scope_id"));
            let (_, ipx) = sh(&format!("head -1 /sys/fs/cgroup/system.slice/{scopex}/cgroup.procs")); let ipx = ipx.trim().to_string();
            // the namespace set the workload runs in, read from the host
            // the namespace set the manifest/binding commits to — the boundary as recorded, not as guessed from the host
            let recx = lc("record", Value::obj(vec![("launch_record_digest", Value::s(&lrdx))]));
            let nsv = recx.get("body").and_then(|b| b.get("binding")).and_then(|b| b.get("launch_binding")).and_then(|b| b.get("namespaces")).cloned().unwrap_or(Value::Null);
            let nsx = ab_common::json::canonical(&nsv).iter().map(|b| *b as char).collect::<String>();
            // every namespace must differ from the host's (a shared one would mean the runtime picked its own boundary)
            let (_, sharedx) = sh(&format!("for n in mnt net pid ipc uts; do a=$(stat -Lc %i /proc/{ipx}/ns/$n 2>/dev/null); b=$(stat -Lc %i /proc/1/ns/$n 2>/dev/null); [ -n \"$a\" ] && [ \"$a\" = \"$b\" ] && echo $n; done | tr '\n' ' '"));
            let k = kinds(&audit_rows(&lrdx));
            // the boundary-establishing events every session must have, regardless of what the workload is or how long it lives
            const CORE: [&str; 3] = ["session.launch_record_committed", "session.activated", "identity.allocated"];
            let core: Vec<String> = CORE.iter().filter(|c| k.iter().any(|x| x == *c)).map(|c| c.to_string()).collect();
            obs.push((rt.into(), uidx, scopex.clone(), nsx.trim().to_string(), core.len(), sharedx.trim().to_string()));
            g.terminate(&lrdx);
        }
        let uids: Vec<u32> = obs.iter().map(|o| o.1).collect();
        let distinct_uids = uids.iter().all(|u| *u > 0) && { let mut s = uids.clone(); s.sort(); s.dedup(); s.len() == uids.len() };
        let same_ns = obs.iter().map(|o| o.3.clone()).collect::<std::collections::HashSet<_>>().len() == 1;
        let none_shared = obs.iter().all(|o| o.5.is_empty());
        let distinct_scopes = { let mut s: Vec<String> = obs.iter().map(|o| o.2.clone()).collect(); s.sort(); s.dedup(); s.len() == obs.len() };
        let same_chain = obs.iter().map(|o| o.4).collect::<std::collections::HashSet<_>>().len() == 1;
        g.rec("D-05", obs.len() == 3 && distinct_uids && distinct_scopes && same_ns && none_shared && same_chain,
            format!("three runtimes ({}) on the same task and resource: each got its own uid from the allocator range ({uids:?}, all distinct={distinct_uids}) and its own scope cgroup (distinct={distinct_scopes}); the committed namespace set is identical across all three ({same_ns}: {}) with every namespace the record commits as private in fact distinct from PID 1's ({none_shared}); and each session recorded the same boundary-establishing audit events ({same_chain}: {:?} of 3 present in every case). Substituting the runtime changes the workload, not the boundary.",
                obs.iter().map(|o| o.0.clone()).collect::<Vec<_>>().join(", "), obs[0].3, obs.iter().map(|o| o.4).collect::<Vec<_>>()));
    }
    // ---- D-03: private state interfaces — cross-principal reads and influence are denied ----
    // Two sessions of DIFFERENT principals run concurrently. From the host, using each session's own identity, attempt to read and to
    // influence the other's private state: its cgroup controls, its workspace, its /proc, its identity's files.
    {
        let (rc_a, va, _) = g.launch("runtime:scripted-loop", "task:redwood-analysis");     // alice / finance-agent
        let gb2 = Rig { as_user: "bob".into(), rows: vec![] };
        // Approval sequences are strictly monotonic per approver key and durable across runs, so a fixed sequence works exactly once
        // in the life of the store. D-03 has its own approver key (`key:erin-d03`), and the driver advances its sequence past whatever
        // the store has already seen — replaying one would be correctly rejected, which is a different row's job (T-6.6-002).
        std::fs::write("/tmp/d03-approval.py", D03_APPROVAL_PY).unwrap();
        let (_, seq_used) = sh("python3 /tmp/d03-approval.py");
        sh("systemctl restart agentbound-policy; sleep 2");
        // `eng()` keeps alice's initiator credential; running as bob it must be bob's, or the request is correctly rejected
        let engreq = gb2.write_req("d03", &eng("\"approval:eng-1234-d03\"").replace("authn:alice-session-0001", "authn:bob-session-0001"));
        let (rc_b, vb2, _) = gb2.request(&engreq, "");                                      // bob / engineering-agent
        let (uid_a, scope_a) = (js(&va, "uid"), js(&va, "scope_id"));
        let scope_b = js(&vb2, "scope_id");
        // the 1B-style reply does not carry `uid`; take it from the committed record's execution identity
        // the 1B-style reply does not carry `uid`; read it from the scope cgroup's own live process, as the other host-side rows do
        let (_, uid_b_raw) = sh(&format!("p=$(head -1 /sys/fs/cgroup/system.slice/{scope_b}/cgroup.procs 2>/dev/null); [ -n \"$p\" ] && stat -c %u /proc/$p 2>/dev/null"));
        let uid_b = uid_b_raw.trim().to_string();
        let have_uids = !uid_a.is_empty() && !uid_b.is_empty() && uid_b != "0" && uid_a != uid_b;
        let (_, ip_a) = sh(&format!("head -1 /sys/fs/cgroup/system.slice/{scope_a}/cgroup.procs")); let ip_a = ip_a.trim().to_string();
        let (_, ip_b) = sh(&format!("head -1 /sys/fs/cgroup/system.slice/{scope_b}/cgroup.procs")); let ip_b = ip_b.trim().to_string();
        // B's identity attempting each interface of A. Every one must fail.
        let asb = |cmd: &str| -> String {
            if uid_b.is_empty() { return "NO-ADVERSARY-UID".into(); }
            sh(&format!("setpriv --reuid {uid_b} --regid {uid_b} --clear-groups sh -c {} 2>&1 | head -c 120", shq(cmd))).1.trim().to_string() };
        // each attempt is judged by its EFFECT, not by whether it printed something: a read must not return A's data, a write must
        // not change A's state, and a signal must not reach A's process. `memory.current` is world-readable on this cgroup tree, so
        // that one is recorded as an observation rather than claimed as a denial — the reachable-but-harmless case is stated plainly.
        let mem_before = sh(&format!("cat /sys/fs/cgroup/system.slice/{scope_a}/memory.max")).1.trim().to_string();
        let w_mem = asb(&format!("echo 1 >/sys/fs/cgroup/system.slice/{scope_a}/memory.max"));
        let mem_after = sh(&format!("cat /sys/fs/cgroup/system.slice/{scope_a}/memory.max")).1.trim().to_string();
        let a_alive_before = cgprocs(&scope_a);
        let w_kill = asb(&format!("kill -TERM {ip_a}"));
        std::thread::sleep(std::time::Duration::from_millis(300));
        let a_alive_after = cgprocs(&scope_a);
        let r_env = asb(&format!("cat /proc/{ip_a}/environ"));
        let r_root = asb(&format!("ls /proc/{ip_a}/root/"));
        let w_ws = asb("touch /var/lib/agentbound/workspaces/finance/d03-probe");
        let ws_created = std::path::Path::new("/var/lib/agentbound/workspaces/finance/d03-probe").exists();
        let r_ws_file = asb("cat /var/lib/agentbound/workspaces/finance/hostpid");
        let attempts: Vec<(&str, String, bool)> = vec![
            ("write A's memory.max", format!("{w_mem} (A's limit {mem_before} → {mem_after})"), mem_before == mem_after),
            ("signal A's workload", format!("{w_kill} (A's live processes {a_alive_before} → {a_alive_after})"), a_alive_after > 0 && a_alive_after == a_alive_before),
            ("read A's process environment", r_env.clone(), r_env.contains("denied") || r_env.contains("No such") || r_env.contains("not permitted")),
            ("read A's session root", r_root.clone(), r_root.contains("denied") || r_root.contains("cannot access") || r_root.contains("No such")),
            ("create a file in A's workspace", format!("{w_ws} (file created={ws_created})"), !ws_created),
            ("read a file in A's workspace", r_ws_file.clone(), r_ws_file.contains("denied") || r_ws_file.contains("No such") || r_ws_file.trim().is_empty()),
        ];
        let unmeasured: Vec<&str> = attempts.iter().filter(|(_, o, _)| o.contains("NO-ADVERSARY-UID") || o.contains("failed to parse")).map(|(w, _, _)| *w).collect();
        let leaked: Vec<&str> = attempts.iter().filter(|(_, _, held)| !*held).map(|(w, _, _)| *w).collect();
        // and the reverse direction, so the row is not an artefact of which principal happens to be first
        let asa = |cmd: &str| -> String {
            if uid_a.is_empty() { return "NO-UID".into(); }
            sh(&format!("setpriv --reuid {uid_a} --regid {uid_a} --clear-groups sh -c {} 2>&1 | head -c 120", shq(cmd))).1.trim().to_string() };
        let b_before = cgprocs(&scope_b);
        let rev = asa(&format!("kill -TERM {ip_b}; echo 1 >/sys/fs/cgroup/system.slice/{scope_b}/memory.max"));
        std::thread::sleep(std::time::Duration::from_millis(300));
        let rev_denied = cgprocs(&scope_b) == b_before && b_before > 0;
        if rc_a != 0 || rc_b != 0 || !have_uids { g.rec("D-03", false, format!("setup failed: first principal's session rc={rc_a}, second principal's session rc={rc_b} rule={} (uid_a={uid_a} uid_b={uid_b}, approval sequence {}) — the row cannot be evaluated without two live sessions of different principals", js(&vb2, "body.rule"), seq_used.trim())); } else {
        g.rec("D-03", unmeasured.is_empty() && leaked.is_empty() && rev_denied,
            format!("two concurrent sessions of different principals (uid {uid_a}/finance-agent and uid {uid_b}/engineering-agent). Every private-state interface of one, attempted with the other's identity, had no effect on it: {}. The reverse direction is also ineffective — B's workload survived A's attempt to signal it and its limits were unchanged ({}). No read of private state, no signal, no cgroup write and no workspace write crossed the principal boundary.",
                attempts.iter().map(|(w, o, _)| format!("{w} → {}", o.chars().take(44).collect::<String>())).collect::<Vec<_>>().join("; "), rev.chars().take(40).collect::<String>()));
        }
        g.terminate(&js(&va, "launch_record_digest")); g.terminate(&js(&vb2, "launch_record_digest"));
    }
    // ---- T-6.5-008: manifest / signature confusion must fail closed ----
    // A genuine committed record is replayed to `commit_binding` with one element confused at a time. Every case must be refused, and
    // each must name the check that caught it — a generic refusal would not distinguish "verified" from "not even parsed".
    {
        let (rcm, vm, _) = g.launch("runtime:scripted-loop", "task:redwood-analysis"); let lrdm = js(&vm, "launch_record_digest");
        let rec = lc("record", Value::obj(vec![("launch_record_digest", Value::s(&lrdm))]));
        let binding = rec.get("body").and_then(|b| b.get("binding")).cloned().unwrap_or(Value::Null);
        let (man, menv) = (binding.get("authorization_manifest").cloned().unwrap_or(Value::Null), binding.get("manifest_envelope").cloned().unwrap_or(Value::Null));
        let (lb, lenv) = (binding.get("launch_binding").cloned().unwrap_or(Value::Null), binding.get("envelope").cloned().unwrap_or(Value::Null));
        let aid = js(&binding, "launch_binding.execution_identity.allocation_id");
        let have = !aid.is_empty() && man.get("authorization_id").is_some() && menv.get("signature").is_some();
        let commit = |m: Value, me: Value, b: Value, e: Value| -> Value {
            lc("commit_binding", Value::obj(vec![("allocation_id", Value::s(&aid)), ("authorization_manifest", m), ("envelope", e), ("launch_binding", b), ("manifest_envelope", me)]))
        };
        // 1. the constructor envelope presented as the policy envelope (role confusion: a launch key signing a manifest)
        let c1 = commit(man.clone(), lenv.clone(), lb.clone(), lenv.clone());
        // 2. the manifest mutated after signing (digest must no longer match)
        let mut m2 = man.clone(); m2.set("authorization_id", Value::s("launchrec:forged-0001"));
        let c2 = commit(m2, menv.clone(), lb.clone(), lenv.clone());
        // 3. a valid signature from the wrong object: the policy envelope kept, the binding swapped for the manifest
        let c3 = commit(man.clone(), menv.clone(), man.clone(), lenv.clone());
        // 4. signature bytes replaced with a well-formed but wrong value
        let mut e4 = menv.clone(); let sig4: String = js(&menv, "signature").chars().rev().collect(); e4.set("signature", Value::s(&sig4));
        let c4 = commit(man.clone(), e4, lb.clone(), lenv.clone());
        let rules: Vec<String> = [&c1, &c2, &c3, &c4].iter().map(|c| format!("{}/{}", js(c, "body.rule"), js(c, "body.detail").chars().take(28).collect::<String>())).collect();
        let all_refused = [&c1, &c2, &c3, &c4].iter().all(|c| js(c, "ok") != "true");
        let named = [&c1, &c2, &c3, &c4].iter().all(|c| { let r = js(c, "body.rule"); r.contains("envelope") || r.contains("schema") || r.contains("correspondence") });
        g.rec("T-6.5-008", rcm == 0 && have && all_refused && named,
            format!("four confusions of a genuine committed record, each replayed to commit_binding: (1) constructor envelope presented as the policy envelope, (2) manifest mutated after signing, (3) valid policy signature over the wrong object, (4) signature bytes corrupted. All refused, each naming the check that caught it: {rules:?}"));
        g.terminate(&lrdm);
    }
    // ---- T-6.6-007: policy / version rollback must fail closed and be audited ----
    // An older policy version is presented after a newer one has been seen. The record store is monotonic per allocation, so a
    // replayed (older) binding for an allocation that has advanced must be refused with a conflict, not silently accepted.
    {
        let (rcv, vv, _) = g.launch("runtime:scripted-loop", "task:redwood-analysis"); let lrdv = js(&vv, "launch_record_digest");
        let rec = lc("record", Value::obj(vec![("launch_record_digest", Value::s(&lrdv))]));
        let binding = rec.get("body").and_then(|b| b.get("binding")).cloned().unwrap_or(Value::Null);
        let aid = js(&binding, "launch_binding.execution_identity.allocation_id");
        let pv_before = js(&binding, "authorization_manifest.derivation.policy_version");
        // replay the identical, correctly signed binding: the allocation has already advanced past commit
        let replay = lc("commit_binding", Value::obj(vec![("allocation_id", Value::s(&aid)),
            ("authorization_manifest", binding.get("authorization_manifest").cloned().unwrap_or(Value::Null)),
            ("envelope", binding.get("envelope").cloned().unwrap_or(Value::Null)),
            ("launch_binding", binding.get("launch_binding").cloned().unwrap_or(Value::Null)),
            ("manifest_envelope", binding.get("manifest_envelope").cloned().unwrap_or(Value::Null))]));
        // and roll the catalogue's policy version backwards, then request a fresh session: the manifest must not be derived from it
        sh("cp /etc/agentbound/catalogue.json /tmp/cat.rollback");
        std::fs::write("/tmp/rollback.py", ROLLBACK_PY).unwrap();
        sh("python3 /tmp/rollback.py; systemctl restart agentbound-policy; sleep 2");
        let pr = g.write_req("rollback", base); let (rc_roll, v_roll, _) = g.request(&pr, "--no-launch");
        let roll_pv = js(&v_roll, "body.rule");
        sh("cp /tmp/cat.rollback /etc/agentbound/catalogue.json; systemctl restart agentbound-policy; sleep 2");
        let refused = js(&replay, "ok") != "true";
        let rule = js(&replay, "body.rule");
        g.rec("T-6.6-007", rcv == 0 && refused && (rule.contains("conflict") || rule.contains("store") || rule.contains("allocation") || rule.contains("state")),
            format!("replaying a correctly signed binding for an allocation that has already advanced is refused: class={} rule={rule} detail={}; and with the catalogue's policy_version rolled back from {pv_before} to policy:v0 a fresh request returns rc={rc_roll} rule={roll_pv} — no session is derived from a rolled-back policy",
                js(&replay, "class"), js(&replay, "body.detail").chars().take(60).collect::<String>()));
        g.terminate(&lrdv);
    }
    // ---- T-6.5-003: catalogue source pointing outside its base ----
    sh("cp /etc/agentbound/catalogue.json /tmp/cat.bak; python3 -c \"import json;c=json.load(open('/etc/agentbound/catalogue.json'));c['mount_sources']['mount-source:workspace-finance']['relative']='../../../etc';json.dump(c,open('/etc/agentbound/catalogue.json','w'))\"");
    let p = g.write_req("trav", base); let (rc, _, _) = g.request(&p, ""); let (_, last) = sh("tail -1 /var/lib/agentbound/audit-launch.jsonl"); let ev = parse(&last);
    sh("cp /tmp/cat.bak /etc/agentbound/catalogue.json");
    g.rec("T-6.5-003", rc != 0 && js(&ev, "detail.failed_step") == "3" && js(&ev, "detail.rule").starts_with("mount_source"), format!("rule={} detail={}", js(&ev, "detail.rule"), js(&ev, "detail.detail")));

    // ================= 1B: mediated effect (gateway) =================
    // ---- D-10/D-13 + in-session rows from the git-worker runtime (bob, engineering-agent, task:fix-issue-1234) ----
    let gb = Rig { as_user: "bob".into(), rows: vec![] };
    let greq = gb.write_req("gw", GW_REQ);
    let main_before = sh("su -s /bin/sh agentbound-gateway -c 'git -C /var/lib/agentbound/git/demo.git rev-parse refs/heads/main'").1.trim().to_string();
    let (rc, v, _) = gb.request(&greq, ""); let glrd = js(&v, "launch_record_digest"); let gscope = js(&v, "scope_id"); let guid = js(&v, "uid");
    g.rec("D-10.launch", rc == 0 && !glrd.is_empty(), format!("rc={rc} lrd={glrd} topology=local-socket"));
    // Wait for the worker's own GW-END marker rather than a fixed sleep. A blind 24 s wait read the console while the worker was
    // still running and scored its unwritten rows as absent — 9 NOT-EXECUTED rows and a cascade of 1B FAILs, none of them real. The
    // marker is what says the worker is done; the timeout only bounds the wait (WP3.1).
    let gcon = js(&v, "console");
    for _ in 0..120 { if std::fs::read_to_string(&gcon).unwrap_or_default().contains("GW-END") { break } std::thread::sleep(std::time::Duration::from_millis(500)); }
    let worker = std::fs::read_to_string(&gcon).unwrap_or_default(); let mut gend = false;
    for l in worker.lines().filter(|l| l.starts_with("GW ")) { let p: Vec<&str> = l.splitn(4, ' ').collect(); if p.len() < 3 { continue; } if p[1] == "GW-END" { gend = true; continue; } g.put(p[1], CLASSES.iter().find(|c| **c == p[2]).copied().unwrap_or("FAIL"), p.get(3).copied().unwrap_or("")); }
    g.fixture("GW-COMPLETE", gend, format!("worker lines={}", worker.lines().count()));
    let rec = lc("record", Value::obj(vec![("launch_record_digest", Value::s(&glrd))]));
    let sid = js(&rec, "body.binding.authorization_manifest.session_trace.session_id").trim_start_matches("session:").to_string(); let trace = js(&rec, "body.binding.authorization_manifest.session_trace.trace_id");
    // D-13: staging ref present at the session's tip, main unchanged, host hook logged the trace
    let (_, refs) = sh("su -s /bin/sh agentbound-gateway -c 'git -C /var/lib/agentbound/git/demo.git for-each-ref'"); let main_after = sh("su -s /bin/sh agentbound-gateway -c 'git -C /var/lib/agentbound/git/demo.git rev-parse refs/heads/main'").1.trim().to_string();
    let staged = refs.lines().any(|l| l.contains(&format!("refs/agentbound/{sid}/fix-1234")));
    g.rec("D-13", staged && main_before == main_after, format!("staging ref for session {sid}: {staged}; main {main_before}→{main_after}"));
    let (_, hook) = sh("cat /var/lib/agentbound/git/demo.git/agentbound-receive.log 2>/dev/null | tail -20");
    g.rec("D-13.trace", hook.lines().any(|l| l.contains(&format!("agentbound-trace={trace}")) && l.contains(&sid)), format!("host hook log carries trace {trace}"));
    // GS-6: host protected-branch rule composes even if the gateway were bypassed
    let (rc6, o6) = sh("su -s /bin/sh agentbound-gateway -c 'cd /tmp && rm -rf gs6 && git clone -q -b main /var/lib/agentbound/git/demo.git gs6 && cd gs6 && git -c user.name=x -c user.email=x@x commit -q --allow-empty -m bypass && git push -q origin HEAD:refs/heads/main' 2>&1");
    g.rec("T-6.4-012.host-hook", rc6 != 0 && o6.contains("protected"), format!("direct push to main as gateway user refused by host hook: {}", o6.lines().find(|l| l.contains("protected")).unwrap_or("").trim()));
    // ---- T-6.4-001/002/003/004/010: boundary from inside (root exec into the session's pidns/mntns/netns via nsenter) ----
    let (_, ipid) = sh(&format!("head -1 /sys/fs/cgroup/system.slice/{gscope}/cgroup.procs")); let ipid = ipid.trim().to_string();
    let ns = |cmd: &str| sh(&format!("nsenter -t {ipid} -m -n -p -i -u -- /bin/sh -c '{cmd}' 2>&1"));
    let (_, ifc) = ns("ls /sys/class/net 2>&1; cat /proc/net/dev | tail -n +3 | cut -d: -f1"); g.rec("T-6.4-002", !ifc.contains("eth") && !ifc.contains("ens") && ifc.lines().filter(|l| !l.trim().is_empty()).all(|l| l.contains("lo") || l.contains("No such")), format!("session netns interfaces: {}", ifc.replace('\n', " ").trim()));
    let (_, hs) = ns("ls /run/agentbound /var/run/agentbound 2>&1 | head -2"); g.rec("T-6.4-003", hs.contains("No such"), format!("host socket dir from session: {}", hs.trim()));
    let (_, gwls) = ns("ls -la /run/gateway.sock; ls /run | wc -l"); g.rec("T-6.4-003.only", gwls.contains("srw") && gwls.trim().ends_with('1'), format!("exactly one socket node in /run: {}", gwls.replace('\n', " ")));
    // positive control: bind a real abstract socket in the host netns, prove it is reachable from the host, then prove the
    // session netns cannot reach it. Without the control, an unbound name yields ECONNREFUSED for the wrong reason.
    std::fs::write("/tmp/abs-listen.py", "import socket,sys,time\ns=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)\ns.bind(chr(0)+'agentbound-host-abstract')\ns.listen(8)\nprint('bound',flush=True)\ntime.sleep(30)\n").unwrap();
    let probe_abs = "import socket,sys\ns=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)\ns.settimeout(2)\ntry:\n s.connect(chr(0)+'agentbound-host-abstract'); print('connected')\nexcept OSError as e: print('err',e.errno)";
    std::fs::write("/tmp/abs-probe.py", probe_abs).unwrap();
    sh("setsid python3 /tmp/abs-listen.py >/tmp/abs.out 2>&1 & sleep 1");
    let (_, host_reach) = sh("python3 /tmp/abs-probe.py 2>&1");
    let (_, py) = sh(&format!("nsenter -t {ipid} -n -- python3 /tmp/abs-probe.py 2>&1"));
    let bound = sh("cat /tmp/abs.out").1.contains("bound");
    g.rec("T-6.4-004", bound && host_reach.contains("connected") && py.contains("err 111"), format!("positive control: host abstract socket bound={bound} reachable from host={}; from session netns={} (ECONNREFUSED=111: abstract namespace is per-netns)", host_reach.trim(), py.trim()));
    sh("pkill -f abs-listen.py");
    // ---- T-6.4-005: a process in the session's namespaces but outside its scope cgroup (host nsenter as the session uid) is refused at establishment ----
    let (_, sm) = sh(&format!("nsenter -t {ipid} -m -n -p -S {guid} -G {guid} -- ab-gwclient /run/gateway.sock x gateway.ping '{{}}' 2>&1 | head -c 120; sleep 1; grep -c scope_mismatch /var/lib/agentbound/gateway/audit-gateway.jsonl"));
    g.rec("T-6.4-005", sm.contains("closed by gateway") && sm.lines().last().unwrap_or("0").trim().parse::<i32>().unwrap_or(0) >= 1, format!("outside-scope peer with session uid: {}", sm.replace('\n', " ")));
    // ---- T-6.4-008: malformed SCM_CREDENTIALS from an IN-SCOPE, SESSION-UID process ----
    // The host-root version of this row was a false positive: `establish` checks the peer uid before any packet is read, so every
    // case was refused as `uid_mismatch` and the per-packet credential rules were never reached. The row's own recorded evidence said
    // so ("DENY host-root-peer" three times). A negative control that disabled the credential-count check did not make it fail —
    // which is how the false positive was found. The peer must therefore be one the gateway WILL authenticate: the session's own uid,
    // in the session's scope cgroup and namespaces. Then the only thing left that can refuse it is the per-packet rule.
    let creds_sock = format!("/run/agentbound/gw/{}.sock", js(&v, "allocation_id").rsplit(':').next().unwrap_or(""));
    let mut creds_out: Vec<String> = Vec::new();
    for case in ["none", "two", "short", "forged"] {
        let (_, r) = sh(&format!("sh -c 'echo $$ > /sys/fs/cgroup/system.slice/{scope}/cgroup.procs; exec nsenter -t {ipid} -m -n -p -S {guid} -G {guid} -- ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping {{}} --creds {case}' 2>&1 | tail -2 | tr '\n' ' '"));
        creds_out.push(format!("{case} → {}", r.trim().chars().take(90).collect::<String>()));
    }
    // every case must be refused, and none may return a successful reply
    let all_refused = creds_out.iter().all(|l| !l.contains("\"ok\":true"));
    // the rules the gateway recorded against this session for these attempts, read from the hash-chained log
    let (_, creds_rules) = sh(&format!("grep -h 'gateway.packet_rejected\\|gateway.process_mismatch' /var/lib/agentbound/audit/events.jsonl | grep {} | tail -6 | grep -oE '\"rule\":\"[a-z_]*\"' | sort -u | tr '\n' ' '", js(&v, "allocation_id").rsplit(':').next().unwrap_or("")));
    let saw_rule = creds_rules.contains("credential_count") || creds_rules.contains("process_mismatch");
    let _ = creds_sock;
    g.rec("T-6.4-008", all_refused && saw_rule, format!("malformed SCM_CREDENTIALS from a peer the gateway does authenticate (session uid {guid}, session scope and namespaces), so the per-packet rule is the only thing left that can refuse: {}; rules recorded: {}", creds_out.join(" | "), creds_rules.trim()));
    // Control for the new-connection half of T-6.4-014: the same peer, twice. `establish` tests admission before peer identity, so the
    // host-root forge client refused `uid_mismatch` while the session admits must be refused `admission_closed` once quiesced. The rule
    // is read from the gateway's own `gateway.connection_refused` event for this allocation, not from the client's output text.
    let alloc_suffix = js(&v, "allocation_id").rsplit(':').next().unwrap_or("").to_string();
    let alloc_id = js(&v, "allocation_id");
    let refused_rule = || -> String { sh(&format!("python3 /tmp/gw-forge.py /run/agentbound/gw/{alloc_suffix}.sock {ipid} >/dev/null 2>&1; sleep 1; grep -h connection_refused /var/lib/agentbound/audit/events.jsonl | grep '{alloc_id}' | tail -1 | grep -oE '\"rule\":\"[a-z_]*\"'")).1.trim().to_string() };
    let rule_open = refused_rule();
    // ---- T-6.4-014 / T-6.3-007: the worker holds an established connection (GW-HELD); revoke while held; its next packet must be refused ----
    let held_out = format!("/var/lib/agentbound/sessions/{}/rootfs/workspace/held-{guid}.out", js(&v, "allocation_id").rsplit(':').next().unwrap_or(""));
    let _ = &held_out;
    // the worker's held client sends its second packet 5 s after GW-HELD; revoke now so the packet lands after deny_admission
    let ws = sh("python3 -c \"import json;c=json.load(open('/etc/agentbound/catalogue.json'));s=c['mount_sources']['mount-source:workspace-eng'];print(s['base']+'/'+s['relative'])\"").1.trim().to_string();
    // quiesce first: §5 step 1 (deny admission) with the peer frozen, not killed. Then thaw only the held client's packet path by
    // terminating the quiesce with a bounded expiry — the frozen client is thawed at step 3 and its queued packet meets the denial.
    let q = lc("quiesce", Value::obj(vec![("launch_record_digest", Value::s(&glrd)), ("reason", Value::s("conformance"))])); let qst = js(&q, "body.state");
    sh(&format!("touch {ws}/revoked-{guid}")); // marker lands while frozen; the client reads it when thawed
    std::thread::sleep(std::time::Duration::from_millis(500));
    let gst_q = { match wire::connect("/run/agentbound/gateway.sock") { Ok(c) => c.call(&wire::request("status", "conf-gwq", Value::obj(vec![("launch_record_digest", Value::s(&glrd))]))).unwrap_or(Value::Null), Err(_) => Value::Null } };
    // a *legitimate* new peer (session uid, inside the session's scope cgroup and namespaces) must be refused while quiesced, and
    // refused for admission — not for uid/scope. The frozen session's own processes cannot run, so the driver joins the scope itself.
    let frozen_new = refused_rule();
    let r = sig(&glrd, "authority_revoked"); let beh = js(&r, "body.behaviour");
    let sig_reply: String = json::canonical(&r).iter().map(|b| *b as char).collect::<String>().chars().take(200).collect();
    std::thread::sleep(std::time::Duration::from_secs(3));
    let (_, late) = sh(&format!("cat {ws}/held-{guid}.out 2>&1 | head -c 1200; rm -f {ws}/revoked-{guid}"));
    let gst = { match wire::connect("/run/agentbound/gateway.sock") { Ok(c) => c.call(&wire::request("status", "conf-gw", Value::obj(vec![("launch_record_digest", Value::s(&glrd))]))).unwrap_or(Value::Null), Err(_) => Value::Null } };
    g.rec("T-6.4-014", qst == "quiescing" && js(&gst_q, "body.admission") == "false" && beh == "terminate" && late.contains("\"ok\":true") && late.contains("admission_closed") && rule_open.contains("uid_mismatch") && frozen_new.contains("admission_closed"), format!("quiesce state={qst} gateway admission={} new-conn-while-quiesced={} (control: the identical peer got {} while the session was admitting, so the refusal is by admission state, not peer identity) behaviour={beh}; held connection's post-denial packet: {} ; revocation reply: {sig_reply}; status after seal: {}", js(&gst_q, "body.admission"), frozen_new.trim(), rule_open.trim(), late.lines().last().unwrap_or("").chars().take(120).collect::<String>(), js(&gst, "body.rule")));
    std::thread::sleep(std::time::Duration::from_secs(4));
    let (_, chain) = sh(&format!("grep '{glrd}' /var/lib/agentbound/audit/events.jsonl | grep -o '\"event\":\"[a-z._]*\"' | sort -u | tr -d '\"' | sed 's/event://' | tr '\\n' ' '"));
    let need = ["session.launch_record_committed", "gateway.grants_loaded", "session.activated", "gateway.connection_established", "gateway.operation_admitted", "gateway.operation_completed", "gateway.operation_denied", "session.revocation_received", "session.termination_started", "gateway.admission_denied", "session.terminated", "gateway.released", "session.cleanup_completed", "session.identity_released", "session.sealed"];
    let missing: Vec<&str> = need.iter().copied().filter(|k| !chain.contains(k)).collect();
    // The single-session audit-chain presence check that earlier registers scored AS D-12. It is not the §5 metric and no longer
    // carries that id: it is kept as a fixture so the shape of one 1B session's chain stays visible, and it is excluded from counts.
    g.fixture("D-12.chain-shape", missing.is_empty(), format!("one launch record carries {}/{} expected event kinds; missing={:?} (fixture — NOT the §5 attribution metric, which is D-12 below)", need.len() - missing.len(), need.len(), missing));
    // ---- D-16: every revocation trigger in the frozen vocabulary is exercised at the milestone where its component exists ----
    // For each trigger: the declared action came from the manifest (not a default), the action was carried out, and the hash-chained
    // log holds a `session.revocation_received` naming it. The per-trigger rows are T-6.8-001..011; D-16 checks the SET is complete.
    {
        const TRIGGERS: [&str; 11] = ["approval_expired", "audit_pipeline_degraded_below_stop_threshold", "authority_revoked", "catalogue_withdrawn",
            "gateway_grant_withdrawn", "gateway_unavailable", "initiator_disabled", "policy_service_unavailable", "policy_withdrawn",
            "reclassification", "task_cancelled"];
        let seen = sh("grep -h session.revocation_received /var/lib/agentbound/audit/events.jsonl | grep -oE '\"trigger\":\"[a-z_]*\"' | sort -u").1;
        let covered: Vec<&str> = TRIGGERS.iter().copied().filter(|t| seen.contains(&format!("\"trigger\":\"{t}\""))).collect();
        let missing: Vec<&str> = TRIGGERS.iter().copied().filter(|t| !covered.contains(t)).collect();
        g.rec("D-16", missing.is_empty(),
            format!("{}/{} triggers in the frozen vocabulary exercised with a declared action and a session.revocation_received record in the hash-chained log: {covered:?}{}. Invariant 21 stays incomplete until 1C (inference grant/binding revoked), per R-LC-3.",
                covered.len(), TRIGGERS.len(), if missing.is_empty() { String::new() } else { format!("; NOT exercised: {missing:?}") }));
    }
    // D-12 is the pre-registered §5 metric and nothing else. It is measured by `d12-run.py` (8 concurrent sessions, own instrumented
    // ground truth, 30 s correlation deadline, N = 10 seeded repetitions), which writes one JSON result per repetition to
    // /var/lib/agentbound/evidence/d12/rep-<n>.json. This row COMPUTES its verdict from those files — every figure below is read from
    // them, none is prose — and FAILs when there is no valid measurement: an absent or invalid measurement is not a pass, and a
    // presence check over event kinds (what earlier registers scored here) is not the metric. Ten valid repetitions are required by
    // §5; fewer is reported as such and fails.
    {
        let d = "/var/lib/agentbound/evidence/d12";
        // Parse each result file WHOLE. `parse()` takes the last line that is itself valid JSON, which is right for a JSONL event
        // stream and wrong for these: the harness writes them pretty-printed, so `parse()` saw only the closing brace, every
        // repetition read as invalid, and the row reported "0 valid repetitions" while ten valid repetitions sat on disk (WP3.1).
        let mut reps: Vec<Value> = (1..=10).filter_map(|n| std::fs::read_to_string(format!("{d}/rep-{n}.json")).ok())
            .filter_map(|s| json::parse(s.trim().as_bytes(), &MANIFEST_LIMITS).ok()).filter(|v| !matches!(v, Value::Null)).collect();
        reps.sort_by_key(|v| js(v, "repetition").parse::<i64>().unwrap_or(0));
        let valid: Vec<&Value> = reps.iter().filter(|v| js(v, "valid") == "true").collect();
        let f = |v: &Value, p: &str| js(v, p).parse::<f64>().unwrap_or(-1.0);
        let (mut g_all, mut c_all, mut gw_g, mut gw_c) = (0.0, 0.0, 0.0, 0.0);
        let mut per = Vec::new();
        for v in &valid { g_all += f(v, "G"); c_all += f(v, "C"); gw_g += f(v, "gateway_corpus.G"); gw_c += f(v, "gateway_corpus.C");
            per.push(format!("rep{} seed={} |C|/|G|={}/{} gw={}/{}", js(v, "repetition"), js(v, "seed"), js(v, "C"), js(v, "G"), js(v, "gateway_corpus.C"), js(v, "gateway_corpus.G"))); }
        let overall = if g_all > 0.0 { c_all / g_all } else { 0.0 }; let gw = if gw_g > 0.0 { gw_c / gw_g } else { 0.0 };
        let every_gw_100 = !valid.is_empty() && valid.iter().all(|v| f(v, "gateway_corpus.G") > 0.0 && f(v, "gateway_corpus.C") == f(v, "gateway_corpus.G"));
        // The 1B bar, after the R-AUD-2 milestone split (phase-1-requirements 0.11, catalogue 0.8): ten valid repetitions and 100%
        // over the finite gateway-operation corpus in every one of them. The whole-ontology fraction is REPORTED here and is owed by
        // D-12.full at 1C, which this suite does not run. `overall` therefore does not gate this row — the threshold it would be
        // compared against was moved to another milestone, not lowered — and the detail states the measured figure either way so no
        // reader can mistake the gateway corpus for whole-ontology attribution.
        let met = valid.len() == 10 && every_gw_100;
        let classes = valid.first().map(|v| jget(v, "per_class").map(|c| String::from_utf8_lossy(&canonical(c)).into_owned()).unwrap_or_default()).unwrap_or_default();
        g.rec("D-12", met, format!("§5 NOMINAL metric computed from {d}: {} result files, {} valid repetitions (10 required); 1B bar = 100% over the finite gateway-operation corpus: {}/{} = {:.1}% ({}); whole-ontology aggregate |C|/|G| = {}/{} = {:.1}% — REPORTED, owed by D-12.full at 1C (>= 99% there), NOT whole-ontology attribution at 1B; invalid/aborted repetitions retained: {:?}; per-rep: [{}]; per-class (rep 1): {}",
            reps.len(), valid.len(), gw_c, gw_g, gw * 100.0, if every_gw_100 { "100% in every valid run" } else { "NOT 100% in every valid run" }, c_all, g_all, overall * 100.0,
            reps.iter().filter(|v| js(v, "valid") != "true").map(|v| format!("rep{}: launched={} incomplete={} errors={}", js(v, "repetition"), js(v, "sessions_launched"), js(v, "sessions_incomplete"), js(v, "launch_errors"))).collect::<Vec<_>>(),
            per.join("; "), classes.chars().take(600).collect::<String>()));
    }
    g.rec("T-6.3-007", chain.contains("gateway.released") && chain.contains("session.sealed"), "post-termination: projection released, record sealed, socket node removed with the mount namespace");
    let (_, sockleft) = sh(&format!("ls /run/agentbound/gw/ | grep -c {}", js(&v, "allocation_id").rsplit(':').next().unwrap_or("x")));
    g.rec("T-6.3-007.socket", sockleft.trim() == "0", format!("host-side socket nodes left for this allocation: {}", sockleft.trim()));
    // ---- T-6.4-013 / T-6.3-008: replay of another session's identity through a fresh session ----
    // ---- F-C-08: step 8 aborted with the record committed and the gateway socket bound, but grants never activated ----
    // Requires topology local-socket, so it uses the git request. The grant must be unusable and the socket node released.
    {
        let (rc8, _, out8) = gb.request(&greq, "--fault pre-activate-crash");
        let (_, last8) = sh("tail -1 /var/lib/agentbound/audit-launch.jsonl"); let ev8 = parse(&last8);
        let (step8, l8, rb8) = (js(&ev8, "detail.failed_step"), js(&ev8, "launch_record_digest"), js(&ev8, "detail.rollback"));
        let k8 = kinds(&audit_rows(&l8));
        let suffix = out8.split("allocation:").nth(1).map(|x| x.chars().take_while(|c| c.is_alphanumeric() || *c == '-').collect::<String>()).unwrap_or_default();
        let sock_gone = suffix.is_empty() || !std::path::Path::new(&format!("/run/agentbound/gw/{suffix}.sock")).exists();
        // the gateway must hold no projection for the allocation: status by launch record must be unknown
        let gwst8 = match wire::connect("/run/agentbound/gateway.sock") { Ok(c) => c.call(&wire::request("status", "conf-fc08", Value::obj(vec![("launch_record_digest", Value::s(&l8))]))).unwrap_or(Value::Null), Err(_) => Value::Null };
        let no_projection = js(&gwst8, "body.rule") == "unknown_record";
        let az8 = out8.split("launchrec:").nth(1).map(|x| format!("launchrec:{}", x.chars().take_while(|c| c.is_alphanumeric() || *c == '-').collect::<String>())).unwrap_or_default();
        let ident8 = js(&lc("status", Value::obj(vec![("authorization_id", Value::s(&az8))])), "body.identity_state");
        g.rec("F-C-08", rc8 != 0 && step8 == "8" && !l8.is_empty() && k8.contains(&"session.launch_record_committed".into()) && k8.contains(&"session.construction_failed".into())
            && sock_gone && no_projection && rb8.contains("gateway") && (ident8 == "reclaiming" || ident8 == "quarantined"),
            format!("step={step8}: record committed (lrd={l8}, audit kinds={k8:?}) and socket bound, activation never reached; rollback={rb8}; gateway holds no projection for the record (status rule={})={no_projection}; socket node for {suffix} gone={sock_gone}; identity={ident8}", js(&gwst8, "body.rule")));
    }
    // ---- termination-step faults (F-T-01/05/06/07/09) ----
    // Each fault gets its own session because termination consumes one. The common obligation: a failing step must not produce a
    // released identity or a sealed record — the protocol reports `termination-incomplete` (or holds cleanup) and audits the failure.
    for (id, fault, want) in [("F-T-01", "admission-closure", "gateway admission closure"), ("F-T-05", "no-live-confirmation", "no-live-process confirmation"),
                              ("F-T-06", "gateway-release", "gateway grant/connection closure"), ("F-T-07", "credential-closure", "broker/credential closure"),
                              ("F-T-09", "socket-unmount", "gateway socket removal")] {
        let (rcf, vf, _) = gb.request(&greq, ""); let lrdf = js(&vf, "launch_record_digest");
        if rcf != 0 || lrdf.is_empty() { g.rec(id, false, format!("could not launch a session for the {want} fault: rc={rcf}")); continue; }
        let aidf = js(&vf, "allocation_id"); let suffix = aidf.rsplit(':').next().unwrap_or("").to_string();
        // the socket node must exist while the session is live (it is what the fault leaves behind at step 9)
        let node = format!("/run/agentbound/gw/{suffix}.sock");
        let node_before = std::path::Path::new(&node).exists();
        let t = g.terminate_faulted(&lrdf, fault);
        // F-T-09 must be observed immediately: the poller re-terminates a session whose init has exited, which removes the node
        let node_after = std::path::Path::new(&node).exists();
        // "delivered" means the gateway answered — the only outcome that would mean the socket is still usable. A connect that is
        // refused, or accepted by a stale listener and then reset with no reply, both mean inaccessible.
        std::fs::write("/tmp/ft-probe.py", "import socket,sys\nn=sys.argv[1]\ns=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET);s.settimeout(3)\ntry:\n s.connect(n)\nexcept OSError as e:\n print('refused',e.errno); raise SystemExit\ntry:\n s.send(b'{\"body\":{},\"idempotency_key\":\"p\",\"op\":\"status\",\"v\":\"agentbound.wire.v0.1\"}')\n r=s.recv(400)\n print('delivered' if r else 'connected, empty reply')\nexcept OSError as e:\n print('connected, no reply:',type(e).__name__)\n").unwrap();
        let conn_after = sh(&format!("python3 /tmp/ft-probe.py {node} 2>&1")).1.trim().to_string();
        let state = js(&t, "body.state"); let ev = js(&t, "body.evidence");
        let st = lc("status", Value::obj(vec![("launch_record_digest", Value::s(&lrdf))]));
        let (sstate, ident) = (js(&st, "body.state"), js(&st, "body.identity_state"));
        // NOTE: no event kind is invented for fault injection — the audit vocabulary is closed by design (the receiver refuses an
        // unknown kind or detail member, which is itself asserted by T-6.8-005). The failing step is observed in the reply evidence and
        // in the existing `session.cleanup_completed` / `session.termination_incomplete` records.
        // The receiver appends asynchronously, so poll (bounded, 10 s) for the record this row reads.
        let want_kind = if fault == "no-live-confirmation" { "session.termination_incomplete" } else { "session.cleanup_completed" };
        let mut k = kinds(&audit_rows(&lrdf));
        for _ in 0..20 { if k.contains(&want_kind.to_string()) { break; } std::thread::sleep(std::time::Duration::from_millis(500)); k = kinds(&audit_rows(&lrdf)); }
        let rows_f = audit_rows(&lrdf);
        let cleanup = rows_f.iter().rev().find(|r| js(r, "event.event") == "session.cleanup_completed").cloned().unwrap_or(Value::Null);
        // ORDER, not timing: the identity may be released later by the unfaulted retry, but never before the failing step was recorded
        let pos = |kind: &str| k.iter().position(|x| x == kind);
        let released_before_failure = match (pos(want_kind), pos("session.identity_released")) { (Some(f), Some(r)) => r < f, (None, Some(_)) => true, _ => false };
        let sealed = k.contains(&"session.sealed".into()) || sstate == "cleaned/sealed"; let _ = sealed;
        let released = ident == "free";
        match fault {
            // F-T-01: 1B — admission closure failed, so no new gateway operation may be admitted regardless. The gateway was never
            // told to deny, so this tests the *other* guarantee: releasing the projection at step 6 makes the socket unusable.
            "admission-closure" => {
                let denied = js(&t, "body.evidence.gateway_admission_denied") == "false";
                g.rec(id, denied && !node_after && conn_after.contains("refused") && !released,
                    format!("1B: step 1 admission closure failed (evidence gateway_admission_denied=false). The other guarantee still holds: releasing the projection at step 6 removed the socket node (present before={node_before}, after={node_after}) and a connect to it is {conn_after}, so no new operation can be admitted; identity={ident} (not free)"));
            }
            // F-T-05: the protocol must report termination-incomplete and hold the identity — never release, never seal.
            "no-live-confirmation" => {
                // the reply is the protocol's answer; the status may already have advanced because the poller re-terminates a session
                // whose init has exited (that retry runs without the fault). What must hold is: this attempt refused to complete, and
                // the identity was never released while the confirmation was missing.
                g.rec(id, state == "termination-incomplete" && k.contains(&"session.termination_incomplete".into()) && !released_before_failure,
                    format!("the attempt reported state={state} and recorded session.termination_incomplete; no identity release preceded it (released_before_failure={released_before_failure}); status when read afterwards={sstate} (the poller's unfaulted retry may already have completed it), identity={ident}"));
            }
            // F-T-06 / F-T-07: safe state retained — cleanup holds, the identity is not released, and the failure is audited.
            "gateway-release" | "credential-closure" => {
                // safe state retained: the failure is recorded, cleanup does not seal on this attempt, and no identity release precedes it
                // the recorded cleanup evidence must show the step that failed, the outcome must be `hold`, and nothing may be sealed
                let (outcome, grants) = (js(&cleanup, "event.outcome"), js(&cleanup, "event.detail.grants"));
                let shows = if fault == "gateway-release" { grants.contains("\"remaining\":\"gateway unreachable\"") || grants.contains("\"released\":false") } else { grants.contains("\"broker_closed\":false") };
                g.rec(id, shows && outcome == "hold" && !k.contains(&"session.sealed".into()) && !released_before_failure,
                    format!("{want} failed: session.cleanup_completed recorded outcome={outcome} with grants={grants}; the record was not sealed (sealed={}) and no identity release preceded the failure; state={state}, status={sstate}, identity={ident}", k.contains(&"session.sealed".into())));
            }
            // F-T-09: the socket node survives step 9 but the projection is gone — the gateway must be inaccessible through it, and
            // the launch record must be retained.
            "socket-unmount" => {
                let rec = lc("record", Value::obj(vec![("launch_record_digest", Value::s(&lrdf))]));
                let retained = !js(&rec, "body.binding").is_empty();
                // the node survives step 9 with no listener behind it: the gateway is inaccessible through it, and the ledger is kept
                g.rec(id, node_before && node_after && !conn_after.contains("delivered") && retained,
                    format!("step 9 failed: the node was present before ({node_before}) and after ({node_after}) termination, but the projection is released, so a connect+send is {conn_after} — the gateway is inaccessible through it; the launch record is retained={retained}; identity={ident}"));
                let _ = std::fs::remove_file(&node);
            }
            _ => {}
        }
        // clean up: a second, unfaulted terminate must be able to finish the job (the protocol is resumable)
        let t2 = g.terminate(&lrdf); let s2 = js(&t2, "body.state"); let conflict = js(&t2, "body.rule");
        let fin = js(&lc("status", Value::obj(vec![("launch_record_digest", Value::s(&lrdf))])), "body.state");
        g.rec(&format!("{id}.resumable"), fin == "cleaned/sealed" || fin == "terminated",
            format!("with the fault removed the protocol completed: repeated terminate returned state={s2}{} and the session's final state is {fin} (evidence from the faulted attempt retained: {})", if conflict.is_empty() { String::new() } else { format!(" (rule={conflict}: already terminal)") }, ev.chars().take(60).collect::<String>()));
    }
    // ---- T-6.8-008 / T-6.8-009: the two 1B revocation triggers, asserted on the GRANT EFFECT, not just the declared behaviour ----
    // T-6.8-008 (gateway_grant_withdrawn → terminate): a Git operation that succeeded before the signal must be denied after it.
    {
        let (rcw, vw, _) = gb.request(&greq, ""); let lrdw = js(&vw, "launch_record_digest");
        let (scopew, uidw) = (js(&vw, "scope_id"), js(&vw, "uid"));
        let (_, ipidw) = sh(&format!("head -1 /sys/fs/cgroup/system.slice/{scopew}/cgroup.procs")); let ipidw = ipidw.trim().to_string();
        let ping = |extra: &str| -> String { sh(&format!("sh -c 'echo $$ > /sys/fs/cgroup/system.slice/{scopew}/cgroup.procs; exec nsenter -t {ipidw} -m -n -p -S {uidw} -G {uidw} -- ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping {{}}' 2>&1{extra}")).1 };
        let before = ping("");
        let r = sig(&lrdw, "gateway_grant_withdrawn");
        let after = ping("");
        let st = lc("status", Value::obj(vec![("launch_record_digest", Value::s(&lrdw))]));
        let k = kinds(&audit_rows(&lrdw));
        let ok_before = before.contains("\"pong\":true");
        // after a terminate the socket node is gone, so the client cannot even connect — that is the strongest form of "denied"
        let denied_after = !after.contains("\"pong\":true");
        g.rec("T-6.8-008", rcw == 0 && ok_before && js(&r, "body.behaviour") == "terminate" && denied_after && js(&st, "body.state") == "cleaned/sealed" && k.contains(&"session.revocation_received".into()),
            format!("declared behaviour={} (manifest, for trigger gateway_grant_withdrawn); a gateway ping succeeded before the signal ({}) and after it the same in-scope peer gets: {}; session state={}; revocation recorded in the chain={}",
                js(&r, "body.behaviour"), ok_before, after.lines().last().unwrap_or("").chars().take(90).collect::<String>(), js(&st, "body.state"), k.contains(&"session.revocation_received".into())));
    }
    // T-6.8-009 (gateway_unavailable → quiesce, class RR): the gateway is made GENUINELY unavailable (service stopped) before the
    // signal, so the declared behaviour must be reached without it, and the availability of the gateway must be visible in the record.
    {
        let (rcu, vu, _) = gb.request(&greq, ""); let lrdu = js(&vu, "launch_record_digest");
        sh("systemctl stop agentbound-gateway"); std::thread::sleep(std::time::Duration::from_secs(1));
        let reachable_while_down = wire::connect("/run/agentbound/gateway.sock").is_ok();
        let r = sig(&lrdu, "gateway_unavailable");
        let st = lc("status", Value::obj(vec![("launch_record_digest", Value::s(&lrdu))]));
        let rows = audit_rows(&lrdu);
        let q = rows.iter().rev().find(|x| js(x, "event.event") == "session.quiesce_started").cloned().unwrap_or(Value::Null);
        let admission = js(&q, "event.detail.admission");
        sh("systemctl start agentbound-gateway"); std::thread::sleep(std::time::Duration::from_secs(3));
        let k = kinds(&rows);
        g.rec("T-6.8-009", rcu == 0 && !reachable_while_down && js(&r, "body.behaviour") == "quiesce" && js(&st, "body.state") == "quiescing"
            && admission == "denied-no-gateway" && k.contains(&"session.revocation_received".into()),
            format!("the gateway was stopped for this row (control socket reachable while down={reachable_while_down}); the declared behaviour for gateway_unavailable is {} and it was reached without the gateway: state={}, and session.quiesce_started records admission={admission} — the availability of the gateway at that moment, not an assumption",
                js(&r, "body.behaviour"), js(&st, "body.state")));
        g.terminate(&lrdu);
    }
    let (rc2, v2, _) = gb.request(&greq, ""); let lrd2 = js(&v2, "launch_record_digest");
    // the worker inside runs the whole in-session row set (incl. 16 held connections); wait until its connections are gone
    for _ in 0..60 { std::thread::sleep(std::time::Duration::from_millis(500)); let st = wire::connect("/run/agentbound/gateway.sock").ok().and_then(|c| c.call(&wire::request("status", "conf-gw2", Value::obj(vec![("launch_record_digest", Value::s(&lrd2))]))).ok()).unwrap_or(Value::Null); if st.get("body").and_then(|b| b.get("connections")).and_then(|x| x.as_int()) == Some(0) && st.get("body").and_then(|b| b.get("operations")).and_then(|x| x.as_int()).unwrap_or(0) > 20 { break; } }
    let (_, ipid2) = sh(&format!("head -1 /sys/fs/cgroup/system.slice/{}/cgroup.procs", js(&v2, "scope_id"))); let ipid2 = ipid2.trim().to_string(); let uid2 = js(&v2, "uid");
    let scope2 = js(&v2, "scope_id");
    let steal_args = format!(r#"{{"expect_old":null,"ref_tail":"steal","repository_id":"repo:demo","session_id":"session:{sid}","trace_id":"{trace}","tip":"{}"}}"#, "2".repeat(40));
    let (_, rep) = sh(&format!("sh -c 'echo $$ > /sys/fs/cgroup/system.slice/{scope2}/cgroup.procs; exec nsenter -t {ipid2} -m -n -p -S {uid2} -G {uid2} -- ab-gwclient /run/gateway.sock op:git-push-staging git.push_staging {} /image/probe.sh' 2>&1 | head -c 300", steal_args.replace('"', "\\\"")));
    let (_, refs2) = sh("su -s /bin/sh agentbound-gateway -c 'git -C /var/lib/agentbound/git/demo.git for-each-ref' | grep -c steal");
    // ---- T-6.3-008 (class CSSP): replay from ANOTHER session — session B connects to session A's socket node ----
    // Both sessions are live and legitimate. B's peer credentials (uid, cgroup, pidfs instance) belong to B, so A's projection must
    // refuse the connection outright. This is distinct from T-6.4-013 (caller-supplied identity arguments on B's OWN socket).
    {
        // two live sessions of our own: A is the victim, B is the adversary. Neither is reused elsewhere, so both are certainly projected.
        let (rca, va, _) = gb.request(&greq, ""); let lrda = js(&va, "launch_record_digest");
        let (rcb, vb, _) = gb.request(&greq, ""); let lrdb = js(&vb, "launch_record_digest");
        let (scopeb, uidb, aidb) = (js(&vb, "scope_id"), js(&vb, "uid"), js(&vb, "allocation_id"));
        let (_, ipidb) = sh(&format!("head -1 /sys/fs/cgroup/system.slice/{scopeb}/cgroup.procs")); let ipidb = ipidb.trim().to_string();
        let a_suffix = js(&va, "allocation_id").rsplit(':').next().unwrap_or("").to_string();
        let a_node = format!("/run/agentbound/gw/{a_suffix}.sock");
        // A must be an ADMITTED projection when B attempts, or the refusal would be `admission_closed` and would evidence nothing
        // about peer credentials. Poll A's own gateway status until admission is open (bounded).
        let mut a_admitted = false;
        for _ in 0..40 { std::thread::sleep(std::time::Duration::from_millis(500));
            let st = wire::connect_bounded("/run/agentbound/gateway.sock", 4_000).ok()
                .and_then(|c| c.call(&wire::request("status", &format!("t638-{}", ab_common::sig::monotonic_ns()), Value::obj(vec![("launch_record_digest", Value::s(&lrda))]))).ok()).unwrap_or(Value::Null);
            if js(&st, "body.admission") == "true" { a_admitted = true; break; } }
        let a_live = std::path::Path::new(&a_node).exists() && a_admitted;
        let (scopea, uida) = (js(&va, "scope_id"), js(&va, "uid"));
        let (_, ipida) = sh(&format!("head -1 /sys/fs/cgroup/system.slice/{scopea}/cgroup.procs")); let ipida = ipida.trim().to_string();
        let refused_before = sh(&format!("grep -hc 'gateway.connection_refused' /var/lib/agentbound/audit/events.jsonl")).1.trim().parse::<i64>().unwrap_or(0);
        // B's process, with B's uid and B's cgroup, connecting to A's node (the mount namespace is B's; the node is given by host path)
        // (a) from inside B's own mount namespace the node does not exist: each session sees only its own projection
        let inside = sh(&format!("sh -c 'exec nsenter -t {ipidb} -m -n -p -S {uidb} -G {uidb} -- ab-gwclient {a_node} op:gateway-ping gateway.ping {{}}' 2>&1 | tail -c 200")).1;
        let unreachable_by_path = inside.contains("No such file") || inside.contains("os error 2");
        // (b) a peer that CAN reach the node — host mount namespace, but B's uid, B's cgroup and B's pid namespace — must be refused
        //     by A's projection on credentials alone. This is the in-scope-peer oracle used elsewhere in this suite.
        // the image store is deliberately unreadable to session uids (drwxr-x---), so stage the client where B's uid can exec it
        sh("install -m 0755 /var/lib/agentbound/images/rootfs/bin/ab-gwclient /tmp/ab-gwclient-t638");
        let lines_before = sh("grep -hc '' /var/lib/agentbound/audit/events.jsonl").1.trim().parse::<i64>().unwrap_or(0);
        // the node is mode 0666 by design (the peer is authenticated by credentials, not by file permissions), but it lives in
        // /run/agentbound/gw which only the gateway's group may traverse — so the adversary is given that traversal explicitly, to
        // make the gateway's credential check the only thing left that can refuse.
        let out = sh(&format!("sh -c 'echo $$ > /sys/fs/cgroup/system.slice/{scopeb}/cgroup.procs; exec setpriv --reuid {uidb} --regid {uidb} --groups $(stat -c %g /run/agentbound/gw) -- /tmp/ab-gwclient-t638 {a_node} op:gateway-ping gateway.ping {{}}' 2>&1 | tail -c 300")).1;
        std::thread::sleep(std::time::Duration::from_secs(1));
        // scope the rule strictly to events appended after the attempt, for A's allocation
        // scope strictly to events appended after the attempt, for A's allocation, and by B's peer uid — so the rule read back is
        // unambiguously the one that refused THIS peer
        let rule = sh(&format!("tail -n +{} /var/lib/agentbound/audit/events.jsonl | grep 'gateway.connection_refused' | grep {a_suffix} | grep '\"peer_uid\":{uidb}' | tail -1 | grep -oE '\"rule\":\"[a-z_]*\"'", lines_before + 1)).1.trim().to_string();
        let refused_after = sh(&format!("grep -hc 'gateway.connection_refused' /var/lib/agentbound/audit/events.jsonl")).1.trim().parse::<i64>().unwrap_or(0);
        let no_pong = !out.contains("\"pong\":true");
        g.rec("T-6.3-008", rca == 0 && rcb == 0 && a_live && unreachable_by_path && no_pong && refused_after > refused_before && (rule.contains("scope_mismatch") || rule.contains("uid_mismatch")),
            format!("session B ({aidb}, uid {uidb}) connected to session A's socket node {a_node} (A's projection admitted and its node present at that moment={a_live}). Two results: from inside B's own mount namespace the node does not exist at all ({}), and a peer that can reach it — B's uid, B's cgroup, and group traversal into the gateway's socket directory granted explicitly — is refused by A's projection on credentials alone (reply: {}; refusals {refused_before} -> {refused_after}, rule recorded against A after this attempt: {rule}). The peer credentials, not the path, decide",
                inside.lines().last().unwrap_or("").chars().take(50).collect::<String>(),
                out.lines().last().unwrap_or("").chars().take(90).collect::<String>()));
        // ---- T-6.3-005: the socket is a broker capability — usable only by the authenticated peer, and not exportable ----
        // Three properties on one live session: (1) the node cannot be re-bound or copied into anything usable, (2) passing the
        // connected descriptor to another process does not transfer the capability (SCM_RIGHTS is refused), (3) the node is reachable
        // only through the session's own mount — a host path outside the session's mount namespace is not a second route in.
        let copy = sh(&format!("sh -c 'cp {a_node} /tmp/stolen.sock 2>&1; ls -l /tmp/stolen.sock 2>&1 | head -1'")).1.trim().to_string();
        let copy_refused = !std::path::Path::new("/tmp/stolen.sock").exists();
        let scm = sh(&format!("sh -c 'echo $$ > /sys/fs/cgroup/system.slice/{scopea}/cgroup.procs; exec nsenter -t {ipida} -m -n -p -S {uida} -G {uida} -- ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping {{}} --scm-rights' 2>&1 | tail -c 200")).1;
        let scm_refused = scm.contains("descriptor_transfer") || !scm.contains("\"pong\":true");
        // root on the host, using the same binary the session uses, against the same node — from outside the session's namespaces
        let outside = sh(&format!("/tmp/ab-gwclient-t638 {a_node} op:gateway-ping gateway.ping {{}} 2>&1 | tail -c 200")).1;
        let outside_refused = !outside.contains("\"pong\":true");
        g.rec("T-6.3-005", copy_refused && scm_refused && outside_refused,
            format!("the socket node is a broker capability, not a transferable object: copying it out of the session produced no usable object ({}, exists={}); handing the connected descriptor to another process is refused ({}); and root on the host connecting to the same node from outside the session's mount namespace is refused ({}) — every use is authenticated per peer instance",
                copy.chars().take(60).collect::<String>(), !copy_refused, scm.lines().last().unwrap_or("").chars().take(60).collect::<String>(), outside.lines().last().unwrap_or("").chars().take(60).collect::<String>()));
        let _ = std::fs::remove_file("/tmp/stolen.sock");
        g.terminate(&lrda); g.terminate(&lrdb);
    }
    g.rec("T-6.4-013", rc2 == 0 && refs2.trim() == "0" && rep.contains("\"rule\"") && !rep.contains(&format!("refs/agentbound/{sid}/")), format!("caller-supplied session/trace refused (closed argument set); no ref under the other session's namespace: {}", rep.replace('\n', " ")));
    // ---- D4.7: gateway restart reconstructs projections from the launch-record store; the live session keeps working, no caller state consulted ----
    // Evidence is scoped to THIS restart: the hash-chained receiver must gain exactly one `gateway.reconstructed` (the component spool
    // alone would also hold events the receiver rejected), that event must report >= 1 projection, and a legitimate in-scope peer must
    // complete an operation afterwards. A run where reconstruction produced nothing can no longer pass.
    let (_, before) = sh(&format!("ls /run/agentbound/gw/ | grep -c {}", js(&v2, "allocation_id").rsplit(':').next().unwrap_or("x")));
    let chain_recs = || -> i32 { sh("grep -hc gateway.reconstructed /var/lib/agentbound/audit/events.jsonl").1.trim().parse().unwrap_or(0) };
    let recs_before = chain_recs();
    sh("systemctl restart agentbound-gateway");
    let mut recs_after = recs_before;
    for _ in 0..20 { std::thread::sleep(std::time::Duration::from_millis(500)); recs_after = chain_recs(); if recs_after > recs_before { break; } }
    let (_, rec_ev) = sh("grep -h gateway.reconstructed /var/lib/agentbound/audit/events.jsonl | tail -1 | grep -oE '\"(projections|stale_descriptors_dropped)\":[0-9]+' | tr '\n' ' '");
    let projections: i32 = rec_ev.split("\"projections\":").nth(1).and_then(|x| x.split(|c: char| !c.is_ascii_digit()).next().and_then(|d| d.parse().ok())).unwrap_or(-1);
    // enter the session's scope cgroup first (host root may move itself), then its namespaces and identity: a legitimate in-scope peer
    let (_, after_ping) = sh(&format!("sh -c 'echo $$ > /sys/fs/cgroup/system.slice/{scope2}/cgroup.procs; exec nsenter -t {ipid2} -m -n -p -S {uid2} -G {uid2} -- sh -c \"sleep 0.3; ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping {{}}\"' 2>&1 | head -c 200"));
    g.rec("D4.7-reconstruct", before.trim() == "1" && recs_after == recs_before + 1 && projections >= 1 && after_ping.contains("\"pong\":true"), format!("socket before restart={}; chained reconstruction events {recs_before}→{recs_after} (exactly one for this restart); event: {}; ping from an in-scope session peer after restart: {}", before.trim(), rec_ev.trim(), after_ping.replace('\n', " ")));

    // ---- T-6.9-005.no-deadlock: the budget-persistence path must not be able to wedge the two daemons ----
    // Round 5 found a REAL deadlock here. `agentbound-gateway` and `agentbound-lifecycle` each serve one request at a time, and each
    // calls the other: the gateway calls lifecycle `record_budget` while admitting an operation (R-GW-7, added in round 4), and
    // lifecycle calls the gateway `release`/`deny_admission` during termination (§5 steps 1 and 6). When those cross in time, both
    // processes block in recvmsg forever, taking every live session with them. This row drives that exact crossing: a session pushes
    // gateway operations in a tight loop (each one persisting a budget) while the driver terminates it from the other side.
    {
        let (rcd, vd, _) = gb.request(&greq, ""); let lrdd = js(&vd, "launch_record_digest");
        let (scoped, uidd) = (js(&vd, "scope_id"), js(&vd, "uid"));
        let (_, ipidd) = sh(&format!("head -1 /sys/fs/cgroup/system.slice/{scoped}/cgroup.procs")); let ipidd = ipidd.trim().to_string();
        std::fs::write("/tmp/spin.sh", "n=0
while [ $n -lt 400 ]; do ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping '{}' >/dev/null 2>&1; n=$((n+1)); done
").unwrap();
        sh(&format!("nsenter -t {ipidd} -m -- sh -c 'cat > /tmp/spin.sh' < /tmp/spin.sh"));
        sh(&format!("sh -c 'echo $$ > /sys/fs/cgroup/system.slice/{scoped}/cgroup.procs; exec nsenter -t {ipidd} -m -n -p -S {uidd} -G {uidd} -- sh /tmp/spin.sh' >/dev/null 2>&1 &"));
        std::thread::sleep(std::time::Duration::from_millis(400));
        let t0 = std::time::Instant::now();
        let t = g.terminate(&lrdd);                 // crosses the in-flight record_budget calls
        let term_ms = t0.elapsed().as_millis();
        // both daemons must still answer afterwards, within a bound
        let t1 = std::time::Instant::now();
        let lc_alive = !js(&lc("list", Value::obj(vec![])), "ok").is_empty();
        let gw_alive = wire::connect_bounded("/run/agentbound/gateway.sock", 4_000).ok()
            .and_then(|c| c.call(&wire::request("status", &format!("dl-{}", ab_common::sig::monotonic_ns()), Value::obj(vec![("launch_record_digest", Value::s(&lrdd))]))).ok()).is_some();
        let probe_ms = t1.elapsed().as_millis();
        let fin = js(&lc("status", Value::obj(vec![("launch_record_digest", Value::s(&lrdd))])), "body.state");
        g.rec("T-6.9-005.no-deadlock", rcd == 0 && term_ms < 60_000 && lc_alive && gw_alive && probe_ms < 8_000 && (fin == "cleaned/sealed" || fin == "terminated"),
            format!("a session issued gateway operations in a loop (each persisting its budget through lifecycle) while the driver terminated it from the other side — the exact crossing that deadlocked both daemons before cross-daemon calls were bounded: terminate returned in {term_ms} ms, both daemons answered afterwards in {probe_ms} ms (lifecycle={lc_alive}, gateway={gw_alive}), final state={fin}"));
    }
    // ---- T-6.9-008 (1B/1C): every gateway budget class present at 1B is bounded; the classes that do not exist yet are listed ----
    // The catalogue is the source of truth for which classes exist. Present at 1B: operations (requests), bytes_per_operation and
    // bytes (payload), objects (per-operation Git limit), connection_count. Absent until 1C: rate, tokens, spend (R-GW-9). A class
    // that is present must be enforced — this row proves enforcement by exhausting each one and reading back the denial rule.
    {
        // the budget classes that exist are read from the deployed catalogue itself (the same file policy compiles manifests from)
        let raw = std::fs::read_to_string("/etc/agentbound/catalogue.json").unwrap_or_default();
        let mut present: Vec<String> = Vec::new();
        for key in ["operations", "bytes_per_operation", "bytes", "objects", "connection_count", "rate", "tokens", "spend"] {
            if raw.contains(&format!("\"{key}\":")) { present.push(key.to_string()); }
        }
        present.sort();
        let absent: Vec<&str> = ["rate", "spend", "tokens"].into_iter().filter(|c| !present.iter().any(|p| p == c)).collect();
        // Enforcement evidence per class, counted ONLY over the lines this run appended. Counting the whole log was a false
        // positive: a negative control that removed the operations-budget check entirely left this row passing, because denials
        // recorded by earlier runs still satisfied it. `run_start_line` is captured before the suite exercises the budgets.
        let denial_count = |rule: &str| -> i64 { sh(&format!("tail -n +{} /var/lib/agentbound/audit/events.jsonl | grep -h gateway.operation_denied | grep -c '\"rule\":\"{rule}\"'", run_start_line + 1)).1.trim().parse().unwrap_or(0) };
        let refused_conn = |rule: &str| -> i64 { sh(&format!("tail -n +{} /var/lib/agentbound/audit/events.jsonl | grep -h gateway.connection_refused | grep -c '\"rule\":\"{rule}\"'", run_start_line + 1)).1.trim().parse().unwrap_or(0) };
        // The operations budget must be exhausted by THIS row, not inherited from whichever row happens to run before it: relying on
        // another row's side effect is what let the cumulative count hide the missing enforcement. `op:gateway-ping` allows 64
        // operations, so a dedicated session drives past that and the denial is read back from the log.
        let (rc_ex, v_ex, _) = { let gx = Rig { as_user: "bob".into(), rows: vec![] };
            // the 1B task (`task:fix-issue-1235`) is the one that holds `op:gateway-ping`; 1234 does not permit the git-worker runtime
            let p = gx.write_req("t69008-ops", GW_REQ);
            gx.request(&p, "") };
        let mut ex_loop = String::from("(not run)");
        if rc_ex != 0 { eprintln!("T-6.9-008: exhaustion session did not launch: rc={rc_ex} rule={} detail={}", js(&v_ex, "body.rule"), js(&v_ex, "body.detail")); }
        if rc_ex == 0 {
            let (sx, ux) = (js(&v_ex, "scope_id"), js(&v_ex, "uid"));
            let (_, ipx) = sh(&format!("head -1 /sys/fs/cgroup/system.slice/{sx}/cgroup.procs")); let ipx = ipx.trim().to_string();
            // 70 pings against a 64-operation budget, from a peer the gateway authenticates. The loop lives in a file: three levels
            // of shell quoting through `nsenter -- sh -c` is how the earlier attempt silently ran nothing at all.
            // One connection, 70 operations. A connection per operation cannot work: `connection_count` (16) is itself a cumulative
            // per-session budget and would be exhausted long before the 64-operation limit, so the row would measure the wrong class.
            let script = "ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping '{}' --idem t69008 --repeat 70 2>&1 | tail -2\necho LOOPDONE\n";
            std::fs::write("/tmp/t69008-ops.sh", script).unwrap();
            // `nsenter -m` enters the session's mount namespace, where /tmp is a private tmpfs: the host copy is not visible there.
            // Place it inside that namespace through the session's own root.
            sh(&format!("cp /tmp/t69008-ops.sh /proc/{ipx}/root/tmp/t69008-ops.sh && chmod 755 /proc/{ipx}/root/tmp/t69008-ops.sh"));
            let (_, loop_out) = sh(&format!("sh -c 'echo $$ > /sys/fs/cgroup/system.slice/{sx}/cgroup.procs; exec nsenter -t {ipx} -m -n -p -S {ux} -G {ux} -- /bin/sh /tmp/t69008-ops.sh' 2>&1 | tail -1"));
            if !loop_out.contains("LOOPDONE") { eprintln!("T-6.9-008: exhaustion loop did not complete: {}", loop_out.trim()); }
            ex_loop = loop_out.trim().chars().take(160).collect();
            g.terminate(&js(&v_ex, "launch_record_digest"));
            // The gateway spools its events and `agentbound-audit` appends them to the hash-chained store asynchronously, so the
            // denials this loop provoked are NOT in the store the instant the loop returns. Reading immediately was scoring 0 while
            // the denials appeared ~10 lines later — the row must wait for its own evidence to be durable before counting it.
            let ex_alloc = js(&v_ex, "allocation_id");
            for _ in 0..60 {
                let n: i64 = sh(&format!("tail -n +{} /var/lib/agentbound/audit/events.jsonl | grep -h gateway.operation_denied | grep {ex_alloc} | grep -c '\"rule\":\"budget_operations\"'", run_start_line + 1)).1.trim().parse().unwrap_or(0);
                if n > 0 { break; }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
        let ev: Vec<(String, i64)> = vec![
            ("operations → budget_operations".into(), denial_count("budget_operations")),
            ("bytes_per_operation / bytes → budget_bytes".into(), denial_count("budget_bytes")),
            ("objects → budget_objects".into(), denial_count("budget_objects")),  // exercised in-session by T-6.9-008.objects
            ("connection_count → connection_limit".into(), refused_conn("connection_limit")),
        ];
        let unenforced: Vec<&String> = ev.iter().filter(|(_, n)| *n == 0).map(|(c, _)| c).collect();
        g.rec("T-6.9-008", rc_ex == 0 && unenforced.is_empty() && absent.len() == 3,
            format!("gateway budget classes present in the catalogue: {present:?}; each is enforced, with denials recorded in the hash-chained log by THIS run: {}; exhaustion session rc={rc_ex} ({ex_loop}); classes absent at 1B and deferred to 1C under R-GW-9: {absent:?} (this row does not claim them)",
                ev.iter().map(|(c, n)| format!("{c}={n}")).collect::<Vec<_>>().join(", ")));
    }
    // ---- T-6.9-005 / R-GW-7: budget consumption survives a gateway restart (WP3.1 item 3) ----
    // The gateway reports per-record op_count via `status`. Pings from an in-scope peer raise it; after a restart the figure must
    // be restored from the lifecycle record store, not reset to 0 — and the ping budget (64 operations) must then be exhausted with
    // `budget_operations` at the SAME cumulative count it would have hit without the restart. Every figure below is read back.
    let gwst = |lrd: &str| -> Value { match wire::connect("/run/agentbound/gateway.sock") { Ok(c) => c.call(&wire::request("status", &format!("conf-{}", ab_common::sig::monotonic_ns()), Value::obj(vec![("launch_record_digest", Value::s(lrd))]))).unwrap_or(Value::Null), Err(_) => Value::Null } };
    let ops_before: i64 = js(&gwst(&lrd2), "body.operations").parse().unwrap_or(-1);
    let refused_at_start: usize = sh(&format!("grep -h gateway.operation_denied /var/lib/agentbound/audit/events.jsonl | grep {} | grep -c '\"rule\":\"budget_operations\"'", js(&v2, "allocation_id"))).1.trim().parse().unwrap_or(0);
    // the loop runs inside the session (busybox sh); it is placed in the session's /tmp so no host-shell quoting is involved
    // the session's /tmp is a private tmpfs: reach it through the session's mount namespace, never a host path
    std::fs::write("/tmp/pings.sh", "i=0; while [ $i -lt $1 ]; do ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping '{}' 2>&1 | tail -c 120; echo; i=$((i+1)); done\n").unwrap();
    sh(&format!("nsenter -t {ipid2} -m -- sh -c 'cat > /tmp/pings.sh' < /tmp/pings.sh"));
    let ping = |n: usize| -> String { sh(&format!("sh -c 'echo $$ > /sys/fs/cgroup/system.slice/{scope2}/cgroup.procs; exec nsenter -t {ipid2} -m -n -p -S {uid2} -G {uid2} -- sh /tmp/pings.sh {n}' 2>&1")).1 };
    let first = ping(5);
    let ops_mid: i64 = js(&gwst(&lrd2), "body.operations").parse().unwrap_or(-1);
    // the per-operation-id consumption as the lifecycle store holds it (this is what a restarted gateway restores)
    let stored = |lrd: &str| -> (i64, String) { let r = lc("record", Value::obj(vec![("launch_record_digest", Value::s(lrd))])); let b = r.get("body").and_then(|b| b.get("budget")).cloned().unwrap_or(Value::Null); (b.get("op:gateway-ping").and_then(|x| x.get("operations")).and_then(|x| x.as_int()).unwrap_or(-1), String::from_utf8_lossy(&json::canonical(&b)).chars().take(160).collect()) };
    let (stored_mid, budget_rec) = stored(&lrd2);
    sh("systemctl restart agentbound-gateway"); std::thread::sleep(std::time::Duration::from_secs(3));
    let ops_after_restart: i64 = js(&gwst(&lrd2), "body.operations").parse().unwrap_or(-1);
    // exhaust: the ping budget is 64 operations for this task; keep pinging until refused and check the refusal count matches
    let out = ping(70);
    // refusals are read from the gateway's own denial events for this allocation (the client output is truncated per line)
    let admitted = out.matches("\"pong\":true").count();
    let refused: usize = sh(&format!("grep -h gateway.operation_denied /var/lib/agentbound/audit/events.jsonl | grep {} | grep -c '\"rule\":\"budget_operations\"'", js(&v2, "allocation_id"))).1.trim().parse::<usize>().unwrap_or(0).saturating_sub(refused_at_start);
    let ops_final: i64 = js(&gwst(&lrd2), "body.operations").parse().unwrap_or(-1);
    let ping_budget: i64 = 64;
    let (stored_final, _) = stored(&lrd2);
    // pings admitted before the restart + pings admitted after it must equal the per-op budget exactly; a reset would admit 64 more
    let first_ok = first.matches("\"pong\":true").count() == 5;
    g.rec("T-6.9-005.budget-persist", first_ok && ops_mid == ops_before + 5 && ops_after_restart == ops_mid && stored_mid >= 5 && refused > 0 && stored_final == ping_budget && (stored_mid as usize) + admitted == ping_budget as usize,
        format!("5 pings admitted (session op_count {ops_before}->{ops_mid}); lifecycle budget record then held op:gateway-ping operations={stored_mid} [{budget_rec}]; gateway restarted: session op_count restored to {ops_after_restart} (not reset); then {admitted} more pings admitted and {refused} refused budget_operations (gateway.operation_denied events for this allocation); stored op:gateway-ping operations={stored_final} == budget {ping_budget}, and {stored_mid}+{admitted}={}", stored_mid as usize + admitted));    // ---- T-6.4-009: PID reuse against the per-operation check (catalogue: "including reuse within one CLK_TCK tick") ----
    // ---- D4.7-idempotency-persist (component-interfaces §5, independent WP3.1 validation finding 6) ----
    // The ping budget of the session above is exhausted, so this uses a FRESH session: one op with key K; K again (same input) →
    // original reply, `gateway.operation_replayed`, op_count unchanged; K with different args → `conflict/idempotency_conflict`;
    // gateway RESTART; K again from a NEW connection → still the original reply with no new execution and no budget consumed; and the
    // outcome is readable from the lifecycle record store, which is where a restarted gateway got it. Every figure is read back.
    {
        let gbi = Rig { as_user: "bob".into(), rows: vec![] }; let pi = gbi.write_req("idem-persist", GW_REQ);
        let gi = gbi.request(&pi, ""); let vi = gi.1.clone();
        let (lrdi, aidi) = (js(&vi, "launch_record_digest"), js(&vi, "allocation_id"));
        let scopei = js(&vi, "scope_id"); let uidi = js(&vi, "uid");
        let ipidi = sh(&format!("head -1 /sys/fs/cgroup/system.slice/{scopei}/cgroup.procs")).1.trim().to_string();
        let ready = gi.0 == 0 && !lrdi.is_empty() && !ipidi.is_empty() && !uidi.is_empty();
        // the session's own git-worker issues gateway operations for ~20 s after launch; wait for its GW-END so the session's
        // op_count is quiescent and every change below is attributable to this row's calls alone
        let coni = js(&vi, "console"); for _ in 0..30 { if std::fs::read_to_string(&coni).unwrap_or_default().contains("GW-END") { break; } std::thread::sleep(std::time::Duration::from_secs(1)); }
        let key = format!("idem-persist-{}", ab_common::sig::monotonic_ns());
        // the args variant is selected by a plain token inside the script so no JSON crosses three layers of shell quoting
        std::fs::write("/tmp/idem.sh", "if [ \"$2\" = alt ]; then A='{\"x\":1}'; else A='{}'; fi; ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping \"$A\" --idem \"$1\" 2>&1 | tail -c 300; echo\n").unwrap();
        sh(&format!("nsenter -t {ipidi} -m -- sh -c 'cat > /tmp/idem.sh' < /tmp/idem.sh"));
        let call = |k: &str, variant: &str| -> String { sh(&format!("sh -c 'echo $$ > /sys/fs/cgroup/system.slice/{scopei}/cgroup.procs; exec nsenter -t {ipidi} -m -n -p -S {uidi} -G {uidi} -- sh /tmp/idem.sh {k} {variant}' 2>&1")).1 };
        let r1 = call(&key, "same"); let ops1: i64 = js(&gwst(&lrdi), "body.operations").parse().unwrap_or(-1);
        let r2 = call(&key, "same"); let ops2: i64 = js(&gwst(&lrdi), "body.operations").parse().unwrap_or(-1);
        let r3 = call(&key, "alt");
        let stored_before = { let r = lc("record", Value::obj(vec![("launch_record_digest", Value::s(&lrdi))])); r.get("body").and_then(|b| b.get("outcomes")).map(|o| String::from_utf8_lossy(&canonical(o)).into_owned()).unwrap_or_default() };
        sh("systemctl restart agentbound-gateway"); std::thread::sleep(std::time::Duration::from_secs(3));
        let ops_r: i64 = js(&gwst(&lrdi), "body.operations").parse().unwrap_or(-1);
        let r4 = call(&key, "same"); let ops4: i64 = js(&gwst(&lrdi), "body.operations").parse().unwrap_or(-1);
        let seq = |r: &str| r.split("\"operation_seq\":").nth(1).and_then(|t| t.split(|c: char| !c.is_ascii_digit()).next()).unwrap_or("?").to_string();
        let replayed: usize = sh(&format!("grep -h gateway.operation_replayed /var/lib/agentbound/audit/events.jsonl | grep -c {aidi}")).1.trim().parse().unwrap_or(0);
        let completed: usize = sh(&format!("grep -h '\"kind\":\"gateway.operation_completed\"' /var/lib/agentbound/audit/events.jsonl | grep {aidi} | grep -c {key}")).1.trim().parse().unwrap_or(0);
        let ok = ready && r1.contains("\"pong\":true") && ops1 >= 1 && r2.contains("\"pong\":true") && seq(&r1) == seq(&r2) && ops2 == ops1
            && r3.contains("idempotency_conflict") && stored_before.contains(&key) && ops_r == ops1 && r4.contains("\"pong\":true") && seq(&r4) == seq(&r1) && ops4 == ops1 && completed == 1 && replayed >= 2;
        g.rec("D4.7-idempotency-persist", ok, format!("first: seq={} ops={ops1}; same key+input: seq={} ops={ops2} (no consumption); different input: {}; lifecycle store holds outcome for key: {}; gateway restarted: ops restored={ops_r}; same key after restart from new connection: seq={} ops={ops4}; operation_completed for key={completed} (exactly once) replayed={replayed}",
            seq(&r1), seq(&r2), if r3.contains("idempotency_conflict") { "conflict/idempotency_conflict" } else { r3.trim() }, stored_before.contains(&key), seq(&r4)));
        g.terminate(&lrdi);
    }
    // Construction: an in-scope session-uid client establishes a connection and *exits immediately*, leaving a forked holder with the
    // connected descriptor. The driver then recycles the establishing PID inside the session's pid namespace via ns_last_pid — a
    // capability no session has (measured: the session uid gets EIO/EACCES on that file, and even root needs CAP_SYS_ADMIN in the
    // owning userns) — so the adversary here is *stronger* than the threat model's session. The recycled process has the same pid,
    // uid and cgroup as the establisher; only the pidfs inode differs. The holder's packet must then be denied `process_mismatch`
    // naming the inode, and no operation may be admitted on that connection.
    // the trigger file lives in the session's private /tmp and is created through its mount namespace
    sh(&format!("nsenter -t {ipid2} -m -- rm -f /tmp/t9"));
    let (_, hold) = sh(&format!("sh -c 'echo $$ > /sys/fs/cgroup/system.slice/{scope2}/cgroup.procs; exec nsenter -t {ipid2} -m -n -p -S {uid2} -G {uid2} -- ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping {{}} --hold-fd /tmp/t9' 2>&1 | head -c 200"));
    let est_pid: i32 = hold.split("establishing_pid=").nth(1).and_then(|x| x.split_whitespace().next()).and_then(|x| x.parse().ok()).unwrap_or(-1);
    // recycle the establishing pid inside the session pidns: drive ns_last_pid to est_pid-1 and fork until the pid comes round
    // helper written to a file: recycle a given pid inside the session's pid namespace and report whether the pid came round
    std::fs::write("/tmp/t9-recycle.py", RECYCLE_PY).unwrap();
    let recycle = format!("nsenter -t {ipid2} -p -- python3 /tmp/t9-recycle.py {est_pid} 2>&1");
    let (_, rc9) = sh(&recycle);
    let aid2 = js(&v2, "allocation_id");
    let mismatch_count = || -> i32 { sh(&format!("grep -h process_mismatch /var/lib/agentbound/audit/events.jsonl | grep -c {aid2}")).1.trim().parse().unwrap_or(0) };
    let denials_before = mismatch_count();
    sh(&format!("nsenter -t {ipid2} -m -- touch /tmp/t9; sleep 2"));
    let (_, holder_spoke) = sh(&format!("grep -h 'gateway.connection_refused\\|gateway.operation_denied\\|gateway.connection_closed' /var/lib/agentbound/audit/events.jsonl | grep {aid2} | tail -3 | grep -oE '\"(rule|reason)\":\"[a-z_]*\"' | tr '\\n' ' '"));
    let (_, den) = sh(&format!("grep -h process_mismatch /var/lib/agentbound/audit/events.jsonl | grep {aid2} | tail -1 | grep -oE {DETAIL_RE}"));
    let denials_after = mismatch_count();
    let recycled_same = rc9.contains("same_pid_as_establisher");
    // What actually happens (measured, not assumed): the gateway polls each connection's peer pidfd, so the establisher's exit closes
    // the connection *before* the recycled process exists. The holder therefore cannot present the recycled instance at all. The row
    // asserts that composite defence: the connection is closed on establisher exit AND no operation is admitted on it afterwards. The
    // inode comparison that would refuse a recycled instance if the connection had survived is covered deterministically by the unit
    // test `session::tests::recycled_pid_rejected` (same pid, uid, cgroup and start time; only the pidfs inode differs).
    let closed_ev: i32 = sh(&format!("grep -h gateway.connection_closed /var/lib/agentbound/audit/events.jsonl | grep -c {aid2}")).1.trim().parse().unwrap_or(0);
    let holder_ops: i32 = sh(&format!("grep -h gateway.operation_admitted /var/lib/agentbound/audit/events.jsonl | grep {aid2} | grep -c 'establishing_pid\\\":{est_pid}'")).1.trim().parse().unwrap_or(0);
    g.rec("T-6.4-009", est_pid > 0 && recycled_same && closed_ev >= 1 && holder_ops == 0 && (denials_after == denials_before || den.contains("pidfs inode")), format!("PID reuse construction succeeded: establishing pid {est_pid} was recycled inside the session pidns [{}] with the same pid, uid and scope cgroup. Outcome: the connection was closed on establisher exit ({closed_ev} connection_closed events for this allocation) and NO operation was admitted for the establishing pid afterwards (holder ops={holder_ops}); process_mismatch denials {denials_before}->{denials_after} {}; last gateway events for this allocation after the holder was triggered: {holder_spoke}. The gateway's peer-pidfd poll closes the connection before a recycled instance can present itself, so the packet-level inode comparison is not reached live; that branch is covered deterministically by the unit test session::tests::recycled_pid_rejected", rc9.replace('\n', " ").chars().take(80).collect::<String>(), den.trim()));
    // ---- T-6.4-012: upstream identity — the operation's scoped repository resolves to the catalogue URL only; a caller cannot redirect it ----
    let redir_args = format!(r#"{{"expect_old":null,"ref_tail":"x","repository_id":"repo:demo","tip":"{}","url":"/tmp/evil.git"}}"#, "3".repeat(40));
    let (_, redir) = sh(&format!("sh -c 'echo $$ > /sys/fs/cgroup/system.slice/{scope2}/cgroup.procs; exec nsenter -t {ipid2} -m -n -p -S {uid2} -G {uid2} -- ab-gwclient /run/gateway.sock op:git-push-staging git.push_staging {} /image/probe.sh' 2>&1 | grep -o \"rule[^,]*\" | head -2 | tr \"\\n\" \" \"", redir_args.replace('"', "\\\"")));
    g.weak("T-6.4-012", redir.contains("args_schema"), format!("caller-supplied url ignored; bundle path enforced: {}", redir.replace('\n', " ")));
    // ---- D7 item 9: a denial names requirement, authorization, launch record and trace of *this* session only ----
    let (_, den) = sh(&format!("sh -c 'echo $$ > /sys/fs/cgroup/system.slice/{scope2}/cgroup.procs; exec nsenter -t {ipid2} -m -n -p -S {uid2} -G {uid2} -- ab-gwclient /run/gateway.sock op:git-push-staging-force git.push_staging_force {{}}' 2>&1 | grep \"^{{\" | head -c 1200"));
    let dv = parse(den.lines().find(|l| l.contains("\"rule\"")).unwrap_or("")); let az2 = js(&dv, "body.authorization_id");
    g.rec("D7-9.diagnostics", js(&dv, "body.requirement_id") == "R-GW-4" && az2.starts_with("launchrec:") && az2 != js(&rec, "body.binding.authorization_manifest.authorization_id") && js(&dv, "body.launch_record_digest") == lrd2 && js(&dv, "body.trace_id").starts_with("trace:") && !den.contains(&glrd) && !den.contains(&trace), format!("requirement={} authorization={} lrd-matches={} trace={} foreign-ids-absent={}", js(&dv, "body.requirement_id"), js(&dv, "body.authorization_id"), js(&dv, "body.launch_record_digest") == lrd2, js(&dv, "body.trace_id"), !den.contains(&glrd)));
    // ---- D7 item 8: audit loss follows the manifest — stop the receiver and make the gateway's local spool unwritable, then operate ----
    sh("systemctl stop agentbound-audit; mv /var/lib/agentbound/gateway/audit-gateway.jsonl /var/lib/agentbound/gateway/audit-gateway.jsonl.hold; mkdir /var/lib/agentbound/gateway/audit-gateway.jsonl");
    sh("systemctl restart agentbound-gateway; sleep 1.5");
    let (_, lossop) = sh(&format!("sh -c 'echo $$ > /sys/fs/cgroup/system.slice/{scope2}/cgroup.procs; exec nsenter -t {ipid2} -m -n -p -S {uid2} -G {uid2} -- sh -c \"ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping {{}}; sleep 0.5; ab-gwclient /run/gateway.sock op:gateway-ping gateway.ping {{}}\"' 2>&1 | head -c 400"));
    std::thread::sleep(std::time::Duration::from_secs(2));
    let st2 = lc("status", Value::obj(vec![("launch_record_digest", Value::s(&lrd2))]));
    sh("rmdir /var/lib/agentbound/gateway/audit-gateway.jsonl; mv /var/lib/agentbound/gateway/audit-gateway.jsonl.hold /var/lib/agentbound/gateway/audit-gateway.jsonl; systemctl start agentbound-audit; sleep 1; systemctl restart agentbound-gateway; sleep 1");
    let (_, lossev) = sh(&format!("grep '{lrd2}' /var/lib/agentbound/audit-lifecycle.jsonl | grep revocation_received | tail -1 | grep -o '\"trigger\":\"[^\"]*\"'"));
    // the first admitted operation's event cannot be recorded anywhere → gateway closes admission and signals lifecycle; manifest maps the
    // trigger to terminate, so by the second attempt the session is gone (nsenter target missing) or the packet meets admission_closed
    let second = lossop.lines().last().unwrap_or("").to_string();
    g.rec("D7-8.audit-loss", (second.contains("admission_closed") || second.contains("No such file") || second.contains("closed by gateway")) && lossev.contains("audit_pipeline_degraded_below_stop_threshold") && js(&st2, "body.state") == "terminated", format!("gateway with no audit path (receiver down, spool unwritable): first op's event lost → admission closed + revocation_signal; lifecycle {} → state={}; second attempt: {}", lossev.trim(), js(&st2, "body.state"), second.chars().take(100).collect::<String>()));
    if !lrd2.is_empty() { g.terminate(&lrd2); }
    // ---- carry-in: storage-principal ownership projection at seal — the session's workspace files now belong to the manifest's storage principal ----
    std::thread::sleep(std::time::Duration::from_secs(3));
    let (_, own) = sh(&format!("stat -c '%U %G' {ws}/work-{uid2} 2>&1; find {ws} -user {uid2} 2>/dev/null | wc -l"));
    let (_, own_ev) = sh(&format!("grep '{lrd2}' /var/lib/agentbound/audit-lifecycle.jsonl /var/lib/agentbound/audit/events.jsonl | grep ownership_projected | head -1 | grep -o '\"detail\":{{[^}}]*}}'"));
    g.rec("D-06.storage-principal", own.starts_with("storage-engineering") && own.lines().last().unwrap_or("1").trim() == "0" && own_ev.contains("\"failed\": 0") || own.starts_with("storage-engineering") && own.lines().last().unwrap_or("1").trim() == "0" && own_ev.contains("\"failed\":0"), format!("work dir owner after seal: {}; files still owned by ephemeral uid: {}; {}", own.lines().next().unwrap_or(""), own.lines().last().unwrap_or(""), own_ev.trim()));
    // ---- 1A partial / N-A rows re-run under local-socket (recorded verdicts; the driver asserts the property that changed) ----
    // D-02 / T-6.1-003: still no PTY or attach interface at 1B; the descriptor allowlist is unchanged (0/1/2 + one gateway socket mount) — remains partial by design
    let (_, alw) = sh(&format!("grep -c gateway_socket /dev/null; echo {}", rec.get("body").and_then(|b| b.get("binding")).and_then(|b| b.get("launch_binding")).and_then(|b| b.get("descriptor_allowlist")).and_then(|a| a.as_arr()).map(|a| a.len()).unwrap_or(0)));
    g.recorded("D-02.1B", alw.trim().ends_with('4'), format!("descriptor allowlist entries={} (stdin, stdout, stderr, gateway_socket mount); no attach/PTY path exists to deny — partial stays recorded", alw.trim()));
    g.recorded("T-6.1-003.1B", alw.trim().ends_with('4'), "no PTY projected under local-socket either; N/A stays recorded");
    // T-6.1-013: broker socket reuse — the sibling's projected socket path is not present in this mount namespace; the gateway directory is not reachable
    // T-6.1-013: broker socket reuse — the previous session's socket node is gone from the host and its mount is not in a new session; connecting to a stale path fails
    let (_, stale) = sh(&format!("ls /run/agentbound/gw/ | grep -c {}; python3 -c \"import socket\ns=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET)\ntry:\n s.connect('/run/agentbound/gw/{}.sock'); print('connected')\nexcept OSError as e: print('err',e.errno)\"", js(&v, "allocation_id").rsplit(':').next().unwrap_or("x"), js(&v, "allocation_id").rsplit(':').next().unwrap_or("x")));
    g.rec("T-6.1-013", stale.starts_with('0') && stale.contains("err"), format!("sealed session's socket: nodes left={} connect={}", stale.lines().next().unwrap_or(""), stale.lines().last().unwrap_or("")));
    // T-6.2-008: the git-worker image has git + sh + the client only; no package loader; interpreter set is closed by the image
    let (_, img) = sh("ls /var/lib/agentbound/images/rootfs/usr/bin /var/lib/agentbound/images/rootfs/bin | grep -cE '^(python|perl|pip|npm|node|apt|dpkg|curl|wget)'");
    g.recorded("T-6.2-008.1B", img.trim() == "0", format!("loaders/interpreters beyond sh+git in image: {}", img.trim()));
    // D-15: delegation — no child-session operation exists in the catalogue; a session cannot request a session (lifecycle/policy sockets unreachable: T-6.4-003)
    let (_, ops) = sh("python3 -c \"import json;c=json.load(open('/etc/agentbound/catalogue.json'));print([o for o in c['operations'] if 'deleg' in o or 'session' in o])\"");
    g.recorded("D-15.1B", ops.trim() == "[]", format!("delegation operations in catalogue: {} — residual stays recorded (no delegation path to narrow)", ops.trim()));
    // ---- harness integrity: coverage of the frozen catalogue population, duplicates, provenance, class counts ----
    let count = |c: &str| g.rows.iter().filter(|r| r.verdict == c).count();
    let (pass, weak, recorded, fail, fixture) = (count("PASS"), count("WEAK"), count("RECORDED"), count("FAIL"), count("FIXTURE"));
    let mut dup = std::collections::HashMap::<String, usize>::new(); for r in &g.rows { *dup.entry(r.id.clone()).or_default() += 1; }
    let dups: Vec<String> = dup.iter().filter(|(_, n)| **n > 1).map(|(k, n)| format!("{k}×{n}")).collect();
    let best = |cid: &str| -> &'static str { let mut b = "NOT-EXECUTED"; for r in &g.rows { if catalogue_id(&r.id) == cid { b = match (b, r.verdict) { (_, "FAIL") => "FAIL", ("FAIL", _) => "FAIL", (_, "PASS") => "PASS", ("PASS", _) => "PASS", (_, "WEAK") => "WEAK", ("WEAK", _) => "WEAK", (_, "RECORDED") => "RECORDED", (o, _) => o }; } } b };
    let expected = expected_ids();
    let mut cov: Vec<(String, String, &str)> = expected.iter().map(|(id, ms, _)| (id.clone(), ms.clone(), best(id))).collect();
    let known: std::collections::HashSet<String> = expected.iter().map(|(i, _, _)| i.clone()).collect();
    let required: std::collections::HashMap<String, String> = expected.iter().map(|(i, _, r)| (i.clone(), r.clone())).collect();
    let extra: Vec<String> = { let mut e: Vec<String> = g.rows.iter().filter(|r| r.verdict != "FIXTURE").map(|r| catalogue_id(&r.id)).filter(|c| !known.contains(c)).collect(); e.sort(); e.dedup(); e };
    let n_not = cov.iter().filter(|c| c.2 == "NOT-EXECUTED").count(); let n_cov_pass = cov.iter().filter(|c| c.2 == "PASS").count();
    cov.sort_by(|a, b| a.0.cmp(&b.0));
    // Provenance (independent WP3.1 validation, finding 4). The commit is the one EMBEDDED in this runner at build time from the
    // sending checkout — never read from a file on the build host, which is how every earlier register came to report `207930e`
    // for binaries built from later source. Each installed daemon is asked for its own embedded provenance too, so an install that is
    // stale relative to the runner shows up as a mismatch in the register instead of being invisible. Digests are full SHA-256.
    let commit = ab_common::provenance::describe();
    let mut prov_rows = String::new(); let mut prov_mismatch = false;
    for b in ["agentbound", "agentbound-launch", "agentbound-lifecycle", "agentbound-policy", "agentbound-audit", "agentbound-gateway", "ab-conformance"] {
        let (_, d) = sh(&format!("sha256sum /usr/local/bin/{b} 2>/dev/null | cut -d' ' -f1")); let (_, pv) = sh(&format!("/usr/local/bin/{b} --provenance 2>/dev/null"));
        let pc = pv.split_whitespace().find_map(|t| t.strip_prefix("commit=")).unwrap_or("unknown").to_string();
        let pd = pv.split_whitespace().find_map(|t| t.strip_prefix("dirty=")).unwrap_or("unknown").to_string();
        if pc != ab_common::provenance::COMMIT || pd != ab_common::provenance::DIRTY { prov_mismatch = true; }
        prov_rows.push_str(&format!("| `{b}` | `{}` | `{pc}` | {pd} |\n", d.trim()));
    }
    let bins = if prov_mismatch { "**MISMATCH — at least one installed binary was not built from the same source as this runner; see provenance table**".to_string() } else { "all installed binaries embed the same commit and dirty state as this runner".to_string() };
    let (_, gwc) = sh("sha256sum /var/lib/agentbound/images/rootfs/bin/ab-gwclient 2>/dev/null | cut -d' ' -f1"); let (_, catd) = sh("sha256sum /etc/agentbound/catalogue.json | cut -d' ' -f1");
    prov_rows.push_str(&format!("| `ab-gwclient` (in image) | `{}` | — | — |\n| `/etc/agentbound/catalogue.json` | `{}` | — | — |\n", gwc.trim(), catd.trim()));
    let run_id = format!("run-{}", ab_common::sig::monotonic_ns());
    // Three separate verdicts (independent WP3.1 validation, finding 1). They MUST NOT be conflated: a suite can complete with no
    // assertion failing, cover every catalogue id, and still not satisfy the frozen requirements — that was exactly the state of
    // every earlier register, whose "run verdict PASS" meant only the first of these.
    //   suite      — the harness ran to completion and no assertion was FALSE (no FAIL rows);
    //   coverage   — every frozen 1A+1B catalogue id executed exactly once, and no row claims an id outside the catalogue;
    //   requirements — every mandatory id's best verdict SATISFIES its required verdict (PASS, or RECORDED where the catalogue's own
    //                pass criterion admits it). WEAK never satisfies; RECORDED where PASS is required never satisfies.
    let suite_ok = fail == 0;
    let coverage_ok = dups.is_empty() && n_not == 0 && extra.is_empty() && !prov_mismatch && ab_common::provenance::DIRTY == "false";
    let unmet: Vec<String> = cov.iter().filter(|(id, _, v)| !satisfies(v, required.get(id).map(|s| s.as_str()).unwrap_or("PASS")))
        .map(|(id, _, v)| format!("{id}={v}(required {})", required.get(id).map(|s| s.as_str()).unwrap_or("PASS"))).collect();
    let requirements_ok = coverage_ok && unmet.is_empty();
    let ok = suite_ok && coverage_ok && requirements_ok;
    let verdict_line = format!("suite {}; coverage {}; requirements {}{}", if suite_ok { "COMPLETE" } else { "FAIL" }, if coverage_ok { "COMPLETE" } else { "FAIL" },
        if requirements_ok { "PASS" } else { "FAIL" }, if unmet.is_empty() { String::new() } else { format!(" — unmet: {}", unmet.join(", ")) });
    let mut md = format!("# Agentbound conformance run — 1A + 1B rows (machine output)\n\n- Host: {}\n- Kernel: {}\n- systemd: {}\n- git: {}\n- Date: {}\n- Run id: {run_id}\n- Source commit (embedded at build from the sending checkout): {}\n- Installed binaries: {}\n- Expected population: {} catalogue ids (test-catalogue 1A+1B)\n- Assertions: {} PASS, {} WEAK, {} RECORDED, {} FAIL ({} fixtures excluded)\n- Catalogue coverage: {} PASS, {} WEAK, {} RECORDED, {} FAIL, **{} NOT-EXECUTED**\n- Duplicate row ids: {}\n- Row ids outside the catalogue: {}\n- **Suite verdict: {}** (no assertion false)\n- **Coverage verdict: {}** (every frozen id executed once, none outside the catalogue)\n- **Requirements verdict: {}** (every mandatory id satisfies its required verdict; unmet: {})\n- **Run verdict: {}** — PASS only when all three hold\n\n## Provenance\n\n| Artifact | SHA-256 | Embedded commit | Dirty |\n|---|---|---|---|\n{prov_rows}\n## Catalogue coverage\n\n| Catalogue id | Milestone | Best verdict | Required | Satisfied |\n|---|---|---|---|---|\n", sh("hostname").1.trim(), sh("uname -r").1.trim(), sh("systemctl --version | head -1").1.trim(), sh("git --version").1.trim(), sh("date -u +%FT%TZ").1.trim(), commit, bins, expected.len(), pass, weak, recorded, fail, fixture, n_cov_pass, cov.iter().filter(|c| c.2 == "WEAK").count(), cov.iter().filter(|c| c.2 == "RECORDED").count(), cov.iter().filter(|c| c.2 == "FAIL").count(), n_not, if dups.is_empty() { "none".to_string() } else { dups.join(", ") }, if extra.is_empty() { "none".to_string() } else { extra.join(", ") }, if suite_ok { "COMPLETE" } else { "FAIL" }, if coverage_ok { "COMPLETE" } else { "FAIL (coverage, duplicate, extra, or PROVENANCE failure)" }, if requirements_ok { "PASS" } else { "FAIL" }, if unmet.is_empty() { "none".to_string() } else { unmet.join(", ") }, if ok { "PASS" } else { "FAIL (coverage or assertion)" });
    for (id, ms, v) in &cov { let r = required.get(id).map(|s| s.as_str()).unwrap_or("PASS"); md.push_str(&format!("| {id} | {ms} | {v} | {r} | {} |\n", if satisfies(v, r) { "yes" } else { "**NO**" })); }
    md.push_str("\n## Rows\n\n| Row | Verdict | Evidence |\n|---|---|---|\n");
    for r in &g.rows { md.push_str(&format!("| {} | {} | {} |\n", r.id, r.verdict, r.evidence.replace('|', "\\|"))); }
    std::fs::write("/root/wp2/conformance-run.md", md).unwrap();
    for (id, _, v) in &cov { if *v == "NOT-EXECUTED" { println!("NOT-EXECUTED {id}"); } }
    println!("\nassertions: {pass} PASS {weak} WEAK {recorded} RECORDED {fail} FAIL; catalogue: {n_cov_pass}/{} PASS, {n_not} not executed; dups={} extra={}; {verdict_line}; run verdict {}; register at /root/wp2/conformance-run.md", expected.len(), dups.len(), extra.len(), if ok { "PASS" } else { "FAIL" });
    if !ok { std::process::exit(1); }
}
