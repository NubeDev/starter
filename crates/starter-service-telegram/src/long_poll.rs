//! HTTP transport for the long-poll: build the `getUpdates` URL,
//! POST, decode, project each `Update` element onto a
//! `(kind, payload)` pair for the [`EventSink`].
//!
//! Lifted from `codeless-telegram::web_api`'s `get_updates` /
//! `decode` helpers; the dispatcher seam is replaced with the typed
//! [`PolledUpdate`] return so the outer service module owns the
//! `EventSink::emit` call (SCOPE R4 — the service does not
//! pattern-match on payloads beyond deserialization).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::TelegramBotError;

/// Build the `getUpdates` URL given a base URL and the bot token.
/// Pulled out so the test suite can assert the trailing-slash
/// normalisation without spinning a real HTTP server.
pub(crate) fn get_updates_url(base_url: &str, bot_token: &str) -> String {
    format!(
        "{}/bot{}/getUpdates",
        base_url.trim_end_matches('/'),
        bot_token,
    )
}

/// Wire body for `getUpdates`. `allowed_updates` is left empty (the
/// Bot API default — every update type the bot is subscribed to)
/// because R4 requires the service to forward everything it receives
/// rather than make a per-kind subscription policy on the consumer's
/// behalf.
#[derive(Debug, Serialize)]
struct GetUpdatesArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<i64>,
    timeout: u64,
}

/// Top-level Bot API response envelope.
#[derive(Debug, Deserialize)]
struct ApiResponse {
    ok: bool,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    result: Option<Vec<Value>>,
}

/// One projected update ready for the sink. `kind` is
/// `telegram.<update_type>` — the field name in the raw JSON that
/// carries the typed body (`message`, `edited_message`,
/// `callback_query`, …). `payload` is the entire update object
/// forwarded verbatim per R4.
#[derive(Debug, Clone)]
pub(crate) struct PolledUpdate {
    /// Stable `update_id` Telegram assigned to the row. The loop
    /// tracks `max(update_id) + 1` as the cookie for the next call.
    pub update_id: i64,
    /// Sink emit kind, e.g. `"telegram.message"`.
    pub kind: String,
    /// Raw update payload — forwarded as the `payload` argument to
    /// `EventSink::emit`.
    pub payload: Value,
}

/// POST `getUpdates` and project the results.
pub(crate) async fn poll_once(
    http: &reqwest::Client,
    url: &str,
    offset: Option<i64>,
    timeout_secs: u64,
) -> Result<Vec<PolledUpdate>, TelegramBotError> {
    let args = GetUpdatesArgs {
        offset,
        timeout: timeout_secs,
    };
    let resp = http.post(url).json(&args).send().await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(TelegramBotError::HttpStatus {
            status: status.as_u16(),
        });
    }
    let body: ApiResponse = resp.json().await?;
    if !body.ok {
        return Err(TelegramBotError::BotApi(body.description));
    }
    let raw = body.result.unwrap_or_default();
    let mut out = Vec::with_capacity(raw.len());
    for value in raw {
        let Some(update_id) = value.get("update_id").and_then(Value::as_i64) else {
            // Without an `update_id` we cannot ack — Telegram would
            // re-deliver forever. Skip; an operator reading the log
            // will see the bad row.
            tracing::warn!(?value, "telegram: update missing update_id; skipping");
            continue;
        };
        let kind = update_kind(&value);
        out.push(PolledUpdate {
            update_id,
            kind,
            payload: value,
        });
    }
    Ok(out)
}

/// Pick the `kind` label for one update. R4 forbids the service from
/// pattern-matching past the envelope shape — we only look at *which*
/// top-level field is present (`message`, `edited_message`, …) and
/// turn its name into `"telegram.<field>"`. The full payload is
/// forwarded verbatim.
fn update_kind(value: &Value) -> String {
    // Telegram's docs list the update-type fields in a stable order.
    // Iterating over the JSON object's own keys would work too, but
    // a fixed list keeps the `kind` label deterministic if Telegram
    // ever lands two type fields on the same update (which they
    // don't, but the SCOPE warns against silent re-labelling).
    const UPDATE_TYPES: &[&str] = &[
        "message",
        "edited_message",
        "channel_post",
        "edited_channel_post",
        "business_connection",
        "business_message",
        "edited_business_message",
        "deleted_business_messages",
        "message_reaction",
        "message_reaction_count",
        "inline_query",
        "chosen_inline_result",
        "callback_query",
        "shipping_query",
        "pre_checkout_query",
        "purchased_paid_media",
        "poll",
        "poll_answer",
        "my_chat_member",
        "chat_member",
        "chat_join_request",
        "chat_boost",
        "removed_chat_boost",
    ];
    if let Some(obj) = value.as_object() {
        for field in UPDATE_TYPES {
            if obj.contains_key(*field) {
                return format!("telegram.{field}");
            }
        }
    }
    "telegram.unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn get_updates_url_handles_trailing_slash() {
        assert_eq!(
            get_updates_url("https://api.telegram.org", "12345:abc"),
            "https://api.telegram.org/bot12345:abc/getUpdates",
        );
        assert_eq!(
            get_updates_url("https://api.telegram.org/", "12345:abc"),
            "https://api.telegram.org/bot12345:abc/getUpdates",
        );
    }

    #[test]
    fn update_kind_picks_first_present_field() {
        assert_eq!(
            update_kind(&json!({"update_id": 1, "message": {"text": "hi"}})),
            "telegram.message",
        );
        assert_eq!(
            update_kind(&json!({"update_id": 2, "callback_query": {"id": "x"}})),
            "telegram.callback_query",
        );
        assert_eq!(
            update_kind(&json!({"update_id": 3, "unknown_field": {}})),
            "telegram.unknown",
        );
    }
}
