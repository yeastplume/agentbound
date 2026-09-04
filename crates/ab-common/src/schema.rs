//! Closed-schema validation for the session request (§2), authorization
//! manifest (§3.2–3.6), launch binding (§3.7), and the §3.1 correspondence
//! checks. Validation is structural and typed; identifier strings are never
//! interpreted (§2.3). Everything here is pure and runs before any privilege.

use crate::json::Value;
use crate::sig::is_digest;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct SchemaError { pub rule: &'static str, pub path: String, pub detail: String }
impl fmt::Display for SchemaError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{} at {}: {}", self.rule, self.path, self.detail) } }
impl std::error::Error for SchemaError {}
fn e(rule: &'static str, path: &str, detail: impl Into<String>) -> SchemaError { SchemaError { rule, path: path.into(), detail: detail.into() } }
type R<T> = Result<T, SchemaError>;

pub const REQUEST_VERSION: &str = "agentbound.session-request.v0.1";
pub const MANIFEST_VERSION: &str = "agentbound.authorization-manifest.v0.1";
pub const BINDING_VERSION: &str = "agentbound.launch-binding.v0.1";

pub const RESOURCE_CLASSES: [&str; 16] = [
    "accelerator", "audit_capacity", "connection_count", "cpu", "delegation_fanout", "disk_bytes", "disk_inodes",
    "external_spend", "file_descriptors", "io_bandwidth", "memory_bytes", "model_tokens", "network_bandwidth", "pids",
    "request_rate", "storage_bytes",
];
pub const BUDGET_CLASSES: [&str; 14] = [
    "cpu_millis", "memory_bytes", "pids", "wall_clock_seconds", "disk_bytes", "disk_inodes", "io_bytes",
    "gateway_requests", "gateway_bytes", "connection_count", "audit_events", "model_tokens", "monetary_microunits", "delegation_fanout",
];
pub const REVOCATION_TRIGGERS: [&str; 11] = [
    "approval_expired", "audit_pipeline_degraded_below_stop_threshold", "authority_revoked", "catalogue_withdrawn",
    "gateway_grant_withdrawn", "gateway_unavailable", "initiator_disabled", "policy_service_unavailable", "policy_withdrawn",
    "reclassification", "task_cancelled",
];
pub const DEGRADED_OK: [&str; 2] = ["policy_service_unavailable", "audit_pipeline_degraded_below_stop_threshold"];

// ---- identifier grammars (§2.3) ----
fn ident(s: &str, tail_max: usize) -> bool {
    let Some((p, t)) = s.split_once(':') else { return false };
    let pb = p.as_bytes();
    if pb.is_empty() || pb.len() > 32 || !pb[0].is_ascii_lowercase() || !pb.iter().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-') { return false; }
    let tb = t.as_bytes();
    tb.len() >= 1 && tb.len() <= tail_max + 1 && tb[0].is_ascii_alphanumeric() && tb[1..].iter().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'/' | b'-'))
}
pub fn is_catalogue_id(s: &str) -> bool { ident(s, 127) }
pub fn is_evidence_ref(s: &str) -> bool { ident(s, 255) }
pub fn is_opaque_local(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty() && b.len() <= 128 && b[0].is_ascii_alphabetic() && b[1..].iter().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b':' | b'-'))
}
fn no_bad_chars(s: &str) -> bool { !s.chars().any(|c| c.is_control() || (0xFDD0..=0xFDEF).contains(&(c as u32)) || (c as u32 & 0xFFFE) == 0xFFFE) }

// ---- generic helpers ----
fn obj<'a>(v: &'a Value, path: &str) -> R<&'a Value> { if v.as_obj().is_some() { Ok(v) } else { Err(e("type", path, "object required")) } }
fn closed(v: &Value, path: &str, required: &[&str], optional: &[&str]) -> R<()> {
    let m = v.as_obj().ok_or_else(|| e("type", path, "object required"))?;
    for k in m.keys() { if !required.contains(&k.0.as_str()) && !optional.contains(&k.0.as_str()) { return Err(e("unknown-member", path, k.0.clone())); } }
    for r in required { if !m.contains_key(&(*r).into()) { return Err(e("missing-member", path, *r)); } }
    Ok(())
}
fn str_<'a>(v: &'a Value, k: &str, path: &str) -> R<&'a str> { v.get(k).and_then(|x| x.as_str()).filter(|s| no_bad_chars(s)).ok_or_else(|| e("type", &format!("{path}.{k}"), "string required")) }
fn arr<'a>(v: &'a Value, k: &str, path: &str, max: usize) -> R<&'a Vec<Value>> {
    let a = v.get(k).and_then(|x| x.as_arr()).ok_or_else(|| e("type", &format!("{path}.{k}"), "array required"))?;
    if a.len() > max { return Err(e("bound", &format!("{path}.{k}"), format!("more than {max} elements"))); }
    Ok(a)
}
fn str_set<'a>(v: &'a Value, k: &str, path: &str, max: usize, ok: fn(&str) -> bool) -> R<Vec<&'a str>> {
    let a = arr(v, k, path, max)?; let mut out: Vec<&str> = Vec::new();
    for (i, x) in a.iter().enumerate() {
        let s = x.as_str().filter(|s| ok(s) && no_bad_chars(s)).ok_or_else(|| e("grammar", &format!("{path}.{k}[{i}]"), "identifier"))?;
        if out.contains(&s) { return Err(e("duplicate-set-member", &format!("{path}.{k}"), s)); }
        out.push(s);
    }
    Ok(out)
}
fn int(v: &Value, k: &str, path: &str) -> R<i64> { v.get(k).and_then(|x| x.as_int()).filter(|n| *n >= 0).ok_or_else(|| e("type", &format!("{path}.{k}"), "non-negative integer required")) }
fn one_of(v: &Value, k: &str, path: &str, allowed: &[&str]) -> R<String> {
    let s = str_(v, k, path)?; if allowed.contains(&s) { Ok(s.into()) } else { Err(e("enum", &format!("{path}.{k}"), s)) }
}

// ================= §2 request =================
#[derive(Debug)]
pub struct Request<'a> { pub agent_principal_id: &'a str, pub task_purpose_id: &'a str, pub requested_runtime: &'a str, pub requested_resources: Vec<&'a str>, pub initiator_credential_ref: &'a str, pub approval_references: Vec<&'a str>, pub budget: Option<&'a Value> }

pub fn validate_request(v: &Value) -> R<Request<'_>> {
    let p = "request";
    closed(v, p, &["agent_principal_id", "approval_references", "initiator_credential_ref", "requested_resources", "requested_runtime", "schema_version", "task_purpose_id"], &["budget"])?;
    if str_(v, "schema_version", p)? != REQUEST_VERSION { return Err(e("version", "request.schema_version", "unsupported")); }
    let cid = |k: &str| -> R<&str> { let s = str_(v, k, p)?; if is_catalogue_id(s) { Ok(s) } else { Err(e("grammar", &format!("{p}.{k}"), "catalogue identifier")) } };
    let eref = |k: &str| -> R<&str> { let s = str_(v, k, p)?; if is_evidence_ref(s) { Ok(s) } else { Err(e("grammar", &format!("{p}.{k}"), "evidence reference")) } };
    let budget = v.get("budget");
    if let Some(b) = budget {
        let m = b.as_obj().ok_or_else(|| e("type", "request.budget", "object"))?;
        if m.len() > 16 { return Err(e("bound", "request.budget", "more than 16 members")); }
        for (k, x) in m { if !BUDGET_CLASSES.contains(&k.0.as_str()) { return Err(e("unknown-member", "request.budget", k.0.clone())); } if x.as_int().filter(|n| *n >= 0).is_none() { return Err(e("type", &format!("request.budget.{}", k.0), "non-negative integer")); } }
    }
    Ok(Request { agent_principal_id: cid("agent_principal_id")?, task_purpose_id: cid("task_purpose_id")?, requested_runtime: cid("requested_runtime")?,
        requested_resources: str_set(v, "requested_resources", p, 32, is_catalogue_id)?, initiator_credential_ref: eref("initiator_credential_ref")?,
        approval_references: str_set(v, "approval_references", p, 16, is_evidence_ref)?, budget })
}

// ================= §3.2–3.6 authorization manifest =================
#[derive(Debug)]
pub struct MountIntent<'a> { pub mount_id: &'a str, pub catalogue_id: &'a str, pub target_template_id: &'a str, pub access: &'a str, pub required: bool }
#[derive(Debug)]
pub struct Manifest<'a> {
    pub v: &'a Value, pub authorization_id: &'a str, pub session_id: &'a str, pub trace_id: &'a str, pub agent_global_id: &'a str,
    pub runtime_catalogue_id: &'a str, pub runtime_artifact_digest: &'a str, pub invocation_profile: &'a str,
    pub topology: &'a str, pub mount_intents: Vec<MountIntent<'a>>, pub grant_intent_ids: Vec<&'a str>, pub operation_ids: Vec<&'a str>,
    pub loss_behaviour: &'a str, pub reclamation_domain_id: &'a str,
}

pub fn validate_manifest(v: &Value) -> R<Manifest<'_>> {
    let p = "manifest";
    closed(v, p, &["actors", "agent", "audit", "authorization_id", "credential_grant_intents", "derivation", "execution_binding", "gateway", "manifest_version", "mount_intents", "resource_limits", "revocation", "runtime", "session_trace", "task", "termination_retention"], &[])?;
    if str_(v, "manifest_version", p)? != MANIFEST_VERSION { return Err(e("version", "manifest.manifest_version", "unsupported")); }
    let authorization_id = str_(v, "authorization_id", p)?; if !is_opaque_local(authorization_id) && !is_catalogue_id(authorization_id) { return Err(e("grammar", "manifest.authorization_id", "")); }

    // agent
    let ag = obj(v.get("agent").unwrap(), "manifest.agent")?; closed(ag, "manifest.agent", &["durable_ownership_projection", "global_id"], &[])?;
    let agent_global_id = str_(ag, "global_id", "manifest.agent")?; if !is_catalogue_id(agent_global_id) { return Err(e("grammar", "manifest.agent.global_id", "")); }
    let dop = obj(ag.get("durable_ownership_projection").unwrap(), "manifest.agent.durable_ownership_projection")?;
    closed(dop, "manifest.agent.durable_ownership_projection", &["kind", "reference"], &[])?;
    one_of(dop, "kind", "manifest.agent.durable_ownership_projection", &["storage-principal", "local-owner-uid"])?;
    str_(dop, "reference", "manifest.agent.durable_ownership_projection")?;

    // session_trace
    let st = obj(v.get("session_trace").unwrap(), "manifest.session_trace")?; closed(st, "manifest.session_trace", &["session_id", "trace_id"], &[])?;
    let session_id = str_(st, "session_id", "manifest.session_trace")?; let trace_id = str_(st, "trace_id", "manifest.session_trace")?;
    if !is_opaque_local(session_id) || !is_opaque_local(trace_id) { return Err(e("grammar", "manifest.session_trace", "opaque local identifier")); }

    // actors (§3.3)
    let ac = obj(v.get("actors").unwrap(), "manifest.actors")?; closed(ac, "manifest.actors", &["approvers", "initiators", "owner", "scheduler"], &[])?;
    let inits = arr(ac, "initiators", "manifest.actors", 16)?; if inits.is_empty() { return Err(e("actors", "manifest.actors.initiators", "at least one initiator")); }
    for (i, a) in inits.iter().enumerate() { let pp = format!("manifest.actors.initiators[{i}]"); closed(a, &pp, &["credential_reference", "id", "relationship"], &[])?; one_of(a, "relationship", &pp, &["delegation", "scheduled", "agent-parent", "service"])?; if !is_evidence_ref(str_(a, "credential_reference", &pp)?) { return Err(e("grammar", &pp, "credential_reference")); } }
    for (i, a) in arr(ac, "approvers", "manifest.actors", 16)?.iter().enumerate() { let pp = format!("manifest.actors.approvers[{i}]"); closed(a, &pp, &["approval_reference", "decision", "expires_at", "id"], &[])?; one_of(a, "decision", &pp, &["approve", "deny"])?; }
    let scheduled = inits.iter().any(|a| a.get("relationship").and_then(|x| x.as_str()) == Some("scheduled")) || !ac.get("scheduler").unwrap().is_null();
    if scheduled && ac.get("owner").unwrap().is_null() { return Err(e("scheduled-without-owner", "manifest.actors.owner", "scheduled request requires an accountable owner")); }

    // task, derivation
    let tk = obj(v.get("task").unwrap(), "manifest.task")?; closed(tk, "manifest.task", &["approval_references", "purpose_id"], &[])?;
    if !is_catalogue_id(str_(tk, "purpose_id", "manifest.task")?) { return Err(e("grammar", "manifest.task.purpose_id", "")); }
    str_set(tk, "approval_references", "manifest.task", 16, is_evidence_ref)?;
    let dv = obj(v.get("derivation").unwrap(), "manifest.derivation")?;
    closed(dv, "manifest.derivation", &["agent_authority_version", "catalogue_version", "derivation_input_digest", "derivation_relation_version", "inputs", "policy_version", "requested_budget_digest", "resolved_resource_ids"], &[])?;
    for k in ["derivation_input_digest", "requested_budget_digest"] { if !is_digest(str_(dv, k, "manifest.derivation")?) { return Err(e("grammar", &format!("manifest.derivation.{k}"), "digest")); } }
    for (i, a) in arr(dv, "inputs", "manifest.derivation", 64)?.iter().enumerate() { closed(a, &format!("manifest.derivation.inputs[{i}]"), &["id", "kind", "version"], &[])?; }
    str_set(dv, "resolved_resource_ids", "manifest.derivation", 32, is_catalogue_id)?;

    // runtime, execution_binding
    let rt = obj(v.get("runtime").unwrap(), "manifest.runtime")?; closed(rt, "manifest.runtime", &["artifact_digest", "catalogue_id", "invocation_profile"], &[])?;
    let runtime_catalogue_id = str_(rt, "catalogue_id", "manifest.runtime")?; let runtime_artifact_digest = str_(rt, "artifact_digest", "manifest.runtime")?; let invocation_profile = str_(rt, "invocation_profile", "manifest.runtime")?;
    if !is_catalogue_id(runtime_catalogue_id) || !is_digest(runtime_artifact_digest) || !is_catalogue_id(invocation_profile) { return Err(e("grammar", "manifest.runtime", "")); }
    let eb = obj(v.get("execution_binding").unwrap(), "manifest.execution_binding")?; closed(eb, "manifest.execution_binding", &["adapters", "endpoint", "inference_pool", "model", "retention_mode", "tenant"], &[])?;
    str_set(eb, "adapters", "manifest.execution_binding", 16, is_catalogue_id)?;

    // mount intents (§3.4)
    let mut mount_intents = Vec::new();
    for (i, m) in arr(v, "mount_intents", p, 32)?.iter().enumerate() {
        let pp = format!("manifest.mount_intents[{i}]"); closed(m, &pp, &["access", "catalogue_id", "mount_id", "required", "target_template_id"], &[])?;
        let mi = MountIntent { mount_id: str_(m, "mount_id", &pp)?, catalogue_id: str_(m, "catalogue_id", &pp)?, target_template_id: str_(m, "target_template_id", &pp)?, access: str_(m, "access", &pp)?, required: m.get("required").and_then(|x| x.as_bool()).ok_or_else(|| e("type", &pp, "required: bool"))? };
        if !is_catalogue_id(mi.mount_id) || !is_catalogue_id(mi.catalogue_id) || !is_catalogue_id(mi.target_template_id) || !["read-only", "read-write"].contains(&mi.access) { return Err(e("grammar", &pp, "")); }
        if mount_intents.iter().any(|x: &MountIntent| x.mount_id == mi.mount_id) { return Err(e("duplicate-set-member", &pp, mi.mount_id)); }
        mount_intents.push(mi);
    }

    // gateway (§3.4)
    let gw = obj(v.get("gateway").unwrap(), "manifest.gateway")?; closed(gw, "manifest.gateway", &["budgets", "channel_topology", "operations"], &[])?;
    let topology = one_of(gw, "channel_topology", "manifest.gateway", &["none", "local-socket"])?;
    let ops = arr(gw, "operations", "manifest.gateway", 32)?; let budgets = obj(gw.get("budgets").unwrap(), "manifest.gateway.budgets")?;
    let mut operation_ids = Vec::new();
    for (i, o) in ops.iter().enumerate() {
        let pp = format!("manifest.gateway.operations[{i}]"); closed(o, &pp, &["adapter_catalogue_id", "budgets", "operation", "operation_id", "scope"], &[])?;
        let id = str_(o, "operation_id", &pp)?; if operation_ids.contains(&id) { return Err(e("duplicate-set-member", &pp, id)); } operation_ids.push(id);
        if !is_catalogue_id(str_(o, "adapter_catalogue_id", &pp)?) { return Err(e("grammar", &pp, "adapter_catalogue_id")); }
    }
    let mut grant_intent_ids = Vec::new();
    for (i, g) in arr(v, "credential_grant_intents", p, 32)?.iter().enumerate() {
        let pp = format!("manifest.credential_grant_intents[{i}]"); closed(g, &pp, &["expiry_policy", "grant_id", "kind", "operation_subset"], &["audience_id"])?;
        let id = str_(g, "grant_id", &pp)?; if grant_intent_ids.contains(&id) { return Err(e("duplicate-set-member", &pp, id)); } grant_intent_ids.push(id);
        for op in str_set(g, "operation_subset", &pp, 32, is_catalogue_id)? { if !operation_ids.contains(&op) { return Err(e("grant-names-unknown-operation", &pp, op)); } }
    }
    if topology == "none" && (!ops.is_empty() || !budgets.as_obj().unwrap().is_empty() || !grant_intent_ids.is_empty()) { return Err(e("topology-none-with-gateway-content", "manifest.gateway", "operations, budgets, and grant intents must be empty")); }
    if topology == "local-socket" && ops.is_empty() { return Err(e("topology-local-socket-without-operations", "manifest.gateway.operations", "")); }

    // resource_limits (§3.5) — closed: exactly the 16 classes
    let rl = obj(v.get("resource_limits").unwrap(), "manifest.resource_limits")?; closed(rl, "manifest.resource_limits", &RESOURCE_CLASSES, &[])?;
    for c in RESOURCE_CLASSES { validate_limit(rl.get(c).unwrap(), &format!("manifest.resource_limits.{c}"))?; }

    // audit, revocation, termination_retention (§3.6)
    let au = obj(v.get("audit").unwrap(), "manifest.audit")?; closed(au, "manifest.audit", &["correlation_keys", "loss_behaviour", "required_events"], &[])?;
    one_of(au, "loss_behaviour", "manifest.audit", &["stop", "quarantine", "continue-with-loss-counter"])?;
    let keys = str_set(au, "correlation_keys", "manifest.audit", 16, |_| true)?;
    for k in ["authorization_id", "trace_id", "agent_global_id"] { if !keys.contains(&k) { return Err(e("correlation-key-missing", "manifest.audit.correlation_keys", k)); } }
    let rv = obj(v.get("revocation").unwrap(), "manifest.revocation")?; closed(rv, "manifest.revocation", &REVOCATION_TRIGGERS, &[])?;
    for t in REVOCATION_TRIGGERS {
        let b = one_of(rv, t, "manifest.revocation", &["terminate", "quiesce", "continue-degraded"])?;
        if b == "continue-degraded" && !DEGRADED_OK.contains(&t) { return Err(e("continue-degraded-not-permitted", &format!("manifest.revocation.{t}"), "only policy_service_unavailable and audit_pipeline_degraded_below_stop_threshold")); }
    }
    let tr = obj(v.get("termination_retention").unwrap(), "manifest.termination_retention")?;
    closed(tr, "manifest.termination_retention", &["audit_retention_class", "credential_revocation_order", "descendant_kill_order", "reclamation_domain_id", "termination_triggers", "workspace_retention"], &[])?;
    let reclamation_domain_id = str_(tr, "reclamation_domain_id", "manifest.termination_retention")?;
    one_of(tr, "descendant_kill_order", "manifest.termination_retention", &["children-before-parent", "before-credential-release"])?;
    one_of(tr, "credential_revocation_order", "manifest.termination_retention", &["revoke-before-cleanup"])?;
    one_of(tr, "workspace_retention", "manifest.termination_retention", &["discard", "retain-until-reclaimed"])?;
    str_set(tr, "termination_triggers", "manifest.termination_retention", 16, |s| REVOCATION_TRIGGERS.contains(&s))?;

    Ok(Manifest { v, authorization_id, session_id, trace_id, agent_global_id, runtime_catalogue_id, runtime_artifact_digest, invocation_profile,
        topology: gw.get("channel_topology").unwrap().as_str().unwrap(), mount_intents, grant_intent_ids, operation_ids, loss_behaviour: au.get("loss_behaviour").unwrap().as_str().unwrap(), reclamation_domain_id })
}

fn validate_limit(l: &Value, path: &str) -> R<()> {
    let status = one_of(l, "status", path, &["enforced", "absent"])?;
    if status == "enforced" { closed(l, path, &["enforcement_owner", "limit", "status", "unit"], &[])?; int(l, "limit", path)?; str_(l, "unit", path)?; }
    else { closed(l, path, &["absence_evidence", "enforcement_owner", "status"], &[])?; str_(l, "absence_evidence", path)?; }
    str_(l, "enforcement_owner", path)?; Ok(())
}
pub struct Limit<'a> { pub class: &'a str, pub enforced: bool, pub limit: i64, pub unit: &'a str, pub owner: &'a str }
pub fn limits<'a>(m: &Manifest<'a>) -> Vec<Limit<'a>> {
    let rl = m.v.get("resource_limits").unwrap();
    RESOURCE_CLASSES.iter().map(|c| { let l = rl.get(c).unwrap(); let enforced = l.get("status").unwrap().as_str() == Some("enforced");
        Limit { class: c, enforced, limit: if enforced { l.get("limit").unwrap().as_int().unwrap() } else { 0 }, unit: if enforced { l.get("unit").unwrap().as_str().unwrap() } else { "" }, owner: l.get("enforcement_owner").unwrap().as_str().unwrap() } }).collect()
}

// ================= §3.7 launch binding =================
#[derive(Debug)]
pub struct Binding<'a> { pub v: &'a Value, pub authorization_id: &'a str, pub manifest_digest: &'a str, pub allocation_id: &'a str, pub uid: u32, pub gids: Vec<u32>, pub scope_id: &'a str, pub pid_namespace_id: &'a str, pub host_id: &'a str, pub boot_id: &'a str }

pub fn validate_binding(v: &Value) -> R<Binding<'_>> {
    let p = "binding";
    closed(v, p, &["authorization_id", "authorization_manifest_digest", "constructor", "credential_grants", "descriptor_allowlist", "execution_identity", "gateway_projection", "host_binding", "launch_binding_version", "mount_projections", "namespaces", "resource_projection"], &[])?;
    if str_(v, "launch_binding_version", p)? != BINDING_VERSION { return Err(e("version", "binding.launch_binding_version", "unsupported")); }
    let authorization_id = str_(v, "authorization_id", p)?; let manifest_digest = str_(v, "authorization_manifest_digest", p)?;
    if !is_digest(manifest_digest) { return Err(e("grammar", "binding.authorization_manifest_digest", "digest")); }
    let ei = obj(v.get("execution_identity").unwrap(), "binding.execution_identity")?; closed(ei, "binding.execution_identity", &["allocation_id", "gids", "mac_context", "uid"], &[])?;
    if !ei.get("mac_context").unwrap().is_null() { return Err(e("mac-context-not-null", "binding.execution_identity.mac_context", "Profile U requires null")); }
    let uid = int(ei, "uid", "binding.execution_identity")? as u32;
    let ga = arr(ei, "gids", "binding.execution_identity", 32)?; if ga.is_empty() { return Err(e("bound", "binding.execution_identity.gids", "non-empty")); }
    let mut gids = Vec::new(); for g in ga { let g = g.as_int().filter(|n| *n >= 0).ok_or_else(|| e("type", "binding.execution_identity.gids", "int"))? as u32; if gids.contains(&g) { return Err(e("duplicate-set-member", "binding.execution_identity.gids", g.to_string())); } gids.push(g); }
    let hb = obj(v.get("host_binding").unwrap(), "binding.host_binding")?; closed(hb, "binding.host_binding", &["boot_id", "host_id", "pid_namespace_id", "scope_id"], &[])?;
    let ns = obj(v.get("namespaces").unwrap(), "binding.namespaces")?; closed(ns, "binding.namespaces", &["ipc", "mount", "pid", "user", "uts"], &["net"])?;
    for k in ["ipc", "mount", "pid", "uts"] { if str_(ns, k, "binding.namespaces")? != "private" { return Err(e("namespace-not-private", &format!("binding.namespaces.{k}"), "")); } }
    one_of(ns, "user", "binding.namespaces", &["private", "inherited"])?;
    for (i, m) in arr(v, "mount_projections", p, 32)?.iter().enumerate() {
        let pp = format!("binding.mount_projections[{i}]"); closed(m, &pp, &["access", "mount_id", "target_template_projection"], &["catalogue_version", "resolved_source_handle"])?;
        if m.get("catalogue_version").is_some() == m.get("resolved_source_handle").is_some() { return Err(e("mount-projection-source-form", &pp, "exactly one of catalogue_version or resolved_source_handle")); }
    }
    let mut kinds = Vec::new();
    for (i, d) in arr(v, "descriptor_allowlist", p, 16)?.iter().enumerate() { let pp = format!("binding.descriptor_allowlist[{i}]"); closed(d, &pp, &["descriptor_id", "kind", "purpose"], &[])?; kinds.push(one_of(d, "kind", &pp, &["stdin", "stdout", "stderr", "pty", "gateway_socket"])?); }
    let gp = v.get("gateway_projection").unwrap();
    if !gp.is_null() { closed(gp, "binding.gateway_projection", &["seqpacket", "socket_mount_id"], &[])?; if gp.get("seqpacket").and_then(|x| x.as_bool()) != Some(true) { return Err(e("gateway-projection-seqpacket", "binding.gateway_projection", "must be true")); } }
    if gp.is_null() != !kinds.iter().any(|k| k == "gateway_socket") { return Err(e("gateway-socket-vs-projection", "binding", "gateway_socket descriptor iff gateway_projection")); }
    if kinds.iter().filter(|k| *k == "gateway_socket").count() > 1 { return Err(e("gateway-socket-count", "binding.descriptor_allowlist", "at most one")); }
    for (i, g) in arr(v, "credential_grants", p, 32)?.iter().enumerate() { closed(g, &format!("binding.credential_grants[{i}]"), &["grant_intent_id", "issued_handle"], &[])?; }
    let rp = obj(v.get("resource_projection").unwrap(), "binding.resource_projection")?; closed(rp, "binding.resource_projection", &RESOURCE_CLASSES, &[])?;
    for c in RESOURCE_CLASSES { let x = rp.get(c).unwrap(); let path = format!("binding.resource_projection.{c}");
        if x.get("status").and_then(|s| s.as_str()) == Some("absent") { closed(x, &path, &["enforcement_owner", "status"], &[])?; } else { closed(x, &path, &["enforcement_owner", "installed_value", "unit"], &[])?; int(x, "installed_value", &path)?; } }
    let cs = obj(v.get("constructor").unwrap(), "binding.constructor")?; closed(cs, "binding.constructor", &["agentbound_launch_version_digest", "invocation_profile_digest", "key_id"], &[])?;
    Ok(Binding { v, authorization_id, manifest_digest, allocation_id: str_(ei, "allocation_id", "binding.execution_identity")?, uid, gids,
        scope_id: str_(hb, "scope_id", "binding.host_binding")?, pid_namespace_id: str_(hb, "pid_namespace_id", "binding.host_binding")?, host_id: str_(hb, "host_id", "binding.host_binding")?, boot_id: str_(hb, "boot_id", "binding.host_binding")? })
}

/// §3.1 correspondence checks 1–7 between a verified manifest and a binding.
pub fn correspond(m: &Manifest<'_>, b: &Binding<'_>, verified_manifest_digest: &str) -> R<()> {
    if m.authorization_id != b.authorization_id { return Err(e("correspondence-1", "authorization_id", "differs")); }
    if b.manifest_digest != verified_manifest_digest { return Err(e("correspondence-2", "authorization_manifest_digest", "differs from verified digest")); }
    let projs = b.v.get("mount_projections").unwrap().as_arr().unwrap();
    let pids: Vec<&str> = projs.iter().map(|p| p.get("mount_id").unwrap().as_str().unwrap()).collect();
    for mi in &m.mount_intents {
        let n = pids.iter().filter(|p| **p == mi.mount_id).count(); if n != 1 { return Err(e("correspondence-3", mi.mount_id, format!("{n} projections"))); }
        let pr = projs.iter().find(|p| p.get("mount_id").unwrap().as_str() == Some(mi.mount_id)).unwrap();
        if pr.get("access").unwrap().as_str() != Some(mi.access) { return Err(e("correspondence-3", mi.mount_id, "access differs")); }
    }
    for p in &pids { if !m.mount_intents.iter().any(|mi| mi.mount_id == *p) { return Err(e("correspondence-3", p, "projection without intent")); } }
    let grants = b.v.get("credential_grants").unwrap().as_arr().unwrap();
    let gids: Vec<&str> = grants.iter().map(|g| g.get("grant_intent_id").unwrap().as_str().unwrap()).collect();
    for gi in &m.grant_intent_ids { if gids.iter().filter(|g| **g == *gi).count() != 1 { return Err(e("correspondence-4", gi, "grant count")); } }
    for g in &gids { if !m.grant_intent_ids.contains(g) { return Err(e("correspondence-4", g, "grant without intent")); } }
    let gp_null = b.v.get("gateway_projection").unwrap().is_null();
    match m.topology { "none" => if !gp_null || !grants.is_empty() { return Err(e("correspondence-5", "gateway_projection", "topology none requires no projection or grant")); },
        _ => if gp_null { return Err(e("correspondence-5", "gateway_projection", "local-socket requires a projection")); } }
    if m.topology == "local-socket" && !pids.contains(&b.v.get("gateway_projection").unwrap().get("socket_mount_id").unwrap().as_str().unwrap()) { return Err(e("correspondence-5", "socket_mount_id", "names no projected mount")); }
    let rp = b.v.get("resource_projection").unwrap();
    for l in limits(m) {
        let x = rp.get(l.class).unwrap();
        if l.enforced { let iv = x.get("installed_value").and_then(|v| v.as_int()).ok_or_else(|| e("correspondence-6", l.class, "enforced class not projected"))?; if iv > l.limit { return Err(e("correspondence-6", l.class, "installed value exceeds policy")); } if x.get("unit").unwrap().as_str() != Some(l.unit) { return Err(e("correspondence-6", l.class, "unit differs")); } }
        else if x.get("status").and_then(|s| s.as_str()) != Some("absent") { return Err(e("correspondence-6", l.class, "absent class projected as enforced")); }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grammars() {
        assert!(is_catalogue_id("agent:engineering-agent")); assert!(!is_catalogue_id("Agent:x")); assert!(!is_catalogue_id("a:/x"));
        assert!(is_opaque_local("launchrec:fix-issue-1234-0001")); assert!(!is_opaque_local("1abc"));
    }
    #[test]
    fn request_closed() {
        let ok = crate::json::parse(br#"{"agent_principal_id":"agent:a","approval_references":[],"initiator_credential_ref":"authn:x","requested_resources":[],"requested_runtime":"runtime:sh","schema_version":"agentbound.session-request.v0.1","task_purpose_id":"task:t"}"#, &crate::json::REQUEST_LIMITS).unwrap();
        assert!(validate_request(&ok).is_ok());
        let mut bad = ok.clone(); bad.set("uid", Value::Int(0));
        assert_eq!(validate_request(&bad).unwrap_err().rule, "unknown-member");
    }
}
