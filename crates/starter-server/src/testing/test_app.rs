//! `TestApp` — spawn a starter server on a random local port and
//! return a base URL the consumer's tests can hit.

use std::net::SocketAddr;

use axum::Router;

/// A running starter server bound to a random localhost port.
///
/// Drop the `TestApp` to shut the server down; the bound port is
/// released immediately.
pub struct TestApp {
    /// `http://127.0.0.1:<port>` — the base URL tests send requests to.
    pub base_url: String,
    /// The bound socket. Kept so callers can inspect it if needed.
    pub addr: SocketAddr,
    shutdown: tokio::sync::oneshot::Sender<()>,
    join: tokio::task::JoinHandle<()>,
}

impl TestApp {
    /// Spawn `router` on a random local port and return a handle.
    ///
    /// Returns once the listener is bound; subsequent HTTP requests
    /// from the same task will hit the server.
    pub async fn spawn(router: Router) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown, rx) = tokio::sync::oneshot::channel();

        let join = tokio::spawn(async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = rx.await;
                })
                .await;
        });

        Self {
            base_url: format!("http://{addr}"),
            addr,
            shutdown,
            join,
        }
    }

    /// Stop the server and wait for the task to finish.
    pub async fn shutdown(self) {
        let _ = self.shutdown.send(());
        let _ = self.join.await;
    }
}
