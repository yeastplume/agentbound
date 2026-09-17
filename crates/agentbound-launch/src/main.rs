//! agentbound-launch: short-lived privileged constructor (session lifecycle §3).
//! The installed sudo entry accepts only --authorization <id>. Configuration
//! and fault options are for direct root administration, never delegated sudo.
pub mod child;
pub mod construct;
pub mod sys;
mod entry;

use ab_common::json::{self, Value, MANIFEST_LIMITS};
use ab_common::sig::{Keyring, Signer_};

fn main() {
    if let Err(e) = run() {
        eprintln!("agentbound-launch: {e}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    // Informational only, including for non-root callers. Never ignore trailing
    // arguments (in particular, do not hide a forbidden configuration override).
    if args.len() == 1 && args[0] == "--provenance" {
        println!("commit={} dirty={} tree={}", ab_common::provenance::COMMIT, ab_common::provenance::DIRTY, ab_common::provenance::TREE);
        return Ok(());
    }
    let env: Vec<_> = std::env::vars_os().collect();
    let mode = entry::invocation(entry::kernel_uids()?, &env)?;
    let args = entry::LaunchArgs::parse(args, mode)?;
    let clean_env = entry::launch_environment(mode, &env)?;
    // SECURITY: no configuration reads, audit opens, directory creation, or
    // calls into construct/rollback may precede the complete gate above.
    if mode == entry::Invocation::Sudo { entry::install_environment(clean_env); }
    let policy_uid = args.policy_uid()?;
    let read = |key| std::fs::read(args.get(key)).map_err(|e| format!("{key}: {e}"));
    let keyring = Keyring::parse(&read("--keyring")?).map_err(|e| format!("keyring parse: {e}"))?;
    let signer = Signer_::from_seed(args.get("--key-id"), &read("--key")?).map_err(|e| format!("launch key: {e}"))?;
    let catalogue = json::parse(&read("--catalogue")?, &MANIFEST_LIMITS).map_err(|e| format!("catalogue parse: {e}"))?;
    let self_digest = std::fs::read("/proc/self/exe").map(|b| ab_common::sig::sha256_hex(&b)).unwrap_or_default();
    let az = &args.authorization;
    let mut cfg = construct::Config {
        spool: args.get("--spool").into(), lease_dir: args.get("--lease-dir").into(), session_root: args.get("--session-root").into(),
        lifecycle_sock: args.get("--lifecycle-socket").into(), gateway_sock: args.get("--gateway-socket").into(), keyring, signer, catalogue,
        image_base: args.get("--image-base").into(), host_id: ab_common::audit::host_id(), boot_id: ab_common::audit::boot_id(),
        self_digest, audit: ab_common::audit::Sink::open(args.get("--audit-spool")), policy_uid,
        fault: if args.get("--fault").is_empty() { None } else { Some(args.get("--fault").into()) },
    };
    let _ = std::fs::create_dir_all(&cfg.lease_dir);
    let mut led = construct::Ledger::default();
    match construct::construct(&mut cfg, az, &mut led) {
        Ok(v) => { println!("{}", String::from_utf8_lossy(&json::canonical(&v))); }
        Err(f) => {
            let d = construct::rollback(&mut cfg, az, &mut led, &f);
            eprintln!("{}", String::from_utf8_lossy(&json::canonical(&Value::obj(vec![("construction_failed", d)]))));
            std::process::exit(1);
        }
    }
    Ok(())
}
