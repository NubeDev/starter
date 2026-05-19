//! `trigger.event` — slot-watch entry node.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "R3 — The engine
//! is a reader of policies, never an owner" (the `trigger` policy
//! variant `event(slot_ref)`) and § "R1 — Everything is a Node" ("A
//! trigger (explicit, event-driven, scheduled, webhook) is a node").
//! Fires whenever the referenced slot's value changes, which is the
//! reactive Rubix shape on the same engine. Scheduled in § "Phase 5
//! — Remaining built-in node kinds".

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.trigger.event";
