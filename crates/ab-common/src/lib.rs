//! Shared, unprivileged building blocks for the Agentbound Phase 1 components.
//! Nothing in this crate performs a privileged operation; it is linked into
//! `agentbound-launch` and `agentbound-lifecycle` and therefore counts toward
//! the R-CON-8 direct-SLOC bound.
pub mod audit;
pub mod envelope;
pub mod json;
pub mod schema;
pub mod sig;
pub mod wire;
