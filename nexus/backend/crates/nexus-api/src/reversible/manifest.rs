//! The coverage-guard manifest: the closed list of kinds that MUST record.
//!
//! WS-12 §3.5b makes "did the owning workstream wire its `record_if_reversible`
//! call?" a *test failure* rather than something a reviewer has to remember. The
//! coverage guard enumerates this list and asserts each entry is both registered
//! in the [`crate::reversible`] registry and reachable from a recording mutation
//! path. Adding a new mutable kind to its owning workstream's PR means adding it
//! here too — the guard flags any registered kind that is absent, and any listed
//! kind that is not registered.
//!
//! Entries are the stable [`starter_spi::authz::ResourceRef::kind`] discriminators
//! (e.g. `nexus.dashboard`), matching the constants in [`crate::authz`].

use crate::authz::{KIND_DASHBOARD, KIND_DATASOURCE, KIND_FOLDER};

/// Every nexus resource kind that is expected to record an audit/undo `Change`
/// on mutation. WS-12 ships the dashboard and datasource reference entries; each
/// further owning workstream appends its kind here in the same PR that adds the
/// kind's [`starter_spi::changelog::Reversible`] impl (C6 convention, ROADMAP
/// §6a). The coverage guard test cross-checks this against the live registry so
/// the two never silently diverge.
pub const KNOWN_MUTABLE_KINDS: &[&str] = &[KIND_DASHBOARD, KIND_FOLDER, KIND_DATASOURCE];

/// Whether `kind` is on the [`KNOWN_MUTABLE_KINDS`] manifest. Used by the
/// coverage guard and available to callers that want to assert a kind is meant
/// to be audited before relying on its changelog rows.
pub fn is_known_mutable_kind(kind: &str) -> bool {
    KNOWN_MUTABLE_KINDS.contains(&kind)
}
