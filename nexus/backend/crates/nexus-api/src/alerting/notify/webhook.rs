//! Webhook delivery: POST the notification as JSON to a configured URL.
//!
//! The universal integration — Slack, PagerDuty, and email gateways all accept
//! an inbound webhook — so it is the one channel kind v1 ships. The only config
//! is the URL; an authenticated webhook (a signing secret / auth header) would
//! store that secret under the R6 envelope, like datasource secrets, when such a
//! channel lands.

use serde::Deserialize;
use serde_json::Value;

use super::Notification;

#[derive(Debug, Deserialize)]
struct WebhookConfig {
    url: String,
}

/// POST the notification to the channel's URL. A non-2xx response or a transport
/// error is returned as a message for the event record.
pub async fn deliver(config: &Value, notification: &Notification) -> Result<(), String> {
    let cfg: WebhookConfig = serde_json::from_value(config.clone())
        .map_err(|e| format!("invalid webhook config: {e}"))?;
    let resp = reqwest::Client::new()
        .post(&cfg.url)
        .json(notification)
        .send()
        .await
        .map_err(|e| format!("webhook post failed: {e}"))?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("webhook returned {}", resp.status()))
    }
}
