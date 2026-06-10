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

use async_trait::async_trait;
use sqlx::PgPool;
use starter_ext_server::{CleanupError, CleanupItem, CleanupKind, CleanupProvider};
use starter_ext_spi::{ExtensionId, Manifest};

use nexus_store::extension_query_kind;

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
