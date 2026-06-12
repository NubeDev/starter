//! Boot-time assembly of the extension runtime (WS-14 §4.1).
//!
//! The single entry point [`boot`] does, in order:
//! 1. **Reap orphans first** — `reap_stale_groups(pidfile_dir)` `killpg`s any
//!    process groups a prior crash left behind, *before* any new supervisor
//!    spawns (the supervisor memory: process groups + boot pidfile reaper).
//! 2. **Scan + validate + commit + seal** the manifest registry from the
//!    extensions dir(s).
//! 3. **Materialise contributed query-kinds + insights** — read each validated
//!    extension's `warehouse_templates[]`, lint them, upsert into
//!    `nexus_extension_query_kinds`, then build the in-memory `extension_kinds`
//!    registry (the dispatcher's third source) from the persisted rows; and read
//!    each extension's `insights[]`, compile-check them, upsert into
//!    `nexus_extension_insights` (resolved per-request by name, no in-memory
//!    registry needed).
//! 4. **Spawn supervisors** for enabled process-flavour extensions, installing
//!    nexus's host-method handler so a `warehouse.query`/`authz.check`/
//!    `dashboard.read` call routes back into nexus under the caller's tenant.
//! 5. **Build `ExtensionAdmin`** — PG enablement store, supervisor factory (with
//!    host methods), the query-kind cleanup provider + post-install hook, and
//!    the nexus audit sink.
//!
//! Returns the assembled [`ExtensionRuntime`]: the admin handle (mounted into
//! the router) and the extension-kinds registry (placed on `AppState`).
//!
//! The host-method handler closes over [`AppState`], but `AppState` itself holds
//! the `extension_kinds` registry this boot builds — a chicken/egg. `main`
//! resolves it by ordering: [`load_extension_kinds`] persists + builds the
//! registry first, `main` places it on `AppState`, then builds the host-method
//! handler over that `AppState` and passes it to [`boot`] (the `host_methods`
//! parameter). So [`boot`] never has to reach back into `AppState` itself.

use std::collections::HashMap;
use std::sync::Arc;

use sqlx::PgPool;
use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_server::{EnablementStore, ExtensionAdmin, WithHostMethodsFactory};
use starter_ext_spi::RuntimeKind;
use starter_ext_store_pg::PgEnablementStore;
use starter_ext_supervisor::{reap_stale_groups, SharedHostMethodHandler};

use super::audit::NexusExtensionAudit;
use super::cleanup::QueryKindCleanupProvider;
use super::cleanup_insights::InsightCleanupProvider;
use super::config::ExtensionsConfig;
use super::contribute::{contributed_query_kinds, record_to_query_kind};
use super::contribute_insights::contributed_insights;
use super::contribute_nodes::register_contributed_nodes;
use super::post_install::ContributionPostInstall;
use crate::kinds::Registry as KindRegistry;
use nexus_store::{extension_insight, extension_query_kind};
use starter_flow::registry::NodeKindRegistry;

/// The assembled extension runtime handed back to `main`.
pub struct ExtensionRuntime {
    /// The admin handle. Mounted into the router via
    /// [`super::router`] and used at shutdown to stop supervisors.
    pub admin: ExtensionAdmin,
    /// The extension-contributed query-kinds registry — the dispatcher's third
    /// source. Placed on `AppState.extension_kinds`.
    pub extension_kinds: Arc<KindRegistry>,
    /// The flow node-kind registry, populated with every enabled extension's
    /// `contributes.nodes[]` bridged to its supervised child via a
    /// `ProcessNodeProxy`. The Setup/Automation Builder's run engine shares this
    /// registry so a setup template's steps resolve to the child that owns them.
    pub flow_node_kinds: Arc<NodeKindRegistry>,
}

/// Assemble the extension runtime. `host_methods` is nexus's
/// [`HostMethodHandler`](starter_ext_supervisor::HostMethodHandler), already
/// closed over the finished `AppState` — built by the caller after the
/// `extension_kinds` registry this function persists is available. (`main`
/// builds the registry-less state first, calls [`load_extension_kinds`], places
/// the registry on `AppState`, then calls `boot` with a handler over that state.)
///
/// Boot never aborts on a *single* bad extension: a manifest that fails to parse
/// or a template that fails its lint is logged and skipped, and the rest of the
/// runtime comes up. A failure to reach the metadata DB (the kinds upsert) *does*
/// propagate, since that signals a broken deployment, not a bad bundle.
pub async fn boot(
    cfg: &ExtensionsConfig,
    metadata: PgPool,
    host_methods: SharedHostMethodHandler,
    extension_kinds: Arc<KindRegistry>,
    peer_supervisors: Arc<super::peer::PeerSupervisors>,
) -> Result<ExtensionRuntime, String> {
    // 1. Reap orphaned process groups from a prior crash before spawning.
    let reaped = reap_stale_groups(&cfg.pidfile_dir);
    if reaped.killed() > 0 {
        tracing::warn!(
            target: "nexus_api::extensions::boot",
            killed = reaped.killed(),
            total = reaped.total(),
            "reaped stale extension process groups from a prior run"
        );
    }

    // 2. Scan + validate + commit + seal the registry.
    let registry = scan_and_seal(cfg);
    let registry = Arc::new(registry);

    // 2b. WS-17 Wave A: create every validated extension's declared
    //     `warehouse_tables[]` as `<ext>__<name>` in the nexus Postgres before
    //     any supervisor (and thus any `warehouse.write`) can run. Idempotent;
    //     per-table failure is logged + skipped inside.
    super::warehouse::create_extension_tables(&metadata, &registry).await;

    // 3. Persistence: PG enablement store. Hydrate enabled state for the spawn
    //    decision below.
    let store = Arc::new(PgEnablementStore::new(metadata.clone()));

    // 4. Spawn supervisors for enabled process-flavour extensions, with nexus's
    //    host methods installed so capability calls route back into nexus.
    let factory = Arc::new(
        WithHostMethodsFactory::new(host_methods).with_pidfile_dir(cfg.pidfile_dir.clone()),
    );
    // The flow node-kind registry the setup engine shares. Each enabled
    // process extension's contributed nodes are bridged into it below.
    let flow_node_kinds = Arc::new(NodeKindRegistry::new());
    let mut supervisors = HashMap::new();
    for record in registry.iter_validated() {
        let Some(ext_id) = record.id.as_ref() else {
            continue;
        };
        let is_process = record
            .manifest
            .as_ref()
            .map(|m| m.runtime.kind == RuntimeKind::Process)
            .unwrap_or(false);
        if !is_process {
            continue;
        }
        // Default to enabled when no row exists yet (the kernel's convention).
        let enabled = match store.get(ext_id).await {
            Ok(Some(state)) => matches!(state, starter_ext_server::EnablementState::Enabled),
            Ok(None) => true,
            Err(e) => {
                tracing::warn!(
                    target: "nexus_api::extensions::boot",
                    extension = %ext_id.as_str(),
                    error = %e,
                    "reading enablement state failed; treating as disabled"
                );
                false
            }
        };
        if !enabled {
            continue;
        }
        match starter_ext_server::SupervisorFactory::spawn(&*factory, record).await {
            Ok(Some(handle)) => {
                // Bridge this extension's contributed flow node-kinds to its
                // freshly-spawned child (FLOW-NODES slice B). `SupervisorHandle`
                // is `Arc`-backed, so the proxies share the same child as the
                // admin's copy moved into `supervisors` below.
                if let Some(manifest) = record.manifest.as_ref() {
                    if !manifest.contributes.nodes.is_empty() {
                        let n =
                            register_contributed_nodes(manifest, &handle, &flow_node_kinds).await;
                        tracing::info!(
                            target: "nexus_api::extensions::boot",
                            extension = %ext_id.as_str(),
                            nodes = n,
                            "bridged contributed flow node-kinds to extension child"
                        );
                    }
                }
                supervisors.insert(ext_id.as_str().to_string(), handle);
            }
            Ok(None) => {} // builtin/wasm — nothing to spawn
            Err(e) => {
                tracing::warn!(
                    target: "nexus_api::extensions::boot",
                    extension = %ext_id.as_str(),
                    error = %e.0,
                    "spawning extension supervisor at boot failed"
                );
            }
        }
    }

    // 4b. WS-18: publish the spawned supervisor handles into the write-once
    //     peer registry `AppState` already shares, so the `extension.call` host
    //     method can reach a callee's child. `SupervisorHandle` is `Arc`-backed,
    //     so this clone shares the same children the admin moves below.
    if peer_supervisors.set(supervisors.clone()).is_err() {
        tracing::warn!(
            target: "nexus_api::extensions::boot",
            "peer supervisor registry already populated; ignoring (boot ran twice?)"
        );
    }

    // 5. Build the admin: PG store, host-method factory, the query-kind cleanup
    //    provider + post-install hook, and the nexus audit sink.
    let table_cleanup = Arc::new(super::cleanup::WarehouseTableCleanupProvider::new(
        metadata.clone(),
        registry.clone(),
    ));
    let admin = ExtensionAdmin::builder(registry)
        .with_enablement_store(store)
        .with_supervisor_factory(factory)
        .with_supervisors(supervisors)
        .with_installs_dir(cfg.installs_dir.clone())
        .with_cleanup_provider(Arc::new(QueryKindCleanupProvider::new(metadata.clone())))
        .with_cleanup_provider(Arc::new(InsightCleanupProvider::new(metadata.clone())))
        .with_cleanup_provider(table_cleanup)
        .with_post_install_hook(Arc::new(ContributionPostInstall::new(
            metadata.clone(),
            cfg.installs_dir.clone(),
        )))
        .with_audit_sink(Arc::new(NexusExtensionAudit::new(metadata)))
        .build();

    Ok(ExtensionRuntime {
        admin,
        extension_kinds,
        flow_node_kinds,
    })
}

/// Scan the extensions dir(s), validate every candidate, commit, and seal. A
/// per-candidate failure is isolated by the loader (it lands as a `Failed`
/// record), so a single bad bundle never takes the registry down.
///
/// Both roots are collected into **one** `Loader::commit` — the registry's
/// `install` *replaces* its contents (two-phase commit, R3), so committing per
/// root would wipe the first root's records with the second's. The installs
/// dir is scanned last, so an uploaded bundle with the same id overrides the
/// in-repo pack copy.
fn scan_and_seal(cfg: &ExtensionsConfig) -> ExtensionRegistry {
    let mut registry = ExtensionRegistry::new();
    let records = scan_roots(cfg);
    let outcome = Loader::commit(records, &mut registry);
    tracing::info!(
        target: "nexus_api::extensions::boot",
        pack = %cfg.extensions_dir.display(),
        installs = %cfg.installs_dir.display(),
        validated = outcome.validated,
        failed = outcome.failed,
        "scanned extension bundles"
    );
    registry.seal();
    registry
}

/// Collect validated records from the read-only in-repo pack dir and the
/// writable installs dir (the loader walks one level under each). A missing
/// dir yields no candidates — the "no extensions" state — rather than an
/// error.
fn scan_roots(cfg: &ExtensionsConfig) -> Vec<starter_ext_host::ExtensionRecord> {
    let mut records = Vec::new();
    for root in [&cfg.extensions_dir, &cfg.installs_dir] {
        if !root.exists() {
            continue;
        }
        records.extend(Loader::scan(root).validate_all());
    }
    records
}

/// Persist every validated extension's contributed query-kinds and build the
/// in-memory registry (the dispatcher's third source) from the persisted rows.
///
/// Called by `main` **before** [`boot`], because the registry it returns is
/// placed on `AppState`, which the host-method handler `boot` installs closes
/// over. Persisting first (rather than only building the in-memory registry)
/// means an extension contributing a kind survives a restart and the cleanup
/// provider can find it by owner. A single extension's bad template is logged
/// and skipped; a metadata-DB failure propagates (broken deployment).
pub async fn load_extension_kinds(
    cfg: &ExtensionsConfig,
    metadata: &PgPool,
) -> Result<LoadedExtensions, String> {
    // Re-scan to read manifests + bundle dirs. Cheap (filesystem walk of a small
    // dir); keeps this independent of `boot`'s sealed registry so ordering is
    // simple. One commit over both roots — `install` replaces, so a per-root
    // commit would wipe the pack's records with the (often empty) installs dir.
    let mut registry = ExtensionRegistry::new();
    let _ = Loader::commit(scan_roots(cfg), &mut registry);
    registry.seal();

    // Materialise each extension's contributed kinds into the provenance table.
    for record in registry.iter_validated() {
        let Some(ext_id) = record.id.as_ref() else {
            continue;
        };
        let Some(manifest) = record.manifest.as_ref() else {
            continue;
        };
        let kinds = match contributed_query_kinds(ext_id.as_str(), &record.bundle_dir, manifest) {
            Ok(k) => k,
            Err(e) => {
                tracing::warn!(
                    target: "nexus_api::extensions::boot",
                    extension = %ext_id.as_str(),
                    error = %e,
                    "skipping extension's query-kind contribution (bad template)"
                );
                continue;
            }
        };
        for new in &kinds {
            extension_query_kind::upsert(metadata, ext_id.as_str(), new)
                .await
                .map_err(|e| format!("persist contributed kind {}: {e}", new.name))?;
        }

        // Materialise contributed insights the same way: compile-check off disk,
        // upsert into the global registry. A single extension's bad script is
        // logged and skipped (parity with the bad-template path); a metadata-DB
        // failure propagates. Insights need no in-memory registry — the query
        // path resolves a contributed insight by name from the table per request,
        // exactly as a stored tenant insight resolves by id.
        match contributed_insights(ext_id.as_str(), &record.bundle_dir, manifest) {
            Ok(insights) => {
                for new in &insights {
                    extension_insight::upsert(metadata, ext_id.as_str(), new)
                        .await
                        .map_err(|e| format!("persist contributed insight {}: {e}", new.name))?;
                }
            }
            Err(e) => {
                tracing::warn!(
                    target: "nexus_api::extensions::boot",
                    extension = %ext_id.as_str(),
                    error = %e,
                    "skipping extension's insight contribution (bad script)"
                );
            }
        }
    }

    // Build the in-memory registry from *all* persisted contributed kinds (not
    // just this boot's manifests) so a kind contributed by a now-removed bundle
    // dir is still resolvable until its owner is purged. Listing by each known
    // extension would miss orphans; instead we read the whole table.
    let mut all = Vec::new();
    for record in registry.iter_validated() {
        if let Some(ext_id) = record.id.as_ref() {
            let rows = extension_query_kind::list_by_extension(metadata, ext_id.as_str())
                .await
                .map_err(|e| format!("listing contributed kinds: {e}"))?;
            all.extend(rows.into_iter().map(record_to_query_kind));
        }
    }

    let kinds = KindRegistry::from_kinds(all)
        .map_err(|e| format!("building extension-kinds registry: {e}"))?;
    Ok(LoadedExtensions {
        kinds: Arc::new(kinds),
        registry: Arc::new(registry),
    })
}

/// What [`load_extension_kinds`] hands back to `main`: the extension-contributed
/// query-kinds registry (the dispatcher's third source) **and** the sealed
/// extension registry. Both are placed on `AppState` — the kinds drive the
/// dispatcher, and the registry lets host methods consult the calling
/// extension's manifest at request time (WS-17 `warehouse.write` own-table
/// allowlist).
pub struct LoadedExtensions {
    /// Placed on `AppState.extension_kinds`.
    pub kinds: Arc<KindRegistry>,
    /// Placed on `AppState.extensions`. Shared with [`boot`] (which re-scans its
    /// own copy for the supervisor spawn loop); the two are equivalent snapshots
    /// of the same on-disk bundles.
    pub registry: Arc<ExtensionRegistry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: the registry's `install` REPLACES its contents, so scanning
    /// the pack dir and the (empty) installs dir must collapse into ONE commit
    /// — a per-root commit wipes the pack's records with the empty installs
    /// scan, leaving the deployment silently extension-less.
    #[test]
    fn empty_installs_dir_does_not_wipe_the_pack_scan() {
        let pack = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("extensions");
        let installs =
            std::env::temp_dir().join(format!("nexus-ext-boot-test-{}", std::process::id()));
        std::fs::create_dir_all(&installs).unwrap();

        let cfg = ExtensionsConfig {
            extensions_dir: pack,
            installs_dir: installs.clone(),
            pidfile_dir: installs.join("pids"),
        };
        let registry = scan_and_seal(&cfg);
        assert!(
            registry.get_by_id_str("com.nexus.hello").is_some(),
            "pack bundle must survive the (empty) installs-dir scan"
        );

        let _ = std::fs::remove_dir_all(&installs);
    }
}
