//! [`TelegramSendMessageTool`] — outbound `Tool` impl wrapping
//! [`sendMessage`](https://core.telegram.org/bots/api#sendmessage).
//!
//! The HTTP call shape is lifted from `codeless-telegram::web_api`;
//! the `Tool` framing (`ToolDefinition`, `invoke -> SpiResult<Value>`),
//! the prometheus surface, and the input/output types are starter's.

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use prometheus::Registry;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_spi::{Error as SpiError, ExposeSecret, Result as SpiResult};

use crate::config::TelegramConfig;
use crate::error::TelegramError;
use crate::metrics::ToolMetrics;

/// Stable tool name advertised in [`ToolDefinition::name`] and used as
/// the `tool.name` field on every tracing event. Keep this constant —
/// dashboards and MCP clients key off it.
pub const TOOL_NAME: &str = "telegram.send_message";

/// Input shape for [`TelegramSendMessageTool`]. Deserialized from the
/// JSON value MCP / REST callers hand to `Tool::invoke`.
#[derive(Debug, Deserialize, Serialize)]
pub struct TelegramSendMessageInput {
    /// Target chat id. Accepts an integer chat id (positive for
    /// users, negative for groups/channels) **or** a `@channelname`.
    /// We deserialize as JSON `Value` and stringify at the wire
    /// boundary so both forms compose.
    pub chat_id: Value,
    /// Message body.
    pub text: String,
    /// Optional `parse_mode` (`"MarkdownV2"` / `"HTML"`). Forwarded
    /// verbatim; escape rules are the caller's responsibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_mode: Option<String>,
    /// Optional `reply_to_message_id` for an in-thread reply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to_message_id: Option<i64>,
    /// Optional `message_thread_id` for a forum-topic post.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_thread_id: Option<i64>,
}

/// Successful response body returned by `Tool::invoke`.
#[derive(Debug, Serialize)]
pub struct TelegramSendMessageOutput {
    /// Telegram-side message id.
    pub message_id: i64,
    /// Numeric chat id Telegram resolved the post to.
    pub chat_id: i64,
}

/// Wire body for `sendMessage`. Private — the public surface is
/// [`TelegramSendMessageInput`].
#[derive(Serialize)]
struct SendMessageBody<'a> {
    chat_id: &'a Value,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_mode: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to_message_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_thread_id: Option<i64>,
}

#[derive(Deserialize)]
struct BotApiResponse {
    ok: bool,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    result: Option<SentMessage>,
    /// Present on `ok=false` 429 responses inside `parameters`.
    #[serde(default)]
    parameters: Option<ResponseParameters>,
}

#[derive(Deserialize)]
struct ResponseParameters {
    #[serde(default)]
    retry_after: Option<u64>,
}

#[derive(Deserialize)]
struct SentMessage {
    message_id: i64,
    chat: ChatInfo,
}

#[derive(Deserialize)]
struct ChatInfo {
    id: i64,
}

/// `Tool` impl for Telegram `sendMessage`.
///
/// Construct once at startup, register into a `ToolRegistry`, share by
/// `Arc` — every field is cheaply cloneable.
pub struct TelegramSendMessageTool {
    http: reqwest::Client,
    /// Pre-built `<base_url>/bot<bot_token>` prefix; method names are
    /// appended at call time. Keeps the token out of every per-call
    /// `format!` argument an operator might accidentally log.
    method_base: String,
    metrics: ToolMetrics,
}

impl TelegramSendMessageTool {
    /// Build the tool. Registers the prometheus collectors on the
    /// supplied [`Registry`] — fails if the metric names are already
    /// present (treat that as a programmer error: every tool should
    /// be constructed exactly once per registry).
    pub fn new(config: TelegramConfig, registry: &Registry) -> Result<Self, prometheus::Error> {
        Self::with_client(config, registry, reqwest::Client::new())
    }

    /// Same as [`Self::new`] but accepts an already-built
    /// [`reqwest::Client`]. Use this when the consumer wants a shared
    /// client (custom timeouts, proxy, connection pool) across all
    /// HTTP-backed tools.
    pub fn with_client(
        config: TelegramConfig,
        registry: &Registry,
        http: reqwest::Client,
    ) -> Result<Self, prometheus::Error> {
        let metrics = ToolMetrics::register(registry)?;
        let method_base = format!(
            "{}/bot{}",
            config.base_url.trim_end_matches('/'),
            config.bot_token.expose_secret()
        );
        Ok(Self {
            http,
            method_base,
            metrics,
        })
    }
}

#[async_trait]
impl Tool for TelegramSendMessageTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: TOOL_NAME.to_string(),
            description: "Send a message into a Telegram chat via the \
                          Bot API sendMessage method. Optionally a \
                          parse_mode, reply_to_message_id, or \
                          message_thread_id for forum-topic posts."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["chat_id", "text"],
                "additionalProperties": false,
                "properties": {
                    "chat_id":             { "type": ["integer", "string"], "description": "Numeric chat id or @channelname." },
                    "text":                { "type": "string", "description": "Message body." },
                    "parse_mode":          { "type": "string", "description": "Optional MarkdownV2 / HTML parse mode." },
                    "reply_to_message_id": { "type": "integer", "description": "Parent message id for an in-thread reply." },
                    "message_thread_id":   { "type": "integer", "description": "Forum-topic thread id." }
                }
            }),
        }
    }

    async fn invoke(&self, input: Value) -> SpiResult<Value> {
        // Deserialize at the boundary — the dispatcher already
        // validated against `input_schema` but the boundary needs the
        // typed shape anyway. A failure here is `Invalid`, not
        // `Internal`: the caller can fix the request.
        let parsed: TelegramSendMessageInput = match serde_json::from_value(input) {
            Ok(v) => v,
            Err(e) => {
                self.metrics.errors.with_label_values(&["bad_input"]).inc();
                return Err(SpiError::Invalid {
                    message: format!("telegram.send_message input: {e}"),
                });
            }
        };

        let start = Instant::now();
        let result = self.call_send_message(&parsed).await;
        let elapsed = start.elapsed().as_secs_f64();
        self.metrics.latency.observe(elapsed);

        match result {
            Ok(out) => {
                tracing::info!(
                    tool.name = TOOL_NAME,
                    chat_id = out.chat_id,
                    message_id = out.message_id,
                    latency_seconds = elapsed,
                    "telegram.send_message ok",
                );
                Ok(serde_json::to_value(out).expect("TelegramSendMessageOutput is plain serde"))
            }
            Err(err) => {
                let kind = error_kind(&err);
                self.metrics.errors.with_label_values(&[kind]).inc();
                tracing::warn!(
                    tool.name = TOOL_NAME,
                    error.kind = kind,
                    error = %err,
                    latency_seconds = elapsed,
                    "telegram.send_message failed",
                );
                Err(err.into())
            }
        }
    }
}

impl TelegramSendMessageTool {
    /// One HTTP round-trip. Pulled out of `invoke` so the metric +
    /// tracing wrapping stays linear above.
    async fn call_send_message(
        &self,
        input: &TelegramSendMessageInput,
    ) -> Result<TelegramSendMessageOutput, TelegramError> {
        let url = format!("{}/sendMessage", self.method_base);
        let body = SendMessageBody {
            chat_id: &input.chat_id,
            text: &input.text,
            parse_mode: input.parse_mode.as_deref(),
            reply_to_message_id: input.reply_to_message_id,
            message_thread_id: input.message_thread_id,
        };
        let resp = self.http.post(url).json(&body).send().await?;
        let status = resp.status();
        // 429 is a documented Bot API response. The retry-after value
        // can land in the header or the `parameters.retry_after`
        // field; branch on it first so a retry layer can read either.
        if status.as_u16() == 429 {
            let header_retry = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<u64>().ok());
            // Try to also pick up the body's `parameters.retry_after`.
            let body_retry = match resp.json::<BotApiResponse>().await {
                Ok(b) => b.parameters.and_then(|p| p.retry_after),
                Err(_) => None,
            };
            return Err(TelegramError::RateLimited {
                retry_after_secs: header_retry.or(body_retry),
            });
        }
        if !status.is_success() {
            return Err(TelegramError::HttpStatus {
                status: status.as_u16(),
            });
        }
        let payload: BotApiResponse = resp.json().await?;
        if !payload.ok {
            return Err(TelegramError::BotApi(payload.description));
        }
        let sent = payload.result.ok_or(TelegramError::MissingResult)?;
        Ok(TelegramSendMessageOutput {
            message_id: sent.message_id,
            chat_id: sent.chat.id,
        })
    }
}

/// Stable label string for the `kind` axis on
/// `starter_tool_telegram_send_message_errors_total`. Keep aligned
/// with the histogram dashboard; reviewers should reject silent
/// re-labelling.
fn error_kind(err: &TelegramError) -> &'static str {
    match err {
        TelegramError::Transport(_) => "transport",
        TelegramError::RateLimited { .. } => "rate_limited",
        TelegramError::HttpStatus { .. } => "http_status",
        TelegramError::BotApi(_) => "bot_api",
        TelegramError::MissingResult => "missing_result",
    }
}

// Silence `unused`-warnings if `Arc` is ever needed: the type is part
// of the documented surface (tool is normally Arc-shared inside
// `ToolRegistry`) but the impl itself never reaches for it.
#[allow(dead_code)]
fn _assert_send_sync() {
    fn t<T: Send + Sync>() {}
    t::<TelegramSendMessageTool>();
    t::<Arc<TelegramSendMessageTool>>();
}
