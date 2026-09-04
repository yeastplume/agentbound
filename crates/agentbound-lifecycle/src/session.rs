//! Termination (§5), quiesce (§6), revocation, reclamation condition (§4.1),
//! and scope observation. Filled in the next increment.
use crate::service::{Reply, Service};
use ab_common::json::Value;
use ab_common::wire;

impl Service {
    pub fn lifecycle_action(&mut self, op: &str, _b: &Value, _uid: u32) -> Reply {
        Err((wire::CLASS_UNAVAILABLE, "not_implemented", op.to_string()))
    }
}
impl Service {
    /// §7/§8: reconcile persisted records before accepting new allocation. Filled next increment.
    pub fn reconcile_on_start(&mut self) {}
    /// Periodic: pidfd liveness, quiesce deadlines, pending reclamation. Filled next increment.
    pub fn poll_sessions(&mut self) {}
}
