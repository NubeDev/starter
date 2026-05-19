//! [`GmailSendTool`] — outbound `Tool` impl wrapping
//! [`users.messages.send`](https://developers.google.com/gmail/api/reference/rest/v1/users.messages/send).
//!
//! The HTTP call shape is lifted from
//! `codeless-tools::email::gmail::GmailMailer`; the `Tool` framing
//! (`ToolDefinition`, `invoke -> SpiResult<Value>`), the prometheus
//! surface, and the input/output types are starter's.

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use prometheus::Registry;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_spi::{Error as SpiError, ExposeSecret, Result as SpiResult};

use crate::config::GmailConfig;
use crate::error::GmailError;
use crate::message::{GmailMailbox, GmailMessage};
use crate::metrics::ToolMetrics;

/// Stable tool name advertised in [`ToolDefinition::name`] and used
/// as the `tool.name` field on every tracing event. Keep this
/// constant — dashboards and MCP clients key off it.
pub const TOOL_NAME: &str = "gmail.send";

/// Input shape for [`GmailSendTool`]. Deserialized from the JSON
/// value MCP / REST callers hand to `Tool::invoke`.
///
/// The input is a thin wrapper around [`GmailMessage`] — every field
/// flows straight through. We do *not* accept a pre-built `raw`
/// blob: the SCOPE rule R4 line "provider crates do not own domain
/// logic" cuts the other way too — letting the consumer pass raw
/// MIME would push MIME knowledge into every starter REST handler.
#[derive(Debug, Deserialize, Serialize)]
pub struct GmailSendInput {
    /// `From:` mailbox. When omitted, Gmail fills it in from the
    /// authenticated account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<GmailMailbox>,
    /// `To:` recipients.
    #[serde(default)]
    pub to: Vec<GmailMailbox>,
    /// `Cc:` recipients.
    #[serde(default)]
    pub cc: Vec<GmailMailbox>,
    /// `Bcc:` recipients. Gmail routes by SMTP envelope.
    #[serde(default)]
    pub bcc: Vec<GmailMailbox>,
    /// Optional `Reply-To:` mailbox.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<GmailMailbox>,
    /// `Subject:` line.
    pub subject: String,
    /// Plain-text body. UTF-8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// HTML body. UTF-8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
}

impl From<GmailSendInput> for GmailMessage {
    fn from(v: GmailSendInput) -> Self {
        Self {
            from: v.from,
            to: v.to,
            cc: v.cc,
            bcc: v.bcc,
            reply_to: v.reply_to,
            subject: v.subject,
            text: v.text,
            html: v.html,
        }
    }
}

/// Successful response body returned by `Tool::invoke`.
#[derive(Debug, Serialize)]
pub struct GmailSendOutput {
    /// Gmail-assigned message id, e.g. `"18f3c1aa9b7c4d12"`.
    pub message_id: String,
}

/// Wire body for `users.messages.send`. The endpoint accepts a
/// single field — the base64url-encoded RFC 5322 blob.
#[derive(Serialize)]
struct SendBody<'a> {
    raw: &'a str,
}

#[derive(Deserialize)]
struct SendResponse {
    #[serde(default)]
    id: Option<String>,
}

/// `Tool` impl for Gmail `users.messages.send`.
///
/// Construct once at startup, register into a `ToolRegistry`, share
/// by `Arc` — every field is cheaply cloneable.
pub struct GmailSendTool {
    http: reqwest::Client,
    /// Fully-resolved URL — `<base_url>/gmail/v1/users/<user_id>/messages/send`.
    /// Built once at construction so the per-call hot path does no
    /// formatting work.
    endpoint: String,
    /// OAuth bearer token, held as `SecretString` per R5; exposed
    /// only once per call via [`ExposeSecret`] when handing to
    /// `reqwest::RequestBuilder::bearer_auth`.
    access_token: starter_spi::SecretString,
    metrics: ToolMetrics,
}

impl GmailSendTool {
    /// Build the tool. Registers the prometheus collectors on the
    /// supplied [`Registry`] — fails if the metric names are already
    /// present (treat that as a programmer error: every tool should
    /// be constructed exactly once per registry).
    pub fn new(config: GmailConfig, registry: &Registry) -> Result<Self, prometheus::Error> {
        Self::with_client(config, registry, reqwest::Client::new())
    }

    /// Same as [`Self::new`] but accepts an already-built
    /// [`reqwest::Client`]. Use this when the consumer wants a
    /// shared client (custom timeouts, proxy, connection pool)
    /// across all HTTP-backed tools.
    pub fn with_client(
        config: GmailConfig,
        registry: &Registry,
        http: reqwest::Client,
    ) -> Result<Self, prometheus::Error> {
        let metrics = ToolMetrics::register(registry)?;
        let endpoint = format!(
            "{}/gmail/v1/users/{}/messages/send",
            config.base_url.trim_end_matches('/'),
            config.user_id,
        );
        Ok(Self {
            http,
            endpoint,
            access_token: config.oauth_access_token,
            metrics,
        })
    }
}

#[async_trait]
impl Tool for GmailSendTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: TOOL_NAME.to_string(),
            description: "Send an email via the Gmail REST \
                          users.messages.send endpoint. The consumer \
                          supplies an already-resolved OAuth2 access \
                          token; this tool does not perform token \
                          acquisition or refresh."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["subject"],
                "additionalProperties": false,
                "properties": {
                    "from":     mailbox_schema(),
                    "to":       { "type": "array", "items": mailbox_schema() },
                    "cc":       { "type": "array", "items": mailbox_schema() },
                    "bcc":      { "type": "array", "items": mailbox_schema() },
                    "reply_to": mailbox_schema(),
                    "subject":  { "type": "string", "description": "Subject line." },
                    "text":     { "type": "string", "description": "Plain-text body." },
                    "html":     { "type": "string", "description": "HTML body." }
                }
            }),
        }
    }

    async fn invoke(&self, input: Value) -> SpiResult<Value> {
        // Deserialize at the boundary — a failure here is `Invalid`,
        // not `Internal`: the caller can fix the request.
        let parsed: GmailSendInput = match serde_json::from_value(input) {
            Ok(v) => v,
            Err(e) => {
                self.metrics.errors.with_label_values(&["bad_input"]).inc();
                return Err(SpiError::Invalid {
                    message: format!("gmail.send input: {e}"),
                });
            }
        };

        let start = Instant::now();
        let result = self.call_send(parsed).await;
        let elapsed = start.elapsed().as_secs_f64();
        self.metrics.latency.observe(elapsed);

        match result {
            Ok(out) => {
                tracing::info!(
                    tool.name = TOOL_NAME,
                    message_id = %out.message_id,
                    latency_seconds = elapsed,
                    "gmail.send ok",
                );
                Ok(serde_json::to_value(out).expect("GmailSendOutput is plain serde"))
            }
            Err(err) => {
                let kind = error_kind(&err);
                self.metrics.errors.with_label_values(&[kind]).inc();
                tracing::warn!(
                    tool.name = TOOL_NAME,
                    error.kind = kind,
                    error = %err,
                    latency_seconds = elapsed,
                    "gmail.send failed",
                );
                Err(err.into())
            }
        }
    }
}

impl GmailSendTool {
    /// One HTTP round-trip. Pulled out of `invoke` so the metric +
    /// tracing wrapping stays linear above.
    async fn call_send(&self, input: GmailSendInput) -> Result<GmailSendOutput, GmailError> {
        let message: GmailMessage = input.into();
        let raw = message.to_rfc5322()?;
        let raw_b64 = URL_SAFE_NO_PAD.encode(raw);

        let body = SendBody { raw: &raw_b64 };
        let resp = self
            .http
            .post(&self.endpoint)
            .bearer_auth(self.access_token.expose_secret())
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        // 401 (expired/missing token) and 403 (missing scope) both
        // map to the SPI's Unauthenticated variant. Read the body
        // best-effort so a wrapper layer can log the precise
        // Google-side error message.
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            let body = resp.text().await.unwrap_or_default();
            return Err(GmailError::Auth {
                status: status.as_u16(),
                body,
            });
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(GmailError::HttpStatus {
                status: status.as_u16(),
                body,
            });
        }

        let payload: SendResponse = resp.json().await?;
        let id = payload.id.ok_or(GmailError::MissingId)?;
        Ok(GmailSendOutput { message_id: id })
    }
}

/// Reusable schema fragment for a single mailbox. Kept inline (vs. a
/// `$ref`) because every MCP dispatcher we target inlines refs
/// anyway — staying flat keeps the surface debuggable.
fn mailbox_schema() -> Value {
    json!({
        "type": "object",
        "required": ["address"],
        "additionalProperties": false,
        "properties": {
            "address": { "type": "string", "description": "addr-spec, e.g. user@example.com." },
            "name":    { "type": "string", "description": "Optional display name." }
        }
    })
}

/// Stable label string for the `kind` axis on
/// `starter_tool_gmail_send_errors_total`. Keep aligned with the
/// histogram dashboard; reviewers should reject silent re-labelling.
fn error_kind(err: &GmailError) -> &'static str {
    match err {
        GmailError::Transport(_) => "transport",
        GmailError::Auth { .. } => "auth",
        GmailError::HttpStatus { .. } => "http_status",
        GmailError::MissingId => "missing_id",
        GmailError::Build(_) => "message_build",
    }
}

// The tool is normally `Arc`-shared inside `ToolRegistry`; the
// `Send + Sync` bounds are required for that.
#[allow(dead_code)]
fn _assert_send_sync() {
    fn t<T: Send + Sync>() {}
    t::<GmailSendTool>();
    t::<Arc<GmailSendTool>>();
}
