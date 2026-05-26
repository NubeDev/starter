//! `rubix.system.disk` — typed Rust client method.
//!
//! Posts a [`DiskUsageRequest`] to `POST /v1/tools/rubix.system.disk`
//! and parses a [`DiskUsageResponse`]. The route name matches the
//! tool id one-to-one — see [docs/design/tools/](../../../../docs/design/tools/README.md)
//! for the routing convention.

use reqwest::Client as Reqwest;
use rubix_spi::dto::system::disk::{DiskUsageRequest, DiskUsageResponse};

use crate::{client::RubixClient, error::RubixClientError};

impl RubixClient {
    /// `POST /v1/tools/rubix.system.disk` — report disk usage on the
    /// rubix-agent host.
    pub async fn system_disk(
        &self,
        req: DiskUsageRequest,
    ) -> Result<DiskUsageResponse, RubixClientError> {
        let url = format!("{}/v1/tools/rubix.system.disk", self.base_url);
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
