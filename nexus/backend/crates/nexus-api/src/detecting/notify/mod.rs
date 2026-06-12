//! Notification dispatch for detections: deliver a finding transition to a
//! detection's channels.
//!
//! This is the alert subsystem's delivery layer, re-homed onto detections. Where
//! the old evaluator notified on an ok→firing state transition, the detection
//! runner notifies on a *finding* transition — a finding opening or
//! auto-resolving (see [`crate::detecting::run`]). A channel's `kind` selects the
//! delivery impl (webhook|slack|email); delivery is retried with bounded backoff
//! ([`retry`]). A channel failure is recorded, never raised — a detection run must
//! survive a flaky downstream.

pub mod email;
pub mod retry;
pub mod slack;
pub mod template;
pub mod webhook;

use std::time::Duration;

use chrono::Utc;
use nexus_store::detection::DetectionRecord;
use nexus_store::finding::FindingTransition;
use nexus_store::notify::{self, NewNotifyEvent};
use serde_json::Value;
use sqlx::PgPool;

use retry::{DeliveryOutcome, MAX_RETRIES};
use template::{render, TemplateContext, DEFAULT_TEMPLATE};

/// The payload a finding transition delivers. `message` is the rendered,
/// human-readable text; the structured fields stay available for providers that
/// format their own (the webhook posts the whole struct as JSON).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Notification {
    /// The detection that produced the finding.
    pub detection_name: String,
    /// The finding transition: `opened` | `resolved`.
    pub transition: String,
    /// The finding's target (the identifying column values).
    pub target: Value,
    /// The finding's numeric value, if the detection carries one.
    pub value: Option<f64>,
    /// The rest of the flagged row — the evidence.
    pub context: Value,
    pub message: String,
}

/// Notify a detection's channels of the findings that opened and resolved this
/// run. Honours an active silence (records the event as `silenced`, sends
/// nothing) and records one [`notify::event`] per transition. Never raises: a
/// delivery failure is recorded on the event and swallowed so the run completes.
///
/// A detection with no `channel_ids` is a pure analytic detection — it returns
/// immediately, having only written findings.
pub async fn notify_transitions(
    pool: &PgPool,
    tenant: &str,
    det: &DetectionRecord,
    opened: &[FindingTransition],
    resolved: &[FindingTransition],
) {
    if det.channel_ids.is_empty() || (opened.is_empty() && resolved.is_empty()) {
        return;
    }

    // An active silence suppresses delivery but not the audit row.
    let silenced = notify::silence::is_silenced(pool, tenant, det.id, Utc::now())
        .await
        .unwrap_or(false);

    let channels = match notify::channel::by_ids(pool, tenant, &det.channel_ids).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(tenant, detection_id = %det.id, error = %e, "notify: channel load failed");
            return;
        }
    };

    for (transition, findings) in [("opened", opened), ("resolved", resolved)] {
        for f in findings {
            let message = render_message(det, transition, f);
            let notification = Notification {
                detection_name: det.name.clone(),
                transition: transition.to_string(),
                target: f.target.clone(),
                value: f.value,
                context: f.context.clone(),
                message,
            };

            let (notified, detail) = if silenced {
                (false, Some("silenced".to_string()))
            } else {
                deliver_all(&channels, &notification).await
            };

            let _ = notify::event::insert(
                pool,
                tenant,
                &NewNotifyEvent {
                    detection_id: det.id,
                    finding_id: Some(f.finding_id),
                    transition: transition.to_string(),
                    value: f.value,
                    silenced,
                    notified,
                    detail,
                },
            )
            .await;
        }
    }
}

/// Deliver one notification to every channel with retry, returning whether at
/// least one delivery succeeded and a summary of per-channel outcomes.
async fn deliver_all(
    channels: &[notify::ChannelRecord],
    notification: &Notification,
) -> (bool, Option<String>) {
    let mut any_ok = false;
    let mut details = Vec::new();
    for ch in channels {
        let outcome = deliver_with_retry(ch, notification).await;
        if outcome.succeeded() {
            any_ok = true;
            if outcome.attempts > 1 {
                details.push(format!(
                    "{}: ok after {} attempts",
                    ch.name, outcome.attempts
                ));
            }
        } else {
            details.push(format!(
                "{}: failed ×{} ({})",
                ch.name,
                outcome.attempts,
                outcome.last_error.unwrap_or_default()
            ));
        }
    }
    let detail = if details.is_empty() {
        None
    } else {
        Some(details.join("; "))
    };
    (any_ok, detail)
}

/// Render the message for a transition, using the detection's `message_template`
/// or the default.
fn render_message(det: &DetectionRecord, transition: &str, f: &FindingTransition) -> String {
    let template = det.message_template.as_deref().unwrap_or(DEFAULT_TEMPLATE);
    render(
        template,
        &TemplateContext {
            detection_name: &det.name,
            transition,
            target: &f.target,
            value: f.value,
        },
    )
}

/// Deliver `notification` over `channel`, retrying with backoff on failure.
pub async fn deliver_with_retry(
    channel: &notify::ChannelRecord,
    notification: &Notification,
) -> DeliveryOutcome {
    retry::with_backoff(
        MAX_RETRIES,
        || deliver(channel, notification),
        |d: Duration| async move { tokio::time::sleep(d).await },
    )
    .await
}

/// Deliver `notification` over `channel` once. The single-attempt seam the retry
/// layer wraps.
pub async fn deliver(
    channel: &notify::ChannelRecord,
    notification: &Notification,
) -> Result<(), String> {
    match channel.kind.as_str() {
        "webhook" => webhook::deliver(&channel.config, notification).await,
        "slack" => slack::deliver(&channel.config, notification).await,
        "email" => email::deliver(&channel.config, notification).await,
        other => Err(format!("unsupported channel kind `{other}`")),
    }
}

/// The config keys that hold a secret per channel kind. Read paths replace these
/// with a placeholder so a token is never returned to a client.
fn secret_keys(kind: &str) -> &'static [&'static str] {
    match kind {
        "slack" => &["url"],
        "email" => &["password"],
        _ => &[],
    }
}

/// Redact a channel's secret keys for the read path.
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
        assert_eq!(redact_config("slack", &cfg)["url"], "__redacted__");
    }

    #[test]
    fn email_password_redacted_but_host_kept() {
        let cfg = json!({ "host": "smtp.example.com", "password": "hunter2" });
        let red = redact_config("email", &cfg);
        assert_eq!(red["password"], "__redacted__");
        assert_eq!(red["host"], "smtp.example.com");
    }

    #[test]
    fn webhook_url_not_redacted() {
        let cfg = json!({ "url": "https://example.com/hook" });
        assert_eq!(
            redact_config("webhook", &cfg)["url"],
            "https://example.com/hook"
        );
    }
}
