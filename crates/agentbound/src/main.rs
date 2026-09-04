//! agentbound: unprivileged operator CLI. Talks to policy and lifecycle over
//! their sockets; invokes the constructor through the provisioned root path.
//!   agentbound request <request.json>      → authorization_id (and launches)
//!   agentbound status <authz|lrd> | list | terminate <lrd> [reason] | quiesce <lrd> | revoke <lrd> <trigger> | audit <authz|lrd>
use ab_common::json::{self, canonical, Value, REQUEST_LIMITS};
use ab_common::wire;

fn call(sock: &str, op: &str, body: Value) -> Value {
    let idem = format!("cli-{}-{}", std::process::id(), ab_common::sig::monotonic_ns());
    match wire::connect(sock).and_then(|c| c.call(&wire::request(op, &idem, body))) { Ok(v) => v, Err(e) => Value::obj(vec![("class", Value::s(wire::CLASS_UNAVAILABLE)), ("detail", Value::s(&e.to_string())), ("ok", Value::Bool(false))]) }
}
fn out(v: &Value) -> bool { println!("{}", String::from_utf8_lossy(&canonical(v))); v.get("ok").and_then(|x| x.as_bool()) == Some(true) }
fn key(x: &str) -> Value { if x.starts_with("sha256:") { Value::obj(vec![("launch_record_digest", Value::s(x))]) } else { Value::obj(vec![("authorization_id", Value::s(x))]) } }

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let env = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_string());
    let (pol, lc, au) = (env("AGENTBOUND_POLICY_SOCKET", "/run/agentbound/policy.sock"), env("AGENTBOUND_LIFECYCLE_SOCKET", "/run/agentbound/lifecycle.sock"), env("AGENTBOUND_AUDIT_SOCKET", "/run/agentbound/audit.sock"));
    let launcher = env("AGENTBOUND_LAUNCHER", "sudo -n /usr/local/bin/agentbound-launch");
    let ok = match a.get(1).map(|s| s.as_str()) {
        Some("request") => {
            let bytes = std::fs::read(&a[2]).expect("request file");
            let req = json::parse(&bytes, &REQUEST_LIMITS).unwrap_or_else(|e| { eprintln!("request: {e}"); std::process::exit(2) });
            let r = call(&pol, "submit_request", Value::obj(vec![("request", req)]));
            if !out(&r) { std::process::exit(1) }
            let az = r.get("body").and_then(|b| b.get("authorization_id")).and_then(|x| x.as_str()).unwrap_or("").to_string();
            if a.get(3).map(|s| s.as_str()) == Some("--no-launch") { true } else {
                let mut parts = launcher.split_whitespace(); let prog = parts.next().unwrap();
                let mut cmd = std::process::Command::new(prog); cmd.args(parts).args(["--authorization", &az]);
                if let Some(f) = a.iter().position(|x| x == "--fault").and_then(|i| a.get(i + 1)) { cmd.args(["--fault", f]); }
                cmd.status().map(|s| s.success()).unwrap_or(false)
            }
        }
        Some("status") => out(&call(&lc, "status", key(&a[2]))),
        Some("list") => out(&call(&lc, "list", Value::obj(vec![]))),
        Some("terminate") => out(&call(&lc, "terminate", Value::obj(vec![("launch_record_digest", Value::s(&a[2])), ("reason", Value::s(a.get(3).map(|s| s.as_str()).unwrap_or("operator")))]))),
        Some("quiesce") => out(&call(&lc, "quiesce", Value::obj(vec![("launch_record_digest", Value::s(&a[2])), ("reason", Value::s("operator"))]))),
        Some("revoke") => out(&call(&lc, "revocation_signal", Value::obj(vec![("launch_record_digest", Value::s(&a[2])), ("source", Value::s("cli")), ("trigger", Value::s(&a[3]))]))),
        Some("audit") => out(&call(&au, "query", key(&a[2]))),
        Some("audit-status") => out(&call(&au, "status", Value::obj(vec![]))),
        _ => { eprintln!("usage: agentbound request <file> [--no-launch] [--fault f] | status <id> | list | terminate <lrd> [reason] | quiesce <lrd> | revoke <lrd> <trigger> | audit <id> | audit-status"); false }
    };
    std::process::exit(if ok { 0 } else { 1 });
}
