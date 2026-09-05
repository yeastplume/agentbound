//! agentbound-lifecycle: the one privileged long-running daemon (session lifecycle §4).
pub mod service;
pub mod session;
pub mod state;
pub mod store;

use ab_common::sig::Keyring;

/// Registered mount sources from the catalogue (`base/relative`): the durable-projection roots.
fn workspace_roots(cat: &str) -> Vec<String> {
    let Ok(b) = std::fs::read(cat) else { return vec![] }; let Ok(v) = ab_common::json::parse(&b, &ab_common::json::MANIFEST_LIMITS) else { return vec![] };
    v.get("mount_sources").and_then(|m| m.as_obj()).map(|m| m.values().filter_map(|s| Some(format!("{}/{}", s.get("base")?.as_str()?, s.get("relative")?.as_str()?))).collect()).unwrap_or_default()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str, d: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned().unwrap_or_else(|| d.to_string());
    let socket = arg("--socket", "/run/agentbound/lifecycle.sock");
    let db = arg("--store", "/var/lib/agentbound/lifecycle.db");
    let keyring = Keyring::parse(&std::fs::read(arg("--keyring", "/etc/agentbound/keyring.json")).expect("keyring")).expect("keyring parse");
    let cli_uids: Vec<u32> = arg("--cli-uids", "").split(',').filter_map(|s| s.parse().ok()).collect();
    let host_id = ab_common::audit::host_id(); let boot_id = ab_common::audit::boot_id();
    let store = store::Store::open(&db, store::Range::default(), &host_id, &boot_id).expect("store open (fail closed on chain/range error)");
    let mut svc = service::Service { store, cfg: service::Config { cli_uids, keyring, host_id, boot_id, launch_version_digest: String::new(), managed_paths: arg("--managed-paths", "/var/lib/agentbound/sessions,/var/lib/agentbound").split(',').map(str::to_string).collect(), workspace_roots: workspace_roots(&arg("--catalogue", "/etc/agentbound/catalogue.json")), gateway_uid: arg("--gateway-uid", "").parse().ok(), gateway_sock: arg("--gateway-socket", "/run/agentbound/gateway.sock") }, sessions: state::Sessions::default(), audit: ab_common::audit::Sink::open(&arg("--audit-spool", "/var/lib/agentbound/audit-lifecycle.jsonl")) };
    svc.reconcile_on_start();
    let listener = ab_common::wire::listen(&socket, 0o660).expect("listen");
    // accept with a bounded wait so the pidfd/deadline poll runs on its own cadence, not only when a client connects
    use std::os::fd::AsRawFd;
    loop {
        let mut pfd = libc::pollfd { fd: listener.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        let ready = unsafe { libc::poll(&mut pfd, 1, 250) };
        if ready > 0 { match ab_common::wire::accept(&listener) { Ok(c) => svc.serve(c), Err(e) => eprintln!("accept: {e}") } }
        svc.poll_sessions();
    }
}
