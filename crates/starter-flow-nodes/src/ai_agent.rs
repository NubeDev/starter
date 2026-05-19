//! `ai-agent` — the LLM-loop node kind.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "R7 — The AI
//! agent is a node kind, not a runtime": the agent foundation
//! collapses to a single node kind whose body owns the model loop,
//! tool dispatch, and session continuity, while topology lives in the
//! flow graph. The body lands in § "Phase 4 — `ai-agent` node kind
//! (D1 resolution)", once the adk-rust-vs-leaner-loop decision is
//! made. Skills bind to this kind's *invocation*, not to topology,
//! per § "Skills bind to the `ai-agent` node kind".

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.ai-agent";
