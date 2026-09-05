//! Git staging-ref adapter (R-GW-5; WP1 git-staging spike shape). The session's objects arrive as a bundle
//! over the authenticated connection; nothing is ever fetched *from* the session. Import → quarantine →
//! verify → push only to `refs/agentbound/<session>/<tail>` of the manifest-scoped repository.
use crate::Gateway;
use ab_common::json::Value;
use std::process::Command;

type R = Result<Value, (&'static str, String)>;

/// GS-4 refusal set enforced by string policy before any Git process runs; `git check-ref-format` is the second filter.
pub fn validate_tail(t: &str) -> Result<(), &'static str> {
    if t.is_empty() || t.len() > 128 { return Err("ref_tail_empty_or_long"); }
    if t.starts_with('+') || t.starts_with('-') || t.starts_with('/') || t.ends_with('/') || t.ends_with('.') { return Err("ref_tail_marker"); }
    if t.contains("..") || t.contains(':') || t.contains('*') || t.contains('?') || t.contains('[') || t.contains('\\') || t.contains('@') || t.contains("//") || t.ends_with(".lock") || t.contains(".lock/") { return Err("ref_tail_grammar"); }
    if t.chars().any(|c| c.is_whitespace() || c.is_control() || c == '~' || c == '^' || !c.is_ascii()) { return Err("ref_tail_charset"); }
    if t.starts_with("refs/") || t == "HEAD" { return Err("ref_tail_names_ref"); }
    Ok(())
}

fn git(dir: &str, args: &[&str]) -> Result<String, String> {
    let o = Command::new("git").arg("-C").arg(dir).args(args).env_clear().env("PATH", "/usr/bin:/bin").env("HOME", dir).env("GIT_CONFIG_NOSYSTEM", "1").env("GIT_TERMINAL_PROMPT", "0").output().map_err(|e| e.to_string())?;
    if o.status.success() { Ok(String::from_utf8_lossy(&o.stdout).trim().to_string()) } else { Err(String::from_utf8_lossy(&o.stderr).trim().to_string()) }
}

pub fn push_staging(gw: &mut Gateway, aid: &str, op: &Value, payload: &[u8], session_id: &str, trace: &str) -> R {
    let args = op.get("args").ok_or(("args_missing", String::new()))?;
    // closed argument set: anything that could redirect the effect (url, remote, refspec…) is a schema error, never silently ignored
    if let Some(o) = args.as_obj() { for (k, _) in o { if !matches!(k.0.as_str(), "expect_old" | "ref_tail" | "repository_id" | "tip") { return Err(("args_schema", format!("unknown member {}", k.0))); } } }
    let s = |k: &str| args.get(k).and_then(|x| x.as_str()).map(str::to_string);
    let (Some(repo), Some(tail), Some(tip)) = (s("repository_id"), s("ref_tail"), s("tip")) else { return Err(("args_schema", "repository_id, ref_tail, tip".into())) };
    // scope (D3): the operation's manifest scope names exactly one repository; the argument must equal it
    let this_op = op.get("operation_id").and_then(|x| x.as_str());
    let grant = gw.by_alloc[aid].ops.iter().find(|o| o.get("operation_id").and_then(|x| x.as_str()) == this_op).cloned().unwrap_or(Value::Null);
    let scoped = grant.get("scope").and_then(|sc| sc.get("repository_id")).and_then(|x| x.as_str()).unwrap_or("").to_string();
    if repo != scoped { return Err(("scope_repository", format!("{repo} not in operation scope"))); }
    let rc = gw.cfg.catalogue.get("repositories").and_then(|r| r.get(&repo)).ok_or(("scope_repository", "unknown repository".into()))?;
    let url = rc.get("url").and_then(|x| x.as_str()).unwrap_or("").to_string();
    validate_tail(&tail).map_err(|r| (r, tail.clone()))?;
    if tip.len() != 40 || !tip.chars().all(|c| c.is_ascii_hexdigit()) { return Err(("tip_grammar", tip)); }
    if payload.is_empty() { return Err(("payload_missing", "bundle required".into())); }
    let sid = session_id.trim_start_matches("session:"); if sid.is_empty() { return Err(("session_id", String::new())); }
    let target_ref = format!("refs/agentbound/{sid}/{tail}");
    if Command::new("git").args(["check-ref-format", &target_ref]).status().map(|s| !s.success()).unwrap_or(true) { return Err(("ref_tail_grammar", target_ref)); }
    let objects_max = grant.get("budgets").and_then(|b| b.get("objects")).and_then(|x| x.as_int()).unwrap_or(10_000);
    // quarantine repository per operation: bundle verify, fetch, fsck, count objects — before anything touches the upstream
    let q = format!("{}/{}-{}", gw.cfg.quarantine, aid.rsplit(':').next().unwrap_or("a"), ab_common::sig::monotonic_ns()); std::fs::create_dir_all(&q).map_err(|e| ("quarantine", e.to_string()))?;
    let cred_helper = format!("credential.helper=!f() {{ echo username=agentbound-gateway; echo password=$(cat {}); }}; f", gw.cfg.credential);
    let res = (|| -> R {
        let bpath = format!("{q}/in.bundle"); std::fs::write(&bpath, payload).map_err(|e| ("quarantine", e.to_string()))?;
        git(&q, &["init", "-q", "--bare", "repo"]).map_err(|e| ("quarantine", e))?; let repo_dir = format!("{q}/repo");
        git(&repo_dir, &["bundle", "verify", &bpath]).map_err(|e| ("bundle_invalid", e))?;
        git(&repo_dir, &["fetch", "-q", "--no-tags", &bpath, &format!("{tip}:refs/quarantine/tip")]).map_err(|e| ("bundle_fetch", e))?;
        let got = git(&repo_dir, &["rev-parse", "refs/quarantine/tip"]).map_err(|e| ("bundle_fetch", e))?; if got != tip { return Err(("tip_mismatch", got)); }
        git(&repo_dir, &["fsck", "--connectivity-only", "--no-dangling", "refs/quarantine/tip"]).map_err(|e| ("fsck", e))?;
        let n: i64 = git(&repo_dir, &["rev-list", "--objects", "refs/quarantine/tip"]).map_err(|e| ("fsck", e))?.lines().count() as i64; if n > objects_max { return Err(("budget_objects", format!("{n} > {objects_max}"))); }
        // upstream push with the gateway-held credential; trace propagated as push options and in the ref namespace itself
        let old = s("expect_old").filter(|o| o != "null").unwrap_or_default();
        let lease = if old.is_empty() { format!("--force-with-lease={target_ref}:") } else { format!("--force-with-lease={target_ref}:{old}") };
        let refspec = format!("refs/quarantine/tip:{target_ref}");
        let trace_opt = format!("--push-option=agentbound-trace={trace}"); let sess_opt = format!("--push-option=agentbound-session={session_id}");
        let out = git(&repo_dir, &["-c", &cred_helper, "push", "-q", "--porcelain", &trace_opt, &sess_opt, &lease, &url, &refspec]).map_err(|e| ("upstream_rejected", e))?;
        Ok(Value::obj(vec![("bytes", Value::Int(payload.len() as i64)), ("new", Value::s(&tip)), ("objects", Value::Int(n)), ("porcelain", Value::s(&out)), ("remote_ref", Value::s(&target_ref)), ("repository_id", Value::s(&repo))]))
    })();
    let _ = std::fs::remove_dir_all(&q);
    res
}
