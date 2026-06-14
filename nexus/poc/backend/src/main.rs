//! Nexus ArkFlow POC backend — bootstrap only.
//!
//! Registers the ArkFlow component builders once, then serves the `/api` router.

mod dto;
mod engine;
mod routes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_target(false).init();

    engine::register_all()?;

    let app = routes::router();
    let addr = "127.0.0.1:8787";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("nexus-arkflow-poc listening on http://{addr}");

    axum::serve(listener, app).await?;
    Ok(())
}
