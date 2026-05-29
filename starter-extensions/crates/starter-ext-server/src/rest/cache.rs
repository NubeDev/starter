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

    /// Iterate every registered `(extension, contribute_id, spec)`.
    /// Order is unspecified — `KindCacheRegistry` is `HashMap`-backed.
    pub fn iter(&self) -> impl Iterator<Item = (&ExtensionId, &str, &CacheSpec)> + '_ {
        self.entries
            .iter()
            .map(|((ext, id), spec)| (ext, id.as_str(), spec))
    }

    /// Check every registered spec against the set of contribute ids
    /// the host knows about. A sidecar named for a kind that does
    /// not exist in the manifest (typo: `useage_bucketed.cache.yaml`,
    /// stale sidecar after a kind was renamed, …) silently degrades
    /// to a no-op without this check — the dispatcher just never
    /// finds a matching spec because the request's contribute_id
    /// doesn't match the sidecar's stem.
    ///
    /// Returns the list of orphan sidecars. Hosts decide whether to
    /// log, fail startup, or both — same posture as
    /// [`load_from_dir`](Self::load_from_dir)'s parse errors.
    ///
    /// `known_ids_for_ext(ext)` should return the iterator of every
    /// contribute id (`contributes.tools[].id`, `contributes.rest[].id`,
    /// …) the manifest declares for `ext`. The validator does the
    /// `HashSet` lookup; the caller picks how to walk the manifest.
    pub fn orphans<'a, F, I>(&'a self, mut known_ids_for_ext: F) -> Vec<OrphanSidecar>
    where
        F: FnMut(&ExtensionId) -> I,
        I: IntoIterator<Item = &'a str>,
    {
        use std::collections::HashSet;
        let mut by_ext: HashMap<ExtensionId, HashSet<String>> = HashMap::new();
        let mut out = Vec::new();
        for (ext, id) in self.entries.keys() {
            let known = by_ext.entry(ext.clone()).or_insert_with(|| {
                known_ids_for_ext(ext)
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect()
            });
            if !known.contains(id) {
                out.push(OrphanSidecar {
                    extension: ext.clone(),
                    contribute_id: id.clone(),
                });
            }
        }
        out.sort_by(|a, b| {
            a.extension
                .as_str()
                .cmp(b.extension.as_str())
                .then_with(|| a.contribute_id.cmp(&b.contribute_id))
        });
        out
    }

    /// Load every `*.cache.yaml` file from the given directory and
    /// associate it with `(extension, <stem>)` — the stem is the
    /// part of the filename before `.cache.yaml`, taken verbatim as
    /// the contribute_id.
    ///
    /// **Filename convention:** the stem must be the **full
    /// reverse-DNS contribute_id** the dispatcher uses
    /// (`com.nubeio.rubixos.warehouse_query.cache.yaml`), not the
    /// bare trailing segment (`warehouse_query.cache.yaml`). The
    /// dispatcher looks up `(extension_id, contribute_id)` and
    /// `contribute_id` is the full reverse-DNS one. A bare-name
    /// sidecar parses fine but never matches anything — call
    /// [`Self::orphans`] after loading to catch this.
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
            match starter_cache::CacheSidecar::from_yaml(&content).and_then(|s| s.into_spec()) {
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

/// A sidecar that parsed cleanly but whose stem does not match any
/// declared contribute id in its extension's manifest. Sees the
/// light of day only after `KindCacheRegistry::orphans` is called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrphanSidecar {
    /// The extension that owns the sidecar.
    pub extension: ExtensionId,
    /// The `<stem>` that did not match any manifest contribute id.
    pub contribute_id: String,
}

/// Per-(extension, contribute_id) handler metadata. Used by the
/// dispatcher to decide whether a successful call should fire
/// write-path invalidation tags.
///
/// v1 of the [opt-in caching proposal][1] makes this a real
/// (not best-effort) invalidation hook for handlers the dispatcher
/// owns. The warehouse-write chokepoint stays a v3 problem.
///
/// [1]: ../../../rubix/docs/proposal/fe-cache-opt-in.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerMeta {
    /// `true` if the handler does no writes. `false` means it
    /// writes and `affects_tables` must be non-empty.
    pub read_only: bool,
    /// Tables this handler writes to. Each entry fires
    /// `invalidate_tags(["table:<name>"])` after a successful call.
    pub affects_tables: Vec<String>,
}

impl HandlerMeta {
    /// A read-only handler — no write-path invalidation.
    pub fn read_only() -> Self {
        Self {
            read_only: true,
            affects_tables: Vec::new(),
        }
    }

    /// A writing handler. `affects_tables` must be non-empty —
    /// constructing one without tables is a programming error
    /// caught at [`HandlerCatalog::register`] time.
    pub fn writing<I, S>(tables: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            read_only: false,
            affects_tables: tables.into_iter().map(Into::into).collect(),
        }
    }

    /// Convert to the v1 invalidation tag set.
    pub fn invalidation_tags(&self) -> Vec<String> {
        self.affects_tables
            .iter()
            .map(|t| format!("table:{t}"))
            .collect()
    }
}

/// Errors raised at [`HandlerCatalog::register`] time. The dispatcher
/// fails fast on these — silently registering a writing handler
/// without a tag declaration is the #1 way caches rot in production
/// per §"Read-only handler declaration" of the proposal.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HandlerRegistrationError {
    /// A writing handler did not declare any `affects_tables`.
    #[error(
        "handler {extension:?}::{contribute_id:?}: writing handler must declare \
         affects_tables; declare it read_only=true if it does no writes"
    )]
    WritingHandlerMissingTables {
        /// The extension id.
        extension: String,
        /// The contribute id.
        contribute_id: String,
    },
}

/// Registry of handler metadata, keyed by `(extension, contribute_id)`.
/// Built at host startup; cloned into [`DispatcherCache`].
#[derive(Debug, Default, Clone)]
pub struct HandlerCatalog {
    entries: Arc<HashMap<(ExtensionId, String), HandlerMeta>>,
}

impl HandlerCatalog {
    /// An empty catalog. Dispatchers wired without a populated
    /// catalog treat every handler as undeclared — no write-path
    /// invalidation fires.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build from a flat list of registrations. Equivalent to calling
    /// [`Self::register`] for each entry in order; the **first**
    /// invalid registration short-circuits with `Err`.
    pub fn from_entries<I>(entries: I) -> Result<Self, HandlerRegistrationError>
    where
        I: IntoIterator<Item = (ExtensionId, String, HandlerMeta)>,
    {
        let mut builder = HandlerCatalogBuilder::new();
        for (ext, cid, meta) in entries {
            builder.register(ext, cid, meta)?;
        }
        Ok(builder.build())
    }

    /// Look up the meta for one handler.
    pub fn get(&self, ext: &ExtensionId, contribute_id: &str) -> Option<&HandlerMeta> {
        self.entries.get(&(ext.clone(), contribute_id.to_string()))
    }

    /// Number of registered entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` if no entries are registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Builder helper that enforces the registration invariants —
/// "a writing handler that does not declare `affects_tables` is a
/// hard error at registration".
#[derive(Default)]
pub struct HandlerCatalogBuilder {
    entries: HashMap<(ExtensionId, String), HandlerMeta>,
}

impl HandlerCatalogBuilder {
    /// Build an empty catalog builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one `(extension, contribute_id) -> meta`. Returns
    /// `Err(HandlerRegistrationError::WritingHandlerMissingTables)`
    /// when `read_only=false` and `affects_tables` is empty.
    pub fn register(
        &mut self,
        extension: ExtensionId,
        contribute_id: impl Into<String>,
        meta: HandlerMeta,
    ) -> Result<&mut Self, HandlerRegistrationError> {
        let cid = contribute_id.into();
        if !meta.read_only && meta.affects_tables.is_empty() {
            return Err(HandlerRegistrationError::WritingHandlerMissingTables {
                extension: extension.as_str().to_string(),
                contribute_id: cid,
            });
        }
        self.entries.insert((extension, cid), meta);
        Ok(self)
    }

    /// Finalise the catalog.
    pub fn build(self) -> HandlerCatalog {
        HandlerCatalog {
            entries: Arc::new(self.entries),
        }
    }
}

/// Bundle the dispatcher receives via [`super::BuiltinRestDispatcher::with_cache`].
#[derive(Clone)]
pub struct DispatcherCache {
    /// Where the work lands.
    pub layer: CacheLayer,
    /// Per-kind specs.
    pub registry: KindCacheRegistry,
    /// Per-handler read-only / writing metadata. Empty by default;
    /// when present, the dispatcher fires
    /// `invalidate_tags(meta.invalidation_tags())` after every
    /// successful call to a writing handler.
    pub handlers: HandlerCatalog,
}

impl DispatcherCache {
    /// Convenience builder. Defaults `handlers` to the empty
    /// catalog — call [`Self::with_handlers`] to wire write-path
    /// invalidation.
    pub fn new(layer: CacheLayer, registry: KindCacheRegistry) -> Self {
        Self {
            layer,
            registry,
            handlers: HandlerCatalog::empty(),
        }
    }

    /// Attach a populated [`HandlerCatalog`].
    pub fn with_handlers(mut self, handlers: HandlerCatalog) -> Self {
        self.handlers = handlers;
        self
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
///
/// **Key-order canonicalisation** rides on serde_json's default
/// `Map` being BTreeMap-backed (alphabetical keys). Two semantically
/// identical JSON objects with different key orders therefore hash
/// identically. Enabling serde_json's `preserve_order` feature
/// **would** silently halve the cache hit-rate for any dispatcher
/// whose callers don't sort keys client-side; the
/// `base_key_canonicalises_object_key_order` test in this module
/// pins the assumption.
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
    fn base_key_canonicalises_object_key_order() {
        // Two semantically identical JSON objects with different key
        // orderings must produce the same base_key — otherwise the
        // cache wastes RAM on duplicates and the hit-rate dashboard
        // lies. This holds today because serde_json's default `Map`
        // is BTreeMap-backed; turning on `preserve_order` would break
        // this assumption (and halve hit-rates in production).
        let ext = ExtensionId::new("com.example.ext").unwrap();
        let a = serde_json::json!({"a": 1, "b": 2, "c": [3, 4]});
        let b = serde_json::json!({"c": [3, 4], "b": 2, "a": 1});
        let ka = dispatch_base_key(&ext, "tool", &a);
        let kb = dispatch_base_key(&ext, "tool", &b);
        assert_eq!(ka, kb, "key-order must not affect base_key");
    }

    #[test]
    fn base_key_distinguishes_distinct_inputs() {
        // Sanity floor: when the inputs really do differ, the keys
        // must differ. `Value::Null`, `{}`, and `false` all serialise
        // to different strings — verified — but pin the property so
        // we catch any future canonicaliser that flattens these.
        let ext = ExtensionId::new("com.example.ext").unwrap();
        let null = serde_json::Value::Null;
        let empty = serde_json::json!({});
        let f = serde_json::Value::Bool(false);
        let kn = dispatch_base_key(&ext, "t", &null);
        let ke = dispatch_base_key(&ext, "t", &empty);
        let kf = dispatch_base_key(&ext, "t", &f);
        assert_ne!(kn, ke);
        assert_ne!(ke, kf);
        assert_ne!(kn, kf);
    }

    #[test]
    fn orphans_flag_unknown_contribute_ids() {
        let ext = ExtensionId::new("com.example.foo").unwrap();
        let spec = CacheSpec::ttl(Duration::from_secs(30));
        let reg = KindCacheRegistry::from_entries([
            ((ext.clone(), "real_kind".to_string()), spec.clone()),
            ((ext.clone(), "typo_kind".to_string()), spec.clone()),
        ]);
        let known = ["real_kind"];
        let orphans = reg.orphans(|e| {
            assert_eq!(e, &ext);
            known.iter().copied()
        });
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].extension, ext);
        assert_eq!(orphans[0].contribute_id, "typo_kind");
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
