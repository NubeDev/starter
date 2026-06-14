//! Slack delivery: POST a Block-Kit message to an incoming webhook URL.
//!
//! Slack's incoming webhooks accept a JSON body and need no SDK — the same
//! transport the generic webhook channel uses, shaped to Slack's message format.
//! The configured `url` is the secret (bearer-equivalent); it is held in the
//! channel config and never returned to the client (the read path redacts it).
//! The message text is placed in a section block with a colour bar keyed off the
//! transition (green on resolve, red on open).

use serde::Deserialize;
use serde_json::{json, Value};

use super::Notification;

#[derive(Debug, Deserialize)]
struct SlackConfig {
    /// The incoming-webhook URL. Bearer-equivalent; treated as a secret.
    url: String,
}

/// POST the notification to the Slack incoming webhook. A non-2xx or transport
/// error is returned as a message for the event record and the retry layer.
pub async fn deliver(config: &Value, notification: &Notification) -> Result<(), String> {
    let cfg: SlackConfig =
        serde_json::from_value(config.clone()).map_err(|e| format!("invalid slack config: {e}"))?;
    let body = block_kit(notification);
    let resp = reqwest::Client::new()
        .post(&cfg.url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("slack post failed: {e}"))?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("slack returned {}", resp.status()))
    }
}

/// Shape the notification as a Slack Block-Kit payload. The attachment colour is
/// red while a finding is open, green on resolve, so the message reads at a glance.
fn block_kit(n: &Notification) -> Value {
    let colour = if n.transition == "resolved" {
        "#2eb886"
    } else {
        "#cc0000"
    };
    json!({
        "text": n.message,
        "attachments": [{
            "color": colour,
            "blocks": [{
                "type": "section",
                "text": { "type": "mrkdwn", "text": n.message },
            }],
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn note(transition: &str) -> Notification {
        Notification {
            detection_name: "High usage".into(),
            transition: transition.into(),
            target: json!({ "site": "s1" }),
            value: Some(95.0),
            context: json!({}),
            message: "Detection High usage opened".into(),
        }
    }

    #[test]
    fn open_is_red_resolved_is_green() {
        assert_eq!(
            block_kit(&note("opened"))["attachments"][0]["color"],
            "#cc0000"
        );
        assert_eq!(
            block_kit(&note("resolved"))["attachments"][0]["color"],
            "#2eb886"
        );
    }

    #[test]
    fn message_is_carried_into_text_and_block() {
        let body = block_kit(&note("opened"));
        assert_eq!(body["text"], "Detection High usage opened");
        assert_eq!(
            body["attachments"][0]["blocks"][0]["text"]["text"],
            "Detection High usage opened"
        );
    }

    #[test]
    fn missing_url_is_a_config_error() {
        let cfg: Result<SlackConfig, _> = serde_json::from_value(json!({}));
        assert!(cfg.is_err());
    }
}
