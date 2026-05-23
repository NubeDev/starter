//! `rubix.system.flow_errors` — typed Rust client method.
//!
//! Posts a [`FlowErrorsRequest`] to
//! `POST /v1/tools/rubix.system.flow_errors` and parses a
//! [`FlowErrorsResponse`]. See
//! [docs/design/tools/](../../../../docs/design/tools/README.md)
//! for the routing convention.

use rubix_spi::dto::system::flow_errors::{FlowErrorsRequest, FlowErrorsResponse};
use reqwest::Client as Reqwest;

use crate::{client::RubixClient, error::RubixClientError};

impl RubixClient {
    /// `POST /v1/tools/rubix.system.flow_errors` — count flow
    /// execution errors observed by the rubix-agent.
    pub async fn system_flow_errors(
        &self,
        req: FlowErrorsRequest,
    ) -> Result<FlowErrorsResponse, RubixClientError> {
        let url = format!("{}/v1/tools/rubix.system.flow_errors", self.base_url);
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
