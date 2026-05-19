//! In-process gRPC test harness.
//!
//! Spins up the `Tools` service on a real loopback `TcpListener` and
//! returns the bound `SocketAddr` plus a `Drop`-guarded shutdown
//! handle. Integration tests connect with the codegen'd client
//! exactly as a remote consumer would.
//!
//! A loopback listener (rather than tonic's in-memory `Channel`)
//! keeps the harness reusable from any tonic-client crate without
//! pulling extra deps, and exercises the full HTTP/2 path.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::TcpListenerStream;

use crate::auth::GrpcAuth;
use crate::registry::ToolRegistry;
use crate::service::tools_server;

/// A live gRPC server bound to a random loopback port. Drop the
/// handle to signal graceful shutdown; the join handle awaits the
/// background task.
pub struct TestServer {
    /// The bound `127.0.0.1:<port>` address. Clients dial this with
    /// `Endpoint::from_shared(format!("http://{addr}"))`.
    pub addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    join: Option<JoinHandle<()>>,
}

impl TestServer {
    /// Bind a server with the given registry + auth policy.
    pub async fn start(registry: Arc<ToolRegistry>, auth: GrpcAuth) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback for TestServer");
        let addr = listener.local_addr().expect("local_addr");
        let (tx, rx) = oneshot::channel::<()>();
        let stream = TcpListenerStream::new(listener);

        let join = tokio::spawn(async move {
            let _ = tonic::transport::Server::builder()
                .add_service(tools_server(registry, auth))
                .serve_with_incoming_shutdown(stream, async move {
                    let _ = rx.await;
                })
                .await;
        });

        Self {
            addr,
            shutdown_tx: Some(tx),
            join: Some(join),
        }
    }

    /// `http://127.0.0.1:<port>` — pass to `tonic::transport::Endpoint::from_shared`.
    pub fn endpoint(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Trigger graceful shutdown + await the background task. Called
    /// automatically on drop, but exposed for tests that want to
    /// assert the server stopped cleanly.
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(join) = self.join.take() {
            let _ = join.await;
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        // Detach the join handle if the test didn't await it — the
        // server will tear down on the shutdown signal.
    }
}
