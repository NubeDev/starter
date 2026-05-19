//! Bind a loopback REST server on an ephemeral port and report the
//! bound address back synchronously, so the UI can render the URL
//! external tools should hit.
//!
//! This is a thin helper for the pattern: spawn a server bound to
//! `127.0.0.1:0`, recover the OS-assigned port via a oneshot, surface
//! `http://127.0.0.1:<port>` into the frontend's settings/info payload
//! before the first command runs.

use std::net::SocketAddr;

use tokio::sync::oneshot;

/// Spawn `serve` and wait for it to report a bound address.
///
/// `serve` is given the desired bind addr (`127.0.0.1:0`) and an
/// `on_bound` callback it must invoke once its listener is ready. The
/// helper drives the server future on a detached task that lives for
/// the lifetime of the process — it never gets joined.
///
/// Returns the actual bound `SocketAddr` (with the OS-assigned port) so
/// the caller can stash `http://<addr>` into the UI's server info.
pub async fn bind_ephemeral<S, F>(serve: S) -> Result<SocketAddr, BindError>
where
    S: FnOnce(SocketAddr, Box<dyn FnOnce(SocketAddr) + Send>) -> F + Send + 'static,
    F: std::future::Future<Output = ()> + Send + 'static,
{
    let (tx, rx) = oneshot::channel::<SocketAddr>();
    let bind = SocketAddr::from(([127, 0, 0, 1], 0));

    tokio::spawn(async move {
        let mut tx = Some(tx);
        let on_bound: Box<dyn FnOnce(SocketAddr) + Send> = Box::new(move |addr| {
            if let Some(tx) = tx.take() {
                let _ = tx.send(addr);
            }
        });
        serve(bind, on_bound).await;
    });

    rx.await.map_err(|_| BindError::ListenerNeverBound)
}

#[derive(Debug, thiserror::Error)]
pub enum BindError {
    #[error("REST listener never reported a bound address")]
    ListenerNeverBound,
}
