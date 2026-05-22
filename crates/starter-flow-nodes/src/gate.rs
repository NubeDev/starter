//! `gate` — review / verify / approval node kind.
//!
//! Semantics defined by `DOCS/flow/scope/SCOPE.md` § "Relationship to
//! existing crates" (the `starter-flow-nodes` row lists "gate
//! (review/verify/approval)") and § "R3 — The engine is a reader of
//! policies, never an owner" (the `on_failure: gate` policy that
//! Codeless's "verify-gated; halt visibly" mode maps to). Scheduled
//! in § "Phase 5 — Remaining built-in node kinds". A `gate` holds
//! propagation until an external approver releases it, with the hold
//! observable via the run timeline.

/// Reverse-DNS kind id in the reserved `starter.flow.*` namespace
/// (per § "R10 — Reverse-DNS ids; namespace ownership enforced").
pub const KIND_ID: &str = "starter.flow.gate";

/// Static metadata for the catalog / discovery surface. Help text is
/// resolved through `starter-i18n`; see `crates/starter-i18n/catalogs/`.
pub static DESCRIPTOR: starter_flow_spi::node::NodeDescriptor =
    starter_flow_spi::node::NodeDescriptor::new(
        KIND_ID,
        "starter.flow.node.gate.label",
        "starter.flow.node.gate.summary",
        "starter.flow.node.gate.help",
    );
