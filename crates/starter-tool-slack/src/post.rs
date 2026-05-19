//! [`SlackPostTool`] — outbound `Tool` impl wrapping
//! [`chat.postMessage`](https://api.slack.com/methods/chat.postMessage).
//!
//! The HTTP call shape is lifted from `codeless-slack::web_api`; the
//! `Tool` framing (`ToolDefinition`, `invoke -> SpiResult<Value>`),
//! the prometheus surface, and the input/output types are starter's.

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use prometheus::Registry;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_spi::{ExposeSecret, Error as SpiError, Result as SpiResult};

use crate::config::SlackConfig;
use crate::error::SlackError;
use crate::metrics::ToolMetrics;

/// Stable tool name advertised in [`ToolDefinition::name`] and used as
/// the `tool.name` field on every tracing event. Keep this constant —
/// dashboards and MCP clients key off it.
pub const TOOL_NAME: &str = "slack.post_message";

/// Input shape for [`SlackPostTool`]. Deserialized from the JSON value
/// MCP / REST callers hand to `Tool::invoke`.
#[derive(Debug, Deserialize, Serialize)]
pub struct SlackPostInput {
    /// Channel id (`C0123…`) or `#name`. Slack resolves the name
    /// server-side; the canonical id comes back in the response.
    pub channel: String,
    /// Message body. Slack renders mrkdwn by default.
    pub text: String,
    /// Optional [Block Kit](https://api.slack.com/block-kit) payload.
    /// Forwarded verbatim as the `blocks` field on `chat.postMessage`;
    /// the tool does not validate the schema (Slack does).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocks: Option<Value>,
    /// Optional `thread_ts` for an in-thread reply. `None` posts at
    /// channel level. Carried so a downstream service-side handler
    /// (Surface 2) can reply inside the thread it received the event
    /// from without inventing a second tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_ts: Option<String>,
}

/// Successful response body returned by `Tool::invoke`.
#[derive(Debug, Serialize)]
pub struct SlackPostOutput {
    /// Canonical channel id Slack resolved the post to.
    pub channel: String,
    /// Slack-side message timestamp. Doubles as the `thread_ts` for
    /// any subsequent reply.
    pub ts: String,
}

/// Wire body for `chat.postMessage`. Private — the public surface is
/// [`SlackPostInput`].
#[derive(Serialize)]
struct ChatPostMessageBody<'a> {
    channel: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    blocks: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_ts: Option<&'a str>,
}

/// Wire response. `ts` is `Option` because Slack only sends it on
/// `ok=true`; the conversion to [`SlackPostOutput`] enforces presence.
#[derive(Deserialize)]
struct ChatPostMessageResp {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    ts: Option<String>,
}

/// `Tool` impl for Slack `chat.postMessage`.
///
/// Construct once at startup, register into a `ToolRegistry`, share by
/// `Arc` — every field is cheaply cloneable.
pub struct SlackPostTool {
    http: reqwest::Client,
    bot_token: starter_spi::SecretString,
    base_url: String,
    metrics: ToolMetrics,
}

impl SlackPostTool {
    /// Build the tool. Registers the prometheus collectors on the
    /// supplied [`Registry`] — fails if the metric names are already
    /// present (treat that as a programmer error: every tool should
    /// be constructed exactly once per registry).
    pub fn new(config: SlackConfig, registry: &Registry) -> Result<Self, prometheus::Error> {
        Self::with_client(config, registry, reqwest::Client::new())
    }

    /// Same as [`Self::new`] but accepts an already-built
    /// [`reqwest::Client`]. Use this when the consumer wants a shared
    /// client (custom timeouts, proxy, connection pool) across all
    /// HTTP-backed tools.
    pub fn with_client(
        config: SlackConfig,
        registry: &Registry,
        http: reqwest::Client,
    ) -> Result<Self, prometheus::Error> {
        let metrics = ToolMetrics::register(registry)?;
        Ok(Self {
            http,
            bot_token: config.bot_token,
            base_url: config.base_url,
            metrics,
        })
    }
}

#[async_trait]
impl Tool for SlackPostTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: TOOL_NAME.to_string(),
            description: "Post a message into a Slack channel via \
                          chat.postMessage. Optionally a Block Kit \
                          payload and/or a thread_ts for in-thread \
                          replies."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["channel", "text"],
                "additionalProperties": false,
                "properties": {
                    "channel":   { "type": "string", "description": "Channel id (C…) or #name." },
                    "text":      { "type": "string", "description": "Message body (mrkdwn)." },
                    "blocks":    { "type": "array",  "description": "Optional Block Kit payload." },
                    "thread_ts": { "type": "string", "description": "Parent message ts for in-thread reply." }
                }
            }),
        }
    }

    async fn invoke(&self, input: Value) -> SpiResult<Value> {
        // Deserialize at the boundary — the dispatcher already
        // validated against `input_schema` but the boundary needs the
        // typed shape anyway. A failure here is `Invalid`, not
        // `Internal`: the caller can fix the request.
        let parsed: SlackPostInput = match serde_json::from_value(input) {
            Ok(v) => v,
            Err(e) => {
                self.metrics.errors.with_label_values(&["bad_input"]).inc();
                return Err(SpiError::Invalid {
                    message: format!("slack.post_message input: {e}"),
                });
            }
        };

        let start = Instant::now();
        let result = self.call_chat_post(&parsed).await;
        let elapsed = start.elapsed().as_secs_f64();
        self.metrics.latency.observe(elapsed);

        match result {
            Ok(out) => {
                tracing::info!(
                    tool.name = TOOL_NAME,
                    channel = %out.channel,
                    ts = %out.ts,
                    latency_seconds = elapsed,
                    "slack.post_message ok",
                );
                Ok(serde_json::to_value(out).expect("SlackPostOutput is plain serde"))
            }
            Err(err) => {
                let kind = error_kind(&err);
                self.metrics.errors.with_label_values(&[kind]).inc();
                tracing::warn!(
                    tool.name = TOOL_NAME,
                    error.kind = kind,
                    error = %err,
                    latency_seconds = elapsed,
                    "slack.post_message failed",
                );
                Err(err.into())
            }
        }
    }
}

impl SlackPostTool {
    /// One HTTP round-trip. Pulled out of `invoke` so the metric +
    /// tracing wrapping stays linear above.
    async fn call_chat_post(&self, input: &SlackPostInput) -> Result<SlackPostOutput, SlackError> {
        let url = format!("{}/chat.postMessage", self.base_url);
        let body = ChatPostMessageBody {
            channel: &input.channel,
            text: &input.text,
            blocks: input.blocks.as_ref(),
            thread_ts: input.thread_ts.as_deref(),
        };
        let resp = self
            .http
            .post(url)
            .bearer_auth(self.bot_token.expose_secret())
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        // 429 is a documented Slack response with a `Retry-After`
        // header. Branch on it first so a retry layer can read the
        // header without us swallowing the body into `HttpStatus`.
        if status.as_u16() == 429 {
            let retry_after_secs = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<u64>().ok());
            return Err(SlackError::RateLimited { retry_after_secs });
        }
        if !status.is_success() {
            return Err(SlackError::HttpStatus {
                status: status.as_u16(),
            });
        }
        let payload: ChatPostMessageResp = resp.json().await?;
        if !payload.ok {
            return Err(SlackError::SlackApi(payload.error));
        }
        let ts = payload.ts.ok_or(SlackError::MissingTs)?;
        // Slack echoes the canonical channel id; if it didn't, the
        // caller's original input is the next-best identifier.
        let channel = payload.channel.unwrap_or_else(|| input.channel.clone());
        Ok(SlackPostOutput { channel, ts })
    }
}

/// Stable label string for the `kind` axis on
/// `starter_tool_slack_post_errors_total`. Keep aligned with the
/// histogram dashboard; reviewers should reject silent re-labelling.
fn error_kind(err: &SlackError) -> &'static str {
    match err {
        SlackError::Transport(_) => "transport",
        SlackError::RateLimited { .. } => "rate_limited",
        SlackError::HttpStatus { .. } => "http_status",
        SlackError::SlackApi(_) => "slack_api",
        SlackError::MissingTs => "missing_ts",
    }
}

// Silence `unused`-warnings if `Arc` is ever needed: the type is part
// of the documented surface (tool is normally Arc-shared inside
// `ToolRegistry`) but the impl itself never reaches for it.
#[allow(dead_code)]
fn _assert_send_sync() {
    fn t<T: Send + Sync>() {}
    t::<SlackPostTool>();
    t::<Arc<SlackPostTool>>();
}
