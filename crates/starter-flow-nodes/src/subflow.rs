//! `subflow` — composes another flow as a single node.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "R1 — Everything
//! is a Node" ("A subflow is a node (composition is free)") and
//! scheduled in § "Phase 5 — Remaining built-in node kinds". The
//! inner flow's input slots become this node's inputs and its output
//! slots become this node's outputs; cancellation and three-level
//! stop (per § "R12") propagate across the boundary unchanged.

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.subflow";
