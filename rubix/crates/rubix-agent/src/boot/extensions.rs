//! Extension-host boot wiring (Phase C.1 of the rubix-extensions-wire job).
//!
//! Composes the seven upstream `starter-extensions` primitives that, taken
//! together, give rubix-agent a fully wired extension surface:
//!
//! 1. [`PgEnablementStore`] — the PostgreSQL-backed persistence impl of the
//!    [`EnablementStore`] trait, landed upstream per SCOPE R2.
//! 2. [`Loader::scan`] + [`Loader::validate_all`] + [`Loader::commit`] —
//!    the two-phase manifest loader that turns `cfg.extensions.dir`'s
//!    immediate child directories into validated [`ExtensionRecord`]s and
//!    drops them into a sealed [`ExtensionRegistry`].
//! 3. [`DefaultSupervisorFactory`] — spawns a process-flavour supervisor
//!    for each enabled record at boot, and is later re-used by the admin
//!    router's `enable` handler.
//! 4. [`ExtensionAdmin`] — the cheap-to-clone shared state every admin
//!    handler in [`starter_ext_server::router`] consumes.
//!
//! The verb is intentionally a single async fn: the orchestration is
//! linear (apply migration → scan dir → spawn enabled supervisors →
//! materialise admin handle), each step delegates to an upstream
//! primitive, and there is no per-step branching beyond the
//! `autostart_enabled_records` knob. Caller (main.rs) decides whether
//! to invoke us at all — when `cfg.extensions.enabled = false` we are
//! never called.
//!
//! See `docs/design/extensions/README.md` for the full bootflow and
//! `.codeless/jobs/rubix-extensions-wire/SCOPE.md` Phase C for the
//! end-to-end wiring contract.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use sqlx::PgPool;
use thiserror::Error;
use tracing::{info, warn};

use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_server::{
    DefaultSupervisorFactory, EnablementState, ExtensionAdmin, SupervisorFactory,
    WithHostMethodsFactory,
};
use starter_ext_spi::ExtensionId;
use starter_ext_store_pg::PgEnablementStore;
use starter_ext_supervisor::{SharedHostMethodHandler, SupervisorHandle};

use crate::extensions::RubixHostMethods;

/// Synthetic principal recorded as the actor for any audit / log entry
/// emitted by the boot-time autostart path. SCOPE OQ-5: operators must
/// be able to tell at a glance whether a row was produced by a real
/// operator action (their subject id, written via the `set_as` helper
/// when they hit `POST /extensions/<id>/{enable,disable}`) or by the
/// agent's own boot replay of persisted-enabled rows.
///
/// The string deliberately namespaces the source (`extensions-`) so
/// downstream tooling can distinguish it from other internal actors
/// (e.g. `system:scheduler`, `system:migration`).
pub const SYSTEM_AUTOSTART_PRINCIPAL: &str = "system:extensions-autostart";

use crate::boot::config::AgentConfig;

/// The migration SQL owned by `starter-ext-store-pg`. Inlined here via
/// `include_str!` so the boot path applies the same single source of
/// truth the upstream crate's tests exercise — no risk of schema drift
/// between rubix-agent and the upstream test suite.
const PG_ENABLEMENT_MIGRATION_SQL: &str = include_str!(
    "../../../../../starter-extensions/crates/starter-ext-store-pg/src/migrations/\
     0001_extensions_enablement.sql"
);

/// Errors surfaced from [`build_extension_admin`]. Distinct from
/// `anyhow::Error` so `main.rs` can match on the failure shape (e.g.
/// "migration failed" stays operator-actionable while "no extensions
/// dir on disk" is a soft warning that should not block boot).
#[derive(Debug, Error)]
pub enum BootError {
    /// `extensions_enablement` migration failed to apply against the
    /// configured Postgres pool. Treated as fatal — without the table
    /// the enable/disable lifecycle cannot persist.
    #[error("apply extensions_enablement migration: {0}")]
    Migration(#[source] sqlx::Error),

    /// Reading the persisted enablement rows failed. Fatal: we cannot
    /// honour `autostart_enabled_records` without knowing which ids
    /// were enabled on the last boot.
    #[error("read persisted enablement state: {0}")]
    ListEnablement(String),

    /// One of the autostart-on-boot supervisor spawns failed. The
    /// other extensions are still wired into the admin handle — the
    /// error surfaces the first failed id so the operator can act,
    /// but boot continues so the rest of the agent comes up.
    #[error("autostart supervisor for `{id}`: {source}")]
    AutostartSpawn {
        /// The extension id whose supervisor failed to spawn.
        id: String,
        /// Underlying factory error stringified for the operator.
        #[source]
        source: AutostartSpawnError,
    },
}

/// Inner error wrapper for [`BootError::AutostartSpawn`]. Kept as a
/// distinct type so the boxed-source chain stays well-typed without
/// pulling `SupervisorFactoryError` (which is `pub` upstream but does
/// not impl `Clone`) into the error variant body.
#[derive(Debug, Error)]
#[error("{0}")]
pub struct AutostartSpawnError(pub String);

/// Construct the rubix-agent's `ExtensionAdmin` handle from the
/// loaded `AgentConfig` and the shared Postgres pool.
///
/// Order of operations:
///
/// 1. Apply `0001_extensions_enablement.sql` against `pg_pool` (idempotent).
/// 2. Wrap the pool in a [`PgEnablementStore`].
/// 3. Walk `cfg.extensions.dir` via the two-phase [`Loader`] and seal
///    the resulting [`ExtensionRegistry`].
/// 4. When `cfg.extensions.autostart_enabled_records` is `true`, read
///    every persisted `Enabled` row from the store and spawn its
///    supervisor via [`DefaultSupervisorFactory`]. A spawn failure for
///    one id logs at warn but does not abort boot — the other ids
///    still come up, and an operator can investigate via the admin
///    route's `GET /extensions/<id>` failure surface.
/// 5. Materialise the [`ExtensionAdmin`] with the pre-populated
///    supervisor map and the PG store wired in.
///
/// The caller (main.rs Phase C.2) then merges
/// `starter_ext_server::router(admin.clone(), ..)` under
/// `/api/v1/extensions/*`. This function does not touch routing.
/// Bundle returned from [`build_extension_admin`]. The `admin` handle
/// is what `starter_ext_server::router` consumes; `registry` and
/// `process_handles` are surfaced so the MCP transport adapter
/// ([`starter_ext_mcp::register_process_tools`]) can wire each
/// process-flavour extension's `contributes.tools[]` into the rubix
/// `ToolRegistry` alongside the bundled `FlowAsTool` entries. Keeping
/// these as fields on the bundle (rather than fishing them back out of
/// `ExtensionAdmin`, whose supervisor accessor is crate-private)
/// avoids reaching into upstream private API.
pub struct ExtensionAdminBundle {
    /// Shared admin state for the REST router.
    pub admin: ExtensionAdmin,
    /// The sealed registry — shared with `admin` (same `Arc`).
    pub registry: Arc<ExtensionRegistry>,
    /// Live supervisor handles for autostarted process-flavour
    /// extensions, keyed by [`ExtensionId`] as
    /// [`starter_ext_mcp::register_process_tools`] expects. Each
    /// handle is wrapped in `Arc` because the MCP adapter clones it
    /// into every per-tool binding it registers.
    pub process_handles: HashMap<ExtensionId, Arc<SupervisorHandle>>,
}

pub async fn build_extension_admin(
    cfg: &AgentConfig,
    pg_pool: &PgPool,
    host_methods: Option<Arc<RubixHostMethods>>,
) -> Result<ExtensionAdminBundle, BootError> {
    // (1) Migration. Idempotent — the SQL uses CREATE TABLE IF NOT
    // EXISTS so a second boot is a no-op.
    sqlx::query(PG_ENABLEMENT_MIGRATION_SQL)
        .execute(pg_pool)
        .await
        .map_err(BootError::Migration)?;

    // (2) PG-backed store. Cheap to clone — wraps an `Arc<PgPool>`.
    let store = Arc::new(PgEnablementStore::new(pg_pool.clone()));

    // (3) Two-phase loader. `Loader::scan` silently treats a missing
    // extensions root as an empty load (per its own doc comment),
    // which keeps `cargo run -p rubix-agent` working out-of-the-box
    // on a fresh checkout that has not built any extensions yet.
    //
    // Canonicalise the configured root to an absolute path *before*
    // handing it to `Loader::scan`. The loader stamps each record's
    // `bundle_dir = scan_root.join(entry_name)`; the process-flavour
    // supervisor then `Command::current_dir(&bundle_dir).arg0(bundle_dir
    // .join(runtime.bin))`, which double-resolves a relative
    // `bundle_dir` against itself and produces a bogus exec path
    // (e.g. `rubix/extensions/<id>/rubix/extensions/<id>/<bin>` -> ENOENT).
    // Resolving to an absolute path here makes the join a no-op for the
    // cwd-change and keeps `runtime.bin` valid. Falls back to the
    // configured value verbatim if the dir doesn't yet exist (the
    // loader will just produce an empty record set in that case).
    let raw_dir: &Path = cfg.extensions.dir.as_ref();
    let owned_dir;
    let dir: &Path = match raw_dir.canonicalize() {
        Ok(abs) => {
            owned_dir = abs;
            owned_dir.as_path()
        }
        Err(_) => raw_dir,
    };
    info!(target: "rubix-agent::boot::extensions", dir = %dir.display(),
        "scanning extensions root");
    let records = Loader::scan(dir).validate_all();
    let validated_count = records
        .iter()
        .filter(|r| matches!(r.state, starter_ext_spi::LifecycleState::Validated))
        .count();
    let failed_count = records.len().saturating_sub(validated_count);
    if failed_count > 0 {
        warn!(target: "rubix-agent::boot::extensions",
            failed = failed_count, validated = validated_count,
            "some extensions failed manifest validation; \
             see GET /api/v1/extensions for per-id reasons");
    }
    let mut registry = ExtensionRegistry::new();
    let outcome = Loader::commit(records, &mut registry);
    registry.seal();
    info!(target: "rubix-agent::boot::extensions",
        validated = outcome.validated, failed = outcome.failed,
        "registry sealed");
    let registry = Arc::new(registry);

    // (4) Autostart. The supervisor factory is shared between the
    // initial spawn loop and the admin route's later `enable` handler
    // — same `Arc` is handed to the builder via
    // `with_supervisor_factory`.
    // Pick the supervisor factory: when the host attached a
    // `HostMethodHandler` (rubix-agent's `RubixHostMethods` for
    // Row-5 routing), every process-flavour spawn — autostart and
    // post-boot `enable` — inherits the same handler. Otherwise
    // the default factory leaves host calls advisory-only.
    //
    // Order matters: install the sealed `ExtensionRegistry` into
    // the handler *before* the factory wraps it, so by the time
    // autostart spawns supervisors and any child issues its first
    // host call the per-resource manifest gate has the registry
    // it needs.
    let factory: Arc<dyn SupervisorFactory> = match host_methods {
        Some(handler) => {
            handler.install_extension_registry(registry.clone());
            let shared: SharedHostMethodHandler = handler;
            Arc::new(WithHostMethodsFactory::new(shared))
        }
        None => Arc::new(DefaultSupervisorFactory),
    };
    let mut supervisors: HashMap<String, SupervisorHandle> = HashMap::new();
    // Parallel map keyed by `ExtensionId`, surfaced in the returned
    // bundle so adapters (notably `starter-ext-mcp`'s
    // `register_process_tools`) can clone the handle per tool binding.
    let mut process_handles: HashMap<ExtensionId, Arc<SupervisorHandle>> = HashMap::new();
    if cfg.extensions.autostart_enabled_records {
        let enabled = store
            .list_all()
            .await
            .map_err(|e| BootError::ListEnablement(e.to_string()))?;
        for (id, state) in enabled {
            if !matches!(state, EnablementState::Enabled) {
                continue;
            }
            let Some(record) = registry.get(&id) else {
                warn!(target: "rubix-agent::boot::extensions", id = %id.as_str(),
                    "persisted enabled row references unknown extension id; \
                     skipping autostart");
                continue;
            };
            match factory.spawn(record).await {
                Ok(Some(handle)) => {
                    supervisors.insert(id.as_str().to_string(), handle.clone());
                    process_handles.insert(id.clone(), Arc::new(handle));
                    info!(target: "rubix-agent::boot::extensions",
                        id = %id.as_str(),
                        actor = SYSTEM_AUTOSTART_PRINCIPAL,
                        "autostarted supervisor");
                }
                Ok(None) => {
                    // Builtin / WASM flavour — no supervisor to spawn.
                    info!(target: "rubix-agent::boot::extensions",
                        id = %id.as_str(),
                        actor = SYSTEM_AUTOSTART_PRINCIPAL,
                        "enabled record has no supervisor (builtin/wasm); skipping");
                }
                Err(e) => {
                    warn!(target: "rubix-agent::boot::extensions",
                        id = %id.as_str(),
                        actor = SYSTEM_AUTOSTART_PRINCIPAL,
                        error = %e,
                        "autostart spawn failed; continuing boot");
                }
            }
        }
    }

    // (5) Materialise the admin handle. `with_supervisors` pre-populates
    // the live-handle map so `GET /extensions/<id>` reports the
    // autostarted records as Running from the first request.
    let autostarted = supervisors.len();
    let admin = ExtensionAdmin::builder(registry.clone())
        .with_supervisors(supervisors)
        .with_enablement_store(store)
        .with_supervisor_factory(factory)
        .with_extensions_dir(dir.to_path_buf())
        .build();
    // Summary boot line consumed by operators + the integration test.
    // Distinct target from the per-step lines above so log filters can
    // pin a single regex on the boot summary.
    info!(
        target: "rubix.boot.extensions",
        loaded = outcome.validated,
        failed = outcome.failed,
        autostarted = autostarted,
        actor = SYSTEM_AUTOSTART_PRINCIPAL,
        "extensions wired"
    );
    Ok(ExtensionAdminBundle {
        admin,
        registry,
        process_handles,
    })
}
