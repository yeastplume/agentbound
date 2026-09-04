//! Lifecycle-owned durable stores (execution-identity lifecycle §3.2–§4,
//! component interfaces §4.2–§4.4; WP1 ID-1 design).
//!
//! Both stores are append-only row streams in one SQLite database
//! (WAL, `synchronous=FULL`); every row carries `prev_hash` and `hash =
//! SHA-256(prev_hash || canonical row)`. State mutation is compare-and-set on
//! `(allocation_id, state_seq)`. Nothing is ever updated or deleted.

use ab_common::json::{canonical, Value};
use ab_common::sig::{fmt_rfc3339, monotonic_ns, now_unix, sha256_hex};
use rusqlite::{params, Connection, OptionalExtension};
use std::fmt;

#[derive(Debug)]
pub enum StoreError { Db(String), Conflict(String), Exhausted, RangeOverlap(String), Chain(String), Clock }
impl fmt::Display for StoreError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") } }
impl std::error::Error for StoreError {}
impl From<rusqlite::Error> for StoreError { fn from(e: rusqlite::Error) -> Self { StoreError::Db(e.to_string()) } }
type R<T> = Result<T, StoreError>;

pub const STATES: [&str; 5] = ["allocated", "in-use", "reclaiming", "quarantined", "free"];
pub fn next_ok(from: &str, to: &str) -> bool {
    matches!((from, to), ("free", "allocated") | ("allocated", "in-use") | ("allocated", "reclaiming") | ("in-use", "reclaiming") | ("reclaiming", "quarantined") | ("quarantined", "free"))
}

#[derive(Clone, Debug)]
pub struct Range { pub lo: u32, pub hi: u32 }
impl Default for Range { fn default() -> Self { Range { lo: 200_000, hi: 299_999 } } }

#[derive(Clone, Debug, PartialEq)]
pub struct Alloc {
    pub seq: i64, pub allocation_id: String, pub uid: u32, pub gid: u32, pub state: String, pub state_seq: i64,
    pub authorization_id: String, pub manifest_digest: String, pub agent_global_id: String, pub session_id: String, pub trace_id: String,
    pub scope_id: Option<String>, pidns_id: Option<String>, pub domain_id: String, pub evidence: String, pub wall_clock: String, pub hash: String,
}
impl Alloc { pub fn pidns_id(&self) -> Option<&str> { self.pidns_id.as_deref() } }

/// Immediate transaction guard: rolls back unless committed.
struct Tx<'a>(&'a Connection);
impl<'a> Tx<'a> { fn commit(self) -> rusqlite::Result<()> { let r = self.0.execute_batch("COMMIT"); std::mem::forget(self); r } }
impl<'a> Drop for Tx<'a> { fn drop(&mut self) { let _ = self.0.execute_batch("ROLLBACK"); } }

pub struct Store { c: Connection, pub range: Range, host_id: String, boot_id: String, pub quarantine_floor_s: i64 }

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS alloc(seq INTEGER PRIMARY KEY, allocation_id TEXT NOT NULL, uid INTEGER NOT NULL, gid INTEGER NOT NULL,
  state TEXT NOT NULL, state_seq INTEGER NOT NULL, authorization_id TEXT NOT NULL, manifest_digest TEXT NOT NULL, agent_global_id TEXT NOT NULL,
  session_id TEXT NOT NULL, trace_id TEXT NOT NULL, host_id TEXT NOT NULL, boot_id TEXT NOT NULL, scope_id TEXT, pidns_id TEXT, domain_id TEXT NOT NULL,
  actor TEXT NOT NULL, wall_clock TEXT NOT NULL, monotonic_ns INTEGER NOT NULL, evidence TEXT NOT NULL, prev_hash TEXT NOT NULL, hash TEXT NOT NULL,
  UNIQUE(allocation_id, state_seq));
CREATE INDEX IF NOT EXISTS alloc_id ON alloc(allocation_id, seq);
CREATE TABLE IF NOT EXISTS record(seq INTEGER PRIMARY KEY, kind TEXT NOT NULL, allocation_id TEXT NOT NULL, launch_record_digest TEXT NOT NULL,
  authorization_id TEXT NOT NULL, payload TEXT NOT NULL, wall_clock TEXT NOT NULL, prev_hash TEXT NOT NULL, hash TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS record_lrd ON record(launch_record_digest, seq);
CREATE TABLE IF NOT EXISTS idem(scope TEXT NOT NULL, key TEXT NOT NULL, body_digest TEXT NOT NULL, reply TEXT NOT NULL, PRIMARY KEY(scope, key));
";

impl Store {
    pub fn open(path: &str, range: Range, host_id: &str, boot_id: &str) -> R<Store> {
        let c = Connection::open(path)?;
        c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;")?;
        c.execute_batch(SCHEMA)?;
        let _ = std::fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(0o600));
        let s = Store { c, range, host_id: host_id.into(), boot_id: boot_id.into(), quarantine_floor_s: 24 * 3600 };
        s.check_range_disjoint()?;
        s.verify_chain()?;
        Ok(s)
    }

    /// §3.1: the range must not overlap any local account or group.
    fn check_range_disjoint(&self) -> R<()> {
        for (file, col) in [("/etc/passwd", 2usize), ("/etc/group", 2usize)] {
            if let Ok(t) = std::fs::read_to_string(file) {
                for l in t.lines() { if let Some(id) = l.split(':').nth(col).and_then(|x| x.parse::<u32>().ok()) {
                    if id >= self.range.lo && id <= self.range.hi { return Err(StoreError::RangeOverlap(format!("{file} has id {id}"))); } } }
            }
        }
        if self.range.lo < 1000 || self.range.hi < self.range.lo { return Err(StoreError::RangeOverlap("range below 1000 or inverted".into())); }
        Ok(())
    }

    fn last_hash(&self, table: &str) -> R<String> {
        Ok(self.c.query_row(&format!("SELECT hash FROM {table} ORDER BY seq DESC LIMIT 1"), [], |r| r.get::<_, String>(0)).optional()?.unwrap_or_else(|| "sha256:".to_string() + &"0".repeat(64)))
    }
    fn row_hash(prev: &str, row: &Value) -> String { let mut b = prev.as_bytes().to_vec(); b.extend(canonical(row)); sha256_hex(&b) }

    /// Latest state row per allocation id.
    pub fn latest(&self, allocation_id: &str) -> R<Option<Alloc>> {
        let mut st = self.c.prepare("SELECT seq,allocation_id,uid,gid,state,state_seq,authorization_id,manifest_digest,agent_global_id,session_id,trace_id,scope_id,pidns_id,domain_id,evidence,wall_clock,hash FROM alloc WHERE allocation_id=?1 ORDER BY seq DESC LIMIT 1")?;
        Ok(st.query_row([allocation_id], row_to_alloc).optional()?)
    }
    pub fn by_authorization(&self, authorization_id: &str) -> R<Option<Alloc>> {
        let id: Option<String> = self.c.query_row("SELECT allocation_id FROM alloc WHERE authorization_id=?1 ORDER BY seq ASC LIMIT 1", [authorization_id], |r| r.get(0)).optional()?;
        match id { Some(id) => self.latest(&id), None => Ok(None) }
    }
    /// Every allocation whose latest state is not `free`.
    pub fn nonfree(&self) -> R<Vec<Alloc>> {
        let mut st = self.c.prepare("SELECT seq,allocation_id,uid,gid,state,state_seq,authorization_id,manifest_digest,agent_global_id,session_id,trace_id,scope_id,pidns_id,domain_id,evidence,wall_clock,hash FROM alloc a WHERE seq=(SELECT MAX(seq) FROM alloc b WHERE b.allocation_id=a.allocation_id) AND state!='free' ORDER BY uid")?;
        let v = st.query_map([], row_to_alloc)?.collect::<Result<Vec<_>, _>>()?; Ok(v)
    }

    fn append_alloc(&self, a: &Alloc, actor: &str, prev_state: &str) -> R<Alloc> {
        if !next_ok(prev_state, &a.state) { return Err(StoreError::Conflict(format!("illegal transition {prev_state} → {}", a.state))); }
        let now = now_unix().map_err(|_| StoreError::Clock)?; let wall = fmt_rfc3339(now); let mono = monotonic_ns();
        let prev = self.last_hash("alloc")?;
        let row = Value::obj(vec![("actor", Value::s(actor)), ("agent_global_id", Value::s(&a.agent_global_id)), ("allocation_id", Value::s(&a.allocation_id)), ("authorization_id", Value::s(&a.authorization_id)),
            ("boot_id", Value::s(&self.boot_id)), ("domain_id", Value::s(&a.domain_id)), ("evidence", Value::s(&a.evidence)), ("gid", Value::Int(a.gid as i64)), ("host_id", Value::s(&self.host_id)),
            ("manifest_digest", Value::s(&a.manifest_digest)), ("monotonic_ns", Value::Int(mono)), ("pidns_id", a.pidns_id.as_deref().map(Value::s).unwrap_or(Value::Null)), ("scope_id", a.scope_id.as_deref().map(Value::s).unwrap_or(Value::Null)),
            ("session_id", Value::s(&a.session_id)), ("state", Value::s(&a.state)), ("state_seq", Value::Int(a.state_seq)), ("trace_id", Value::s(&a.trace_id)), ("uid", Value::Int(a.uid as i64)), ("wall_clock", Value::s(&wall))]);
        let hash = Self::row_hash(&prev, &row);
        let r = self.c.execute("INSERT INTO alloc(allocation_id,uid,gid,state,state_seq,authorization_id,manifest_digest,agent_global_id,session_id,trace_id,host_id,boot_id,scope_id,pidns_id,domain_id,actor,wall_clock,monotonic_ns,evidence,prev_hash,hash) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)",
            params![a.allocation_id, a.uid, a.gid, a.state, a.state_seq, a.authorization_id, a.manifest_digest, a.agent_global_id, a.session_id, a.trace_id, self.host_id, self.boot_id, a.scope_id, a.pidns_id, a.domain_id, actor, wall, mono, a.evidence, prev, hash]);
        match r { Ok(_) => {}, Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation => return Err(StoreError::Conflict("state_seq already appended".into())), Err(e) => return Err(e.into()) }
        self.latest(&a.allocation_id)?.ok_or_else(|| StoreError::Db("append not visible".into()))
    }

    /// `free → allocated`: the allocation commit point (§4.3). Serialized by an IMMEDIATE transaction;
    /// one active allocation per authorization ID; lowest UID whose latest state is `free` (or never used).
    pub fn reserve(&mut self, authorization_id: &str, manifest_digest: &str, agent_global_id: &str, session_id: &str, trace_id: &str, domain_id: &str, actor: &str) -> R<Alloc> {
        self.c.execute_batch("BEGIN IMMEDIATE")?; let tx = Tx(&self.c);
        if let Some(existing) = self.by_authorization(authorization_id)? { if existing.state != "free" { return Err(StoreError::Conflict(format!("authorization already allocated: {}", existing.allocation_id))); } }
        let taken: Vec<u32> = self.nonfree()?.into_iter().map(|a| a.uid).collect();
        let uid = (self.range.lo..=self.range.hi).find(|u| taken.binary_search(u).is_err()).ok_or(StoreError::Exhausted)?;
        let n: i64 = self.c.query_row("SELECT COUNT(*) FROM alloc WHERE state='allocated'", [], |r| r.get(0))?;
        let allocation_id = format!("allocation:{}-{:08}", self.host_id.trim_start_matches("host:").chars().take(8).collect::<String>(), n + 1);
        let a = Alloc { seq: 0, allocation_id, uid, gid: uid, state: "allocated".into(), state_seq: 1, authorization_id: authorization_id.into(), manifest_digest: manifest_digest.into(),
            agent_global_id: agent_global_id.into(), session_id: session_id.into(), trace_id: trace_id.into(), scope_id: None, pidns_id: None, domain_id: domain_id.into(), evidence: String::new(), wall_clock: String::new(), hash: String::new() };
        let out = self.append_alloc(&a, actor, "free")?;
        tx.commit()?; Ok(out)
    }

    /// Compare-and-set state transition. `expected_state_seq` must equal the latest row's `state_seq`.
    pub fn transition(&mut self, allocation_id: &str, expected_state_seq: i64, to: &str, evidence: &str, scope_id: Option<&str>, pidns_id: Option<&str>, actor: &str) -> R<Alloc> {
        self.c.execute_batch("BEGIN IMMEDIATE")?; let tx = Tx(&self.c);
        let cur = self.latest(allocation_id)?.ok_or_else(|| StoreError::Conflict("unknown allocation".into()))?;
        if cur.state_seq != expected_state_seq { return Err(StoreError::Conflict(format!("state_seq {} != expected {expected_state_seq}", cur.state_seq))); }
        if to == "free" {
            let now = now_unix().map_err(|_| StoreError::Clock)?;
            let q_at = ab_common::sig::parse_rfc3339(&cur.wall_clock).ok_or(StoreError::Clock)?;
            if cur.state != "quarantined" || now < q_at + self.quarantine_floor_s { return Err(StoreError::Conflict("quarantine floor not elapsed".into())); }
        }
        let a = Alloc { state: to.into(), state_seq: cur.state_seq + 1, evidence: evidence.into(), scope_id: scope_id.map(str::to_string).or(cur.scope_id.clone()), pidns_id: pidns_id.map(str::to_string).or(cur.pidns_id.clone()), ..cur.clone() };
        let out = self.append_alloc(&a, actor, &cur.state)?;
        tx.commit()?; Ok(out)
    }

    // ---- launch-record store (§4.2) ----
    pub fn record_exists(&self, kind: &str, key_col: &str, key: &str) -> R<bool> {
        Ok(self.c.query_row(&format!("SELECT 1 FROM record WHERE kind=?1 AND {key_col}=?2 LIMIT 1"), params![kind, key], |_| Ok(())).optional()?.is_some())
    }
    /// Append a record row (`binding`, `event`, `seal`, `correction`). A second `binding` for the same
    /// allocation, authorization ID, or digest is a `Conflict` (§5).
    pub fn append_record(&mut self, kind: &str, allocation_id: &str, launch_record_digest: &str, authorization_id: &str, payload: &Value) -> R<i64> {
        self.c.execute_batch("BEGIN IMMEDIATE")?; let tx = Tx(&self.c);
        if kind == "binding" {
            for (col, key) in [("allocation_id", allocation_id), ("authorization_id", authorization_id), ("launch_record_digest", launch_record_digest)] {
                if self.record_exists("binding", col, key)? { return Err(StoreError::Conflict(format!("binding already committed for {col}"))); }
            }
        }
        if kind == "seal" && self.record_exists("seal", "launch_record_digest", launch_record_digest)? { return Err(StoreError::Conflict("already sealed".into())); }
        if self.record_exists("seal", "launch_record_digest", launch_record_digest)? && kind != "correction" { return Err(StoreError::Conflict("record sealed; only corrections may follow".into())); }
        let now = now_unix().map_err(|_| StoreError::Clock)?; let wall = fmt_rfc3339(now);
        let prev = self.last_hash("record")?;
        let row = Value::obj(vec![("allocation_id", Value::s(allocation_id)), ("authorization_id", Value::s(authorization_id)), ("kind", Value::s(kind)), ("launch_record_digest", Value::s(launch_record_digest)), ("payload", payload.clone()), ("wall_clock", Value::s(&wall))]);
        let hash = Self::row_hash(&prev, &row);
        self.c.execute("INSERT INTO record(kind,allocation_id,launch_record_digest,authorization_id,payload,wall_clock,prev_hash,hash) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![kind, allocation_id, launch_record_digest, authorization_id, String::from_utf8(canonical(payload)).unwrap(), wall, prev, hash])?;
        let seq = self.c.last_insert_rowid(); tx.commit()?; Ok(seq)
    }
    pub fn records(&self, launch_record_digest: &str) -> R<Vec<(String, Value)>> {
        let mut st = self.c.prepare("SELECT kind,payload FROM record WHERE launch_record_digest=?1 ORDER BY seq")?;
        let v = st.query_map([launch_record_digest], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?.collect::<Result<Vec<_>, _>>()?;
        Ok(v.into_iter().map(|(k, p)| (k, ab_common::json::parse(p.as_bytes(), &ab_common::json::MANIFEST_LIMITS).unwrap())).collect())
    }
    pub fn bindings(&self) -> R<Vec<(String, String, String)>> {
        let mut st = self.c.prepare("SELECT allocation_id,launch_record_digest,authorization_id FROM record WHERE kind='binding' ORDER BY seq")?;
        let v = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<Result<Vec<_>, _>>()?; Ok(v)
    }

    // ---- idempotency (component interfaces §2) ----
    pub fn idem_lookup(&self, scope: &str, key: &str, body_digest: &str) -> R<Option<Result<Value, ()>>> {
        let r: Option<(String, String)> = self.c.query_row("SELECT body_digest,reply FROM idem WHERE scope=?1 AND key=?2", params![scope, key], |r| Ok((r.get(0)?, r.get(1)?))).optional()?;
        Ok(r.map(|(d, reply)| if d == body_digest { Ok(ab_common::json::parse(reply.as_bytes(), &ab_common::json::MANIFEST_LIMITS).unwrap()) } else { Err(()) }))
    }
    pub fn idem_store(&self, scope: &str, key: &str, body_digest: &str, reply: &Value) -> R<()> {
        self.c.execute("INSERT OR IGNORE INTO idem(scope,key,body_digest,reply) VALUES(?1,?2,?3,?4)", params![scope, key, body_digest, String::from_utf8(canonical(reply)).unwrap()])?; Ok(())
    }

    /// Recompute both hash chains (§8.1 step 1; §4.4 integrity failure ⇒ fail closed).
    pub fn verify_chain(&self) -> R<()> {
        let mut prev = "sha256:".to_string() + &"0".repeat(64);
        let mut st = self.c.prepare("SELECT actor,agent_global_id,allocation_id,authorization_id,boot_id,domain_id,evidence,gid,host_id,manifest_digest,monotonic_ns,pidns_id,scope_id,session_id,state,state_seq,trace_id,uid,wall_clock,prev_hash,hash FROM alloc ORDER BY seq")?;
        let rows = st.query_map([], |r| {
            let os = |i: usize| -> rusqlite::Result<Value> { Ok(r.get::<_, Option<String>>(i)?.map(|s| Value::s(&s)).unwrap_or(Value::Null)) };
            let row = Value::obj(vec![("actor", Value::s(&r.get::<_, String>(0)?)), ("agent_global_id", Value::s(&r.get::<_, String>(1)?)), ("allocation_id", Value::s(&r.get::<_, String>(2)?)), ("authorization_id", Value::s(&r.get::<_, String>(3)?)),
                ("boot_id", Value::s(&r.get::<_, String>(4)?)), ("domain_id", Value::s(&r.get::<_, String>(5)?)), ("evidence", Value::s(&r.get::<_, String>(6)?)), ("gid", Value::Int(r.get(7)?)), ("host_id", Value::s(&r.get::<_, String>(8)?)),
                ("manifest_digest", Value::s(&r.get::<_, String>(9)?)), ("monotonic_ns", Value::Int(r.get(10)?)), ("pidns_id", os(11)?), ("scope_id", os(12)?), ("session_id", Value::s(&r.get::<_, String>(13)?)), ("state", Value::s(&r.get::<_, String>(14)?)),
                ("state_seq", Value::Int(r.get(15)?)), ("trace_id", Value::s(&r.get::<_, String>(16)?)), ("uid", Value::Int(r.get(17)?)), ("wall_clock", Value::s(&r.get::<_, String>(18)?))]);
            Ok((row, r.get::<_, String>(19)?, r.get::<_, String>(20)?))
        })?;
        for (i, r) in rows.enumerate() { let (row, ph, h) = r?; if ph != prev || Self::row_hash(&ph, &row) != h { return Err(StoreError::Chain(format!("alloc row {i}"))); } prev = h; }
        let mut prev = "sha256:".to_string() + &"0".repeat(64);
        let mut st = self.c.prepare("SELECT allocation_id,authorization_id,kind,launch_record_digest,payload,wall_clock,prev_hash,hash FROM record ORDER BY seq")?;
        let rows = st.query_map([], |r| Ok(((0..6).map(|i| r.get::<_, String>(i)).collect::<Result<Vec<_>, _>>()?, r.get::<_, String>(6)?, r.get::<_, String>(7)?)))?;
        for (i, r) in rows.enumerate() {
            let (c, ph, h) = r?; let payload = ab_common::json::parse(c[4].as_bytes(), &ab_common::json::MANIFEST_LIMITS).map_err(|e| StoreError::Chain(e.to_string()))?;
            let row = Value::obj(vec![("allocation_id", Value::s(&c[0])), ("authorization_id", Value::s(&c[1])), ("kind", Value::s(&c[2])), ("launch_record_digest", Value::s(&c[3])), ("payload", payload), ("wall_clock", Value::s(&c[5]))]);
            if ph != prev || Self::row_hash(&ph, &row) != h { return Err(StoreError::Chain(format!("record row {i}"))); } prev = h;
        }
        Ok(())
    }
    /// Test hook: corrupt a row to prove the chain check fails closed.
    #[cfg(test)]
    fn tamper(&self) { self.c.execute("UPDATE alloc SET evidence='tampered' WHERE seq=1", []).unwrap(); }
}

fn row_to_alloc(r: &rusqlite::Row<'_>) -> rusqlite::Result<Alloc> {
    Ok(Alloc { seq: r.get(0)?, allocation_id: r.get(1)?, uid: r.get::<_, i64>(2)? as u32, gid: r.get::<_, i64>(3)? as u32, state: r.get(4)?, state_seq: r.get(5)?, authorization_id: r.get(6)?, manifest_digest: r.get(7)?,
        agent_global_id: r.get(8)?, session_id: r.get(9)?, trace_id: r.get(10)?, scope_id: r.get(11)?, pidns_id: r.get(12)?, domain_id: r.get(13)?, evidence: r.get(14)?, wall_clock: r.get(15)?, hash: r.get(16)? })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tmp() -> Store { let p = format!("/tmp/ab-store-test-{}-{}.db", std::process::id(), monotonic_ns()); Store::open(&p, Range::default(), "host:testhost", "boot:b1").unwrap() }
    #[test]
    fn reserve_transition_cas_and_reuse_rules() {
        let mut s = tmp();
        let a = s.reserve("authz:1", "sha256:aa", "agent:x", "sess:1", "trace:1", "domain:d", "test").unwrap();
        assert_eq!((a.uid, a.gid, a.state.as_str(), a.state_seq), (200000, 200000, "allocated", 1));
        let b = s.reserve("authz:2", "sha256:bb", "agent:x", "sess:2", "trace:2", "domain:d", "test").unwrap();
        assert_eq!(b.uid, 200001);
        assert!(matches!(s.reserve("authz:1", "sha256:aa", "agent:x", "sess:1", "trace:1", "domain:d", "test"), Err(StoreError::Conflict(_))));
        assert!(matches!(s.transition(&a.allocation_id, 99, "in-use", "", None, None, "test"), Err(StoreError::Conflict(_))));
        assert!(matches!(s.transition(&a.allocation_id, 1, "free", "", None, None, "test"), Err(StoreError::Conflict(_))));
        let a2 = s.transition(&a.allocation_id, 1, "in-use", "exec ok", Some("agentbound-session-1.scope"), Some("pidns:1"), "test").unwrap();
        let a3 = s.transition(&a2.allocation_id, 2, "reclaiming", "terminated", None, None, "test").unwrap();
        let a4 = s.transition(&a3.allocation_id, 3, "quarantined", "condition met", None, None, "test").unwrap();
        // quarantine floor blocks reuse; uid stays taken
        assert!(matches!(s.transition(&a4.allocation_id, 4, "free", "", None, None, "test"), Err(StoreError::Conflict(_))));
        assert_eq!(s.reserve("authz:3", "sha256:cc", "agent:x", "sess:3", "trace:3", "domain:d", "test").unwrap().uid, 200002);
        s.quarantine_floor_s = 0;
        let a5 = s.transition(&a4.allocation_id, 4, "free", "", None, None, "test").unwrap();
        assert_eq!(a5.state, "free");
        assert_eq!(s.reserve("authz:4", "sha256:dd", "agent:x", "sess:4", "trace:4", "domain:d", "test").unwrap().uid, 200000);
        s.verify_chain().unwrap();
    }
    #[test]
    fn binding_once_only_and_seal() {
        let mut s = tmp();
        let a = s.reserve("authz:1", "sha256:aa", "agent:x", "sess:1", "trace:1", "domain:d", "test").unwrap();
        let p = Value::obj(vec![("x", Value::Int(1))]);
        s.append_record("binding", &a.allocation_id, "sha256:lrd1", "authz:1", &p).unwrap();
        assert!(matches!(s.append_record("binding", &a.allocation_id, "sha256:lrd2", "authz:9", &p), Err(StoreError::Conflict(_))));
        assert!(matches!(s.append_record("binding", "allocation:other", "sha256:lrd1", "authz:9", &p), Err(StoreError::Conflict(_))));
        s.append_record("event", &a.allocation_id, "sha256:lrd1", "authz:1", &p).unwrap();
        s.append_record("seal", &a.allocation_id, "sha256:lrd1", "authz:1", &p).unwrap();
        assert!(matches!(s.append_record("event", &a.allocation_id, "sha256:lrd1", "authz:1", &p), Err(StoreError::Conflict(_))));
        s.append_record("correction", &a.allocation_id, "sha256:lrd1", "authz:1", &p).unwrap();
        assert_eq!(s.records("sha256:lrd1").unwrap().len(), 4);
        s.verify_chain().unwrap();
    }
    #[test]
    fn tamper_detected() {
        let mut s = tmp();
        s.reserve("authz:1", "sha256:aa", "agent:x", "sess:1", "trace:1", "domain:d", "test").unwrap();
        s.tamper();
        assert!(matches!(s.verify_chain(), Err(StoreError::Chain(_))));
    }
    #[test]
    fn idempotency() {
        let s = tmp();
        let reply = Value::obj(vec![("ok", Value::Bool(true))]);
        assert!(s.idem_lookup("uid0:reserve", "k1", "sha256:b1").unwrap().is_none());
        s.idem_store("uid0:reserve", "k1", "sha256:b1", &reply).unwrap();
        assert_eq!(s.idem_lookup("uid0:reserve", "k1", "sha256:b1").unwrap(), Some(Ok(reply)));
        assert_eq!(s.idem_lookup("uid0:reserve", "k1", "sha256:b2").unwrap(), Some(Err(())));
    }
}
