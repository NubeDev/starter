//! `tool-call` — wraps any registered `starter_spi::Tool`.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "R8 — Nodes are
//! not Tools; Tools are one node kind": the input slot accepts the
//! tool's input shape, the output slot carries its return value, and
//! every registered `Tool` is invocable from any flow via this single
//! node kind. Also listed in § "Phase 2 — `starter-flow` engine
//! (in-memory stores)" as one of the two built-ins shipped with the
//! engine.

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.tool-call";
