//! agentbound-policy: unprivileged resolver (component interfaces §3.1–3.2,
//! manifest schema §2–3). Phase 1 is a file-backed stub with the frozen
//! interface: catalogue JSON in, policy-signed authorization manifest out.
//! It never sees a UID, path, or host object; it emits identifiers only.

use ab_common::json::{self, canonical, Value, MANIFEST_LIMITS, REQUEST_LIMITS};
use ab_common::schema::{self, validate_request, REVOCATION_TRIGGERS};
use ab_common::sig::{fmt_rfc3339, now_unix, object_digest, parse_rfc3339, sha256_hex, Signer_};
use ab_common::{audit, envelope, wire};
use std::io::Write;

struct Policy { cat: Value, signer: Signer_, spool: String, store_path: String, audit: audit::Sink, cli_uids: Vec<u32> }

type Rej = (&'static str, String);
fn rej<T>(rule: &'static str, d: impl Into<String>) -> Result<T, Rej> { Err((rule, d.into())) }
fn strs(v: Option<&Value>) -> Vec<&str> { v.and_then(|x| x.as_arr()).map(|a| a.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default() }

impl Policy {
    fn cat_str(&self, k: &str) -> &str { self.cat.get(k).and_then(|x| x.as_str()).unwrap_or("") }

    /// Append-only policy store: approval sequences (R-ID-4) and authorization records; one canonical line each.
    fn store_append(&self, kind: &str, v: Value) -> Result<(), Rej> {
        let mut line = canonical(&Value::obj(vec![("kind", Value::s(kind)), ("v", v)])); line.push(b'\n');
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&self.store_path).map_err(|e| ("store_unavailable", e.to_string()))?;
        f.write_all(&line).and_then(|_| f.sync_data()).map_err(|e| ("store_unavailable", e.to_string()))
    }
    fn store_scan(&self, kind: &str) -> Vec<Value> {
        std::fs::read_to_string(&self.store_path).unwrap_or_default().lines().filter_map(|l| json::parse(l.as_bytes(), &MANIFEST_LIMITS).ok())
            .filter(|r| r.get("kind").and_then(|k| k.as_str()) == Some(kind)).filter_map(|r| r.get("v").cloned()).collect()
    }
    fn highest_seq(&self, key: &str) -> i64 { self.store_scan("approval_seq").iter().filter(|r| r.get("key").and_then(|k| k.as_str()) == Some(key)).filter_map(|r| r.get("seq").and_then(|s| s.as_int())).max().unwrap_or(0) }

    /// Derive and sign (R-ID-1..5). Returns (authorization_id, manifest, envelope).
    fn derive(&mut self, req_bytes: &[u8], peer_uid: u32) -> Result<(String, Value, Value), Rej> {
        let rv = json::parse(req_bytes, &REQUEST_LIMITS).map_err(|e| ("request_parse", e.to_string()))?;
        let rq = validate_request(&rv).map_err(|e| ("request_schema", e.to_string()))?;
        let now = now_unix().map_err(|_| ("clock_unavailable", String::new()))?;
        // registries (R-ID-1): principal, task, runtime, resources, initiator
        let pr = self.cat.get("principals").and_then(|p| p.get(rq.agent_principal_id)).ok_or(("unknown_principal", rq.agent_principal_id.to_string()))?.clone();
        let tk = self.cat.get("tasks").and_then(|p| p.get(rq.task_purpose_id)).ok_or(("unknown_task", rq.task_purpose_id.to_string()))?.clone();
        let rt = self.cat.get("runtimes").and_then(|p| p.get(rq.requested_runtime)).ok_or(("unknown_runtime", rq.requested_runtime.to_string()))?.clone();
        let init = self.cat.get("initiators").and_then(|p| p.get(rq.initiator_credential_ref)).ok_or(("initiator_unauthenticated", String::new()))?.clone();
        if init.get("enabled").and_then(|x| x.as_bool()) != Some(true) { return rej("initiator_disabled", ""); }
        // local credential mechanism (§3.1): the initiator reference must belong to the connecting UID
        if init.get("uid").and_then(|x| x.as_int()) != Some(peer_uid as i64) { return rej("initiator_unauthenticated", "credential reference not bound to caller"); }
        let relationship = init.get("relationship").and_then(|x| x.as_str()).unwrap_or("delegation");
        if relationship == "scheduled" && init.get("owner").map(|o| o.is_null()).unwrap_or(true) { return rej("scheduled_without_owner", ""); }
        // Auth_session ⊆ Auth_agent ∩ Task (R-ID-3): runtime and resources must be permitted by both
        if !strs(pr.get("runtimes")).contains(&rq.requested_runtime) || !strs(tk.get("runtimes")).contains(&rq.requested_runtime) { return rej("authority_exceeded", format!("runtime {}", rq.requested_runtime)); }
        let (ag_res, tk_res) = (strs(pr.get("resources")), strs(tk.get("resources")));
        for r in &rq.requested_resources {
            if !ag_res.contains(r) || !tk_res.contains(r) { return rej("authority_exceeded", format!("resource {r}")); }
            if self.cat.get("resources").and_then(|x| x.get(r)).is_none() { return rej("unknown_resource", r.to_string()); }
        }
        // approvals (R-ID-4): required count, resolvable, approve, unexpired, subject-bound, per-key monotonic sequence
        let need = tk.get("approvals_required").and_then(|x| x.as_int()).unwrap_or(0);
        let mut approvers = Vec::new(); let mut seqs = Vec::new();
        for aref in &rq.approval_references {
            let ap = self.cat.get("approvals").and_then(|a| a.get(aref)).ok_or(("approval_missing", aref.to_string()))?;
            let exp = ap.get("expires_at").and_then(|x| x.as_str()).and_then(parse_rfc3339).ok_or(("approval_missing", "no expiry".into()))?;
            if exp <= now { return rej("approval_expired", aref.to_string()); }
            if ap.get("subject").and_then(|x| x.as_str()) != Some(rq.task_purpose_id) { return rej("approval_missing", "subject differs"); }
            if ap.get("decision").and_then(|x| x.as_str()) != Some("approve") { return rej("approval_missing", "not an approval"); }
            let (key, seq) = (ap.get("approver_key").and_then(|x| x.as_str()).unwrap_or(""), ap.get("sequence").and_then(|x| x.as_int()).unwrap_or(0));
            if seq <= self.highest_seq(key) || seqs.iter().any(|(k, _): &(String, i64)| k == key) { return rej("approval_replayed", format!("{key} seq {seq}")); }
            seqs.push((key.to_string(), seq));
            approvers.push(Value::obj(vec![("approval_reference", Value::s(aref)), ("decision", Value::s("approve")), ("expires_at", Value::s(&fmt_rfc3339(exp))), ("id", Value::s(ap.get("approver_id").and_then(|x| x.as_str()).unwrap_or("")))]));
        }
        if (approvers.len() as i64) < need { return rej("approval_missing", format!("{need} required")); }
        // budget: requested ≤ agent budget per class (R-ID-3/R-ID-5); requested lower bound narrows the enforced limit
        let mut limits = self.cat.get("resource_limits").cloned().ok_or(("catalogue_invalid", "resource_limits".into()))?;
        let agent_budget = pr.get("budget").cloned().unwrap_or(Value::obj(vec![]));
        if let Some(b) = rq.budget { for (k, v) in b.as_obj().unwrap() {
            let n = v.as_int().unwrap();
            if agent_budget.get(&k.0).and_then(|x| x.as_int()).map(|c| n > c).unwrap_or(true) { return rej("budget_exceeds_policy", k.0.clone()); }
            if let Some(cls) = match k.0.as_str() { "pids" | "memory_bytes" | "disk_bytes" | "disk_inodes" => Some(k.0.as_str()), _ => None } {
                if let Some(mut l) = limits.get(cls).cloned() { if l.get("status").and_then(|s| s.as_str()) == Some("enforced") && l.get("limit").and_then(|x| x.as_int()).map(|cur| n < cur).unwrap_or(false) { l.set("limit", Value::Int(n)); limits.set(cls, l); } }
            }
        } }
        let revocation = tk.get("revocation").cloned().unwrap_or_else(|| Value::obj(REVOCATION_TRIGGERS.iter().map(|t| (*t, Value::s("terminate"))).collect()));
        let mounts: Vec<Value> = rq.requested_resources.iter().filter_map(|r| self.cat.get("resources").and_then(|x| x.get(r))).filter(|r| r.get("mount_id").is_some())
            .map(|r| Value::obj(vec![("access", r.get("access").cloned().unwrap_or(Value::s("read-only"))), ("catalogue_id", r.get("catalogue_id").cloned().unwrap_or(Value::Null)), ("mount_id", r.get("mount_id").cloned().unwrap_or(Value::Null)), ("required", Value::Bool(true)), ("target_template_id", r.get("target_template_id").cloned().unwrap_or(Value::Null))])).collect();
        // gateway (1B): operations are the intersection of the task's and the agent's operation ids; each names a catalogue operation.
        // Topology is `local-socket` iff at least one operation results; grants follow the operation set (manifest-schema §3.4).
        let (ag_ops, tk_ops) = (strs(pr.get("operations")), strs(tk.get("operations")));
        let mut gw_ops = Vec::new();
        for oid in tk_ops.iter().filter(|o| ag_ops.contains(o)) {
            let o = self.cat.get("operations").and_then(|x| x.get(oid)).ok_or(("catalogue_invalid", format!("operation {oid}")))?;
            gw_ops.push(Value::obj(vec![("adapter_catalogue_id", o.get("adapter_catalogue_id").cloned().unwrap_or(Value::Null)), ("budgets", o.get("budgets").cloned().unwrap_or(Value::obj(vec![]))), ("operation", o.get("operation").cloned().unwrap_or(Value::Null)), ("operation_id", Value::s(oid)), ("scope", o.get("scope").cloned().unwrap_or(Value::obj(vec![])))]));
        }
        let topology = if gw_ops.is_empty() { "none" } else { "local-socket" };
        let grants: Vec<Value> = if gw_ops.is_empty() { vec![] } else { strs(tk.get("grants")).iter().filter_map(|g| self.cat.get("grants").and_then(|x| x.get(g)).map(|gr| (g.to_string(), gr.clone()))).map(|(gid, gr)| Value::obj(vec![("expiry_policy", gr.get("expiry_policy").cloned().unwrap_or(Value::s("session"))), ("grant_id", Value::s(&gid)), ("kind", gr.get("kind").cloned().unwrap_or(Value::s("git-credential"))), ("operation_subset", Value::Arr(strs(gr.get("operation_subset")).iter().filter(|o| gw_ops.iter().any(|x| x.get("operation_id").and_then(|y| y.as_str()) == Some(*o))).map(|o| Value::s(o)).collect()))])).collect() };
        if topology == "local-socket" && grants.is_empty() { return rej("catalogue_invalid", "local-socket without a grant"); }
        let gw_budgets = if gw_ops.is_empty() { Value::obj(vec![]) } else { self.cat.get("gateway_budgets").cloned().unwrap_or(Value::obj(vec![("connection_count", Value::Int(16))])) };
        // manifest-schema §3.4/binding rules: the gateway socket is a projected mount (correspondence-3), so the intent is declared here
        let mut mounts = mounts;
        if topology == "local-socket" { mounts.push(Value::obj(vec![("access", Value::s("read-only")), ("catalogue_id", Value::s("mount-source:gateway-socket")), ("mount_id", Value::s("mount:gateway-socket")), ("required", Value::Bool(true)), ("target_template_id", Value::s("mount-target:gateway-socket"))])); }
        let n = self.store_scan("authorization").len() + 1;
        let authz = format!("launchrec:{}-{:06}", rq.task_purpose_id.split(':').nth(1).unwrap_or("t").replace('/', "-"), n);
        let session_id = format!("session:{}", &sha256_hex(authz.as_bytes())[7..23]); let trace_id = format!("trace:{}", &sha256_hex(session_id.as_bytes())[7..39]);
        let init_id = init.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let inputs = Value::Arr(vec![Value::obj(vec![("id", Value::s(rq.agent_principal_id)), ("kind", Value::s("agent")), ("version", Value::s(self.cat_str("agent_authority_version")))]),
            Value::obj(vec![("id", Value::s(&init_id)), ("kind", Value::s("initiator")), ("version", Value::s(rq.initiator_credential_ref))]),
            Value::obj(vec![("id", Value::s(rq.task_purpose_id)), ("kind", Value::s("task")), ("version", Value::s(tk.get("version").and_then(|x| x.as_str()).unwrap_or("task:v1")))])]);
        let m = Value::obj(vec![
            ("actors", Value::obj(vec![("approvers", Value::Arr(approvers)), ("initiators", Value::Arr(vec![Value::obj(vec![("credential_reference", Value::s(rq.initiator_credential_ref)), ("id", Value::s(&init_id)), ("relationship", Value::s(relationship))])])),
                ("owner", init.get("owner").cloned().unwrap_or(Value::Null)), ("scheduler", if relationship == "scheduled" { Value::s(&init_id) } else { Value::Null })])),
            ("agent", Value::obj(vec![("durable_ownership_projection", pr.get("durable_ownership_projection").cloned().unwrap_or(Value::Null)), ("global_id", Value::s(rq.agent_principal_id))])),
            ("audit", Value::obj(vec![("correlation_keys", Value::arr_of_str(&["authorization_id", "trace_id", "agent_global_id", "execution_allocation_id", "execution_uid_boot"])), ("loss_behaviour", tk.get("audit_loss_behaviour").cloned().unwrap_or(Value::s("quarantine"))), ("required_events", Value::arr_of_str(&["launch", "revocation", "termination"]))])),
            ("authorization_id", Value::s(&authz)), ("credential_grant_intents", Value::Arr(grants)),
            ("derivation", Value::obj(vec![("agent_authority_version", Value::s(self.cat_str("agent_authority_version"))), ("catalogue_version", Value::s(self.cat_str("catalogue_version"))), ("derivation_input_digest", Value::s(&object_digest(&inputs))), ("derivation_relation_version", Value::s("derive:v0.1")), ("inputs", inputs),
                ("policy_version", Value::s(self.cat_str("policy_version"))), ("requested_budget_digest", Value::s(&object_digest(rq.budget.unwrap_or(&Value::obj(vec![]))))), ("resolved_resource_ids", Value::Arr(rq.requested_resources.iter().map(|r| Value::s(r)).collect()))])),
            ("execution_binding", Value::obj(vec![("adapters", Value::Arr(vec![])), ("endpoint", Value::Null), ("inference_pool", Value::Null), ("model", Value::Null), ("retention_mode", Value::s("retention:ephemeral")), ("tenant", Value::Null)])),
            ("gateway", Value::obj(vec![("budgets", gw_budgets), ("channel_topology", Value::s(topology)), ("operations", Value::Arr(gw_ops))])),
            ("manifest_version", Value::s(schema::MANIFEST_VERSION)), ("mount_intents", Value::Arr(mounts)), ("resource_limits", limits), ("revocation", revocation),
            ("runtime", Value::obj(vec![("artifact_digest", rt.get("artifact_digest").cloned().unwrap_or(Value::Null)), ("catalogue_id", Value::s(rq.requested_runtime)), ("invocation_profile", rt.get("invocation_profile").cloned().unwrap_or(Value::Null))])),
            ("session_trace", Value::obj(vec![("session_id", Value::s(&session_id)), ("trace_id", Value::s(&trace_id))])),
            ("task", Value::obj(vec![("approval_references", Value::Arr(rq.approval_references.iter().map(|r| Value::s(r)).collect())), ("purpose_id", Value::s(rq.task_purpose_id))])),
            ("termination_retention", Value::obj(vec![("audit_retention_class", Value::s("retention:phase1")), ("credential_revocation_order", Value::s("revoke-before-cleanup")), ("descendant_kill_order", Value::s("children-before-parent")), ("reclamation_domain_id", Value::s("domain:session-default")), ("termination_triggers", Value::arr_of_str(&["task_cancelled", "approval_expired"])), ("workspace_retention", Value::s("discard"))])),
        ]);
        // the validator enforces, among others, continue-degraded-only-where-permitted (§3.6) on the task's map
        schema::validate_manifest(&m).map_err(|e| (if e.rule == "continue-degraded-not-permitted" { "continue_degraded_not_permitted" } else { "manifest_invalid" }, e.to_string()))?;
        for (key, seq) in seqs { self.store_append("approval_seq", Value::obj(vec![("key", Value::s(&key)), ("seq", Value::Int(seq))]))?; }
        let env = envelope::policy_envelope(&self.signer, &m, &authz, now);
        self.store_append("authorization", Value::obj(vec![("authorization_id", Value::s(&authz)), ("envelope", env.clone()), ("manifest_digest", Value::s(&object_digest(&m)))]))?;
        // wire-format §5: spool handoff, mode 0640 (group = constructor identity), atomic rename
        let pair = canonical(&Value::obj(vec![("authorization_manifest", m.clone()), ("envelope", env.clone())]));
        let tmp = format!("{}/.{authz}.tmp", self.spool); let fin = format!("{}/{authz}.manifest.json", self.spool);
        { use std::os::unix::fs::OpenOptionsExt; let mut f = std::fs::OpenOptions::new().create_new(true).write(true).mode(0o640).open(&tmp).map_err(|e| ("spool_unavailable", e.to_string()))?; f.write_all(&pair).and_then(|_| f.sync_all()).map_err(|e| ("spool_unavailable", e.to_string()))?; }
        std::fs::rename(&tmp, &fin).map_err(|e| ("spool_unavailable", e.to_string()))?;
        Ok((authz, m, env))
    }

    fn serve(&mut self, conn: wire::Conn) {
        let Ok(Some(msg)) = conn.recv() else { return };
        let reply = match wire::parse_request(&msg) {
            Err(e) => wire::reply_err(wire::CLASS_INVALID, "envelope", e),
            Ok(_) if !self.cli_uids.contains(&conn.peer.uid) => wire::reply_err(wire::CLASS_UNAUTHENTICATED, "peer_not_permitted", ""),
            Ok(r) => match r.op {
                "submit_request" => {
                    let body_req = r.body.get("request").cloned().unwrap_or(Value::Null);
                    let c = audit::Correlation::default();
                    self.audit.emit(&audit::event("session.requested", "agentbound-policy", "ok", &c, Value::obj(vec![("peer_uid", Value::Int(conn.peer.uid as i64)), ("request_digest", Value::s(&object_digest(&body_req)))])));
                    match self.derive(&canonical(&body_req), conn.peer.uid) {
                        Ok((az, m, env)) => {
                            let c = audit::Correlation { authorization_id: Some(az.clone()), ..Default::default() };
                            self.audit.emit(&audit::event("session.authorized", "agentbound-policy", "ok", &c, Value::obj(vec![("manifest_digest", Value::s(&object_digest(&m)))])));
                            wire::reply_ok(Value::obj(vec![("authorization_id", Value::s(&az)), ("authorization_manifest", m), ("envelope", env), ("state", Value::s("authorized"))]))
                        }
                        Err((rule, d)) => {
                            self.audit.emit(&audit::event("session.rejected", "agentbound-policy", "reject", &c, Value::obj(vec![("failed_input", Value::s(rule)), ("request_digest", Value::s(&object_digest(&body_req)))])));
                            wire::reply_err("reject", rule, &d)
                        }
                    }
                }
                "request_status" => {
                    let az = r.body.get("authorization_id").and_then(|x| x.as_str()).unwrap_or("");
                    let known = self.store_scan("authorization").iter().any(|a| a.get("authorization_id").and_then(|x| x.as_str()) == Some(az));
                    wire::reply_ok(Value::obj(vec![("reason", Value::Null), ("state", Value::s(if known { "authorized" } else { "rejected" }))]))
                }
                op => wire::reply_err(wire::CLASS_INVALID, "unknown_op", op),
            },
        };
        let _ = conn.send(&reply);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("keygen") {
        // agentbound-policy keygen <seed-path> <key_id> <role>  → prints the keyring entry
        let s = Signer_::generate(&args[3]);
        { use std::os::unix::fs::OpenOptionsExt; std::fs::OpenOptions::new().create_new(true).write(true).mode(0o600).open(&args[2]).unwrap().write_all(&s.seed()).unwrap(); }
        println!("{{\"key_id\":\"{}\",\"public_key\":\"{}\",\"not_before\":0,\"not_after\":4102444800,\"role\":\"{}\",\"status\":\"active\"}}", args[3], s.public_hex(), args[4]);
        return;
    }
    let arg = |k: &str, d: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned().unwrap_or_else(|| d.to_string());
    let cat = json::parse(&std::fs::read(arg("--catalogue", "/etc/agentbound/catalogue.json")).expect("catalogue"), &MANIFEST_LIMITS).expect("catalogue parse");
    let seed = std::fs::read(arg("--key", "/etc/agentbound/policy.key")).expect("policy key");
    let signer = Signer_::from_seed(&arg("--key-id", "key:policy-ed25519-01"), &seed).expect("key");
    let cli_uids = arg("--cli-uids", "").split(',').filter_map(|s| s.parse().ok()).collect();
    let mut p = Policy { cat, signer, spool: arg("--spool", "/var/lib/agentbound/spool"), store_path: arg("--store", "/var/lib/agentbound/policy.jsonl"), audit: audit::Sink::open(&arg("--audit-spool", "/var/lib/agentbound/audit-policy.jsonl")), cli_uids };
    let listener = wire::listen(&arg("--socket", "/run/agentbound/policy.sock"), 0o660).expect("listen");
    loop { if let Ok(c) = wire::accept(&listener) { p.serve(c); } }
}
