//! Per-kind [`Reversible`] impls + the coverage-guard manifest (WS-12).
//!
//! Each undoable resource kind implements [`starter_spi::changelog::Reversible`]
//! once: `apply_inverse` (undo), `apply_forward` (redo), `clone_with`
//! (duplicate). WS-12 ships **reference** impls for dashboards and datasources to
//! prove the pattern and exercise the substrate end-to-end; the remaining kinds'
//! impls land in their owning workstreams' PRs per the C6 convention (ROADMAP
//! §6a), each adding itself to [`KNOWN_MUTABLE_KINDS`] so the coverage guard sees
//! it.
//!
//! Snapshot vs patch (per the matrix in `reversible.rs`): dashboards and
//! datasources are both **snapshot** — small, lifecycle-shaped (create/delete flip
//! existence), and dashboards are pinned to snapshot by ROADMAP §6a D2 so
//! restore-to-version has an absolute state.

mod dashboard;
mod datasource;
mod folder;
mod manifest;
mod nav_node;
mod panel;
mod query_kind;

use std::sync::Arc;

use sqlx::PgPool;
use starter_undo::ReversibleRegistry;

use nexus_store::datasource::Envelope;

pub use dashboard::snapshot_json as dashboard_snapshot_json;
pub use datasource::snapshot_json as datasource_snapshot_json;
pub use folder::snapshot_json as folder_snapshot_json;
pub use manifest::{is_known_mutable_kind, KNOWN_MUTABLE_KINDS};
pub use nav_node::snapshot_json as nav_node_snapshot_json;
pub use panel::snapshot_json as panel_snapshot_json;
pub use query_kind::snapshot_json as query_kind_snapshot_json;

/// Register every nexus [`Reversible`] into `registry`. Called once at boot. Each
/// impl closes over the metadata pool (and, for secret-bearing kinds, the secret
/// envelope) so undo can apply inverses against the store inside a tenant
/// transaction.
pub fn register_all(
    registry: ReversibleRegistry,
    metadata: PgPool,
    envelope: Envelope,
) -> ReversibleRegistry {
    registry
        .insert(Arc::new(dashboard::DashboardReversible::new(
            metadata.clone(),
        )))
        .insert(Arc::new(panel::PanelReversible::new(metadata.clone())))
        .insert(Arc::new(folder::FolderReversible::new(metadata.clone())))
        .insert(Arc::new(nav_node::NavNodeReversible::new(metadata.clone())))
        .insert(Arc::new(query_kind::QueryKindReversible::new(
            metadata.clone(),
        )))
        .insert(Arc::new(datasource::DatasourceReversible::new(
            metadata, envelope,
        )))
}

/// The kinds [`register_all`] wires, as their stable discriminators. This is the
/// single list the boot registration and the coverage guard both read, so a kind
/// added to one without the other is a test failure rather than a silent gap. A
/// new owning workstream's reference impl appends here in the same PR that adds
/// its `register_all` line. Kept in lockstep with [`KNOWN_MUTABLE_KINDS`] by
/// [`tests::every_registered_kind_is_on_the_manifest`].
pub const REGISTERED_KINDS: &[&str] = &[
    crate::authz::KIND_DASHBOARD,
    crate::authz::KIND_PANEL,
    crate::authz::KIND_FOLDER,
    crate::authz::KIND_DATASOURCE,
    crate::authz::KIND_QUERY_KIND,
    crate::authz::KIND_NAV_NODE,
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Coverage guard (WS-12 §3.5b), the pure half: every kind the registry wires
    /// must be on the "known mutable kinds" manifest, and vice versa. A registered
    /// kind absent from the manifest (or a manifest kind never registered) is the
    /// silently-partial-audit failure mode — caught here, in CI, not in prod.
    #[test]
    fn every_registered_kind_is_on_the_manifest() {
        for kind in REGISTERED_KINDS {
            assert!(
                is_known_mutable_kind(kind),
                "registered kind {kind:?} is missing from KNOWN_MUTABLE_KINDS — \
                 add it so the audit coverage guard tracks it",
            );
        }
        for kind in KNOWN_MUTABLE_KINDS {
            assert!(
                REGISTERED_KINDS.contains(kind),
                "manifest kind {kind:?} has no Reversible registered in register_all — \
                 the audit log would be silently partial for it",
            );
        }
    }

    /// The reference impls' `kind()` discriminators must match the registry list,
    /// so a typo in an impl can't diverge from what the guard checks.
    #[test]
    fn reference_impls_report_registered_kinds() {
        assert!(REGISTERED_KINDS.contains(&crate::authz::KIND_DASHBOARD));
        assert!(REGISTERED_KINDS.contains(&crate::authz::KIND_DATASOURCE));
    }
}
