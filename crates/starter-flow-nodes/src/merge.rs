//! `merge` — fan-in join node kind.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "R1 — Everything
//! is a Node" (the `join` half of the parallel-merge example in § "R7
//! — The AI agent is a node kind, not a runtime") and scheduled in
//! § "Phase 5 — Remaining built-in node kinds". Combines multiple
//! upstream slot values into one output, with an explicit policy for
//! how to wait and how to combine.

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.merge";

/// Static metadata for the catalog / discovery surface. Help text is
/// resolved through `starter-i18n`; see `crates/starter-i18n/catalogs/`.
pub const DESCRIPTOR: starter_flow_spi::node::NodeDescriptor =
    starter_flow_spi::node::NodeDescriptor::new(
        KIND_ID,
        "starter.flow.node.merge.label",
        "starter.flow.node.merge.summary",
        "starter.flow.node.merge.help",
    );
