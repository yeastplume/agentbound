//! agentbound-audit: unprivileged receiver (R-AUD-1..4). Hash-chained
//! append-only store, dedup by event_id, closed per-kind detail schema,
//! capacity accounting with a host-global loss counter.
mod events;
use ab_common::json::{self, canonical, Value, MANIFEST_LIMITS};
use ab_common::sig::{object_digest, sha256_hex};
use ab_common::wire;
use std::io::Write;

struct Audit { path: String, prev: String, seq: i64, seen: std::collections::HashSet<String>, capacity: i64, lost: u64, writers: Vec<u32> }

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
        Audit { path: path.into(), prev, seq, seen, capacity, lost: 0, writers }
    }
    fn append(&mut self, ev: &Value) -> Result<i64, &'static str> {
        if self.seq >= self.capacity { self.lost += 1; return Err("capacity"); }
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
                            Err("capacity") => wire::reply_err("audit-loss", "capacity_exhausted", &format!("lost={}", self.lost)), Err(e) => wire::reply_err(wire::CLASS_UNAVAILABLE, e, "") } }
                    }
                }
            }
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
    let args: Vec<String> = std::env::args().collect();
    let arg = |k: &str, d: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned().unwrap_or_else(|| d.to_string());
    let writers = arg("--writer-uids", "0").split(',').filter_map(|s| s.parse().ok()).collect();
    let mut a = Audit::open(&arg("--store", "/var/lib/agentbound/audit/events.jsonl"), arg("--capacity", "1000000").parse().unwrap_or(1_000_000), writers);
    let l = wire::listen(&arg("--socket", "/run/agentbound/audit.sock"), 0o666).expect("listen");
    loop { if let Ok(c) = wire::accept(&l) { a.serve(c); } }
}
