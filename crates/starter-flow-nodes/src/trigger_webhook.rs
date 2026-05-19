//! `trigger.webhook` — HTTP-fired entry node.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "R3 — The engine
//! is a reader of policies, never an owner" (the `trigger` policy
//! variant `webhook(...)`), § "R1 — Everything is a Node" ("A
//! trigger (explicit, event-driven, scheduled, webhook) is a node"),
//! and § "R9 — Flows are first-class Tools and first-class Services":
//! a flow whose entry is a `trigger.webhook` is, simultaneously, an
//! HTTP endpoint. Scheduled in § "Phase 5 — Remaining built-in node
//! kinds".

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.trigger.webhook";
