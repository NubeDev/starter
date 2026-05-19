//! `branch` — conditional routing node kind.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "R1 — Everything
//! is a Node" (a branch / merge / loop-condition is a node) and
//! scheduled in § "Phase 5 — Remaining built-in node kinds". The
//! `branch` evaluates a predicate over its input slot and routes the
//! propagation token down exactly one of its outgoing edges.

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.branch";
