//! Insights node-kind bodies contributed to `starter-flow-nodes`'s
//! `NodeKindRegistry`. Per R-ins-9, chaining / branching / retry /
//! error-routing live in the flow engine — insights ships node
//! *bodies*, not a parallel orchestrator.

pub mod rule_rhai;
pub mod rule_rust;
#[cfg(feature = "sqlite")]
pub mod rule_sql;
pub mod verdict_join;
pub mod windowing;

/// JSON slot key carrying a serialised [`starter_spi::insights::Verdict`].
pub const VERDICT_SLOT: &str = "verdict";
