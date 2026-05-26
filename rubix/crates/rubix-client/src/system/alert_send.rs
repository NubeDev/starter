//! `rubix.alert.send` — typed Rust client method.
//!
//! Posts an [`AlertSendRequest`] to
//! `POST /v1/tools/rubix.alert.send` and parses an
//! [`AlertSendResponse`]. See
//! [docs/design/tools/](../../../../docs/design/tools/README.md)
//! for the routing convention.

use reqwest::Client as Reqwest;
use rubix_spi::dto::system::alert_send::{AlertSendRequest, AlertSendResponse};

use crate::{client::RubixClient, error::RubixClientError};

impl RubixClient {
    /// `POST /v1/tools/rubix.alert.send` — emit a single
    /// operator alert through the rubix-agent's alert sink.
    pub async fn system_alert_send(
        &self,
        req: AlertSendRequest,
    ) -> Result<AlertSendResponse, RubixClientError> {
        let url = format!("{}/v1/tools/rubix.alert.send", self.base_url);
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
