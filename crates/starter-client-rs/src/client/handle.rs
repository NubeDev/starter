//! `Client` — the long-lived handle each endpoint module hangs
//! methods off via `impl Client`.

use reqwest::Client as Reqwest;

/// HTTP client handle. Cheap to clone (reqwest's `Client` is
/// internally arc'd).
#[derive(Clone)]
#[allow(dead_code)]
pub struct Client {
    pub(crate) http: Reqwest,
    pub(crate) base_url: String,
    pub(crate) bearer: Option<String>,
    pub(crate) cookie: Option<String>,
}

impl Client {
    /// Construct directly; prefer the [`super::ClientBuilder`] for
    /// most cases.
    pub fn new(
        base_url: String,
        bearer: Option<String>,
        cookie: Option<String>,
    ) -> Result<Self, reqwest::Error> {
        Ok(Self {
            http: Reqwest::builder().build()?,
            base_url,
            bearer,
            cookie,
        })
    }
}
