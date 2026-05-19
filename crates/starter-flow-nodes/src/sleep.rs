//! `sleep` — timed delay node kind.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "Relationship to
//! existing crates" (the `starter-flow-nodes` row lists `sleep`
//! alongside the other built-ins) and scheduled in § "Phase 5 —
//! Remaining built-in node kinds". Holds propagation for a declared
//! duration then forwards its input to its output. Cancellation
//! interrupts the wait per § "R13 — Streaming, cancellation,
//! observability reuse existing seams".

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.sleep";
