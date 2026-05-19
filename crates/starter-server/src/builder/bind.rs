//! Bind the built `axum::Router` to a TCP listener and serve until
//! Ctrl-C / SIGTERM.

use std::net::SocketAddr;

use axum::Router;

/// Run the given router on `addr` until a shutdown signal arrives.
///
/// Graceful shutdown listens for `tokio::signal::ctrl_c()` and (on
/// Unix) `SIGTERM`. Returns once the server has finished draining
/// in-flight requests.
pub async fn bind(router: Router, addr: SocketAddr) -> Result<(), std::io::Error> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
