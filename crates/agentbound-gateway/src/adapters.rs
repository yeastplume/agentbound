//! Typed adapters (R-GW-4). Only named operations; no generic forwarding.
use crate::Gateway;
use ab_common::json::Value;

pub fn run(gw: &mut Gateway, aid: &str, name: &str, op: &Value, payload: Option<&[u8]>, session_id: &str, trace: &str) -> Result<Value, (&'static str, String)> {
    match name {
        "gateway.ping" => Ok(Value::obj(vec![("pong", Value::Bool(true))])),
        "git.push_staging" => crate::git::push_staging(gw, aid, op, payload.unwrap_or(&[]), session_id, trace),
        _ => Err(("operation_unknown", name.to_string())),
    }
}
