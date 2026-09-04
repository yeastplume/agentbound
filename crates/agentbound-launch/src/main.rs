//! agentbound-launch: short-lived privileged constructor (session lifecycle §3).
//! Invoked by the CLI (via lifecycle-approved path) or directly by root:
//!   agentbound-launch --authorization <id> [--fault <name>] [options]
pub mod child;
pub mod construct;
pub mod sys;

use ab_common::json::{self, Value, MANIFEST_LIMITS};
use ab_common::sig::{Keyring, Signer_};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str, d: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned().unwrap_or_else(|| d.to_string());
    let az = arg("--authorization", ""); if az.is_empty() { eprintln!("--authorization required"); std::process::exit(2); }
    let keyring = Keyring::parse(&std::fs::read(arg("--keyring", "/etc/agentbound/keyring.json")).expect("keyring")).expect("keyring parse");
    let signer = Signer_::from_seed(&arg("--key-id", "key:launch-ed25519-01"), &std::fs::read(arg("--key", "/etc/agentbound/launch.key")).expect("launch key")).expect("key");
    let catalogue = json::parse(&std::fs::read(arg("--catalogue", "/etc/agentbound/catalogue.json")).expect("catalogue"), &MANIFEST_LIMITS).expect("catalogue parse");
    let self_digest = std::fs::read("/proc/self/exe").map(|b| ab_common::sig::sha256_hex(&b)).unwrap_or_default();
    let policy_uid = arg("--policy-uid", "").parse().unwrap_or_else(|_| uid_of("agentbound-policy").unwrap_or(0));
    let mut cfg = construct::Config { spool: arg("--spool", "/var/lib/agentbound/spool"), lease_dir: arg("--lease-dir", "/run/agentbound/leases"), session_root: arg("--session-root", "/var/lib/agentbound/sessions"),
        lifecycle_sock: arg("--lifecycle-socket", "/run/agentbound/lifecycle.sock"), keyring, signer, catalogue, image_base: arg("--image-base", "/var/lib/agentbound/images"), host_id: ab_common::audit::host_id(), boot_id: ab_common::audit::boot_id(),
        self_digest, audit: ab_common::audit::Sink::open(&arg("--audit-spool", "/var/lib/agentbound/audit-launch.jsonl")), policy_uid, fault: args.iter().position(|a| a == "--fault").and_then(|i| args.get(i + 1)).cloned() };
    let _ = std::fs::create_dir_all(&cfg.lease_dir);
    let mut led = construct::Ledger::default();
    match construct::construct(&mut cfg, &az, &mut led) {
        Ok(v) => { println!("{}", String::from_utf8_lossy(&json::canonical(&v))); }
        Err(f) => { let d = construct::rollback(&mut cfg, &az, &mut led, &f); eprintln!("{}", String::from_utf8_lossy(&json::canonical(&Value::obj(vec![("construction_failed", d)])))); std::process::exit(1); }
    }
}
fn uid_of(name: &str) -> Option<u32> { std::fs::read_to_string("/etc/passwd").ok()?.lines().find(|l| l.starts_with(&format!("{name}:")))?.split(':').nth(2)?.parse().ok() }
