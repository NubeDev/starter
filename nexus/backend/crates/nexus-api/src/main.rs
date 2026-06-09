//! Nexus control-plane server entrypoint.
//!
//! Connects the datasource pool, assembles the router, and serves. Identity,
//! the metadata store, and the engine handles join `AppState` as their
//! milestones land; M0 serves the one-shot query path.

use std::net::SocketAddr;
use std::time::Duration;

use nexus_api::middleware::StreamTokenSigner;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::LiveRunner;
use nexus_store::QueryGuards;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let datasource_url = std::env::var("NEXUS_DATASOURCE_URL")
        .map_err(|_| "NEXUS_DATASOURCE_URL must be set (the datasource Postgres DSN)")?;
    let bind: SocketAddr = std::env::var("NEXUS_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8080".into())
        .parse()?;

    // The stream-token signing key. Required: a forged or absent key would let
    // anyone open SSE subscriptions, so the server refuses to start without it.
    let stream_key = std::env::var("NEXUS_STREAM_TOKEN_KEY")
        .map_err(|_| "NEXUS_STREAM_TOKEN_KEY must be set (the SSE token signing key)")?;
    if stream_key.len() < 32 {
        return Err("NEXUS_STREAM_TOKEN_KEY must be at least 32 bytes".into());
    }

    let datasource = sqlx::PgPool::connect(&datasource_url).await?;
    let state = AppState {
        datasource,
        guards: default_guards(),
        live: LiveRunner::new().map_err(|e| format!("engine init: {e}"))?,
        stream_signer: StreamTokenSigner::new(stream_key.into_bytes()),
        stream_token_ttl: Duration::from_secs(60),
    };

    tracing::info!(%bind, "nexus-api listening");
    starter_server::builder::bind(serve::router(state), bind).await?;
    Ok(())
}

/// The server-enforced query bounds. Conservative defaults; per-datasource
/// overrides arrive with datasource CRUD.
fn default_guards() -> QueryGuards {
    QueryGuards {
        statement_timeout: Duration::from_secs(30),
        max_rows: 10_000,
        max_bytes: 16 * 1024 * 1024,
    }
}
