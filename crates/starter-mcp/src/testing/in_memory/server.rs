//! Server half of the in-memory transport pair.
//!
//! Owns the dispatch task: pulls JSON-RPC frames off the inbound mpsc,
//! routes them through [`crate::server::dispatch`], pushes responses
//! back on the outbound mpsc. Session locale capture + optional
//! principal binding mirror the stdio loop and HTTP handler
//! respectively, so the surface tested through this transport matches
//! what consumers hit on the wire (see
//! `docs/design/starter-changes/README.md`, Phase 2b U2).

use std::sync::Arc;

use serde_json::Value;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use starter_spi::auth::Principal;
use starter_spi::i18n::LanguageTag;

use crate::registry::ToolRegistry;
use crate::server::dispatch;

use super::client::InMemoryClient;
use super::Frame;

/// Handle to the spawned dispatch task. The task runs until the paired
/// [`InMemoryClient`] is dropped (which closes its sender, ending the
/// server's receive loop).
pub struct InMemoryServer {
    join: JoinHandle<()>,
}

impl InMemoryServer {
    /// Spawn a dispatch task wired to a fresh client. The optional
    /// `principal` is bound for the lifetime of the task via
    /// [`crate::with_principal`]; the session locale is captured per
    /// session from the `initialize` frame's `params._meta.acceptLanguage`
    /// (see `crate::server::stdio_loop`).
    pub(super) fn spawn(
        registry: Arc<ToolRegistry>,
        principal: Option<Principal>,
    ) -> (InMemoryClient, Self) {
        // Bounded channels keep tests honest about backpressure — a hung
        // server task surfaces as `send().await` blocking instead of an
        // unbounded buffer hiding the bug.
        let (client_to_server_tx, client_to_server_rx) = mpsc::channel::<Frame>(16);
        let (server_to_client_tx, server_to_client_rx) = mpsc::channel::<Frame>(16);

        let join = tokio::spawn(run(
            registry,
            principal,
            client_to_server_rx,
            server_to_client_tx,
        ));

        let client = InMemoryClient::new(client_to_server_tx, server_to_client_rx);
        (client, Self { join })
    }

    /// Wait for the dispatch task to exit. Tests that want a clean
    /// shutdown can drop the client and await this.
    pub async fn join(self) -> Result<(), tokio::task::JoinError> {
        self.join.await
    }
}

async fn run(
    registry: Arc<ToolRegistry>,
    principal: Option<Principal>,
    mut inbound: mpsc::Receiver<Frame>,
    outbound: mpsc::Sender<Frame>,
) {
    match principal {
        Some(p) => crate::with_principal(p, dispatch_loop(registry, &mut inbound, &outbound)).await,
        None => dispatch_loop(registry, &mut inbound, &outbound).await,
    }
}

async fn dispatch_loop(
    registry: Arc<ToolRegistry>,
    inbound: &mut mpsc::Receiver<Frame>,
    outbound: &mpsc::Sender<Frame>,
) {
    let mut session_locale: Option<LanguageTag> = None;

    while let Some(frame) = inbound.recv().await {
        if let Some(tag) = locale_from_initialize_frame(&frame) {
            session_locale = Some(tag);
        }

        let response = match session_locale.clone() {
            Some(tag) => crate::with_locale(tag, dispatch(&registry, &frame)).await,
            None => dispatch(&registry, &frame).await,
        };

        let Some(resp) = response else { continue };
        let encoded = serde_json::to_string(&resp).unwrap_or_else(|_| "null".into());
        if outbound.send(encoded).await.is_err() {
            // Client dropped its receiver — no one is listening. Stop.
            break;
        }
    }
}

/// Pick the BCP-47 tag a client offered through MCP's `_meta`
/// convention on `initialize`. Identical contract to
/// `crate::server::stdio_loop::locale_from_initialize_frame`; kept
/// private here so the in-memory transport advertises no separate
/// convention.
fn locale_from_initialize_frame(raw: &str) -> Option<LanguageTag> {
    let value: Value = serde_json::from_str(raw).ok()?;
    if value.get("method")?.as_str()? != "initialize" {
        return None;
    }
    let header = value
        .get("params")?
        .get("_meta")?
        .get("acceptLanguage")?
        .as_str()?;
    crate::locale_local::locale_from_accept_language(header)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_meta_accept_language_on_initialize() {
        let frame = r#"{"jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"_meta":{"acceptLanguage":"es-AR"}}}"#;
        assert_eq!(
            locale_from_initialize_frame(frame).map(|t| t.as_str().to_string()),
            Some("es-AR".into())
        );
    }

    #[test]
    fn ignores_non_initialize_frames() {
        let frame = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list",
            "params":{"_meta":{"acceptLanguage":"es-AR"}}}"#;
        assert!(locale_from_initialize_frame(frame).is_none());
    }
}
