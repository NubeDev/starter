//! `transform` — pure-function node kind.
//!
//! Semantics: a stateless map over slot values, declared as a Rhai
//! expression (per `DOCS/flow/scope/SCOPE.md` § "Relationship to
//! existing crates", `starter-flow-nodes` row, and § "Phase 2 —
//! `starter-flow` engine (in-memory stores)" which lists `transform`
//! and `tool-call` as the two built-ins that ship with the engine).
//! The kind itself is one of the canonical examples in § "R1 —
//! Everything is a Node".

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.transform";
