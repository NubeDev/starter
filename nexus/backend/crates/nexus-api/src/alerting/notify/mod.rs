//! Notification dispatch: deliver a transition to a rule's channels.
//!
//! A channel's `kind` selects the delivery impl; v1 shipped `webhook`, and this
//! adds `slack` and `email`. Adding a kind is a new arm here plus its impl, with
//! no change to the evaluator. Delivery is retried with bounded backoff
//! ([`retry`]); the attempt count and last error flow back so the evaluator
//! records them on the event. A channel failure is reported, never raised —
//! alerting must survive a flaky downstream.

pub mod email;
pub mod retry;
pub mod slack;
pub mod webhook;

use std::time::Duration;

use nexus_store::alert::ChannelRecord;
use serde_json::Value;

use retry::{DeliveryOutcome, MAX_RETRIES};

/// The payload a transition delivers. `message` is the rendered, human-readable
/// text (from the rule/channel template); the raw fields stay available for
/// providers that format their own (the webhook posts the whole struct as JSON).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Notification {
    pub rule_name: String,
    pub transition: String,
    pub value: Option<f64>,
    pub threshold: f64,
    pub op: String,
    pub message: String,
}

/// Deliver `notification` over `channel`, retrying with backoff on failure.
/// Returns the attempt count and last error so the caller records delivery
/// outcome on the event. Never raises — a downstream failure is data, not a panic.
pub async fn deliver_with_retry(
    channel: &ChannelRecord,
    notification: &Notification,
) -> DeliveryOutcome {
    retry::with_backoff(
        MAX_RETRIES,
        || deliver(channel, notification),
        |d: Duration| async move { tokio::time::sleep(d).await },
    )
    .await
}

/// Deliver `notification` over `channel` once. Returns `Ok(())` on success or a
/// message describing the failure. The single-attempt seam the retry layer wraps.
pub async fn deliver(channel: &ChannelRecord, notification: &Notification) -> Result<(), String> {
    match channel.kind.as_str() {
        "webhook" => webhook::deliver(&channel.config, notification).await,
        "slack" => slack::deliver(&channel.config, notification).await,
        "email" => email::deliver(&channel.config, notification).await,
        other => Err(format!("unsupported channel kind `{other}`")),
    }
}

/// The config keys that hold a secret per channel kind. Read paths replace these
/// with a placeholder so a token is never returned to a client — the same posture
/// as datasource secrets, kept here because channel config is free-form jsonb.
fn secret_keys(kind: &str) -> &'static [&'static str] {
    match kind {
        // A Slack incoming-webhook URL is bearer-equivalent.
        "slack" => &["url"],
        // An SMTP password is the only credential; host/from/to are not secret.
        "email" => &["password"],
        // A bare webhook URL is not treated as secret in v1 (it has no auth).
        _ => &[],
    }
}

/// Redact a channel's secret keys for the read path: each secret key present in
/// the config is replaced with a fixed placeholder so the client can show that a
/// secret is set without ever receiving its value. A round-trip that re-submits
/// the placeholder is rejected upstream (the create/update path treats it as "no
/// change"); v1 simply never echoes the value.
pub fn redact_config(kind: &str, config: &Value) -> Value {
    let mut out = config.clone();
    if let Some(obj) = out.as_object_mut() {
        for key in secret_keys(kind) {
            if obj.contains_key(*key) {
                obj.insert((*key).to_string(), Value::String("__redacted__".into()));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn slack_url_is_redacted_on_read() {
        let cfg = json!({ "url": "https://hooks.slack.com/secret" });
        let red = redact_config("slack", &cfg);
        assert_eq!(red["url"], "__redacted__");
    }

    #[test]
    fn email_password_redacted_but_host_kept() {
        let cfg = json!({ "host": "smtp.example.com", "password": "hunter2" });
        let red = redact_config("email", &cfg);
        assert_eq!(red["password"], "__redacted__");
        assert_eq!(red["host"], "smtp.example.com");
    }

    #[test]
    fn webhook_url_not_redacted_in_v1() {
        let cfg = json!({ "url": "https://example.com/hook" });
        let red = redact_config("webhook", &cfg);
        assert_eq!(red["url"], "https://example.com/hook");
    }
}
