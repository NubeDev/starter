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

/// Static metadata for the catalog / discovery surface. Help text is
/// resolved through `starter-i18n`; see `crates/starter-i18n/catalogs/`.
pub const DESCRIPTOR: starter_flow_spi::node::NodeDescriptor =
    starter_flow_spi::node::NodeDescriptor::new(
        KIND_ID,
        "starter.flow.node.sleep.label",
        "starter.flow.node.sleep.summary",
        "starter.flow.node.sleep.help",
    );
