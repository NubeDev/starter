//! `GET /health` client method.

use starter_spi::dto::Health;

use crate::{client::Client, error::ClientError};

impl Client {
    /// Hit `GET /health` and parse the response body.
    pub async fn health(&self) -> Result<Health, ClientError> {
        let url = format!("{}/health", self.base_url);
        let body = self.http.get(&url).send().await?.json().await?;
        Ok(body)
    }
}
