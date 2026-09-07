//! Shared, unprivileged building blocks for the Agentbound Phase 1 components.
//! Nothing in this crate performs a privileged operation; it is linked into
//! `agentbound-launch` and `agentbound-lifecycle` and therefore counts toward
//! the R-CON-8 direct-SLOC bound.
pub mod acl;
pub mod audit;
pub mod envelope;
pub mod json;
pub mod schema;
pub mod sig;
pub mod wire;

/// Source provenance embedded at build time by `build.rs` from `crates/SOURCE-PROVENANCE` (written by `crates/build.sh` from the
/// sending checkout). `commit` is the full hash; `dirty` is "true" when the working tree differed from it, in which case no reviewed
/// commit produced this binary and every consumer MUST say so; `tree` is the git tree hash of the synced sources.
pub mod provenance {
    pub const COMMIT: &str = env!("AB_SOURCE_COMMIT");
    pub const DIRTY: &str = env!("AB_SOURCE_DIRTY");
    pub const TREE: &str = env!("AB_SOURCE_TREE");
    /// One-line human form, e.g. `9f3c1a2… (clean)` or `9f3c1a2… (DIRTY — not attributable to a reviewed commit)`.
    pub fn describe() -> String { format!("{}{}", COMMIT, if DIRTY == "true" { " (DIRTY — not attributable to a reviewed commit)" } else if DIRTY == "false" { " (clean)" } else { " (provenance unknown)" }) }
}
