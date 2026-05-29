//! Data cleanup — the reusable mechanism for purging everything an
//! extension owns, with the *knowledge* of how to drop a given resource
//! supplied by per-consumer providers.
//!
//! This is the crux of the starter/rubix split (scope §4): the
//! **mechanism** — discovering what an extension owns and orchestrating
//! its removal in a dry-run-able, audited, idempotent way — lives here.
//! The **knowledge of how to drop a Timescale table or unregister a
//! skill** lives in rubix, because only rubix owns the warehouse and the
//! skill registry.
//!
//! ```ignore
//! pub trait CleanupProvider {
//!     async fn discover(&self, id, manifest) -> Vec<CleanupItem>;
//!     async fn purge(&self, id, items) -> Result<(), CleanupError>;
//! }
//! ```
//!
//! Built-in providers ship here because they need no rubix knowledge:
//!
//! - [`EnablementRowProvider`] — `DELETE`s the persistence row outright
//!   (today's uninstall only flips it to `Disabled`, leaving a ghost row;
//!   see `lifecycle.rs`).
//! - [`UiCacheProvider`] / [`I18nCacheProvider`] — evict the ETag/byte
//!   caches in [`crate::ui`] / [`crate::i18n`] for the extension's path
//!   prefix. This is the literal "sidebar" cleanup: the `sidebar` /
//!   `sidebar-nav` Module-Federation slots are served from that cache, so
//!   an uninstalled panel lingers until restart unless the bytes are
//!   dropped.
//!
//! Rubix supplies the warehouse-table and skill providers at compose time
//! via [`crate::ExtensionAdminBuilder::with_cleanup_provider`].
//!
//! Every destructive step logs `target: "starter_ext_server::cleanup"`
//! with the caller principal (see [`crate::admin::ExtensionAdmin`]).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;
use starter_ext_host::ExtensionRegistry;
use starter_ext_spi::{ExtensionId, Manifest};

use crate::etag::EtagCache;
use crate::store::EnablementStore;

/// The class of a reclaimable resource. Stable, non-localised wire token —
/// consumers map it onto their own `MessageKey` catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupKind {
    /// A warehouse table (and its continuous aggregates) owned by the
    /// extension's `com_<id>__*` namespace. Supplied by rubix.
    WarehouseTable,
    /// The extension's `extensions_enablement` persistence row.
    EnablementRow,
    /// Cached UI bundle bytes/ETags served from `GET /extensions/<id>/ui`.
    UiCache,
    /// Cached i18n catalog bytes/ETags served from
    /// `GET /extensions/<id>/i18n/<lang>`.
    I18nCache,
    /// A `SKILL.md` bundle the extension contributed. Supplied by rubix.
    Skill,
    /// An event-bus / warehouse subscription the extension owns. Supplied
    /// by rubix.
    Subscription,
}

/// One reclaimable resource an extension owns. The `bytes` field is a
/// best-effort size for the dry-run report; `None` when the provider
/// cannot cheaply size the resource (e.g. a single DB row).
#[derive(Debug, Clone, Serialize)]
pub struct CleanupItem {
    /// What class of resource this is.
    pub kind: CleanupKind,
    /// Human-readable label — e.g. `com_rubix_geo__pins` for a table, or
    /// the canonical on-disk path for a cached bundle file.
    pub label: String,
    /// Best-effort size in bytes for the dry-run report.
    pub bytes: Option<u64>,
}

/// Error returned by a [`CleanupProvider::purge`]. Surfaced as a logged
/// warning by the orchestrator; one provider failing never aborts the
/// others (purge is best-effort and idempotent).
#[derive(Debug, thiserror::Error)]
#[error("cleanup error: {0}")]
pub struct CleanupError(pub String);

impl CleanupError {
    /// Construct from any displayable type.
    pub fn new(msg: impl std::fmt::Display) -> Self {
        Self(msg.to_string())
    }
}

/// A reclaimer for one class of extension-owned data.
///
/// `discover` is read-only and powers the `GET /extensions/<id>/cleanup`
/// dry-run; `purge` performs the removal and runs on
/// `DELETE /extensions/<id>?purge=true`.
///
/// `manifest` is `Option` rather than `&Manifest` (the scope sketch's
/// shape) so a provider can still reclaim leftovers for a `Failed` record
/// whose manifest never parsed — the enablement row and cached bytes
/// outlive a broken manifest, and the ghost-row idempotency contract
/// requires they still be reachable.
#[async_trait]
pub trait CleanupProvider: Send + Sync {
    /// Enumerate the resources this provider would remove for `id`,
    /// without removing anything.
    async fn discover(&self, id: &ExtensionId, manifest: Option<&Manifest>) -> Vec<CleanupItem>;

    /// Remove the supplied `items` (the ones this provider's own
    /// `discover` returned). Must be idempotent — re-running against an
    /// already-clean extension is a no-op `Ok(())`.
    async fn purge(&self, id: &ExtensionId, items: &[CleanupItem]) -> Result<(), CleanupError>;
}

// ---------------------------------------------------------------------------
// Built-in: enablement row
// ---------------------------------------------------------------------------

/// Removes the `extensions_enablement` row for the extension outright,
/// killing the ghost row today's uninstall leaves behind (it only flips
/// the row to `Disabled`).
pub struct EnablementRowProvider {
    store: Arc<dyn EnablementStore>,
}

impl EnablementRowProvider {
    /// Build against the same [`EnablementStore`] the toggle endpoints use.
    pub fn new(store: Arc<dyn EnablementStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl CleanupProvider for EnablementRowProvider {
    async fn discover(&self, id: &ExtensionId, _manifest: Option<&Manifest>) -> Vec<CleanupItem> {
        match self.store.get(id).await {
            Ok(Some(_)) => vec![CleanupItem {
                kind: CleanupKind::EnablementRow,
                label: id.as_str().to_owned(),
                bytes: None,
            }],
            // No row, or a transient store error — nothing to offer. A
            // genuine error surfaces again (and is logged) at purge time.
            _ => Vec::new(),
        }
    }

    async fn purge(&self, id: &ExtensionId, items: &[CleanupItem]) -> Result<(), CleanupError> {
        if items.iter().any(|i| i.kind == CleanupKind::EnablementRow) {
            self.store
                .delete(id)
                .await
                .map_err(|e| CleanupError::new(e.0))?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Built-in: UI + i18n bundle caches
// ---------------------------------------------------------------------------

/// Evicts the ETag/byte cache entries for the extension's UI bundle —
/// the literal "sidebar" cleanup. Scoped strictly to the extension's own
/// `ui` path prefix, so it can never touch another extension's cache.
pub struct UiCacheProvider {
    registry: Arc<ExtensionRegistry>,
    cache: Arc<EtagCache>,
}

impl UiCacheProvider {
    pub(crate) fn new(registry: Arc<ExtensionRegistry>, cache: Arc<EtagCache>) -> Self {
        Self { registry, cache }
    }

    /// Directory the UI wildcard is served from — `<bundle_dir>/<dir of
    /// the `ui.entry`>`. `None` when the extension contributes no UI or has
    /// no on-disk record.
    fn ui_root(&self, id: &ExtensionId, manifest: Option<&Manifest>) -> Option<PathBuf> {
        let rec = self.registry.get_by_id_str(id.as_str())?;
        let manifest = manifest.or(rec.manifest.as_ref())?;
        let entry = &manifest.contributes.ui.as_ref()?.entry;
        rec.bundle_dir.join(entry).parent().map(Path::to_path_buf)
    }
}

#[async_trait]
impl CleanupProvider for UiCacheProvider {
    async fn discover(&self, id: &ExtensionId, manifest: Option<&Manifest>) -> Vec<CleanupItem> {
        let Some(root) = self.ui_root(id, manifest) else {
            return Vec::new();
        };
        // Match against the canonical prefix while the files still exist
        // (this is the dry-run / pre-purge path); fall back to the lexical
        // root otherwise.
        let prefix = root.canonicalize().unwrap_or(root);
        self.cache
            .entries_under_prefix(&prefix)
            .into_iter()
            .map(|(path, bytes)| CleanupItem {
                kind: CleanupKind::UiCache,
                label: path.display().to_string(),
                bytes: Some(bytes),
            })
            .collect()
    }

    async fn purge(&self, _id: &ExtensionId, items: &[CleanupItem]) -> Result<(), CleanupError> {
        // Evict the exact canonical keys discovered, so eviction is correct
        // even if the bundle directory has already been removed on disk.
        for item in items.iter().filter(|i| i.kind == CleanupKind::UiCache) {
            self.cache.evict_exact(Path::new(&item.label));
        }
        Ok(())
    }
}

/// Evicts the ETag/byte cache entries for the extension's i18n catalog
/// files. Scoped to the catalog paths the manifest declares.
pub struct I18nCacheProvider {
    registry: Arc<ExtensionRegistry>,
    cache: Arc<EtagCache>,
}

impl I18nCacheProvider {
    pub(crate) fn new(registry: Arc<ExtensionRegistry>, cache: Arc<EtagCache>) -> Self {
        Self { registry, cache }
    }

    /// Resolve every declared catalog file path against the bundle dir.
    fn catalog_paths(&self, id: &ExtensionId, manifest: Option<&Manifest>) -> Vec<PathBuf> {
        let Some(rec) = self.registry.get_by_id_str(id.as_str()) else {
            return Vec::new();
        };
        let Some(manifest) = manifest.or(rec.manifest.as_ref()) else {
            return Vec::new();
        };
        let Some(i18n) = manifest.contributes.i18n.as_ref() else {
            return Vec::new();
        };
        i18n.catalogs
            .values()
            .map(|rel| rec.bundle_dir.join(rel))
            .collect()
    }
}

#[async_trait]
impl CleanupProvider for I18nCacheProvider {
    async fn discover(&self, id: &ExtensionId, manifest: Option<&Manifest>) -> Vec<CleanupItem> {
        let mut out = Vec::new();
        for path in self.catalog_paths(id, manifest) {
            let prefix = path.canonicalize().unwrap_or(path);
            for (cached, bytes) in self.cache.entries_under_prefix(&prefix) {
                out.push(CleanupItem {
                    kind: CleanupKind::I18nCache,
                    label: cached.display().to_string(),
                    bytes: Some(bytes),
                });
            }
        }
        out
    }

    async fn purge(&self, _id: &ExtensionId, items: &[CleanupItem]) -> Result<(), CleanupError> {
        for item in items.iter().filter(|i| i.kind == CleanupKind::I18nCache) {
            self.cache.evict_exact(Path::new(&item.label));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryEnablementStore;

    #[tokio::test]
    async fn enablement_row_discover_and_purge() {
        let store = Arc::new(InMemoryEnablementStore::new());
        let id = ExtensionId::new("com.acme.rows").unwrap();
        store
            .set(&id, crate::store::EnablementState::Disabled)
            .await
            .unwrap();
        let provider = EnablementRowProvider::new(store.clone());

        // discover finds the (ghost) row.
        let items = provider.discover(&id, None).await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, CleanupKind::EnablementRow);

        // purge deletes it; a re-discover finds nothing.
        provider.purge(&id, &items).await.unwrap();
        assert!(store.get(&id).await.unwrap().is_none());
        assert!(provider.discover(&id, None).await.is_empty());

        // purge on an absent row is an idempotent no-op.
        provider.purge(&id, &items).await.unwrap();
    }

    // --- UI / i18n cache providers ----------------------------------------

    use starter_ext_host::record::ExtensionRecord;
    use starter_ext_host::ExtensionRegistry;
    use starter_ext_spi::LifecycleState;
    use std::collections::HashMap;

    fn manifest_yaml(id: &str) -> String {
        format!(
            r#"
v: 1
id: {id}
version: 0.1.0
display_name: "Cache Test"
description_file: docs/README.md
authors: ["ap@nube-io.com"]
runtime:
  kind: builtin
  crate_name: cache-test
contributes:
  ui:
    entry: ui/remoteEntry.js
    exposes:
      - name: Panel
        module: ./Panel
        slot: sidebar
  i18n:
    catalogs:
      en: i18n/en.json
"#
        )
    }

    /// Build a tempdir bundle with a `ui/remoteEntry.js` and an
    /// `i18n/en.json`, returning `(tempdir, record, canonical ui file,
    /// canonical i18n file)`.
    fn make_bundle(id: &str) -> (tempfile::TempDir, ExtensionRecord, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join(id);
        std::fs::create_dir_all(bundle.join("ui")).unwrap();
        std::fs::create_dir_all(bundle.join("i18n")).unwrap();
        std::fs::write(bundle.join("ui/remoteEntry.js"), b"export const x = 1;").unwrap();
        std::fs::write(bundle.join("i18n/en.json"), b"{\"k\":\"v\"}").unwrap();
        let manifest: Manifest = serde_yaml::from_str(&manifest_yaml(id)).unwrap();
        let record = ExtensionRecord {
            id: Some(ExtensionId::new(id).unwrap()),
            id_hint: id.to_owned(),
            bundle_dir: bundle.clone(),
            state: LifecycleState::Validated,
            manifest: Some(manifest),
            failure: None,
            origin: starter_ext_host::BundleOrigin::default(),
        };
        let ui_file = bundle.join("ui/remoteEntry.js").canonicalize().unwrap();
        let i18n_file = bundle.join("i18n/en.json").canonicalize().unwrap();
        (dir, record, ui_file, i18n_file)
    }

    fn registry_of(records: Vec<ExtensionRecord>) -> Arc<ExtensionRegistry> {
        let mut reg = ExtensionRegistry::new();
        let map: HashMap<String, ExtensionRecord> = records
            .into_iter()
            .map(|r| (r.id_hint.clone(), r))
            .collect();
        reg.install(map);
        reg.seal();
        Arc::new(reg)
    }

    #[tokio::test]
    async fn ui_cache_discover_purge_and_scope() {
        let (_da, rec_a, ui_a, _i_a) = make_bundle("com.acme.a");
        let (_db, rec_b, ui_b, _i_b) = make_bundle("com.acme.b");
        let registry = registry_of(vec![rec_a, rec_b]);
        let cache = Arc::new(EtagCache::new());
        // Populate the cache by serving each extension's UI file.
        cache.etag_and_bytes(&ui_a).await.unwrap();
        cache.etag_and_bytes(&ui_b).await.unwrap();

        let id_a = ExtensionId::new("com.acme.a").unwrap();
        let provider = UiCacheProvider::new(registry.clone(), cache.clone());

        // discover sees only A's entry.
        let items = provider.discover(&id_a, None).await;
        assert_eq!(items.len(), 1, "exactly one UI cache entry for A");
        assert_eq!(items[0].kind, CleanupKind::UiCache);
        assert!(items[0].bytes.is_some());

        // purge evicts A; B remains (namespace scope).
        provider.purge(&id_a, &items).await.unwrap();
        assert!(cache.evict_exact(&ui_a).is_none(), "A's entry already gone");
        assert!(cache.evict_exact(&ui_b).is_some(), "B's entry untouched");
    }

    #[tokio::test]
    async fn i18n_cache_discover_purge_and_scope() {
        let (_da, rec_a, _ui_a, i_a) = make_bundle("com.acme.a");
        let (_db, rec_b, _ui_b, i_b) = make_bundle("com.acme.b");
        let registry = registry_of(vec![rec_a, rec_b]);
        let cache = Arc::new(EtagCache::new());
        cache.etag_and_bytes(&i_a).await.unwrap();
        cache.etag_and_bytes(&i_b).await.unwrap();

        let id_a = ExtensionId::new("com.acme.a").unwrap();
        let provider = I18nCacheProvider::new(registry.clone(), cache.clone());

        let items = provider.discover(&id_a, None).await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, CleanupKind::I18nCache);

        provider.purge(&id_a, &items).await.unwrap();
        assert!(cache.evict_exact(&i_a).is_none(), "A's catalog evicted");
        assert!(cache.evict_exact(&i_b).is_some(), "B's catalog untouched");
    }

    #[tokio::test]
    async fn enablement_row_namespace_scope() {
        let store = Arc::new(InMemoryEnablementStore::new());
        let a = ExtensionId::new("com.acme.a").unwrap();
        let b = ExtensionId::new("com.acme.b").unwrap();
        store
            .set(&a, crate::store::EnablementState::Enabled)
            .await
            .unwrap();
        store
            .set(&b, crate::store::EnablementState::Enabled)
            .await
            .unwrap();
        let provider = EnablementRowProvider::new(store.clone());

        let items = provider.discover(&a, None).await;
        provider.purge(&a, &items).await.unwrap();

        // a is gone, b is untouched.
        assert!(store.get(&a).await.unwrap().is_none());
        assert_eq!(
            store.get(&b).await.unwrap(),
            Some(crate::store::EnablementState::Enabled)
        );
    }
}
