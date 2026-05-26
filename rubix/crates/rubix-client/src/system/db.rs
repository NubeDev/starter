//! `rubix.system.db` — typed Rust client method.
//!
//! Posts a [`DbHealthRequest`] to `POST /v1/tools/rubix.system.db`
//! and parses a [`DbHealthResponse`]. The route name matches the
//! tool id one-to-one — see [docs/design/tools/](../../../../docs/design/tools/README.md)
//! for the routing convention.

use reqwest::Client as Reqwest;
use rubix_spi::dto::system::db::{DbHealthRequest, DbHealthResponse};

use crate::{client::RubixClient, error::RubixClientError};

impl RubixClient {
    /// `POST /v1/tools/rubix.system.db` — report DB engine health
    /// for the rubix-agent's configured database.
    pub async fn system_db(
        &self,
        req: DbHealthRequest,
    ) -> Result<DbHealthResponse, RubixClientError> {
        let url = format!("{}/v1/tools/rubix.system.db", self.base_url);
        let resp = Reqwest::new().post(&url).json(&req).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(RubixClientError::Server {
                status: status.as_u16(),
            });
        }
        Ok(resp.json().await?)
    }
}
