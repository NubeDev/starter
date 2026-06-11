//! Nexus control-plane server entrypoint.
//!
//! Connects the metadata and datasource pools, runs migrations, mounts the
//! identity routers, and serves the product surface behind the principal layer.

use std::net::SocketAddr;
use std::time::Duration;

use nexus_api::middleware::StreamTokenSigner;
use nexus_api::state::AppState;
use nexus_api::{bootstrap, identity, serve};
use nexus_engine::{FlowManager, LiveRunner};
use nexus_store::datasource::Envelope;
use nexus_store::QueryGuards;
use starter_store_postgres::Pool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The zag agent tier drives `claude --print` non-interactively (using the
    // CLI's own login — no API key). zag disables print mode unless this env var
    // is set, to stop accidental API-token spend; for the control plane that
    // headless mode is exactly the intent, so opt in unless the operator has
    // already chosen a value. Set before any session can spawn the CLI.
    if std::env::var_os("ZAG_CLAUDE_ALLOW_PRINT").is_none() {
        // SAFETY: first statement in `main`, before pools/tasks start; no other
        // thread reads this var until a much-later session run.
        std::env::set_var("ZAG_CLAUDE_ALLOW_PRINT", "1");
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = Config::from_env()?;

    // The metadata pool owns the control plane's tenant-scoped tables. In
    // production it connects under the non-BYPASSRLS runtime role; migrations are
    // applied by an owner role out of band. For a single-DSN dev setup the same
    // pool runs both.
    let metadata = sqlx::PgPool::connect(&cfg.metadata_url).await?;
    let metadata_pool = Pool::from_sqlx(metadata.clone());
    bootstrap::migrate_all(&metadata_pool).await?;

    let identity = identity::build(metadata_pool).await?;

    let datasource = sqlx::PgPool::connect(&cfg.datasource_url).await?;

    // Load the built-in query-kinds pack at boot. A malformed pack (bad schema,
    // an undeclared `$param`, or a missing `$caller_tenant_id` predicate on a
    // tenant-scoped table) aborts startup rather than shipping an unsafe kind.
    let kinds = nexus_api::kinds::Registry::load_dir(&cfg.kinds_dir)?;
    tracing::info!(count = kinds.len(), dir = %cfg.kinds_dir.display(), "loaded query-kinds");

    // Load the built-in datasource-kinds pack at boot (WS-08b). A malformed
    // declaration (bad config schema, or a secret_field that names no config
    // property) aborts startup rather than shipping a connector that would leave
    // a credential unsealed.
    let datasource_kinds =
        nexus_api::datasource_kinds::Registry::load_dir(&cfg.datasource_kinds_dir)?;
    tracing::info!(
        count = datasource_kinds.len(),
        dir = %cfg.datasource_kinds_dir.display(),
        "loaded datasource-kinds"
    );

    let prefs = nexus_api::prefs::prefs_store(metadata.clone());
    let envelope = Envelope::new(cfg.master_key.as_bytes(), 1).map_err(|e| e.to_string())?;
    // WS-12 audit/undo: build the reversible registry + redo cursor once at boot.
    // Reversibles for secret-bearing kinds close over the same envelope so undo
    // can re-seal a rotated secret through the store.
    let changelog = nexus_api::changelog::ChangelogHandles::new(metadata.clone(), envelope.clone());

    // WS-14 extensions: resolve the runtime dirs and materialise every installed
    // extension's contributed query-kinds (the dispatcher's third source) into
    // the provenance table, building the in-memory registry placed on AppState.
    // This runs before AppState is built because the host-method handler the
    // supervisors install closes over the finished AppState (which holds this
    // registry). A bad bundle is logged + skipped inside; only a metadata-DB
    // failure aborts boot.
    let ext_cfg = nexus_api::extensions::ExtensionsConfig::from_env();
    if let Err(e) = ext_cfg.ensure_writable_dirs() {
        tracing::warn!(err = %e, "creating extension installs/pidfile dirs failed (install/reaper may degrade)");
    }
    let loaded_extensions =
        nexus_api::extensions::load_extension_kinds(&ext_cfg, &metadata).await?;
    let extension_kinds = loaded_extensions.kinds;
    let extension_registry = loaded_extensions.registry;
    tracing::info!(
        count = extension_kinds.len(),
        "loaded extension-contributed query-kinds"
    );

    // The runtime liveness canary (WS-16) ticks an atomic once per second. Spawn
    // it before AppState is built so the `/livez` route reads a fresh timestamp
    // from the very first request; the tick task is wrapped in the task watchdog
    // below alongside the schedulers.
    let (canary, canary_task) = nexus_api::boot::runtime_canary::spawn();

    let state = AppState {
        metadata: metadata.clone(),
        datasource,
        datasource_pools: Default::default(),
        envelope,
        guards: default_guards(),
        live: LiveRunner::new().map_err(|e| format!("engine init: {e}"))?,
        flows: FlowManager::new().map_err(|e| format!("flow manager init: {e}"))?,
        sessions: nexus_api::agents::SessionRunner::new(
            cfg.knowledge_root.clone(),
            nexus_skills::BrevityMode::Off,
        ),
        stream_signer: StreamTokenSigner::new(cfg.stream_key.into_bytes()),
        stream_token_ttl: Duration::from_secs(60),
        engine: identity.engine.clone() as std::sync::Arc<dyn starter_spi::authz::PolicyEngine>,
        kinds: std::sync::Arc::new(kinds),
        extension_kinds: extension_kinds.clone(),
        extensions: extension_registry,
        datasource_kinds: std::sync::Arc::new(datasource_kinds),
        prefs,
        changelog,
        query_cache: nexus_api::cache::CacheConfig::from_env().build(),
        quotas: nexus_api::quota::TenantQuotas::new(nexus_api::quota::QuotaConfig::from_env()),
        rate_limiter: nexus_api::ratelimit::TenantRateLimiter::new(
            nexus_api::ratelimit::RateLimitConfig::from_env(),
        ),
        canary,
    };

    // Now AppState exists: build nexus's host-method handler over it and boot the
    // extension runtime (reap orphans → seal registry → spawn enabled process
    // supervisors with host methods installed → assemble the admin handle).
    let host_methods = nexus_api::extensions::NexusHostMethods::shared(state.clone());
    let ext_runtime = nexus_api::extensions::boot(
        &ext_cfg,
        metadata.clone(),
        host_methods,
        extension_kinds,
    )
    .await?;
    let ext_admin = ext_runtime.admin;

    // Setup/Automation Builder: build the run service over the metadata pool and
    // the flow node-kind registry the extension boot just populated (each enabled
    // extension's `contributes.nodes[]` bridged to its child via a
    // `ProcessNodeProxy`), import every enabled extension's bundled setup
    // templates into the global catalog, and keep the service to mount `/setup/*`
    // under the principal layer. Validating templates against the same registry
    // means a template referencing an unprovided node-kind is rejected at boot.
    let setup_pool = Pool::from_sqlx(metadata.clone());
    let setup_service = nexus_api::setup::build_service(setup_pool, ext_runtime.flow_node_kinds);
    let imported = nexus_api::setup::import_extension_templates(
        ext_admin.registry(),
        &setup_service,
        setup_service.engine().kinds(),
    )
    .await;
    tracing::info!(
        target: "nexus_api::setup",
        templates = imported,
        "setup builder ready; /setup/* routes mounted"
    );

    // Every supposed-to-be-eternal background task is wrapped in the task
    // watchdog (WS-16): if one panics, returns early, or is aborted, a single
    // ERROR line is emitted (`target=nexus.task_watchdog watcher=<label>`)
    // instead of the death being inferred later from the absence of a log line.
    // The `let _x = ...` leak pattern is preserved — same lifetime, just
    // observable death.
    use nexus_api::boot::task_watchdog::watch;

    // The runtime canary tick task spawned above its AppState build.
    let _canary_watch = watch("runtime_canary", canary_task);

    // The detection scheduler runs for the process's lifetime, running due
    // detections on their own cadence — each producing findings and, for
    // alert-type detections, notifying their channels. Single-node for v1. This
    // subsumes the former standalone alert scheduler.
    let _detection_watch =
        watch("detection_scheduler", nexus_api::detecting::schedule::spawn(state.clone()));

    // The audit-retention sweep prunes ledger rows past the retention horizon so
    // the append-only log stays bounded. Runs for the process's lifetime.
    let _prune_watch = watch(
        "changelog_prune",
        nexus_api::changelog::prune::spawn(
            state.clone(),
            nexus_api::changelog::RetentionPolicy::from_env(),
        ),
    );

    // The extension admin router is a sibling of the authz/tenants routers: the
    // kernel applies its own `with_principal` → `with_role(Admin)` layer, so it
    // must NOT be wrapped in nexus's product principal layer (that would run the
    // layer twice). `serve::assemble` merges it after the identity routers.
    // The extension admin surface is cookie-authenticated admin mutation
    // (enable/disable/install/uninstall) — exactly what CSRF protects. The
    // kernel router bakes in its own `with_principal`; wrap the CSRF guard
    // outermost (it reads only raw cookie/header bytes and short-circuits a
    // forged cookie mutation with 403 before principal resolution).
    let ext_router = starter_server::auth::csrf_guard(
        nexus_api::extensions::router(ext_admin.clone(), identity.authenticator.clone()),
    );

    // The `/setup/*` surface reads the verified `Principal` (trusted identity
    // seeding + the per-template team check), so it is wrapped in its own
    // principal layer and merged as a sibling of the identity routers — never
    // inside the product router's layer. Its router state is the `RunService`
    // (set via `with_state`), so the resulting `Router<AppState>` is stateless
    // w.r.t. `AppState`. Nest under `/api/v1` to match the documented surface
    // (`/api/v1/setup/...`).
    let setup_router: axum::Router<nexus_api::state::AppState> = axum::Router::new().nest(
        "/api/v1",
        starter_server::auth::with_principal(
            // Same CSRF double-submit guard the product surface uses — cookie
            // sessions must echo `X-CSRF-Token`; bearer clients / safe methods
            // are exempt.
            starter_server::auth::csrf_guard(starter_setup::rest::router(setup_service)),
            identity.authenticator.clone(),
        ),
    );

    let router = serve::assemble(
        state,
        identity.auth,
        identity.authz,
        identity.tenants,
        ext_router,
        setup_router,
        identity.authenticator,
    );
    tracing::info!(bind = %cfg.bind, "nexus-api listening");

    // On shutdown, stop every supervised extension (SIGTERM → grace → SIGKILL)
    // so no child outlives nexus. `bind` returns when the graceful-shutdown
    // signal fires; we then drain the supervisors.
    let serve_result = starter_server::builder::bind(router, cfg.bind).await;
    tracing::info!("nexus-api shutting down; stopping extension supervisors");
    ext_admin.shutdown_all().await;
    serve_result?;
    Ok(())
}

/// Required configuration, read from the environment. The server refuses to
/// start if a secret-bearing value is missing or too weak rather than falling
/// back to an insecure default.
struct Config {
    metadata_url: String,
    datasource_url: String,
    master_key: String,
    stream_key: String,
    bind: SocketAddr,
    /// Root dir holding `skills/` and `rules/` markdown for agent prompt
    /// injection. Optional; defaults to `./knowledge`. A missing dir just means
    /// no knowledge is injected.
    knowledge_root: std::path::PathBuf,
    /// Directory holding the built-in query-kinds pack (`manifest.yaml` + the
    /// `*.sql`/`*_params.json` files). Optional; defaults to `./kinds`. A missing
    /// dir means no kinds are registered.
    kinds_dir: std::path::PathBuf,
    /// Directory holding the built-in datasource-kinds pack (`manifest.yaml` + the
    /// `*_config.json` schema files). Optional; defaults to `./datasource-kinds`.
    /// A missing dir means no connector type is declared.
    datasource_kinds_dir: std::path::PathBuf,
}

impl Config {
    fn from_env() -> Result<Self, String> {
        let master_key = req("NEXUS_MASTER_KEY")?;
        if master_key.len() != 32 {
            return Err("NEXUS_MASTER_KEY must be exactly 32 bytes".into());
        }
        let stream_key = req("NEXUS_STREAM_TOKEN_KEY")?;
        if stream_key.len() < 32 {
            return Err("NEXUS_STREAM_TOKEN_KEY must be at least 32 bytes".into());
        }
        Ok(Self {
            metadata_url: req("NEXUS_METADATA_URL")?,
            datasource_url: req("NEXUS_DATASOURCE_URL")?,
            master_key,
            stream_key,
            bind: std::env::var("NEXUS_BIND")
                .unwrap_or_else(|_| "127.0.0.1:4780".into())
                .parse()
                .map_err(|e| format!("NEXUS_BIND: {e}"))?,
            knowledge_root: std::env::var("NEXUS_KNOWLEDGE_ROOT")
                .unwrap_or_else(|_| "./knowledge".into())
                .into(),
            kinds_dir: std::env::var("NEXUS_KINDS_DIR")
                .unwrap_or_else(|_| "./kinds".into())
                .into(),
            datasource_kinds_dir: std::env::var("NEXUS_DATASOURCE_KINDS_DIR")
                .unwrap_or_else(|_| "./datasource-kinds".into())
                .into(),
        })
    }
}

fn req(key: &str) -> Result<String, String> {
    std::env::var(key).map_err(|_| format!("{key} must be set"))
}

/// The server-enforced query bounds. Conservative defaults; per-datasource
/// overrides arrive with datasource policy.
fn default_guards() -> QueryGuards {
    QueryGuards {
        statement_timeout: Duration::from_secs(30),
        max_rows: 10_000,
        max_bytes: 16 * 1024 * 1024,
    }
}
