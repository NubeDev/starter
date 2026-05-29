//! Opt-in cache wiring for the kind dispatcher.
//!
//! This is the **only** integration point the v0 caching cut wires
//! up — per the proposal's "Minimum viable v0" section, SDUI
//! integration, tower layer, and the rest stay off until the
//! un-defer conditions fire.
//!
//! Shape:
//!
//! 1. A host loads `*.cache.yaml` sidecars next to each kind file
//!    (e.g. `kinds/usage_bucketed.cache.yaml`) and builds a
//!    [`KindCacheRegistry`] mapping `(extension, contribute_id) ->
//!    CacheSpec`.
//! 2. The host calls
//!    [`super::BuiltinRestDispatcher::with_cache`] to hand the
//!    registry and a [`CacheLayer`] to the dispatcher.
//! 3. On `dispatch()`, the dispatcher looks up the spec; if present,
//!    it wraps the call in `cache_layer.get_or_load(...)`, hashing
//!    the input + extension + contribute_id for the base key. If
//!    absent, the dispatcher behaves exactly as before (no-op).
//!
//! Write-path invalidation is **best-effort** in v0 — see the
//! `// TODO(cache-invalidation):` markers in the warehouse write
//! sites. The unified `WarehouseWriter` chokepoint is a separate
//! project (one of the proposal's un-defer conditions).

use sha2::{Digest, Sha256};
use starter_cache::{CacheLayer, CacheSpec, CallerScope};
use starter_ext_spi::identity::CallerIdentity;
use starter_ext_spi::ExtensionId;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Maps `(extension, contribute_id)` to a parsed [`CacheSpec`].
/// Built at host startup; immutable thereafter in v0.
#[derive(Debug, Default, Clone)]
pub struct KindCacheRegistry {
    entries: Arc<HashMap<(ExtensionId, String), CacheSpec>>,
}

impl KindCacheRegistry {
    /// Build an empty registry.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build from a flat list of `((extension, contribute_id), spec)`
    /// entries. Last entry wins on duplicate keys (no v0 collision
    /// rule — the host is expected to load each sidecar once).
    pub fn from_entries<I>(entries: I) -> Self
    where
        I: IntoIterator<Item = ((ExtensionId, String), CacheSpec)>,
    {
        let map: HashMap<_, _> = entries.into_iter().collect();
        Self {
            entries: Arc::new(map),
        }
    }

    /// Look up the spec for one kind.
    pub fn get(&self, ext: &ExtensionId, contribute_id: &str) -> Option<&CacheSpec> {
        self.entries.get(&(ext.clone(), contribute_id.to_string()))
    }

    /// Number of registered specs.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True if no specs are registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Load every `*.cache.yaml` file from the given directory and
    /// associate it with `(extension, <stem>)` — the stem is the
    /// part of the filename before `.cache.yaml`, taken as the
    /// contribute_id.
    ///
    /// Returns the registry plus any parse errors (so a typo in one
    /// sidecar does not block the rest from loading). Hosts decide
    /// whether to log-or-die on errors.
    pub fn load_from_dir(
        ext: &ExtensionId,
        dir: &Path,
    ) -> std::io::Result<(Self, Vec<SidecarLoadError>)> {
        let mut entries: HashMap<(ExtensionId, String), CacheSpec> = HashMap::new();
        let mut errors: Vec<SidecarLoadError> = Vec::new();

        let read = std::fs::read_dir(dir)?;
        for ent in read.flatten() {
            let path = ent.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };
            let stem = match name.strip_suffix(".cache.yaml") {
                Some(s) => s.to_string(),
                None => continue,
            };
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    errors.push(SidecarLoadError {
                        path: path.clone(),
                        message: format!("read: {e}"),
                    });
                    continue;
                }
            };
            match starter_cache::CacheSidecar::from_yaml(&content)
                .and_then(|s| s.into_spec())
            {
                Ok(spec) => {
                    entries.insert((ext.clone(), stem), spec);
                }
                Err(e) => {
                    errors.push(SidecarLoadError {
                        path,
                        message: format!("parse: {e}"),
                    });
                }
            }
        }
        Ok((
            Self {
                entries: Arc::new(entries),
            },
            errors,
        ))
    }
}

/// A sidecar that failed to load. The host decides whether to fail
/// startup or just warn — the loader returns these inline.
#[derive(Debug)]
pub struct SidecarLoadError {
    /// The file path that failed.
    pub path: std::path::PathBuf,
    /// Human-readable error message.
    pub message: String,
}

/// Bundle the dispatcher receives via [`super::BuiltinRestDispatcher::with_cache`].
#[derive(Clone)]
pub struct DispatcherCache {
    /// Where the work lands.
    pub layer: CacheLayer,
    /// Per-kind specs.
    pub registry: KindCacheRegistry,
}

impl DispatcherCache {
    /// Convenience builder.
    pub fn new(layer: CacheLayer, registry: KindCacheRegistry) -> Self {
        Self { layer, registry }
    }
}

/// Translate a `CallerIdentity` into the [`CallerScope`] shape the
/// cache layer needs.
pub(crate) fn caller_scope(caller: Option<&CallerIdentity>) -> CallerScope {
    match caller {
        Some(c) => CallerScope {
            tenant: c.tenant_id.clone(),
            user: c.user_id.clone(),
        },
        None => CallerScope::system(),
    }
}

/// Derive the cache layer's `base_key` for a kind dispatch:
/// `<extension>::<contribute_id>::<sha256(input)>`. The hash is
/// truncated to the first 16 hex chars — collisions across distinct
/// inputs are still cryptographically negligible and keeps keys
/// readable in tracing.
pub(crate) fn dispatch_base_key(
    ext: &ExtensionId,
    contribute_id: &str,
    input: &serde_json::Value,
) -> String {
    let canonical = input.to_string();
    let mut h = Sha256::new();
    h.update(canonical.as_bytes());
    let digest = h.finalize();
    let hex = hex::encode(&digest[..8]);
    format!("{}::{}::{}", ext.as_str(), contribute_id, hex)
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_cache::{CacheScope, CacheSpec};
    use std::time::Duration;

    #[test]
    fn from_entries_round_trips() {
        let ext = ExtensionId::new("com.example.foo").unwrap();
        let spec = CacheSpec::ttl(Duration::from_secs(30)).scope(CacheScope::User);
        let r = KindCacheRegistry::from_entries([((ext.clone(), "bar".to_string()), spec.clone())]);
        assert_eq!(r.len(), 1);
        assert_eq!(r.get(&ext, "bar").unwrap().ttl, Duration::from_secs(30));
        assert!(r.get(&ext, "baz").is_none());
    }

    #[test]
    fn load_from_dir_picks_up_sidecars() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("usage_bucketed.cache.yaml"),
            "cache:\n  ttl: 60s\n  scope: user\n  invalidate_on:\n    tables:\n      - readings\n",
        )
        .unwrap();
        // A non-cache file should be ignored.
        std::fs::write(dir.path().join("usage_bucketed.sql"), "select 1").unwrap();

        let ext = ExtensionId::new("com.example.ex").unwrap();
        let (reg, errors) = KindCacheRegistry::load_from_dir(&ext, dir.path()).unwrap();
        assert!(errors.is_empty(), "errors: {errors:?}");
        let spec = reg.get(&ext, "usage_bucketed").expect("spec present");
        assert_eq!(spec.ttl, Duration::from_secs(60));
        assert_eq!(spec.scope, CacheScope::User);
    }
}
