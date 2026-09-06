//! Per-connection packet handling (ADR-0002 D2 per-operation evidence, D3 authorization).
use crate::{adapters, Gateway};
use ab_common::json::{self, Value};
use ab_common::wire::{self, ProcInstance};
use std::os::fd::{AsRawFd, OwnedFd};

pub const MAX_PACKET: usize = 128 * 1024; // measured: 256 KiB passes, 1 MiB EMSGSIZE on the baseline
pub const PROTOCOL: &str = "agentbound.gateway.v0.1";

pub struct Pending { pub op: Value, pub op_seq: i64, pub expect_len: usize, pub sha: String, pub buf: Vec<u8>, pub idem: String }
pub struct Conn { pub last_idem: String, pub fd: OwnedFd, pub pidfd: OwnedFd, pub inst: ProcInstance, pub allocation_id: String, pub uid: u32, pub gid: u32, pub ops: u64, pub last_cred_pid: i32, pub pending: Option<Pending> }
impl Conn { pub fn describe(&self) -> Value { Value::obj(vec![("cgroup", Value::s(&self.inst.cgroup)), ("establishing_pid", Value::Int(self.inst.pid as i64)), ("pidfd", Value::s("acquired")), ("pidfs_inode", Value::Int(self.inst.pidfs_ino as i64)), ("pidns", Value::Int(self.inst.pidns as i64)), ("start_time", Value::Int(self.inst.start_time as i64)), ("uid", Value::Int(self.uid as i64))]) } }

pub type Deny = (&'static str, &'static str, String, bool); // class, rule, detail, close

/// Compare the establishing process instance with the instance now behind the credential PID. `None` = same instance.
/// Keyed on the pidfs inode (never reused within a boot), with pidns and start time corroborating: a recycled PID therefore
/// fails here even when the pid, uid and cgroup all match. Pure so it can be tested without a live peer.
pub fn instance_mismatch(est: &ProcInstance, now: &ProcInstance) -> Option<String> {
    if now.pidfs_ino != est.pidfs_ino { return Some(format!("pidfs inode {} vs establishing {} (pid {} recycled to another process instance)", now.pidfs_ino, est.pidfs_ino, est.pid)); }
    if now.pidns != est.pidns { return Some(format!("pidns {} vs establishing {}", now.pidns, est.pidns)); }
    if now.start_time != est.start_time { return Some(format!("start time {} vs establishing {} (inode equal — kernel invariant violated)", now.start_time, est.start_time)); }
    None
}

/// Every packet: exactly one kernel credential, no descriptors, same process instance as establishment.
pub fn handle(gw: &mut Gateway, i: usize, pk: wire::Packet) -> Result<(), Deny> {
    if pk.rights_fds > 0 { return Err((wire::CLASS_INVALID, "descriptor_transfer", format!("{} descriptors", pk.rights_fds), true)); }
    if pk.creds.len() != 1 { return Err((wire::CLASS_INVALID, "credential_count", format!("{} SCM_CREDENTIALS", pk.creds.len()), true)); }
    if pk.truncated { return Err((wire::CLASS_INVALID, "oversize_packet", String::new(), true)); }
    let cred = pk.creds[0].clone(); gw.conns[i].last_cred_pid = cred.pid;
    let est = gw.conns[i].inst.clone();
    if cred.pid != est.pid || cred.uid != gw.conns[i].uid { return Err((wire::CLASS_UNAUTHENTICATED, "process_mismatch", format!("credential pid {} uid {} vs establishing {} {}", cred.pid, cred.uid, est.pid, gw.conns[i].uid), true)); }
    let (_pf, now) = wire::proc_instance(cred.pid).map_err(|_| (wire::CLASS_UNAUTHENTICATED, "process_mismatch", "credential process gone".to_string(), true))?;
    // The instance comparison is the PID-reuse defence (T-6.4-009, WP1 F-1): the pid may be identical after recycling, so the
    // pidfs inode — unique per process instance, never reused — decides, with pidns and start time corroborating.
    if let Some(d) = instance_mismatch(&est, &now) { return Err((wire::CLASS_UNAUTHENTICATED, "process_mismatch", d, true)); }
    // per-operation status re-check (D4): the record must still admit operations
    let aid = gw.conns[i].allocation_id.clone();
    let adm = gw.by_alloc.get(&aid).map(|p| p.admission).unwrap_or(false);
    if !adm { return Err((wire::CLASS_UNAUTHORIZED, "admission_closed", "session not admitting operations".into(), false)); }
    // payload chunk for a pending operation
    if gw.conns[i].pending.is_some() {
        let (got, want) = { let pend = gw.conns[i].pending.as_mut().unwrap();
            if pend.buf.len() + pk.bytes.len() > pend.expect_len { gw.conns[i].pending = None; return Err((wire::CLASS_INVALID, "payload_overrun", String::new(), false)); }
            pend.buf.extend_from_slice(&pk.bytes); (pend.buf.len(), pend.expect_len) };
        if got < want { return reply(gw, i, wire::reply_ok(Value::obj(vec![("received", Value::Int(got as i64))]))); }
        let pend = gw.conns[i].pending.take().unwrap();
        if ab_common::sig::sha256_hex(&pend.buf) != pend.sha { return Err((wire::CLASS_INVALID, "payload_digest", String::new(), false)); }
        return execute(gw, i, pend.op, pend.op_seq, Some(pend.buf), pend.idem);
    }
    let v = json::parse_canonical(&pk.bytes, &json::REQUEST_LIMITS).map_err(|e| (wire::CLASS_INVALID, "parse", e.to_string(), false))?;
    if v.get("v").and_then(|x| x.as_str()) != Some(PROTOCOL) { return Err((wire::CLASS_INVALID, "version", String::new(), false)); }
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);
    let (Some(op), Some(opid)) = (s("operation"), s("operation_id")) else { return Err((wire::CLASS_INVALID, "envelope", "operation, operation_id".into(), false)) };
    // component-interfaces §5: every request carries an idempotency key scoped to (caller identity, operation, target record). The
    // gateway's own scope is (allocation, operation_id, key). It is the workload's handle on the effect, so it is also what makes
    // the audit record correlatable to the workload's own log (test-catalogue §5 requires the class, outcome AND key to match).
    let idem = v.get("idempotency_key").and_then(|x| x.as_str()).filter(|k| !k.is_empty() && k.len() <= 128)
        .ok_or((wire::CLASS_INVALID, "envelope", "idempotency_key (1-128 bytes)".to_string(), false))?.to_string();
    // stash it before any check that can refuse: a denial is an in-scope effect and must carry the workload's key too
    gw.conns[i].last_idem = idem.clone();
    // a repeated key with an identical operation returns the original outcome; with a different one it is a conflict
    if let Some((prev_op, prev_seq, prev_reply)) = gw.by_alloc[&aid].idem.get(&(opid.clone(), idem.clone())).cloned() {
        if prev_op != op { return Err((wire::CLASS_CONFLICT, "idempotency_conflict", format!("key already used for {prev_op}"), false)); }
        gw.emit("gateway.operation_replayed", "ok", &Gateway::corr(&gw.by_alloc[&aid]),
            Value::obj(vec![("idempotency_key", Value::s(&idem)), ("operation", Value::s(&op)), ("operation_seq", Value::Int(prev_seq))]));
        return reply(gw, i, prev_reply);
    }
    // D3: the operation must be granted by the committed record; caller-supplied trace ids are ignored
    let granted = gw.by_alloc[&aid].ops.iter().any(|o| o.get("operation_id").and_then(|x| x.as_str()) == Some(&opid) && o.get("operation").and_then(|x| x.as_str()) == Some(&op));
    if !granted { return Err((wire::CLASS_UNAUTHORIZED, "operation_not_granted", format!("{opid} {op}"), false)); }
    let budgets = gw.by_alloc[&aid].ops.iter().find(|o| o.get("operation_id").and_then(|x| x.as_str()) == Some(&opid)).and_then(|o| o.get("budgets").cloned()).unwrap_or(Value::obj(vec![]));
    // R-GW-7 budgets, per operation id: `operations` (count), `bytes_per_operation` (largest single payload), `bytes` (total payload
    // across the session). All three are checked BEFORE the operation is counted, from durable counters (see Gateway::persist_budget).
    let per_op = budgets.get("bytes_per_operation").and_then(|x| x.as_int()).unwrap_or(8 << 20) as usize;
    let max_ops = budgets.get("operations").and_then(|x| x.as_int()).unwrap_or(i64::MAX) as u64;
    let max_bytes = budgets.get("bytes").and_then(|x| x.as_int()).map(|b| b as u64);
    let (used_ops, used_bytes) = gw.by_alloc[&aid].used.get(&opid).copied().unwrap_or((0, 0));
    if used_ops >= max_ops { return Err((wire::CLASS_UNAUTHORIZED, "budget_operations", format!("{used_ops} of {max_ops} used"), false)); }
    let plen = v.get("payload_len").and_then(|x| x.as_int()).unwrap_or(0) as usize;
    if plen > per_op { return Err((wire::CLASS_UNAUTHORIZED, "budget_bytes", format!("{plen} > {per_op} per operation"), false)); }
    if let Some(mb) = max_bytes { if used_bytes + plen as u64 > mb { return Err((wire::CLASS_UNAUTHORIZED, "budget_bytes", format!("{used_bytes}+{plen} > {mb} total"), false)); } }
    { let p = gw.by_alloc.get_mut(&aid).unwrap(); p.op_count += 1; let e = p.used.entry(opid.clone()).or_insert((0, 0)); e.0 += 1; e.1 += plen as u64; p.bytes_used += plen as u64; }
    gw.conns[i].ops += 1;
    // consumption is durable before the operation proceeds; if it cannot be recorded the operation is refused and admission closes
    if !gw.persist_budget(&aid) { return Err((wire::CLASS_UNAVAILABLE, "budget_persist", "consumption could not be recorded".into(), false)); }
    let op_seq = gw.by_alloc[&aid].op_count as i64;
    if plen > 0 {
        let Some(sha) = s("payload_sha256") else { return Err((wire::CLASS_INVALID, "envelope", "payload_sha256".into(), false)) };
        gw.conns[i].pending = Some(Pending { op: v.clone(), op_seq, expect_len: plen, sha, buf: Vec::with_capacity(plen), idem: idem.clone() });
        return reply(gw, i, wire::reply_ok(Value::obj(vec![("awaiting_payload", Value::Int(plen as i64)), ("operation_seq", Value::Int(op_seq))])));
    }
    execute(gw, i, v, op_seq, None, idem)
}

fn execute(gw: &mut Gateway, i: usize, op: Value, op_seq: i64, payload: Option<Vec<u8>>, idem: String) -> Result<(), Deny> {
    let aid = gw.conns[i].allocation_id.clone();
    let cr = Gateway::corr(&gw.by_alloc[&aid]);
    let name = op.get("operation").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let inst = gw.conns[i].inst.clone(); let bytes = payload.as_ref().map(|p| p.len()).unwrap_or(0);
    gw.emit("gateway.operation_admitted", "ok", &cr, Value::obj(vec![("credential_pid", Value::Int(inst.pid as i64)), ("idempotency_key", Value::s(&idem)), ("operation", Value::s(&name)), ("operation_seq", Value::Int(op_seq)), ("payload_bytes", Value::Int(bytes as i64)), ("pidfs_inode", Value::Int(inst.pidfs_ino as i64))]));
    let trace = gw.by_alloc[&aid].record.as_ref().and_then(|b| b.get("authorization_manifest")).and_then(|b| b.get("session_trace")).and_then(|b| b.get("trace_id")).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let session_id = gw.by_alloc[&aid].record.as_ref().and_then(|b| b.get("authorization_manifest")).and_then(|b| b.get("session_trace")).and_then(|b| b.get("session_id")).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let res = adapters::run(gw, &aid, &name, &op, payload.as_deref(), &session_id, &trace);
    match res {
        Ok(body) => { gw.emit("gateway.operation_completed", "ok", &cr, Value::obj(vec![("idempotency_key", Value::s(&idem)), ("operation", Value::s(&name)), ("operation_seq", Value::Int(op_seq)), ("result", body.clone())]));
            let r = wire::reply_ok(Value::obj(vec![("operation_seq", Value::Int(op_seq)), ("result", body), ("trace_id", Value::s(&trace))]));
            if let Some(p) = gw.by_alloc.get_mut(&aid) { p.idem.insert((op.get("operation_id").and_then(|x| x.as_str()).unwrap_or("").to_string(), idem.clone()), (name.clone(), op_seq, r.clone())); }
            reply(gw, i, r) }
        Err((rule, detail)) => { let kind = if rule == "upstream_rejected" { "gateway.upstream_rejected" } else { "gateway.operation_denied" }; gw.emit(kind, "deny", &cr, if kind == "gateway.upstream_rejected" { Value::obj(vec![("detail", Value::s(&detail)), ("operation", Value::s(&name)), ("operation_seq", Value::Int(op_seq)), ("rule", Value::s(rule))]) } else { Value::obj(vec![("class", Value::s(wire::CLASS_UNAUTHORIZED)), ("credential_pid", Value::Int(inst.pid as i64)), ("detail", Value::s(&detail)), ("establishing_pid", Value::Int(inst.pid as i64)), ("idempotency_key", Value::s(&idem)), ("operation", Value::s(&name)), ("operation_seq", Value::Int(op_seq)), ("rule", Value::s(rule))]) }); reply(gw, i, wire::reply_err(wire::CLASS_UNAUTHORIZED, rule, &detail)) }
    }
}

fn reply(gw: &mut Gateway, i: usize, v: Value) -> Result<(), Deny> { wire::send_raw(gw.conns[i].fd.as_raw_fd(), &json::canonical(&v)).map_err(|e| (wire::CLASS_INTERNAL, "send", e.to_string(), true)) }

#[cfg(test)]
mod tests {
    use super::*;
    fn inst(pid: i32, ino: u64, start: u64, ns: u64) -> ProcInstance { ProcInstance { pid, pidfs_ino: ino, start_time: start, pidns: ns, cgroup: "/system.slice/agentbound-a.scope".into() } }
    #[test]
    fn same_instance_accepted() { assert!(instance_mismatch(&inst(42, 700, 900, 4), &inst(42, 700, 900, 4)).is_none()); }
    #[test]
    fn recycled_pid_rejected() {
        // identical pid, uid, cgroup and even start time: only the pidfs inode differs. This is the same-tick PID-reuse case.
        let d = instance_mismatch(&inst(42, 700, 900, 4), &inst(42, 701, 900, 4)).expect("recycled pid must be rejected");
        assert!(d.contains("pidfs inode 701 vs establishing 700"), "{d}");
    }
    #[test]
    fn pidns_change_rejected() { assert!(instance_mismatch(&inst(42, 700, 900, 4), &inst(42, 700, 900, 5)).is_some()); }
    #[test]
    fn start_time_change_rejected() { assert!(instance_mismatch(&inst(42, 700, 900, 4), &inst(42, 700, 901, 4)).is_some()); }
}
