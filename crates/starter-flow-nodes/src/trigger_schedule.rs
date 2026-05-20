//! `trigger.schedule` — cron-fired entry node.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "R3 — The engine
//! is a reader of policies, never an owner" (the `trigger` policy
//! variant `schedule(cron)`) and § "R1 — Everything is a Node" ("A
//! trigger (explicit, event-driven, scheduled, webhook) is a node").
//! Backed by the durable scheduler noted in § "What this scope is
//! *not*"; lands alongside the rest of the trigger family in
//! § "Phase 5 — Remaining built-in node kinds".

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.trigger.schedule";

/// Static metadata for the catalog / discovery surface. Help text is
/// resolved through `starter-i18n`; see `crates/starter-i18n/catalogs/`.
pub const DESCRIPTOR: starter_flow_spi::node::NodeDescriptor =
    starter_flow_spi::node::NodeDescriptor::new(
        KIND_ID,
        "starter.flow.node.trigger-schedule.label",
        "starter.flow.node.trigger-schedule.summary",
        "starter.flow.node.trigger-schedule.help",
    );
