//! `RubixClient` — the long-lived handle that rubix-specific endpoint
//! methods hang off via `impl RubixClient`. Wraps the underlying
//! `starter_client_rs::Client` for the auth + transport plumbing.

use starter_client_rs::Client as InnerClient;

/// HTTP client handle for the rubix endpoint surface. Cheap to clone.
#[derive(Clone)]
pub struct RubixClient {
    pub(crate) inner: InnerClient,
    pub(crate) base_url: String,
}

impl RubixClient {
    /// Construct from a fully-built starter client and the rubix
    /// base URL (e.g. `"http://127.0.0.1:8088"`).
    pub fn new(inner: InnerClient, base_url: impl Into<String>) -> Self {
        Self {
            inner,
            base_url: base_url.into(),
        }
    }

    /// Access the underlying starter client (auth, openapi, prefs, …).
    pub fn starter(&self) -> &InnerClient {
        &self.inner
    }
}
