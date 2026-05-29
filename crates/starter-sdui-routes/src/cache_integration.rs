//! v3 — SDUI cache integration (§Layer 2).
//!
//! The SDUI resolver wraps its `/ui/resolve` work in the
//! [`starter_cache::CacheLayer`] when the resolved [`ComponentTree`]
//! carries a `cache:` block. Cached objects are the
//! [`ResolveResponse`] (rendered tree + subscription plan); the key
//! covers `(tenant, user, page_id, target_ref, stack_hash,
//! page_state_hash, units_hash, ir_version, page_content_hash)` so
//! schema or page-content shifts implicitly invalidate.
//!
//! `/ui/table` caches independently with its own key shape
//! `(source_id, page, sort, filter, scope-vars)`. `/ui/action` is
//! never cached but action handlers can fire `invalidate_tags` via
//! the v1 read-only declaration mechanism extended to SDUI handlers
//! through [`SduiActionMeta`].
//!
//! The IR `cache:` block is additive on `ComponentTree`. There is no
//! IR version bump per §"What changed in this revision".

use sha2::{Digest, Sha256};
use starter_cache::{CacheLayer, CacheScope, CacheSpec, CallerScope, InvalidateOn};
use starter_ui_ir::{ComponentTree, PageCacheBlock, IR_VERSION};
use std::time::Duration;

/// Derive a [`CacheSpec`] from a SDUI `cache:` block.
pub fn spec_from_block(block: &PageCacheBlock) -> CacheSpec {
    let scope = match block.scope.as_str() {
        "user" => CacheScope::User,
        "global" => CacheScope::Global,
        _ => CacheScope::Tenant,
    };
    let mut spec = CacheSpec::ttl(Duration::from_secs(block.ttl_seconds)).scope(scope);
    spec.stale_while_revalidate = Duration::from_secs(block.stale_while_revalidate_seconds);
    spec.invalidate_on = InvalidateOn {
        tables: block.invalidate_on_tables.clone(),
        events: Vec::new(),
        buckets: None,
    };
    spec
}

/// Build the per-resolve base key. Caller provides every dimension
/// the proposal pins; we hash them into one opaque key string.
#[allow(clippy::too_many_arguments)]
pub fn resolve_base_key(
    page_id: &str,
    target_ref: &str,
    stack_hash: &str,
    page_state_hash: &str,
    units_hash: &str,
    page_content_hash: &str,
) -> String {
    let mut h = Sha256::new();
    h.update(b"v3-resolve");
    h.update(page_id.as_bytes());
    h.update([0u8]);
    h.update(target_ref.as_bytes());
    h.update([0u8]);
    h.update(stack_hash.as_bytes());
    h.update([0u8]);
    h.update(page_state_hash.as_bytes());
    h.update([0u8]);
    h.update(units_hash.as_bytes());
    h.update([0u8]);
    h.update(page_content_hash.as_bytes());
    h.update([0u8]);
    h.update(IR_VERSION.to_le_bytes());
    format!("sdui-resolve::{:x}", h.finalize())
}

/// Base key shape for `/ui/table` per §Layer 2: independent of the
/// resolve cache, keyed by `(source_id, page, sort, filter,
/// scope-vars)`.
pub fn table_base_key(
    source_id: &str,
    page: u32,
    sort: &str,
    filter: &str,
    scope_vars: &str,
) -> String {
    let mut h = Sha256::new();
    h.update(b"v3-table");
    h.update(source_id.as_bytes());
    h.update([0u8]);
    h.update(page.to_le_bytes());
    h.update(sort.as_bytes());
    h.update([0u8]);
    h.update(filter.as_bytes());
    h.update([0u8]);
    h.update(scope_vars.as_bytes());
    format!("sdui-table::{:x}", h.finalize())
}

/// v3 — read-only handler declaration extended to SDUI action
/// handlers. Mirrors `starter_ext_server::HandlerMeta` so the v1
/// dispatcher mechanism applies uniformly across surfaces; SDUI
/// keeps its own copy so the SDUI crate stays decoupled from the
/// extension dispatcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SduiActionMeta {
    /// `true` for handlers that do no writes (refresh, recompute,
    /// export).
    pub read_only: bool,
    /// Tables this handler writes to — fires
    /// `invalidate_tags(["table:<name>", …])` after success.
    pub affects_tables: Vec<String>,
}

impl SduiActionMeta {
    /// Compose the invalidation tag list (writing handlers only).
    pub fn invalidation_tags(&self) -> Vec<String> {
        self.affects_tables
            .iter()
            .map(|t| format!("table:{t}"))
            .collect()
    }
}

/// Helper the resolver calls after a successful `/ui/action`
/// dispatch. No-op for read-only handlers and when the cache layer
/// is unwired.
pub async fn fire_action_invalidation(layer: Option<&CacheLayer>, meta: &SduiActionMeta) {
    if meta.read_only {
        return;
    }
    let tags = meta.invalidation_tags();
    if tags.is_empty() {
        return;
    }
    if let Some(layer) = layer {
        layer.invalidator().invalidate_tags(&tags).await;
    }
}

/// Convenience: get the cache block from a [`ComponentTree`].
pub fn cache_block(tree: &ComponentTree) -> Option<&PageCacheBlock> {
    tree.cache.as_ref()
}

/// Wrap a resolver future under the page's `cache:` block. Returns
/// the resolved JSON bytes — caller deserialises to its
/// `ResolveResponse` type. When the tree has no cache block, the
/// helper bypasses the layer.
pub async fn wrap_resolve<F, Fut, E>(
    layer: Option<&CacheLayer>,
    spec_id: Option<&str>,
    tree: &ComponentTree,
    caller: &CallerScope,
    base_key: &str,
    load: F,
) -> Result<std::sync::Arc<Vec<u8>>, E>
where
    F: FnOnce() -> Fut + Send,
    Fut: std::future::Future<Output = Result<std::sync::Arc<Vec<u8>>, E>> + Send,
    E: Send + Sync + 'static,
{
    let Some((layer, block)) = layer.zip(tree.cache.as_ref()) else {
        return load().await;
    };
    let spec = spec_from_block(block);
    layer
        .get_or_load_labelled(&spec, spec_id, caller, base_key, load)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_ui_ir::{Component, ComponentTree};

    fn block() -> PageCacheBlock {
        PageCacheBlock {
            ttl_seconds: 60,
            scope: "user".into(),
            stale_while_revalidate_seconds: 30,
            invalidate_on_tables: vec!["dashboards".into()],
            tags: vec!["dashboard".into()],
        }
    }

    #[test]
    fn spec_from_block_carries_ttl_and_swr() {
        let s = spec_from_block(&block());
        assert_eq!(s.ttl, Duration::from_secs(60));
        assert_eq!(s.scope, CacheScope::User);
        assert_eq!(s.stale_while_revalidate, Duration::from_secs(30));
        assert_eq!(s.invalidate_on.tables, vec!["dashboards"]);
    }

    #[test]
    fn resolve_key_varies_per_dimension() {
        let a = resolve_base_key("p", "t1", "s", "ps", "u", "pc");
        let b = resolve_base_key("p", "t2", "s", "ps", "u", "pc");
        assert_ne!(a, b);
    }

    #[test]
    fn table_key_varies_per_input() {
        let a = table_base_key("src", 1, "asc", "f", "");
        let b = table_base_key("src", 2, "asc", "f", "");
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn wrap_resolve_bypasses_when_no_block() {
        let tree = ComponentTree::new(Component::Page {
            id: "p".into(),
            title: None,
            header_actions: vec![],
            children: vec![],
            style: None,
            default_row_gap: None,
            default_column_gap: None,
            default_page_padding: None,
            default_max_width: None,
        });
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let c1 = calls.clone();
        let r = wrap_resolve::<_, _, std::convert::Infallible>(
            None,
            None,
            &tree,
            &CallerScope::system(),
            "k",
            || async move {
                c1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(std::sync::Arc::new(b"hi".to_vec()))
            },
        )
        .await
        .unwrap();
        assert_eq!(&*r, b"hi");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn action_writing_handler_fires_tags() {
        use starter_cache::{CacheLayer, LayerConfig};
        let layer = CacheLayer::new(LayerConfig::default());
        let meta = SduiActionMeta {
            read_only: false,
            affects_tables: vec!["dashboards".into()],
        };
        fire_action_invalidation(Some(&layer), &meta).await;
        // Read-only handler is a no-op.
        let ro = SduiActionMeta {
            read_only: true,
            affects_tables: vec![],
        };
        fire_action_invalidation(Some(&layer), &ro).await;
    }
}
