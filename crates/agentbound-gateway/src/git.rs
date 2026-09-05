//! Git staging-ref adapter — filled in the next increment.
use crate::Gateway;
use ab_common::json::Value;
pub fn push_staging(_gw: &mut Gateway, _aid: &str, _op: &Value, _payload: &[u8], _session_id: &str, _trace: &str) -> Result<Value, (&'static str, String)> { Err(("adapter_unavailable", "git adapter not yet implemented".into())) }
