//! Slack delivery: POST a Block-Kit message to an incoming webhook URL.
//!
//! Slack's incoming webhooks accept a JSON body and need no SDK — the same
//! transport the generic webhook channel uses, shaped to Slack's message format.
//! The configured `url` is the secret (a Slack webhook URL is bearer-equivalent);
//! it is held in the channel config and never returned to the client (the channel
//! read path redacts it). The rendered message text is placed in a section block
//! so it renders formatted in the channel, with a colour bar keyed off the state.

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
    let cfg: SlackConfig = serde_json::from_value(config.clone())
        .map_err(|e| format!("invalid slack config: {e}"))?;
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
/// red while firing, green on resolve, so the message reads at a glance.
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

    fn note(transition: &str) -> Notification {
        Notification {
            rule_name: "CPU".into(),
            transition: transition.into(),
            value: Some(95.0),
            threshold: 90.0,
            op: "gt".into(),
            message: "Alert CPU is firing".into(),
        }
    }

    #[test]
    fn firing_is_red_resolved_is_green() {
        let firing = block_kit(&note("firing"));
        let resolved = block_kit(&note("resolved"));
        assert_eq!(firing["attachments"][0]["color"], "#cc0000");
        assert_eq!(resolved["attachments"][0]["color"], "#2eb886");
    }

    #[test]
    fn message_is_carried_into_text_and_block() {
        let body = block_kit(&note("firing"));
        assert_eq!(body["text"], "Alert CPU is firing");
        assert_eq!(
            body["attachments"][0]["blocks"][0]["text"]["text"],
            "Alert CPU is firing"
        );
    }

    #[test]
    fn missing_url_is_a_config_error() {
        let cfg: Result<SlackConfig, _> = serde_json::from_value(serde_json::json!({}));
        assert!(cfg.is_err());
    }
}
