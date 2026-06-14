//! nexus-owned [`CleanupProvider`] for extension-contributed insights — the
//! dual of [`super::cleanup`] for the insight stage.
//!
//! nexus owns the insights an extension contributes (`nexus_extension_insights`,
//! the global insight registry), so [`InsightCleanupProvider`] discovers and
//! purges those rows on uninstall. Registered alongside the query-kind provider
//! via a second `ExtensionAdminBuilder::with_cleanup_provider` call (the kernel
//! holds providers in a `Vec`).
//!
//! `discover` powers the dry-run manifest the admin UI shows before a purge;
//! `purge` is idempotent — re-running on an already-clean extension deletes
//! nothing and returns `Ok(())`, satisfying the WS-14 re-purge contract.

use async_trait::async_trait;
use sqlx::PgPool;
use starter_ext_server::{CleanupError, CleanupItem, CleanupKind, CleanupProvider};
use starter_ext_spi::{ExtensionId, Manifest};

use nexus_store::extension_insight;

/// Reclaims the insights an extension contributed (`contributes.insights[]` →
/// the `nexus_extension_insights` registry). Holds the metadata pool so it can
/// list (dry-run) and delete (purge) by `extension_id`.
pub struct InsightCleanupProvider {
    metadata: PgPool,
}

impl InsightCleanupProvider {
    /// Wrap the metadata pool. Registered via
    /// `ExtensionAdminBuilder::with_cleanup_provider`.
    pub fn new(metadata: PgPool) -> Self {
        Self { metadata }
    }
}

#[async_trait]
impl CleanupProvider for InsightCleanupProvider {
    async fn discover(&self, id: &ExtensionId, _manifest: Option<&Manifest>) -> Vec<CleanupItem> {
        // Source of truth is the persisted rows, not the manifest: an insight the
        // extension contributed on a previous version is still owned by this id
        // and must surface in the dry-run even if the current manifest no longer
        // declares it.
        match extension_insight::list_by_extension(&self.metadata, id.as_str()).await {
            Ok(insights) => insights
                .into_iter()
                .map(|i| CleanupItem {
                    // The kernel's `CleanupKind` enum is fixed; `WarehouseTable`
                    // is the closest honest class (it names rows in a nexus table
                    // the extension owns, exactly as the query-kind provider
                    // does), and the label carries the insight name.
                    kind: CleanupKind::WarehouseTable,
                    label: format!("insight {}", i.name),
                    bytes: None,
                })
                .collect(),
            Err(e) => {
                tracing::warn!(
                    target: "nexus_api::extensions::cleanup_insights",
                    extension = %id.as_str(),
                    error = %e,
                    "insight cleanup discover failed"
                );
                Vec::new()
            }
        }
    }

    async fn purge(&self, id: &ExtensionId, _items: &[CleanupItem]) -> Result<(), CleanupError> {
        // Delete by owner, not by the discovered item list: purge must remove
        // everything this extension owns even if discover under-reported.
        // Idempotent — a second purge deletes zero rows and still returns Ok.
        let removed = extension_insight::delete_by_extension(&self.metadata, id.as_str())
            .await
            .map_err(|e| CleanupError::new(format!("delete insights for {}: {e}", id.as_str())))?;
        if removed > 0 {
            tracing::info!(
                target: "nexus_api::extensions::cleanup_insights",
                extension = %id.as_str(),
                removed,
                "purged contributed insights"
            );
        }
        Ok(())
    }
}
