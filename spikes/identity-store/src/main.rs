//! WP1 spike ID-1 / LC-1 (allocator half): append-only, hash-chained allocator store
//! with compare-and-set state transitions in single-writer SQLite WAL, owned by the
//! lifecycle daemon; crash consistency under process kill at arbitrary points;
//! double-allocation fail-closed under concurrent writers; state-machine enforcement.
//! Identity lifecycle §3.2, §3.3, §4; register ID-1.
//!
//! Crash model: SIGKILL of the writer process at random points (daemon crash, the
//! F-C/F-T "lifecycle daemon dies mid-step" fault). Power loss is not modelled here;
//! durability against it rests on synchronous=FULL + WAL fsync, which SQLite documents
//! and which this spike does not independently test.
//!
//! Throwaway code: not TCB, not SLOC-counted.
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::fs;
use std::time::Instant;

const DB: &str = "/var/lib/ab-spike-idstore/alloc.db";
const RANGE: (u32, u32) = (200000, 299999);
const STATES: [&str; 5] = ["free", "allocated", "in-use", "reclaiming", "quarantined"];
fn result(item: &str, pass: bool, detail: &str) { println!("RESULT {item} {} {detail}", if pass { "PASS" } else { "FAIL" }); }
fn next_ok(from: &str, to: &str) -> bool { matches!((from, to), ("free", "allocated") | ("allocated", "in-use") | ("allocated", "reclaiming") | ("in-use", "reclaiming") | ("reclaiming", "quarantined") | ("reclaiming", "reclaiming") | ("quarantined", "free")) }

struct Store { c: Connection }
#[derive(Debug)] enum Err { Cas(String), Sql(rusqlite::Error), Range(String) }
impl From<rusqlite::Error> for Err { fn from(e: rusqlite::Error) -> Self { Err::Sql(e) } }

impl Store {
    fn open() -> Store {
        fs::create_dir_all("/var/lib/ab-spike-idstore").unwrap();
        let c = Connection::open(DB).unwrap();
        c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA busy_timeout=5000;
            CREATE TABLE IF NOT EXISTS records(
              seq INTEGER PRIMARY KEY,           -- monotonic append sequence
              record_id TEXT NOT NULL,           -- allocation record ID (one per UID lifetime)
              uid INTEGER NOT NULL,
              state TEXT NOT NULL,
              authz_id TEXT,                     -- authorization ID (launch record binding)
              actor TEXT NOT NULL, ts INTEGER NOT NULL,
              prev_hash BLOB NOT NULL, hash BLOB NOT NULL);
            CREATE INDEX IF NOT EXISTS by_uid ON records(uid, seq);
            CREATE INDEX IF NOT EXISTS by_rec ON records(record_id, seq);
            CREATE UNIQUE INDEX IF NOT EXISTS one_live_binding ON records(authz_id) WHERE state='allocated';").unwrap();
        // append-only: triggers forbid UPDATE/DELETE even for the owner connection
        c.execute_batch("CREATE TRIGGER IF NOT EXISTS no_upd BEFORE UPDATE ON records BEGIN SELECT RAISE(ABORT,'append-only'); END;
                         CREATE TRIGGER IF NOT EXISTS no_del BEFORE DELETE ON records BEGIN SELECT RAISE(ABORT,'append-only'); END;").unwrap();
        Store { c }
    }
    fn tail(&self) -> (i64, Vec<u8>) { self.c.query_row("SELECT seq, hash FROM records ORDER BY seq DESC LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?))).optional().unwrap().unwrap_or((0, vec![0u8; 32])) }
    fn current(&self, uid: u32) -> Option<(i64, String, String, Option<String>)> { self.c.query_row("SELECT seq, record_id, state, authz_id FROM records WHERE uid=?1 ORDER BY seq DESC LIMIT 1", [uid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).optional().unwrap() }
    /// Compare-and-set append: the caller states the (record_id, seq) it observed; the append succeeds only if that is still the tail for the UID and the transition is legal.
    fn transition(&mut self, uid: u32, expect: Option<(String, i64)>, to: &str, authz: Option<&str>, actor: &str) -> Result<i64, Err> {
        if uid < RANGE.0 || uid > RANGE.1 { return Err(Err::Range(format!("uid {uid} outside range"))); }
        let tx = self.c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?; // single writer
        let cur = { tx.query_row("SELECT seq, record_id, state, authz_id FROM records WHERE uid=?1 ORDER BY seq DESC LIMIT 1", [uid], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<String>>(3)?))).optional()? };
        let (from, rec_id, authz_id) = match (&cur, &expect) {
            (None, None) => ("free".to_string(), format!("rec-{uid}-{}", now_ns()), authz.map(str::to_string)),
            (Some((seq, rid, st, az)), Some((erid, eseq))) if rid == erid && seq == eseq => (st.clone(), if st == "quarantined" && to == "free" { rid.clone() } else { rid.clone() }, az.clone().or(authz.map(str::to_string))),
            (Some((seq, rid, st, _)), Some((erid, eseq))) => return Err(Err::Cas(format!("uid {uid}: expected ({erid},{eseq}) but tail is ({rid},{seq},{st})"))),
            (Some((seq, rid, st, _)), None) if st == "free" => (st.clone(), format!("rec-{uid}-{}", now_ns()), authz.map(str::to_string)),
            (Some((seq, rid, st, _)), None) => return Err(Err::Cas(format!("uid {uid}: allocation attempted but tail is ({rid},{seq},{st}) — double allocation"))),
            (None, Some(e)) => return Err(Err::Cas(format!("uid {uid}: expected {e:?} but no record"))),
        };
        // a fresh allocation from quarantined→free→allocated gets a new record id: handled by requiring to=='allocated' only from free
        if !next_ok(&from, to) { return Err(Err::Cas(format!("uid {uid}: illegal transition {from}→{to}"))); }
        let rec_id = if from == "free" && to == "allocated" && cur.is_some() { format!("rec-{uid}-{}", now_ns()) } else { rec_id };
        let (tseq, prev) = tx.query_row("SELECT seq, hash FROM records ORDER BY seq DESC LIMIT 1", [], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?))).optional()?.unwrap_or((0, vec![0u8; 32]));
        let seq = tseq + 1; let ts = now_ns();
        let mut h = Sha256::new(); h.update(&prev); h.update(seq.to_le_bytes()); h.update(rec_id.as_bytes()); h.update(uid.to_le_bytes()); h.update(to.as_bytes()); h.update(authz_id.as_deref().unwrap_or("").as_bytes()); h.update(actor.as_bytes()); h.update(ts.to_le_bytes());
        let hash = h.finalize().to_vec();
        tx.execute("INSERT INTO records(seq,record_id,uid,state,authz_id,actor,ts,prev_hash,hash) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![seq, rec_id, uid, to, authz_id, actor, ts, prev, hash])?;
        tx.commit()?;
        Ok(seq)
    }
    /// Verify the chain and the per-UID state machine over the whole store.
    fn verify(&self) -> Result<(usize, usize), String> {
        let mut st = self.c.prepare("SELECT seq,record_id,uid,state,authz_id,actor,ts,prev_hash,hash FROM records ORDER BY seq").unwrap();
        let mut prev = vec![0u8; 32]; let mut expect_seq = 1i64; let mut n = 0; let mut per_uid: std::collections::HashMap<u32, String> = Default::default(); let mut live_authz: std::collections::HashMap<String, u32> = Default::default();
        let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, u32>(2)?, r.get::<_, String>(3)?, r.get::<_, Option<String>>(4)?, r.get::<_, String>(5)?, r.get::<_, i64>(6)?, r.get::<_, Vec<u8>>(7)?, r.get::<_, Vec<u8>>(8)?))).unwrap();
        for row in rows { let (seq, rid, uid, state, az, actor, ts, ph, h) = row.unwrap();
            if seq != expect_seq { return Err(format!("seq gap at {seq} (expected {expect_seq})")); }
            if ph != prev { return Err(format!("prev_hash mismatch at seq {seq}")); }
            let mut hh = Sha256::new(); hh.update(&ph); hh.update(seq.to_le_bytes()); hh.update(rid.as_bytes()); hh.update(uid.to_le_bytes()); hh.update(state.as_bytes()); hh.update(az.as_deref().unwrap_or("").as_bytes()); hh.update(actor.as_bytes()); hh.update(ts.to_le_bytes());
            if hh.finalize().to_vec() != h { return Err(format!("hash mismatch at seq {seq}")); }
            let from = per_uid.get(&uid).cloned().unwrap_or("free".into());
            if !next_ok(&from, &state) { return Err(format!("illegal transition {from}→{state} for uid {uid} at seq {seq}")); }
            if state == "allocated" { if let Some(a) = &az { if let Some(other) = live_authz.insert(a.clone(), uid) { if other != uid { return Err(format!("authz {a} bound to uids {other} and {uid}")); } } } }
            if state == "free" || state == "quarantined" { if let Some(a) = &az { live_authz.remove(a); } }
            per_uid.insert(uid, state); prev = h; expect_seq += 1; n += 1;
        }
        Ok((n, per_uid.len()))
    }
}
fn now_ns() -> i64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as i64 }

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "writer" { return writer(args[2].parse().unwrap(), args[3].parse().unwrap()); }
    if args.len() > 1 && args[1] == "reader" { return reader(); }
    if args.len() > 1 && args[1] == "racer" { return racer(args[2].parse().unwrap(), args[3].parse().unwrap()); }
    println!("spike identity-store (ID-1); sqlite {}", rusqlite::version());
    let _ = fs::remove_dir_all("/var/lib/ab-spike-idstore");
    let mut s = Store::open();
    let mode = fs::metadata(DB).unwrap().permissions(); let _ = mode;

    // ---------- range check (§3.1): reject overlap with /etc/passwd ----------
    let passwd: Vec<u32> = fs::read_to_string("/etc/passwd").unwrap().lines().filter_map(|l| l.split(':').nth(2)?.parse().ok()).collect();
    let overlap = passwd.iter().any(|u| *u >= RANGE.0 && *u <= RANGE.1);
    let nobody_overlap = passwd.iter().any(|u| *u >= 65534 && *u <= 65534 && RANGE.0 <= 65534 && 65534 <= RANGE.1);
    result("ID-0.range-disjoint-from-local-accounts", !overlap && !nobody_overlap, &format!("{} passwd entries, none in {}–{}", passwd.len(), RANGE.0, RANGE.1));

    // ---------- state machine + CAS ----------
    let uid = 200001;
    let s1 = s.transition(uid, None, "allocated", Some("authz-A"), "launch").unwrap();
    let (seq, rid, _, _) = s.current(uid).unwrap();
    let skip = s.transition(uid, Some((rid.clone(), seq)), "free", None, "lifecycle");           // allocated→free: illegal
    let skip2 = s.transition(uid, Some((rid.clone(), seq)), "quarantined", None, "lifecycle");   // allocated→quarantined: skips reclaiming
    result("ID-2.no-skipping-reclaiming-or-quarantined", matches!(skip, Err(Err::Cas(_))) && matches!(skip2, Err(Err::Cas(_))), &format!("allocated→free: {:?}; allocated→quarantined: {:?}", skip.err(), skip2.err()));
    let stale = s.transition(uid, Some((rid.clone(), seq - 1)), "in-use", None, "constructor");
    result("ID-3.cas-rejects-stale-sequence", matches!(stale, Err(Err::Cas(_))), &format!("{:?}", stale.err()));
    let s2 = s.transition(uid, Some((rid.clone(), seq)), "in-use", None, "constructor").unwrap();
    let dup = s.transition(uid, None, "allocated", Some("authz-B"), "launch");
    result("ID-4.double-allocation-same-uid-fails-closed", matches!(dup, Err(Err::Cas(_))), &format!("{:?} → identity.double_allocation_detected", dup.err()));
    let dup_authz = s.transition(200002, None, "allocated", Some("authz-A"), "launch");
    result("ID-5.one-launch-record-two-uids-fails-closed", matches!(dup_authz, Err(Err::Sql(_))), &format!("second allocation for authz-A on uid 200002: {:?} (partial unique index on live allocated bindings)", dup_authz.err().map(|e| format!("{e:?}").chars().take(90).collect::<String>())));
    let (seq, rid, _, _) = s.current(uid).unwrap();
    let s3 = s.transition(uid, Some((rid.clone(), seq)), "reclaiming", None, "lifecycle").unwrap();
    let (seq, rid, _, _) = s.current(uid).unwrap();
    let s3b = s.transition(uid, Some((rid.clone(), seq)), "reclaiming", None, "lifecycle").unwrap(); // stays reclaiming on uncertainty
    let (seq, rid, _, _) = s.current(uid).unwrap();
    let s4 = s.transition(uid, Some((rid.clone(), seq)), "quarantined", None, "lifecycle").unwrap();
    let (seq, rid, _, _) = s.current(uid).unwrap();
    let s5 = s.transition(uid, Some((rid.clone(), seq)), "free", None, "lifecycle").unwrap();
    let s6 = s.transition(uid, None, "allocated", Some("authz-C"), "launch").unwrap();
    let (_, rid2, _, _) = s.current(uid).unwrap();
    result("ID-6.full-cycle-and-reuse-gets-new-record-id", [s1, s2, s3, s3b, s4, s5, s6].windows(2).all(|w| w[1] > w[0]) && rid2 != rid, &format!("seqs {:?}; record id {rid} → {rid2} on reuse", [s1, s2, s3, s3b, s4, s5, s6]));
    // append-only enforcement
    let upd = s.c.execute("UPDATE records SET state='free' WHERE uid=?1", [uid]); let del = s.c.execute("DELETE FROM records WHERE uid=?1", [uid]);
    result("ID-7.append-only-enforced-in-store", upd.is_err() && del.is_err(), &format!("UPDATE: {:?}; DELETE: {:?}", upd.err().map(|e| e.to_string()), del.err().map(|e| e.to_string())));
    // tamper detection
    let v0 = s.verify(); 
    s.c.execute_batch("DROP TRIGGER no_upd; UPDATE records SET state='in-use' WHERE seq=1; CREATE TRIGGER no_upd BEFORE UPDATE ON records BEGIN SELECT RAISE(ABORT,'append-only'); END;").unwrap();
    let v1 = s.verify();
    result("ID-8.hash-chain-detects-tamper", v0.is_ok() && v1.is_err(), &format!("before: {v0:?}; after in-place edit of seq 1: {v1:?}"));
    drop(s); let _ = fs::remove_dir_all("/var/lib/ab-spike-idstore"); let mut s = Store::open();

    // ---------- concurrent racers: 8 processes each try to allocate the same 50 UIDs ----------
    let t = Instant::now();
    let kids: Vec<i32> = (0..8).map(|i| { let p = unsafe { libc::fork() }; if p == 0 { let e = std::process::Command::new(std::env::current_exe().unwrap()).args(["racer", &i.to_string(), "50"]).status().unwrap(); unsafe { libc::_exit(e.code().unwrap_or(1)) } } p }).collect();
    for k in &kids { let mut st = 0; unsafe { libc::waitpid(*k, &mut st, 0); } }
    let race_ms = t.elapsed().as_millis();
    let allocated: i64 = s.c.query_row("SELECT count(*) FROM records WHERE state='allocated'", [], |r| r.get(0)).unwrap();
    let distinct: i64 = s.c.query_row("SELECT count(DISTINCT uid) FROM records WHERE state='allocated'", [], |r| r.get(0)).unwrap();
    let v = s.verify();
    result("ID-9.concurrent-allocators-exactly-one-winner-per-uid", allocated == 50 && distinct == 50 && v.is_ok(), &format!("8 racers × 50 UIDs in {race_ms} ms: {allocated} allocated records over {distinct} distinct UIDs; verify={v:?} (losers got CAS/busy errors, never a second record)"));
    drop(s); let _ = fs::remove_dir_all("/var/lib/ab-spike-idstore");

    // ---------- crash consistency: writer SIGKILLed at random points, 150 rounds ----------
    let mut acked_total = 0; let mut rounds = 0; let mut bad = vec![]; let t = Instant::now();
    for round in 0..150 {
        let mut pfd = [0; 2]; unsafe { libc::pipe(pfd.as_mut_ptr()); }
        let p = unsafe { libc::fork() };
        if p == 0 { unsafe { libc::dup2(pfd[1], 1); libc::close(pfd[0]); } let e = std::process::Command::new(std::env::current_exe().unwrap()).args(["writer", &round.to_string(), "400"]).status().unwrap(); unsafe { libc::_exit(e.code().unwrap_or(1)) } }
        unsafe { libc::close(pfd[1]); }
        // read acks until we decide to kill (after a pseudo-random number of acks)
        let kill_after = 1 + (round * 7919) % 37;
        let mut acks: Vec<String> = vec![]; let mut buf = [0u8; 4096]; let mut partial = String::new();
        'outer: loop {
            let n = unsafe { libc::read(pfd[0], buf.as_mut_ptr() as *mut libc::c_void, 4096) }; if n <= 0 { break; }
            partial.push_str(&String::from_utf8_lossy(&buf[..n as usize]));
            while let Some(i) = partial.find('\n') { let line = partial[..i].to_string(); partial = partial[i + 1..].to_string(); acks.push(line); if acks.len() >= kill_after { unsafe { libc::kill(p, libc::SIGKILL); } break 'outer; } }
        }
        let mut st = 0; unsafe { libc::waitpid(p, &mut st, 0); libc::close(pfd[0]); }
        // reopen and verify: chain intact; every acked (uid,state,seq) present
        let s = Store::open(); let v = s.verify();
        if let Err(e) = &v { bad.push(format!("round {round}: {e}")); }
        for a in &acks { let f: Vec<&str> = a.split(' ').collect(); let (uid, state, seq): (u32, &str, i64) = (f[0].parse().unwrap(), f[1], f[2].parse().unwrap());
            let present: bool = s.c.query_row("SELECT count(*) FROM records WHERE seq=?1 AND uid=?2 AND state=?3", params![seq, uid, state], |r| r.get::<_, i64>(0)).unwrap() == 1;
            if !present { bad.push(format!("round {round}: acked {a} missing after crash")); } }
        acked_total += acks.len(); rounds += 1; drop(s);
    }
    let s = Store::open(); let (n, uids) = s.verify().unwrap_or((0, 0));
    result("ID-10.crash-consistency-under-sigkill", bad.is_empty(), &format!("{rounds} rounds, writer killed after 1–37 acknowledged commits each; {acked_total} acknowledged transitions all present after reopen; chain verified each round; final store {n} records over {uids} UIDs; {} anomalies {:?}; {} ms", bad.len(), bad.iter().take(3).collect::<Vec<_>>(), t.elapsed().as_millis()));
    // recovery rule: after crash, records left in `allocated`/`in-use` with no live scope must go to reclaiming, never to free
    let stuck: i64 = s.c.query_row("SELECT count(*) FROM (SELECT uid, state FROM records r WHERE seq=(SELECT max(seq) FROM records WHERE uid=r.uid)) WHERE state IN ('allocated','in-use')", [], |r| r.get(0)).unwrap();
    let mut s = s; let mut moved = 0; let mut illegal = 0;
    let stuck_uids: Vec<(u32, String, i64)> = { let mut q = s.c.prepare("SELECT uid, record_id, seq FROM records r WHERE seq=(SELECT max(seq) FROM records WHERE uid=r.uid) AND state IN ('allocated','in-use')").unwrap(); q.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap().map(|x| x.unwrap()).collect() };
    for (uid, rid, seq) in stuck_uids { if s.transition(uid, Some((rid.clone(), seq)), "free", None, "recovery").is_err() { illegal += 1; } if s.transition(uid, Some((rid, seq)), "reclaiming", None, "recovery").is_ok() { moved += 1; } }
    result("ID-11.crash-recovery-moves-orphans-to-reclaiming-only", moved == stuck && illegal == stuck, &format!("{stuck} identities orphaned in allocated/in-use by crashes; direct →free rejected {illegal}×; →reclaiming appended {moved}×"));
    // store ACL: owner-only
    fs::set_permissions("/var/lib/ab-spike-idstore", fs::Permissions::from(std::os::unix::fs::PermissionsExt::from_mode(0o700))).unwrap();
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new("setpriv").args(["--reuid=200042", "--regid=200042", "--clear-groups", exe.to_str().unwrap(), "reader"]).output().unwrap();
    let as_session = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let as_root = std::process::Command::new(&exe).arg("reader").output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap();
    result("ID-12.session-uid-cannot-read-store", !out.status.success() && as_root.starts_with("read "), &format!("store dir 0700 owned by daemon identity (root here); as uid 200042: {as_session:?} (exit {:?}); as owner: {as_root:?}", out.status.code()));
    // performance: allocation latency with synchronous=FULL
    let t = Instant::now(); for u in 210000..210100 { s.transition(u, None, "allocated", Some(&format!("authz-p{u}")), "launch").unwrap(); } let per = t.elapsed().as_micros() / 100;
    println!("allocation latency (synchronous=FULL, WAL, per durable commit): {per} µs");
    let _ = fs::remove_dir_all("/var/lib/ab-spike-idstore");
    println!("done");
}
fn writer(round: u32, n: u32) {
    // allocate and cycle identities as fast as possible, acking each durable commit on stdout
    let mut s = Store::open(); let base = 220000 + (round % 20) * 100;
    for i in 0..n { let uid = base + i % 100;
        let r = match s.current(uid) { None => s.transition(uid, None, "allocated", Some(&format!("az-{round}-{i}")), "w"),
            Some((seq, rid, st, _)) => { let to = match st.as_str() { "free" => "allocated", "allocated" => "in-use", "in-use" => "reclaiming", "reclaiming" => "quarantined", _ => "free" }; if to == "allocated" { s.transition(uid, None, to, Some(&format!("az-{round}-{i}")), "w") } else { s.transition(uid, Some((rid, seq)), to, None, "w") } } };
        if let Ok(seq) = r { let st = s.current(uid).unwrap().2; use std::io::Write; if writeln!(std::io::stdout(), "{uid} {st} {seq}").and_then(|_| std::io::stdout().flush()).is_err() { return; } } }
}
fn racer(i: u32, n: u32) {
    let mut s = Store::open();
    // common start: spin until the next 500 ms boundary so all racers begin together
    let start = { let now = now_ns(); now - now % 500_000_000 + 500_000_000 }; while now_ns() < start {}
    let mut order: Vec<u32> = (0..n).collect(); for k in 0..n as usize { let j = ((k as u64 * 2654435761 + i as u64 * 97) % n as u64) as usize; order.swap(k, j); }
    let mut won = 0; let mut cas = 0; let mut busy = 0;
    for u in order { match s.transition(230000 + u, None, "allocated", Some(&format!("race-{i}-{u}")), &format!("racer{i}")) { Ok(_) => won += 1, Err(Err::Cas(_)) => cas += 1, Err(_) => busy += 1 } }
    eprintln!("racer {i} won {won} cas-lost {cas} busy/other {busy}");
}
fn reader() { let c = Connection::open_with_flags(DB, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY); match c { Ok(c) => match c.query_row("SELECT count(*) FROM records", [], |r| r.get::<_, i64>(0)) { Ok(n) => { println!("read {n}"); std::process::exit(0) } Err(e) => { println!("query error {e}"); std::process::exit(2) } }, Err(e) => { println!("open error {e}"); std::process::exit(3) } } }
