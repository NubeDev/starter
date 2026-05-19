//! `GET /openapi.json` — used by the codegen step and by the
//! `starter-cli openapi` subcommand.

use crate::{client::Client, error::ClientError};

impl Client {
    /// Fetch the server's OpenAPI document as raw JSON.
    pub async fn openapi(&self) -> Result<serde_json::Value, ClientError> {
        let url = format!("{}/openapi.json", self.base_url);
        let body = self.http.get(&url).send().await?.json().await?;
        Ok(body)
    }
}
