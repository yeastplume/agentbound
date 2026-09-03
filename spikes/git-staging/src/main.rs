//! WP1 spike: Git staging-ref adapter and protected-branch behaviour (R-GW-5, D-13,
//! invariant 19). Mechanism questions:
//!   1. Can the gateway accept a session's objects without giving the session any
//!      credential or network path, and push them only to refs/agentbound/<session>/…?
//!   2. Does ref-name policy in the gateway (not the hook) refuse main, another
//!      session's namespace, refspec tricks (`+`, `:`, `..`, empty, non-UTF-8), and a
//!      trace mismatch?
//!   3. Does the Git host's protected-branch rule (an assumption) compose: even if the
//!      gateway were bypassed, refs/heads/main is refused by the host?
//!   4. Object transfer shape: `git bundle` over the SEQPACKET connection (descriptor
//!      transfer of a memfd is D7-3, already verified) → gateway `git fetch` from the
//!      bundle into a quarantine repo → `git push` with the gateway-held credential.
//!
//! Throwaway code: not TCB, not SLOC-counted.
use std::fs;
use std::process::{Command, Stdio};

fn result(item: &str, pass: bool, detail: &str) { println!("RESULT {item} {} {detail}", if pass { "PASS" } else { "FAIL" }); }
fn git(dir: &str, args: &[&str]) -> (bool, String) { git_env(dir, args, &[]) }
fn git_env(dir: &str, args: &[&str], env: &[(&str, &str)]) -> (bool, String) {
    let mut c = Command::new("git"); c.arg("-C").arg(dir).args(args).stdin(Stdio::null()).env_clear().env("PATH", "/usr/bin:/bin").env("HOME", "/tmp/ab-git/nohome").env("GIT_CONFIG_NOSYSTEM", "1").env("GIT_TERMINAL_PROMPT", "0");
    for (k, v) in env { c.env(k, v); }
    let o = c.output().unwrap(); (o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)).trim().to_string())
}
const ROOT: &str = "/tmp/ab-git";
const SESSION: &str = "sess-7f3a"; const OTHER: &str = "sess-0b19";

/// Gateway-side ref policy: the ONLY refs a push may touch. Returns the full target ref or a denial reason.
fn policy(session: &str, requested: &str, trace_in_request: &str, trace_authenticated: &str) -> Result<String, String> {
    if trace_in_request != trace_authenticated { return Err(format!("trace mismatch: request {trace_in_request} vs authenticated {trace_authenticated}")); }
    let prefix = format!("refs/agentbound/{session}/");
    if !requested.starts_with(&prefix) { return Err(format!("ref outside session staging namespace: {requested}")); }
    let tail = &requested[prefix.len()..];
    if tail.is_empty() || tail.contains("..") || tail.starts_with('/') || tail.ends_with('/') || tail.ends_with(".lock") || tail.contains(':') || tail.starts_with('+') || tail.contains('\\') || tail.chars().any(|c| c.is_control() || c == ' ' || c == '~' || c == '^' || c == '?' || c == '*' || c == '[') { return Err(format!("malformed ref tail: {tail:?}")); }
    let (ok, _) = git("/tmp", &["check-ref-format", requested]); if !ok { return Err(format!("git check-ref-format rejected {requested}")); }
    Ok(requested.to_string())
}

fn main() {
    println!("spike git-staging; {}", git("/tmp", &["--version"]).1);
    let _ = fs::remove_dir_all(ROOT); fs::create_dir_all(format!("{ROOT}/nohome")).unwrap();
    // --- "Git host": bare repo with protected-branch rule (assumption modelled as pre-receive hook) ---
    let host = format!("{ROOT}/host.git"); git(ROOT, &["init", "-q", "--bare", "host.git"]);
    fs::write(format!("{host}/hooks/pre-receive"), "#!/bin/sh\nwhile read old new ref; do case \"$ref\" in refs/heads/main|refs/heads/release/*) echo \"host: protected branch $ref refused\" >&2; exit 1;; esac; done\nexit 0\n").unwrap();
    fs::set_permissions(format!("{host}/hooks/pre-receive"), std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    // seed main via an admin clone
    let admin = format!("{ROOT}/admin"); git(ROOT, &["clone", "-q", &host, "admin"]);
    git(&admin, &["-c", "user.name=admin", "-c", "user.email=a@x", "commit", "-q", "--allow-empty", "-m", "seed"]); git(&admin, &["branch", "-M", "main"]);
    // host hook would refuse main from anyone; seed by writing the ref directly on the host (admin action)
    let (_, seed) = git(&admin, &["rev-parse", "HEAD"]); git(&admin, &["push", "-q", &host, &format!("{seed}:refs/heads/seed-import")]); git(&host, &["update-ref", "refs/heads/main", &seed]); git(&host, &["update-ref", "-d", "refs/heads/seed-import"]);
    // --- session workspace: a clone with NO remote credential and no network; produces commits and a bundle ---
    let ws = format!("{ROOT}/session-ws"); git(ROOT, &["clone", "-q", &host, "session-ws"]); git(&ws, &["remote", "remove", "origin"]);
    fs::write(format!("{ws}/fix.txt"), "agent change\n").unwrap();
    git(&ws, &["add", "."]); git(&ws, &["-c", "user.name=agent", "-c", "user.email=agent@session", "commit", "-q", "-m", "fix issue 1234"]);
    let (_, head) = git(&ws, &["rev-parse", "HEAD"]);
    let bundle = format!("{ROOT}/session.bundle"); let (b_ok, b_out) = git(&ws, &["bundle", "create", &bundle, &format!("{seed}..HEAD"), "HEAD"]);
    result("GS-1.session-produces-bundle-without-remote", b_ok && git(&ws, &["remote"]).1.is_empty(), &format!("bundle {} bytes; session repo has no remotes; commit {}", fs::metadata(&bundle).map(|m| m.len()).unwrap_or(0), &head[..12]));
    let _ = b_out;
    // --- gateway: quarantine repo; fetch from bundle only (no transport), verify objects, then push with gateway-held credential ---
    let gw = format!("{ROOT}/gateway-quarantine.git"); git(ROOT, &["init", "-q", "--bare", "gateway-quarantine.git"]);
    let (f_ok, f_out) = git(&gw, &["fetch", "-q", &bundle, &format!("HEAD:refs/incoming/{SESSION}")]);
    let (fsck_ok, fsck_out) = git(&gw, &["fsck", "--connectivity-only", "--no-dangling", &format!("refs/incoming/{SESSION}")]);
    let (_, incoming) = git(&gw, &["rev-parse", &format!("refs/incoming/{SESSION}")]);
    result("GS-2.gateway-imports-bundle-into-quarantine-and-fscks", f_ok && fsck_ok && incoming == head, &format!("fetch ok={f_ok} fsck ok={fsck_ok} {} imported {}", fsck_out.lines().next().unwrap_or(""), &incoming[..12]));
    let _ = f_out;
    // the gateway's credential: a helper only the gateway process has (here: a file readable only by root; the session UID cannot read it)
    let cred = format!("{ROOT}/gateway-cred"); fs::write(&cred, "protocol=https\nhost=example\nusername=agentbound-gw\npassword=SECRET\n").unwrap(); fs::set_permissions(&cred, std::os::unix::fs::PermissionsExt::from_mode(0o600)).unwrap();
    // pushes go over a file:// URL here; the credential is irrelevant to file:// but we verify the session cannot read it and never sees the remote URL
    let push = |requested: &str, trace_req: &str| -> (bool, String) {
        match policy(SESSION, requested, trace_req, "trace-7f3a") {
            Err(e) => (false, format!("gateway denied: {e}")),
            Ok(target) => { let (ok, out) = git(&gw, &["push", "-q", &host, &format!("refs/incoming/{SESSION}:{target}")]); (ok, if ok { format!("pushed {target}") } else { out }) }
        }
    };
    let (ok, out) = push(&format!("refs/agentbound/{SESSION}/fix-issue-1234"), "trace-7f3a");
    let (_, on_host) = git(&host, &["rev-parse", &format!("refs/agentbound/{SESSION}/fix-issue-1234")]);
    result("GS-3.staging-ref-push-permitted", ok && on_host == head, &format!("{out}; host now has {} at {}", format!("refs/agentbound/{SESSION}/fix-issue-1234"), &on_host[..12]));
    let denials = [
        ("refs/heads/main", "trace-7f3a", "main"),
        ("refs/heads/feature-x", "trace-7f3a", "any other branch"),
        (&*format!("refs/agentbound/{OTHER}/fix"), "trace-7f3a", "another session's namespace"),
        (&*format!("refs/agentbound/{SESSION}/fix"), "trace-0b19", "forged/mismatched trace"),
        (&*format!("refs/agentbound/{SESSION}/../../heads/main"), "trace-7f3a", "dot-dot traversal"),
        (&*format!("refs/agentbound/{SESSION}/"), "trace-7f3a", "empty tail"),
        (&*format!("refs/agentbound/{SESSION}/x:refs/heads/main"), "trace-7f3a", "embedded refspec colon"),
        (&*format!("+refs/agentbound/{SESSION}/x"), "trace-7f3a", "force marker"),
        (&*format!("refs/agentbound/{SESSION}/a.lock"), "trace-7f3a", ".lock suffix"),
        (&*format!("refs/agentbound/{SESSION}/a b"), "trace-7f3a", "whitespace"),
        ("refs/tags/v1", "trace-7f3a", "tag"),
        ("HEAD", "trace-7f3a", "HEAD"),
    ];
    let mut all = true; let mut det = vec![];
    for (r, t, why) in denials { let (ok, out) = push(r, t); if ok { all = false; } det.push(format!("{why}: {}", if ok { "ACCEPTED (!)".into() } else { out.replace("gateway denied: ", "") })); }
    result("GS-4.all-non-staging-and-forged-pushes-denied-by-gateway", all, &format!("{} cases: {}", det.len(), det.join(" | ")));
    let (_, main_now) = git(&host, &["rev-parse", "refs/heads/main"]);
    result("GS-5.main-unchanged", main_now == seed, &format!("refs/heads/main still {}", &seed[..12]));
    // --- protected-branch assumption composes: bypass the gateway policy and push main directly → host hook refuses ---
    // force so the client-side fast-forward check is out of the way and the host hook is what decides
    let (ok, out) = git(&gw, &["push", "--force", &host, &format!("refs/incoming/{SESSION}:refs/heads/main")]);
    let (_, main_after_bypass) = git(&host, &["rev-parse", "refs/heads/main"]);
    result("GS-6.host-protected-branch-refuses-even-if-gateway-bypassed", !ok && out.contains("protected branch") && main_after_bypass == seed, &format!("direct push to main: ok={ok}; {}", out.lines().find(|l| l.contains("protected")).unwrap_or(&out)));
    // --- session cannot reach the credential or the host repo: run a push as the session UID with no remote ---
    let sess_push = Command::new("setpriv").args(["--reuid=200042", "--regid=200042", "--clear-groups", "git", "-C", &ws, "push", &host, "HEAD:refs/heads/main"]).env_clear().env("PATH", "/usr/bin:/bin").output().unwrap();
    let sess_cred = Command::new("setpriv").args(["--reuid=200042", "--regid=200042", "--clear-groups", "cat", &cred]).output().unwrap();
    // note: in the real system the host repo is not on the session's filesystem at all (mount-construct); here it is reachable by path, so this tests the hook + credential ACL only
    result("GS-7.session-uid-cannot-read-gateway-credential", !sess_cred.status.success(), &format!("cat gateway-cred as 200042: exit {:?}; direct push as session (path reachable in this spike only) → host hook: {}", sess_cred.status.code(), String::from_utf8_lossy(&sess_push.stderr).lines().find(|l| l.contains("protected") || l.contains("denied") || l.contains("fatal")).unwrap_or("").trim()));
    // --- second push to same staging ref: non-fast-forward within own namespace ---
    fs::write(format!("{ws}/fix.txt"), "rewritten\n").unwrap(); git(&ws, &["-c", "user.name=agent", "-c", "user.email=agent@session", "commit", "-q", "-a", "--amend", "-m", "fix issue 1234 (amended)"]);
    git(&ws, &["bundle", "create", &bundle, &format!("{seed}..HEAD"), "HEAD"]); git(&gw, &["fetch", "-q", &bundle, &format!("+HEAD:refs/incoming/{SESSION}")]);
    let (ok_nff, out_nff) = push(&format!("refs/agentbound/{SESSION}/fix-issue-1234"), "trace-7f3a");
    result("GS-8.non-fast-forward-to-own-staging-ref-rejected-without-explicit-force", !ok_nff && out_nff.contains("non-fast-forward") , &format!("amended history push: ok={ok_nff}; {} — the gateway decides whether `force` is a distinct, separately-authorised operation (policy question for WP2, mechanism refuses by default)", out_nff.lines().find(|l| l.contains("rejected") || l.contains("non-fast")).unwrap_or("").trim()));
    let _ = fs::remove_dir_all(ROOT);
    println!("done");
}
