//! agentbound-audit: unprivileged receiver (R-AUD-1..4). Hash-chained
//! append-only store, dedup by event_id, closed per-kind detail schema,
//! capacity accounting with a host-global loss counter.
mod events;
use ab_common::json::{self, canonical, Value, MANIFEST_LIMITS};
use ab_common::sig::{object_digest, sha256_hex};
use ab_common::wire;
use std::io::Write;

struct Audit { path: String, prev: String, seq: i64, seen: std::collections::HashSet<String>, capacity: i64, lost: u64, writers: Vec<u32>,
    /// per-session event budget (R-RES-2 `audit_capacity`): authorization_id → (installed capacity, events accepted, events lost)
    sessions: std::collections::HashMap<String, (i64, i64, i64)> }

impl Audit {
    fn open(path: &str, capacity: i64, writers: Vec<u32>) -> Audit {
        let (mut prev, mut seq, mut seen) = ("sha256:".to_string() + &"0".repeat(64), 0i64, std::collections::HashSet::new());
        for l in std::fs::read_to_string(path).unwrap_or_default().lines() {
            let Ok(r) = json::parse(l.as_bytes(), &MANIFEST_LIMITS) else { eprintln!("audit store: unparseable line; refusing to start"); std::process::exit(3) };
            let (Some(p), Some(n), Some(ev)) = (r.get("prev").and_then(|x| x.as_str()), r.get("seq").and_then(|x| x.as_int()), r.get("event")) else { std::process::exit(3) };
            if p != prev || n != seq + 1 { eprintln!("audit store: chain break at seq {n}; refusing to start"); std::process::exit(3); }
            if let Some(id) = ev.get("event_id").and_then(|x| x.as_str()) { seen.insert(id.to_string()); }
            let mut b = prev.as_bytes().to_vec(); b.extend(canonical(ev)); prev = sha256_hex(&b); seq = n;
        }
        Audit { path: path.into(), prev, seq, seen, capacity, lost: 0, writers, sessions: std::collections::HashMap::new() }
    }
    fn append(&mut self, ev: &Value) -> Result<i64, &'static str> {
        if self.seq >= self.capacity { self.lost += 1; return Err("capacity"); }
        // per-session bound: an event correlated to a reserved session counts against that session's installed capacity
        if let Some(az) = ev.get("authorization_id").and_then(|x| x.as_str()) {
            if let Some((cap, used, lost)) = self.sessions.get_mut(az) { if *used >= *cap { *lost += 1; return Err("session_capacity"); } *used += 1; }
        }
        let mut b = self.prev.as_bytes().to_vec(); b.extend(canonical(ev)); let h = sha256_hex(&b);
        let row = Value::obj(vec![("event", ev.clone()), ("prev", Value::s(&self.prev)), ("seq", Value::Int(self.seq + 1))]);
        let mut line = canonical(&row); line.push(b'\n');
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&self.path).map_err(|_| "store")?;
        f.write_all(&line).and_then(|_| f.sync_data()).map_err(|_| "store")?;
        self.prev = h; self.seq += 1; Ok(self.seq)
    }
    fn serve(&mut self, conn: wire::Conn) {
        let Ok(Some(msg)) = conn.recv() else { return };
        let reply = match wire::parse_request(&msg) {
            Err(e) => wire::reply_err(wire::CLASS_INVALID, "envelope", e),
            Ok(_) if !self.writers.contains(&conn.peer.uid) => wire::reply_err(wire::CLASS_UNAUTHENTICATED, "peer_not_permitted", ""),
            Ok(r) if r.op == "emit" => {
                let ev = r.body.clone();
                match events::check(&ev) {
                    Err(rule) => wire::reply_err(wire::CLASS_INVALID, rule, ""),
                    Ok(()) => {
                        let id = ev.get("event_id").unwrap().as_str().unwrap().to_string();
                        let mut without = ev.clone(); without.as_obj_mut().unwrap().retain(|k, _| k.0 != "event_id");
                        if object_digest(&without) != id { wire::reply_err(wire::CLASS_INVALID, "event_id_mismatch", "") }
                        else if self.seen.contains(&id) { wire::reply_ok(Value::obj(vec![("accepted", Value::Bool(true)), ("duplicate", Value::Bool(true))])) }
                        else { match self.append(&ev) { Ok(seq) => { self.seen.insert(id); wire::reply_ok(Value::obj(vec![("accepted", Value::Bool(true)), ("seq", Value::Int(seq))])) }
                            Err("capacity") => wire::reply_err("audit-loss", "capacity_exhausted", &format!("lost={}", self.lost)),
                            Err("session_capacity") => wire::reply_err("audit-loss", "session_capacity_exhausted", ev.get("authorization_id").and_then(|x| x.as_str()).unwrap_or("")),
                            Err(e) => wire::reply_err(wire::CLASS_UNAVAILABLE, e, "") } }
                    }
                }
            }
            // constructor (root) reserves a session's event budget before commit and reads back what was installed
            Ok(r) if r.op == "reserve" && conn.peer.uid == 0 => match (r.body.get("authorization_id").and_then(|x| x.as_str()), r.body.get("capacity").and_then(|x| x.as_int())) {
                (Some(az), Some(c)) if c > 0 => { let inst = c.min(self.capacity - self.seq).max(0); if inst == 0 { wire::reply_err("audit-loss", "capacity_exhausted", "no headroom for a session budget") } else { self.sessions.insert(az.to_string(), (inst, 0, 0)); wire::reply_ok(Value::obj(vec![("authorization_id", Value::s(az)), ("installed", Value::Int(inst))])) } }
                _ => wire::reply_err(wire::CLASS_INVALID, "body", "authorization_id, capacity") },
            Ok(r) if r.op == "release" && conn.peer.uid == 0 => { let az = r.body.get("authorization_id").and_then(|x| x.as_str()).unwrap_or(""); let s = self.sessions.remove(az); wire::reply_ok(Value::obj(vec![("released", Value::Bool(s.is_some())), ("used", Value::Int(s.map(|x| x.1).unwrap_or(0))), ("lost", Value::Int(s.map(|x| x.2).unwrap_or(0)))])) }
            Ok(r) if r.op == "status" => wire::reply_ok(Value::obj(vec![("capacity", Value::Int(self.capacity)), ("head", Value::s(&self.prev)), ("lost", Value::Int(self.lost as i64)), ("seq", Value::Int(self.seq))])),
            Ok(r) if r.op == "query" => { // by authorization_id or launch_record_digest; CLI-facing
                let (k, v) = if let Some(a) = r.body.get("authorization_id").and_then(|x| x.as_str()) { ("authorization_id", a) } else { ("launch_record_digest", r.body.get("launch_record_digest").and_then(|x| x.as_str()).unwrap_or("")) };
                let rows: Vec<Value> = std::fs::read_to_string(&self.path).unwrap_or_default().lines().filter_map(|l| json::parse(l.as_bytes(), &MANIFEST_LIMITS).ok()).filter(|row| row.get("event").and_then(|e| e.get(k)).and_then(|x| x.as_str()) == Some(v)).collect();
                wire::reply_ok(Value::obj(vec![("rows", Value::Arr(rows))])) }
            Ok(r) => wire::reply_err(wire::CLASS_INVALID, "unknown_op", r.op),
        };
        let _ = conn.send(&reply);
    }
}

fn main() {
    // `--provenance`: print the source provenance embedded at build time and exit (independent WP3.1 validation, finding 4). The
    // conformance runner asks every installed binary, so a stale install is visible as a commit mismatch rather than hidden.
    if std::env::args().nth(1).as_deref() == Some("--provenance") { println!("commit={} dirty={} tree={}", ab_common::provenance::COMMIT, ab_common::provenance::DIRTY, ab_common::provenance::TREE); return; }
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str, d: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned().unwrap_or_else(|| d.to_string());
    let writers = arg("--writer-uids", "0").split(',').filter_map(|s| s.parse().ok()).collect();
    let mut a = Audit::open(&arg("--store", "/var/lib/agentbound/audit/events.jsonl"), arg("--capacity", "1000000").parse().unwrap_or(1_000_000), writers);
    let l = wire::listen(&arg("--socket", "/run/agentbound/audit.sock"), 0o666).expect("listen");
    loop { if let Ok(c) = wire::accept(&l) { a.serve(c); } }
}
