//! `http-out` — outbound HTTP request node kind.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "Relationship to
//! existing crates" (the `starter-flow-nodes` row lists `http-out`
//! alongside the other built-ins) and scheduled in § "Phase 5 —
//! Remaining built-in node kinds". Issues a single outbound HTTP
//! request shaped by its input slot and emits the response on its
//! output slot. All retry / timeout / safe-state behaviour is read
//! from the policy block on the node, per § "R3".

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.http-out";
