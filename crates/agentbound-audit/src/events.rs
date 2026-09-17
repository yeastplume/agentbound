//! Closed event schema (R-AUD-1 fields + per-kind detail member sets). Unknown kind or member ⇒ reject.
use ab_common::json::Value;

const BASE: [&str; 17] = ["actor", "allocation_id", "authorization_id", "boot_id", "clock_source", "detail", "event", "event_id", "execution_uid", "host_id", "launch_record_digest", "monotonic_ns", "outcome", "session_id", "trace_id", "wall_clock", "wall_clock_trusted"];

fn detail_members(kind: &str) -> Option<&'static [&'static str]> {
    Some(match kind {
        "session.requested" => &["peer_uid", "request_digest"], "session.rejected" => &["failed_input", "request_digest"], "session.authorized" => &["manifest_digest"],
        "session.manifest_verified" => &["key_id", "manifest_digest"], "identity.allocated" => &["gid", "state_seq"],
        "session.launch_record_committed" => &["commit_seq", "manifest_digest", "trust_anchor"], "session.activated" => &["event", "privilege_disposal", "runtime_artifact_digest"],
        "session.construction_failed" => &["detail", "failed_step", "ledger", "rollback", "rule"],
        "session.revocation_received" => &["behaviour", "source", "trigger"], "session.quiesce_started" => &["admission", "bound_s", "freeze_requested", "trigger"],
        "session.degraded" => &["compensating_control", "remaining_authority", "trigger"], "session.termination_started" => &["bound_s", "ordering_deviation", "reason", "scope_id"],
        "session.terminated" | "session.termination_incomplete" => &["cgroup_kill_written", "cgroup_procs_remaining", "credential_scan_inside_scope", "credential_scan_outside_scope", "d_state", "elapsed_ms", "freeze_written", "frozen_observed", "gateway_admission_denied", "init_pid", "init_pidfd_exited", "sigterm_sent"],
        "identity.scope_escape_suspected" => &["pids", "uid"], "session.cleanup_completed" => &["acl_entries_removed", "acl_removal_failures", "grants", "ipc_namespace", "residue", "unmounts"],
        "session.identity_released" => &["allocation_id", "quarantine_state_seq", "reclamation_proof"], "session.sealed" => &["seal_seq", "termination_reason"],
        "session.recovery_reconciled" => &["cgroup_live", "credential_scan_inside", "credential_scan_outside", "identity_state", "scope_id"],
        "session.ownership_projected" => &["bytes", "failed", "files", "storage_principal"],
        "audit.self_test" => &["note"],
        // gateway (ADR-0002 Decision 5): connection, operation, denial, revocation and close records
        "gateway.reconstructed" => &["projections", "stale_descriptors_dropped"], "gateway.projected" => &["socket_type", "topology"], "gateway.grants_loaded" => &["operations", "source"],
        "gateway.admission_denied" => &["reason"], "gateway.released" => &["connections_closed"],
        "gateway.budget_persist_retried" => &["attempts", "deadline_ms"],
        "gateway.socket_removal_failed" => &["path_digest"],
        "gateway.connection_established" => &["cgroup", "establishing_pid", "pidfd", "pidfs_inode", "pidns", "start_time", "uid"],
        "gateway.connection_refused" => &["detail", "peer_pid", "peer_uid", "rule"], "gateway.connection_closed" => &["establishing_pid", "operations", "reason"],
        "gateway.process_mismatch" | "gateway.descriptor_transfer_rejected" | "gateway.packet_rejected" => &["class", "credential_pid", "detail", "establishing_pid", "rule"],
        "gateway.operation_admitted" => &["credential_pid", "idempotency_key", "operation", "operation_seq", "payload_bytes", "pidfs_inode"],
        "gateway.operation_completed" => &["idempotency_key", "operation", "operation_seq", "result"],
        // a repeated idempotency key returns the original outcome without repeating the action (component-interfaces §5); the replay
        // is recorded so a reconstruction can tell one intended effect from its retries (test-catalogue §5 dedup rule)
        "gateway.operation_replayed" => &["idempotency_key", "operation", "operation_seq"],
        "gateway.upstream_rejected" => &["detail", "operation", "operation_seq", "rule"],
        "gateway.operation_denied" => &["class", "credential_pid", "detail", "establishing_pid", "idempotency_key", "operation", "operation_seq", "rule"],
        _ => return None,
    })
}
pub fn check(ev: &Value) -> Result<(), &'static str> {
    let m = ev.as_obj().ok_or("event_not_object")?;
    if m.len() != BASE.len() { return Err("event_member_count"); }
    for k in m.keys() { if !BASE.contains(&k.0.as_str()) { return Err("unknown_event_member"); } }
    // The receiver reads this string after schema validation; malformed input must not panic there.
    ev.get("event_id").and_then(|x| x.as_str()).ok_or("event_id")?;
    let kind = ev.get("event").and_then(|x| x.as_str()).ok_or("event")?;
    let want = detail_members(kind).ok_or("unknown_event_kind")?;
    let d = ev.get("detail").and_then(|x| x.as_obj()).ok_or("detail_not_object")?;
    for k in d.keys() { if !want.contains(&k.0.as_str()) { return Err("unknown_detail_member"); } }
    for k in want { if d.keys().all(|x| x.0 != *k) { return Err("missing_detail_member"); } }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn self_test() -> Value {
        let mut ev = ab_common::audit::event("audit.self_test", "agentbound-audit", "ok", &Default::default(),
            Value::obj(vec![("note", Value::s("receiver contract"))]));
        // Sink hashes the envelope without event_id; never use a fabricated identifier in a valid fixture.
        ev.as_obj_mut().unwrap().retain(|k, _| k.0 != "event_id");
        ev.set("event_id", Value::s(&ab_common::sig::object_digest(&ev)));
        ev
    }

    #[test]
    fn rejects_non_string_event_id_without_receiver_panic() {
        let ev = self_test();
        assert_eq!(check(&ev), Ok(()));
        for id in [Value::Null, Value::Int(1), Value::Bool(true), Value::Arr(vec![]), Value::obj(vec![])] {
            let mut malformed = ev.clone();
            malformed.set("event_id", id);
            assert_eq!(check(&malformed), Err("event_id"));
        }
    }

    #[test]
    fn envelope_and_details_remain_closed() {
        let ev = self_test();
        let mut unknown = ev.clone();
        unknown.set("event", Value::s("gateway.arbitrary_event"));
        assert_eq!(check(&unknown), Err("unknown_event_kind"));
        let mut extra = ev.clone();
        extra.set("unexpected", Value::Null);
        assert_eq!(check(&extra), Err("event_member_count"));
        let mut renamed = ev.clone();
        renamed.as_obj_mut().unwrap().retain(|k, _| k.0 != "trace_id");
        renamed.set("unexpected", Value::Null);
        assert_eq!(check(&renamed), Err("unknown_event_member"));
        for key in ev.as_obj().unwrap().keys() {
            let mut missing = ev.clone();
            missing.as_obj_mut().unwrap().retain(|k, _| k != key);
            assert_eq!(check(&missing), Err("event_member_count"), "{}", key.0);
        }
        let mut missing = ev.clone();
        missing.set("detail", Value::obj(vec![]));
        assert_eq!(check(&missing), Err("missing_detail_member"));
        let mut extra = ev.clone();
        extra.set("detail", Value::obj(vec![("note", Value::Null), ("unexpected", Value::Null)]));
        assert_eq!(check(&extra), Err("unknown_detail_member"));
        let mut not_object = ev;
        not_object.set("detail", Value::Null);
        assert_eq!(check(&not_object), Err("detail_not_object"));
    }
}
