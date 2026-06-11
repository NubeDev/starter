//! nexus-owned [`CleanupProvider`]s — the knowledge of how to reclaim
//! nexus-side state an extension contributed, registered with the kernel's
//! cleanup mechanism (WS-14 §4.1.4).
//!
//! The kernel ships the built-in providers (enablement row, UI/i18n cache); the
//! *consumer* supplies providers for state only it owns. nexus owns the
//! query-kinds an extension contributes (`nexus_extension_query_kinds`, the
//! third dispatch source), so [`QueryKindCleanupProvider`] discovers and purges
//! those rows on uninstall. This is the single place extension-owned nexus
//! cleanup is declared; a future datasource-kind provider (WS-08) registers the
//! same way.
//!
//! `discover` powers the dry-run manifest the admin UI shows before a purge;
//! `purge` is idempotent — re-running on an already-clean extension deletes
//! nothing and returns `Ok(())`, satisfying the WS-14 re-purge contract.

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::{Executor, PgPool};
use starter_ext_host::ExtensionRegistry;
use starter_ext_server::{CleanupError, CleanupItem, CleanupKind, CleanupProvider};
use starter_ext_spi::{ExtensionId, Manifest};

use nexus_store::extension_query_kind;

use super::warehouse::full_table_name;

/// Reclaims the query-kinds an extension contributed (WS-10 templates → the
/// `nexus_extension_query_kinds` third source). Holds the metadata pool so it
/// can list (dry-run) and delete (purge) by `extension_id`.
pub struct QueryKindCleanupProvider {
    metadata: PgPool,
}

impl QueryKindCleanupProvider {
    /// Wrap the metadata pool. Registered via
    /// `ExtensionAdminBuilder::with_cleanup_provider`.
    pub fn new(metadata: PgPool) -> Self {
        Self { metadata }
    }
}

#[async_trait]
impl CleanupProvider for QueryKindCleanupProvider {
    async fn discover(&self, id: &ExtensionId, _manifest: Option<&Manifest>) -> Vec<CleanupItem> {
        // Source of truth is the persisted rows, not the manifest: a kind the
        // extension contributed on a *previous* version is still owned by this
        // id and must surface in the dry-run even if the current manifest no
        // longer declares it. Listing by owner captures exactly what purge will
        // remove.
        match extension_query_kind::list_by_extension(&self.metadata, id.as_str()).await {
            Ok(kinds) => kinds
                .into_iter()
                .map(|k| CleanupItem {
                    // The closest stable wire token: a contributed query-kind is
                    // a warehouse template materialised into a table. The kernel's
                    // `CleanupKind` enum is fixed; `WarehouseTable` is the honest
                    // class (it names rows in a nexus table the extension owns),
                    // and the human-readable `label` carries the kind name so the
                    // admin UI shows exactly which kind goes.
                    kind: CleanupKind::WarehouseTable,
                    label: format!("query-kind {}", k.name),
                    bytes: None,
                })
                .collect(),
            Err(e) => {
                // A discovery failure must not block the dry-run for the other
                // providers; surface nothing and log. The purge below still runs
                // the delete, which is the authoritative cleanup.
                tracing::warn!(
                    target: "nexus_api::extensions::cleanup",
                    extension = %id.as_str(),
                    error = %e,
                    "query-kind cleanup discover failed"
                );
                Vec::new()
            }
        }
    }

    async fn purge(&self, id: &ExtensionId, _items: &[CleanupItem]) -> Result<(), CleanupError> {
        // Delete by owner, not by the discovered item list: the list is a
        // best-effort dry-run snapshot, but purge must remove *everything* this
        // extension owns even if discover under-reported (e.g. raced an install).
        // Idempotent — a second purge deletes zero rows and still returns Ok.
        let removed = extension_query_kind::delete_by_extension(&self.metadata, id.as_str())
            .await
            .map_err(|e| CleanupError::new(format!("delete query-kinds for {}: {e}", id.as_str())))?;
        if removed > 0 {
            tracing::info!(
                target: "nexus_api::extensions::cleanup",
                extension = %id.as_str(),
                removed,
                "purged contributed query-kinds"
            );
        }
        Ok(())
    }
}

/// Reclaims the **owned tables** an extension declared in
/// `contributes.warehouse_tables[]` (WS-17 §4.1.4). On `DELETE
/// /extensions/<id>?purge=true` it `DROP TABLE IF EXISTS`es each
/// `<sanitized_ext_id>__<name>` from the nexus Postgres.
///
/// Table names come from the **sealed registry's manifest**, not the DB:
/// there is no provenance table for owned tables (the table itself is the
/// provenance — its `<ext>__` prefix is the ownership marker), so the manifest
/// is the source of truth for what to drop. The dry-run (`discover`) lists the
/// fully-qualified names that purge will drop; `purge` is idempotent
/// (`IF EXISTS`).
pub struct WarehouseTableCleanupProvider {
    metadata: PgPool,
    registry: Arc<ExtensionRegistry>,
}

impl WarehouseTableCleanupProvider {
    /// Wrap the metadata pool + the sealed registry (to read the purging
    /// extension's declared tables). Registered via
    /// `ExtensionAdminBuilder::with_cleanup_provider`.
    pub fn new(metadata: PgPool, registry: Arc<ExtensionRegistry>) -> Self {
        Self { metadata, registry }
    }

    /// Resolve the host-managed (DDL-owned) table names this extension declares,
    /// fully qualified with the `<ext>__` prefix. Prefers the manifest passed to
    /// `discover`; falls back to the sealed registry (so `purge`, which gets no
    /// manifest, can still find them). Continuous-aggregate entries are excluded
    /// — the host never created them, so it must not drop them.
    fn owned_tables(&self, id: &ExtensionId, manifest: Option<&Manifest>) -> Vec<String> {
        let from_registry;
        let manifest = match manifest {
            Some(m) => Some(m),
            None => {
                from_registry = self
                    .registry
                    .get_by_id_str(id.as_str())
                    .and_then(|r| r.manifest.clone());
                from_registry.as_ref()
            }
        };
        let Some(manifest) = manifest else {
            return Vec::new();
        };
        manifest
            .contributes
            .warehouse_tables
            .iter()
            .filter(|t| t.kind.host_manages_ddl())
            .map(|t| full_table_name(id, &t.name))
            .collect()
    }
}

#[async_trait]
impl CleanupProvider for WarehouseTableCleanupProvider {
    async fn discover(&self, id: &ExtensionId, manifest: Option<&Manifest>) -> Vec<CleanupItem> {
        self.owned_tables(id, manifest)
            .into_iter()
            .map(|full| CleanupItem {
                kind: CleanupKind::WarehouseTable,
                label: format!("table {full}"),
                bytes: None,
            })
            .collect()
    }

    async fn purge(&self, id: &ExtensionId, _items: &[CleanupItem]) -> Result<(), CleanupError> {
        // Drop by manifest declaration (resolved from the registry), not by the
        // `_items` snapshot — purge must remove everything the extension owns
        // even if discover under-reported. `DROP TABLE IF EXISTS` is idempotent,
        // so a second purge is a clean no-op.
        let tables = self.owned_tables(id, None);
        for full in &tables {
            let sql = format!("DROP TABLE IF EXISTS \"{}\"", full.replace('"', "\"\""));
            self.metadata.execute(sql.as_str()).await.map_err(|e| {
                CleanupError::new(format!("drop owned table {full} for {}: {e}", id.as_str()))
            })?;
        }
        if !tables.is_empty() {
            tracing::info!(
                target: "nexus_api::extensions::cleanup",
                extension = %id.as_str(),
                dropped = tables.len(),
                "purged extension-owned warehouse tables"
            );
        }
        Ok(())
    }
}
