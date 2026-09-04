//! agentbound-lifecycle: the one privileged long-running daemon (session lifecycle §4).
pub mod service;
pub mod session;
pub mod state;
pub mod store;

use ab_common::sig::Keyring;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str, d: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned().unwrap_or_else(|| d.to_string());
    let socket = arg("--socket", "/run/agentbound/lifecycle.sock");
    let db = arg("--store", "/var/lib/agentbound/lifecycle.db");
    let keyring = Keyring::parse(&std::fs::read(arg("--keyring", "/etc/agentbound/keyring.json")).expect("keyring")).expect("keyring parse");
    let cli_uids: Vec<u32> = arg("--cli-uids", "").split(',').filter_map(|s| s.parse().ok()).collect();
    let host_id = ab_common::audit::host_id(); let boot_id = ab_common::audit::boot_id();
    let store = store::Store::open(&db, store::Range::default(), &host_id, &boot_id).expect("store open (fail closed on chain/range error)");
    let mut svc = service::Service { store, cfg: service::Config { cli_uids, keyring, host_id, boot_id, launch_version_digest: String::new(), managed_paths: arg("--managed-paths", "/var/lib/agentbound/sessions,/var/lib/agentbound").split(',').map(str::to_string).collect() }, sessions: state::Sessions::default(), audit: ab_common::audit::Sink::open(&arg("--audit-spool", "/var/lib/agentbound/audit-lifecycle.jsonl")) };
    svc.reconcile_on_start();
    let listener = ab_common::wire::listen(&socket, 0o660).expect("listen");
    loop {
        match ab_common::wire::accept(&listener) { Ok(c) => svc.serve(c), Err(e) => eprintln!("accept: {e}") }
        svc.poll_sessions();
    }
}
