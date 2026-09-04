//! Audit event construction (requirements R-AUD-1; session lifecycle §8) and
//! the append-only local event sink shared by every component.
//!
//! Every event carries the R-AUD-1 correlation fields. Fields not yet bound
//! (for example the launch-record digest before binding) are `null`, never
//! omitted, so a consumer can distinguish "not yet known" from "not recorded".

use crate::json::Value;
use crate::sig::{fmt_rfc3339, monotonic_ns, now_unix, CLOCK_SOURCE};
use std::io::Write;

#[derive(Clone, Debug, Default)]
pub struct Correlation {
    pub authorization_id: Option<String>, pub launch_record_digest: Option<String>, pub allocation_id: Option<String>,
    pub session_id: Option<String>, pub trace_id: Option<String>, pub execution_uid: Option<u32>,
}

fn opt(o: &Option<String>) -> Value { o.as_deref().map(Value::s).unwrap_or(Value::Null) }

pub fn host_id() -> String {
    std::fs::read_to_string("/etc/machine-id").map(|s| format!("host:{}", s.trim())).unwrap_or_else(|_| "host:unknown".into())
}
pub fn boot_id() -> String {
    std::fs::read_to_string("/proc/sys/kernel/random/boot_id").map(|s| format!("boot:{}", s.trim())).unwrap_or_else(|_| "boot:unknown".into())
}

/// Build one event. `detail` is event-specific, closed by the wire-format document.
pub fn event(kind: &str, actor: &str, outcome: &str, c: &Correlation, detail: Value) -> Value {
    let wall = now_unix().ok();
    Value::obj(vec![
        ("actor", Value::s(actor)),
        ("allocation_id", opt(&c.allocation_id)),
        ("authorization_id", opt(&c.authorization_id)),
        ("boot_id", Value::s(&boot_id())),
        ("clock_source", Value::s(CLOCK_SOURCE)),
        ("detail", detail),
        ("event", Value::s(kind)),
        ("execution_uid", c.execution_uid.map(|u| Value::Int(u as i64)).unwrap_or(Value::Null)),
        ("host_id", Value::s(&host_id())),
        ("launch_record_digest", opt(&c.launch_record_digest)),
        ("monotonic_ns", Value::Int(monotonic_ns())),
        ("outcome", Value::s(outcome)),
        ("session_id", opt(&c.session_id)),
        ("trace_id", opt(&c.trace_id)),
        ("wall_clock", wall.map(|t| Value::s(&fmt_rfc3339(t))).unwrap_or(Value::Null)),
        ("wall_clock_trusted", Value::Bool(wall.is_some())),
    ])
}

/// Append-only sink: one canonical JSON line per event, O_APPEND, fsync per write.
/// Component-local buffer used when the audit daemon is unreachable; the
/// daemon-side store (agentbound-audit) hash-chains the events.
pub struct Sink { file: Option<std::fs::File>, pub lost: u64 }
impl Sink {
    pub fn open(path: &str) -> Sink {
        let file = std::fs::OpenOptions::new().create(true).append(true).mode_0600().open(path).ok();
        Sink { file, lost: 0 }
    }
    pub fn emit(&mut self, ev: &Value) {
        let mut line = crate::json::canonical(ev); line.push(b'\n');
        match self.file.as_mut().and_then(|f| f.write_all(&line).and_then(|_| f.sync_data()).ok()) { Some(()) => {}, None => self.lost += 1 }
    }
}
trait Mode0600 { fn mode_0600(&mut self) -> &mut Self; }
impl Mode0600 for std::fs::OpenOptions { fn mode_0600(&mut self) -> &mut Self { use std::os::unix::fs::OpenOptionsExt; self.mode(0o600) } }
