//! Lifecycle SEQPACKET service (wire formats §3): request dispatch, peer
//! authorization, idempotency, and the constructor-facing operations.
//! Termination and observation live in `session.rs`.

use crate::store::{Store, StoreError};
use crate::state::Sessions;
use ab_common::audit::{self, Correlation};
use ab_common::json::{canonical, Value};
use ab_common::schema;
use ab_common::sig::{launch_record_digest, now_unix, object_digest, Keyring};
use ab_common::wire::{self, Conn, Req};
use std::os::fd::OwnedFd;

pub struct Config { pub cli_uids: Vec<u32>, pub keyring: Keyring, pub host_id: String, pub boot_id: String, pub launch_version_digest: String, pub managed_paths: Vec<String>, pub workspace_roots: Vec<String>, pub gateway_uid: Option<u32>, pub gateway_sock: String, pub storage_principals: Vec<(String, u32, u32)> }

pub struct Service { pub store: Store, pub cfg: Config, pub sessions: Sessions, pub audit: audit::Sink }

pub type Reply = Result<Value, (&'static str, &'static str, String)>;
fn err<T>(class: &'static str, rule: &'static str, detail: impl Into<String>) -> Result<T, (&'static str, &'static str, String)> { Err((class, rule, detail.into())) }
fn store_err(e: StoreError) -> (&'static str, &'static str, String) {
    match e { StoreError::Conflict(d) => (wire::CLASS_CONFLICT, "store_conflict", d), StoreError::Exhausted => (wire::CLASS_UNAVAILABLE, "identity_range_exhausted", String::new()),
        StoreError::Chain(d) => (wire::CLASS_UNAVAILABLE, "store_integrity", d), StoreError::Clock => (wire::CLASS_UNAVAILABLE, "clock_unavailable", String::new()), e => (wire::CLASS_INTERNAL, "store", e.to_string()) }
}
pub fn gs<'a>(b: &'a Value, k: &str) -> Result<&'a str, (&'static str, &'static str, String)> { b.get(k).and_then(|x| x.as_str()).ok_or((wire::CLASS_INVALID, "body_member", k.to_string())) }
fn closed(b: &Value, want: &[&str]) -> Result<(), (&'static str, &'static str, String)> {
    let m = b.as_obj().ok_or((wire::CLASS_INVALID, "body", String::new()))?;
    for k in m.keys() { if !want.contains(&k.0.as_str()) { return err(wire::CLASS_INVALID, "unknown_member", k.0.clone()); } }
    for k in want { if b.get(k).is_none() { return err(wire::CLASS_INVALID, "missing_member", *k); } }
    Ok(())
}

const CONSTRUCTOR_OPS: [&str; 5] = ["reserve_identity", "commit_binding", "register_session", "report_activation", "report_construction_failed"];
const OBSERVER_OPS: [&str; 5] = ["status", "list", "terminate", "quiesce", "revocation_signal"];
/// Gateway (ADR-0002 D4.7): reads only; reconstructs grants from the signed launch-record store.
const GATEWAY_OPS: [&str; 3] = ["status", "list", "record"];

impl Service {
    /// Handle one connection: one request, one reply (descriptors only on `register_session`).
    pub fn serve(&mut self, conn: Conn) {
        let (msg, fds) = match conn.recv_with_fds(4) { Ok(Some(x)) => x, _ => return };
        let reply = match self.dispatch(&conn, &msg, fds) { Ok(body) => wire::reply_ok(body), Err((c, r, d)) => wire::reply_err(c, r, &d) };
        let _ = conn.send(&reply);
    }

    fn dispatch(&mut self, conn: &Conn, msg: &Value, fds: Vec<OwnedFd>) -> Reply {
        let Req { op, idem, body } = wire::parse_request(msg).map_err(|e| (wire::CLASS_INVALID, "envelope", e.to_string()))?;
        let uid = conn.peer.uid;
        let allowed = (uid == 0 && (CONSTRUCTOR_OPS.contains(&op) || OBSERVER_OPS.contains(&op) || op == "record")) || (self.cfg.cli_uids.contains(&uid) && OBSERVER_OPS.contains(&op)) || (self.cfg.gateway_uid == Some(uid) && GATEWAY_OPS.contains(&op));
        if !allowed { return err(wire::CLASS_UNAUTHENTICATED, "peer_not_permitted", format!("uid {uid} may not call {op}")); }
        if op != "register_session" && !fds.is_empty() { return err(wire::CLASS_INVALID, "unexpected_descriptors", ""); }
        let scope = format!("uid{uid}:{op}"); let bd = object_digest(body);
        match self.store.idem_lookup(&scope, idem, &bd).map_err(store_err)? { Some(Ok(r)) => return Ok(r), Some(Err(())) => return err(wire::CLASS_CONFLICT, "idempotency_key_reused", "same key, different body"), None => {} }
        let r = match op {
            "reserve_identity" => self.reserve(body),
            "commit_binding" => self.commit_binding(body),
            "register_session" => self.register_session(body, fds),
            "report_activation" => self.report_activation(body),
            "report_construction_failed" => self.report_failed(body),
            "status" => self.status(body),
            "record" => self.record(body),
            "list" => self.list(),
            "terminate" | "quiesce" | "revocation_signal" => self.lifecycle_action(op, body, uid),
            _ => err(wire::CLASS_INVALID, "unknown_op", op),
        };
        if let Ok(v) = &r { let _ = self.store.idem_store(&scope, idem, &bd, v); }
        r
    }

    fn emit(&mut self, kind: &str, outcome: &str, c: &Correlation, detail: Value) { let ev = audit::event(kind, "agentbound-lifecycle", outcome, c, detail); self.audit.emit(&ev); }

    fn reserve(&mut self, b: &Value) -> Reply {
        closed(b, &["agent_global_id", "authorization_id", "authorization_manifest_digest", "reclamation_domain_id", "session_id", "trace_id"])?;
        let (az, dg) = (gs(b, "authorization_id")?, gs(b, "authorization_manifest_digest")?);
        if !ab_common::sig::is_digest(dg) { return err(wire::CLASS_INVALID, "digest_form", ""); }
        let a = self.store.reserve(az, dg, gs(b, "agent_global_id")?, gs(b, "session_id")?, gs(b, "trace_id")?, gs(b, "reclamation_domain_id")?, "agentbound-launch").map_err(store_err)?;
        let c = Correlation { authorization_id: Some(az.into()), allocation_id: Some(a.allocation_id.clone()), session_id: Some(a.session_id.clone()), trace_id: Some(a.trace_id.clone()), execution_uid: Some(a.uid), ..Default::default() };
        self.emit("identity.allocated", "ok", &c, Value::obj(vec![("gid", Value::Int(a.gid as i64)), ("state_seq", Value::Int(a.state_seq))]));
        Ok(Value::obj(vec![("allocation_id", Value::s(&a.allocation_id)), ("gids", Value::Arr(vec![Value::Int(a.gid as i64)])), ("state_seq", Value::Int(a.state_seq)), ("uid", Value::Int(a.uid as i64))]))
    }

    /// Verify both envelopes and schemas, the §3.1 correspondence, allocation match; append binding (commit point).
    fn commit_binding(&mut self, b: &Value) -> Reply {
        closed(b, &["allocation_id", "authorization_manifest", "envelope", "launch_binding", "manifest_envelope"])?;
        let aid = gs(b, "allocation_id")?;
        let (m, me, lb, le) = (b.get("authorization_manifest").unwrap(), b.get("manifest_envelope").unwrap(), b.get("launch_binding").unwrap(), b.get("envelope").unwrap());
        let mv = schema::validate_manifest(m).map_err(|e| (wire::CLASS_INVALID, "manifest_schema", e.to_string()))?;
        let bv = schema::validate_binding(lb).map_err(|e| (wire::CLASS_INVALID, "binding_schema", e.to_string()))?;
        let now = now_unix().map_err(|_| (wire::CLASS_UNAVAILABLE, "clock_unavailable", String::new()))?;
        let pm = ab_common::envelope::verify_policy(&self.cfg.keyring, m, me, mv.authorization_id, now).map_err(|e| (wire::CLASS_INVALID, "manifest_envelope", e.to_string()))?;
        let cb = ab_common::envelope::verify_constructor(&self.cfg.keyring, lb, le, mv.authorization_id, &pm.digest, &self.cfg.host_id, &self.cfg.boot_id, now).map_err(|e| (wire::CLASS_INVALID, "constructor_envelope", e.to_string()))?;
        schema::correspond(&mv, &bv, &pm.digest).map_err(|e| (wire::CLASS_INVALID, "correspondence", e.to_string()))?;
        let a = self.store.latest(aid).map_err(store_err)?.ok_or((wire::CLASS_CONFLICT, "unknown_allocation", String::new()))?;
        if a.state != "allocated" || a.authorization_id != mv.authorization_id || a.manifest_digest != pm.digest || bv.allocation_id != aid || bv.uid != a.uid || bv.gids != vec![a.gid] {
            return err(wire::CLASS_CONFLICT, "binding_allocation_mismatch", "binding does not match the reservation");
        }
        if le.get("allocation_id").and_then(|x| x.as_str()) != Some(aid) { return err(wire::CLASS_INVALID, "constructor_envelope", "allocation_id differs"); }
        let lrd = launch_record_digest(&pm.digest, &cb.digest).map_err(|e| (wire::CLASS_INTERNAL, "digest", e.to_string()))?;
        let payload = Value::obj(vec![("authorization_manifest", m.clone()), ("envelope", le.clone()), ("launch_binding", lb.clone()), ("manifest_envelope", me.clone())]);
        let seq = self.store.append_record("binding", aid, &lrd, mv.authorization_id, &payload).map_err(store_err)?;
        self.sessions.bind(aid, &lrd, mv.authorization_id, bv.scope_id, mv.session_id, mv.trace_id, a.uid, a.gid, mv.reclamation_domain_id, mv.topology, mv.storage_ref);
        let c = Correlation { authorization_id: Some(mv.authorization_id.into()), launch_record_digest: Some(lrd.clone()), allocation_id: Some(aid.into()), session_id: Some(mv.session_id.into()), trace_id: Some(mv.trace_id.into()), execution_uid: Some(a.uid) };
        self.emit("session.launch_record_committed", "ok", &c, Value::obj(vec![("commit_seq", Value::Int(seq)), ("manifest_digest", Value::s(&pm.digest)), ("trust_anchor", Value::s(&format!("{}+{}", pm.key_id, cb.key_id)))]));
        Ok(Value::obj(vec![("launch_record_digest", Value::s(&lrd)), ("store_seq", Value::Int(seq))]))
    }

    fn register_session(&mut self, b: &Value, fds: Vec<OwnedFd>) -> Reply {
        closed(b, &["allocation_id", "descriptors", "init_pid", "launch_record_digest", "pid_namespace_id", "scope_id", "session_dir"])?;
        let (aid, lrd) = (gs(b, "allocation_id")?, gs(b, "launch_record_digest")?);
        if !self.store.record_exists("binding", "launch_record_digest", lrd).map_err(store_err)? { return err(wire::CLASS_CONFLICT, "binding_not_committed", ""); }
        let ds = b.get("descriptors").and_then(|x| x.as_arr()).ok_or((wire::CLASS_INVALID, "descriptors", String::new()))?;
        if ds.len() != fds.len() { return err(wire::CLASS_INVALID, "descriptor_count", format!("{} named, {} received", ds.len(), fds.len())); }
        let mut init_pidfd = None; let mut cgroup_dir = None;
        for (i, d) in ds.iter().enumerate() {
            match (gs(d, "kind")?, d.get("index").and_then(|x| x.as_int())) {
                ("init_pidfd", Some(ix)) if ix as usize == i => init_pidfd = Some(i), ("cgroup_dir", Some(ix)) if ix as usize == i => cgroup_dir = Some(i), ("rootfs_mount", Some(ix)) if ix as usize == i => {}
                _ => return err(wire::CLASS_INVALID, "descriptor_kind", ""),
            }
        }
        let (Some(pi), Some(ci)) = (init_pidfd, cgroup_dir) else { return err(wire::CLASS_INVALID, "descriptors", "init_pidfd and cgroup_dir required") };
        let mut fds: Vec<Option<OwnedFd>> = fds.into_iter().map(Some).collect();
        let init_pid = b.get("init_pid").and_then(|x| x.as_int()).ok_or((wire::CLASS_INVALID, "init_pid", String::new()))? as i32;
        self.sessions.register(lrd, fds[pi].take().unwrap(), fds[ci].take().unwrap(), init_pid, gs(b, "scope_id")?, gs(b, "pid_namespace_id")?).map_err(|r| (wire::CLASS_CONFLICT, r, String::new()))?;
        if let Some(s) = self.sessions.get_mut(lrd) { s.session_dir = b.get("session_dir").and_then(|x| x.as_str()).map(str::to_string); }
        let _ = aid;
        Ok(Value::obj(vec![("registered", Value::Bool(true))]))
    }

    fn report_activation(&mut self, b: &Value) -> Reply {
        closed(b, &["allocation_id", "launch_record_digest", "privilege_disposal", "runtime_artifact_digest"])?;
        let (aid, lrd) = (gs(b, "allocation_id")?, gs(b, "launch_record_digest")?);
        let s = self.sessions.get(lrd).ok_or((wire::CLASS_CONFLICT, "session_not_registered", String::new()))?;
        if s.allocation_id != aid { return err(wire::CLASS_CONFLICT, "allocation_mismatch", ""); }
        let a = self.store.latest(aid).map_err(store_err)?.ok_or((wire::CLASS_CONFLICT, "unknown_allocation", String::new()))?;
        let a = self.store.transition(aid, a.state_seq, "in-use", "activation reported", Some(&s.scope_id), Some(&s.pidns_id), "agentbound-launch").map_err(store_err)?;
        let payload = Value::obj(vec![("event", Value::s("session.activated")), ("privilege_disposal", b.get("privilege_disposal").unwrap().clone()), ("runtime_artifact_digest", Value::s(gs(b, "runtime_artifact_digest")?))]);
        self.store.append_record("event", aid, lrd, &a.authorization_id, &payload).map_err(store_err)?;
        self.sessions.set_state(lrd, "active", None);
        let c = self.sessions.correlation(lrd);
        self.emit("session.activated", "ok", &c, payload);
        Ok(Value::obj(vec![("state", Value::s("active"))]))
    }

    fn report_failed(&mut self, b: &Value) -> Reply {
        closed(b, &["allocation_id", "failed_step", "launch_record_digest", "ledger", "rule"])?;
        let aid = gs(b, "allocation_id")?; let lrd = b.get("launch_record_digest").and_then(|x| x.as_str());
        let a = self.store.latest(aid).map_err(store_err)?.ok_or((wire::CLASS_CONFLICT, "unknown_allocation", String::new()))?;
        let a = if a.state == "allocated" || a.state == "in-use" { self.store.transition(aid, a.state_seq, "reclaiming", "construction failed", None, None, "agentbound-launch").map_err(store_err)? } else { a };
        let payload = Value::obj(vec![("event", Value::s("session.construction_failed")), ("failed_step", b.get("failed_step").unwrap().clone()), ("ledger", b.get("ledger").unwrap().clone()), ("rule", Value::s(gs(b, "rule")?))]);
        if let Some(lrd) = lrd { self.store.append_record("event", aid, lrd, &a.authorization_id, &payload).map_err(store_err)?; self.sessions.set_state(lrd, "construction-failed", Some(gs(b, "rule")?)); }
        let c = Correlation { authorization_id: Some(a.authorization_id.clone()), launch_record_digest: lrd.map(str::to_string), allocation_id: Some(aid.into()), session_id: Some(a.session_id.clone()), trace_id: Some(a.trace_id.clone()), execution_uid: Some(a.uid) };
        self.emit("session.construction_failed", "construction-failed", &c, payload);
        // the identity is reclaimed by the ordinary condition check (session.rs), never freed here
        self.sessions.reclaim_later(aid, lrd);
        Ok(Value::obj(vec![("identity_state", Value::s(&a.state)), ("state", Value::s("construction-failed"))]))
    }

    fn status(&mut self, b: &Value) -> Reply {
        let lrd = match (b.get("launch_record_digest").and_then(|x| x.as_str()), b.get("authorization_id").and_then(|x| x.as_str())) {
            (Some(l), None) => l.to_string(), (None, Some(a)) => match self.sessions.by_authorization(a) { Some(l) => l, None => return self.status_prebinding(a) }, _ => return err(wire::CLASS_INVALID, "body", "exactly one of launch_record_digest or authorization_id"),
        };
        let s = self.sessions.get(&lrd).ok_or((wire::CLASS_INVALID, "unknown_record", String::new()))?;
        let ident = self.store.latest(&s.allocation_id).map_err(store_err)?.map(|a| a.state).unwrap_or_default();
        Ok(Value::obj(vec![("identity_state", Value::s(&ident)), ("observation_seq", Value::Int(s.observation_seq)), ("reason", s.reason.as_deref().map(Value::s).unwrap_or(Value::Null)), ("record_ref", Value::s(&lrd)), ("state", Value::s(&s.state))]))
    }
    /// Committed binding record (manifest + binding + envelopes) for one digest, with the live session state.
    fn record(&mut self, b: &Value) -> Reply {
        let lrd = b.get("launch_record_digest").and_then(|x| x.as_str()).ok_or((wire::CLASS_INVALID, "body", "launch_record_digest".to_string()))?;
        let recs = self.store.records(lrd).map_err(store_err)?;
        let binding = recs.iter().find(|(k, _)| k == "binding").map(|(_, v)| v.clone()).ok_or((wire::CLASS_INVALID, "unknown_record", String::new()))?;
        let sealed = recs.iter().any(|(k, _)| k == "seal");
        let (state, ident) = match self.sessions.get(lrd) { Some(s) => (s.state.clone(), self.store.latest(&s.allocation_id).map_err(store_err)?.map(|a| a.state).unwrap_or_default()), None => ("unknown".into(), String::new()) };
        Ok(Value::obj(vec![("binding", binding), ("identity_state", Value::s(&ident)), ("sealed", Value::Bool(sealed)), ("state", Value::s(&state))]))
    }
    fn status_prebinding(&mut self, az: &str) -> Reply {
        let a = self.store.by_authorization(az).map_err(store_err)?.ok_or((wire::CLASS_INVALID, "unknown_record", String::new()))?;
        Ok(Value::obj(vec![("identity_state", Value::s(&a.state)), ("observation_seq", Value::Int(0)), ("reason", Value::Null), ("record_ref", Value::s(az)), ("state", Value::s(if a.state == "allocated" { "constructing" } else { "construction-failed" }))]))
    }
    fn list(&mut self) -> Reply {
        Ok(Value::obj(vec![("sessions", Value::Arr(self.sessions.all().into_iter().map(|s| Value::obj(vec![("authorization_id", Value::s(&s.authorization_id)), ("launch_record_digest", Value::s(&s.lrd)), ("state", Value::s(&s.state))])).collect()))]))
    }
    pub fn _canon(v: &Value) -> Vec<u8> { canonical(v) }
}
