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

    let prefs = nexus_api::prefs::prefs_store(metadata.clone());
    let state = AppState {
        metadata,
        datasource,
        datasource_pools: Default::default(),
        envelope: Envelope::new(cfg.master_key.as_bytes(), 1).map_err(|e| e.to_string())?,
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
        prefs,
    };

    // The alert scheduler runs for the process's lifetime, evaluating due rules
    // on its own cadence. Single-node for v1.
    nexus_api::alerting::schedule::spawn(state.clone());

    let router = serve::assemble(
        state,
        identity.auth,
        identity.authz,
        identity.tenants,
        identity.authenticator,
    );
    tracing::info!(bind = %cfg.bind, "nexus-api listening");
    starter_server::builder::bind(router, cfg.bind).await?;
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
