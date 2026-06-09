//! Notification dispatch: deliver a transition to a rule's channels.
//!
//! A channel's `kind` selects the delivery impl; v1 ships `webhook`. Adding a
//! kind is a new arm here plus its impl, with no change to the evaluator. A
//! channel failure is reported back (the evaluator records it on the event) but
//! never propagates as a hard error — alerting must survive a flaky downstream.

pub mod webhook;

use nexus_store::alert::ChannelRecord;

/// The payload a transition delivers. Kept provider-neutral (a webhook posts it
/// as JSON; a future Slack/email impl formats from the same fields).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Notification {
    pub rule_name: String,
    pub transition: String,
    pub value: Option<f64>,
    pub threshold: f64,
    pub op: String,
}

/// Deliver `notification` over `channel`. Returns `Ok(())` on success or a
/// message describing the failure, which the caller records — it does not raise.
pub async fn deliver(channel: &ChannelRecord, notification: &Notification) -> Result<(), String> {
    match channel.kind.as_str() {
        "webhook" => webhook::deliver(&channel.config, notification).await,
        other => Err(format!("unsupported channel kind `{other}`")),
    }
}
