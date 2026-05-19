//! Fluent builder for [`super::Client`].

use super::handle::Client;

/// Configures and builds a [`Client`].
pub struct ClientBuilder {
    base_url: String,
    bearer: Option<String>,
    cookie: Option<String>,
}

impl ClientBuilder {
    /// Start with the server's base URL (e.g. `http://localhost:8080`).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            bearer: None,
            cookie: None,
        }
    }

    /// Attach an `Authorization: Bearer …` header to every request.
    pub fn with_bearer(mut self, token: impl Into<String>) -> Self {
        self.bearer = Some(token.into());
        self
    }

    /// Attach a `starter_session=…` cookie to every request.
    pub fn with_session_cookie(mut self, cookie: impl Into<String>) -> Self {
        self.cookie = Some(cookie.into());
        self
    }

    /// Build the final client.
    pub fn build(self) -> Result<Client, reqwest::Error> {
        Client::new(self.base_url, self.bearer, self.cookie)
    }
}
