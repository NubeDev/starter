//! [`GitHubCreateIssueTool`] — outbound `Tool` impl wrapping
//! [`POST /repos/{owner}/{repo}/issues`](https://docs.github.com/en/rest/issues/issues#create-an-issue).

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use prometheus::Registry;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_spi::{Error as SpiError, ExposeSecret, Result as SpiResult};

use crate::config::GitHubConfig;
use crate::error::GitHubError;
use crate::metrics::ToolMetrics;

/// Stable tool name advertised in [`ToolDefinition::name`].
pub const TOOL_NAME: &str = "github.create_issue";

/// Input shape for [`GitHubCreateIssueTool`]. Deserialized from the
/// JSON value MCP / REST callers hand to `Tool::invoke`.
#[derive(Debug, Deserialize, Serialize)]
pub struct GitHubCreateIssueInput {
    /// Repository owner (user or org).
    pub owner: String,
    /// Repository name.
    pub repo: String,
    /// Issue title.
    pub title: String,
    /// Optional issue body (Markdown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Optional labels to apply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    /// Optional assignees (GitHub usernames).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignees: Option<Vec<String>>,
}

/// Successful response body returned by `Tool::invoke`.
#[derive(Debug, Serialize)]
pub struct GitHubCreateIssueOutput {
    /// Numeric issue id.
    pub id: u64,
    /// Issue number within the repository.
    pub number: u64,
    /// HTML URL of the created issue.
    pub html_url: String,
}

/// Wire body for `POST /repos/{owner}/{repo}/issues`.
#[derive(Serialize)]
struct CreateIssueBody<'a> {
    title: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<&'a [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    assignees: Option<&'a [String]>,
}

/// Wire response from GitHub issues endpoint.
#[derive(Deserialize)]
struct CreateIssueResp {
    id: u64,
    number: u64,
    html_url: String,
}

/// `Tool` impl for GitHub issue creation.
///
/// Construct once at startup, register into a `ToolRegistry`, share by
/// `Arc` — every field is cheaply cloneable.
pub struct GitHubCreateIssueTool {
    http: reqwest::Client,
    access_token: starter_spi::SecretString,
    base_url: String,
    metrics: ToolMetrics,
}

impl GitHubCreateIssueTool {
    /// Build the tool. Registers the prometheus collectors on the
    /// supplied [`Registry`].
    pub fn new(config: GitHubConfig, registry: &Registry) -> Result<Self, prometheus::Error> {
        Self::with_client(config, registry, default_client())
    }

    /// Same as [`Self::new`] but accepts an already-built
    /// [`reqwest::Client`].
    pub fn with_client(
        config: GitHubConfig,
        registry: &Registry,
        http: reqwest::Client,
    ) -> Result<Self, prometheus::Error> {
        let metrics = ToolMetrics::register(registry)?;
        Ok(Self {
            http,
            access_token: config.access_token,
            base_url: config.base_url,
            metrics,
        })
    }
}

fn default_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("starter-tool-github")
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest client builds with no I/O")
}

#[async_trait]
impl Tool for GitHubCreateIssueTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: TOOL_NAME.to_string(),
            description: "Create an issue on a GitHub repository via \
                          POST /repos/{owner}/{repo}/issues."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["owner", "repo", "title"],
                "additionalProperties": false,
                "properties": {
                    "owner":     { "type": "string", "description": "Repository owner (user or org)." },
                    "repo":      { "type": "string", "description": "Repository name." },
                    "title":     { "type": "string", "description": "Issue title." },
                    "body":      { "type": "string", "description": "Issue body (Markdown)." },
                    "labels":    { "type": "array", "items": { "type": "string" }, "description": "Labels to apply." },
                    "assignees": { "type": "array", "items": { "type": "string" }, "description": "GitHub usernames to assign." }
                }
            }),
        }
    }

    async fn invoke(&self, input: Value) -> SpiResult<Value> {
        let parsed: GitHubCreateIssueInput = match serde_json::from_value(input) {
            Ok(v) => v,
            Err(e) => {
                self.metrics.errors.with_label_values(&["bad_input"]).inc();
                return Err(SpiError::Invalid {
                    message: format!("github.create_issue input: {e}"),
                });
            }
        };

        let start = Instant::now();
        let result = self.call_create_issue(&parsed).await;
        let elapsed = start.elapsed().as_secs_f64();
        self.metrics.latency.observe(elapsed);

        match result {
            Ok(out) => {
                tracing::info!(
                    tool.name = TOOL_NAME,
                    issue.number = out.number,
                    issue.url = %out.html_url,
                    latency_seconds = elapsed,
                    "github.create_issue ok",
                );
                Ok(serde_json::to_value(&out).expect("GitHubCreateIssueOutput is plain serde"))
            }
            Err(err) => {
                let kind = error_kind(&err);
                self.metrics.errors.with_label_values(&[kind]).inc();
                tracing::warn!(
                    tool.name = TOOL_NAME,
                    error.kind = kind,
                    error = %err,
                    latency_seconds = elapsed,
                    "github.create_issue failed",
                );
                Err(err.into())
            }
        }
    }
}

impl GitHubCreateIssueTool {
    async fn call_create_issue(
        &self,
        input: &GitHubCreateIssueInput,
    ) -> Result<GitHubCreateIssueOutput, GitHubError> {
        let url = format!(
            "{}/repos/{}/{}/issues",
            self.base_url, input.owner, input.repo
        );
        let body = CreateIssueBody {
            title: &input.title,
            body: input.body.as_deref(),
            labels: input.labels.as_deref(),
            assignees: input.assignees.as_deref(),
        };
        let resp = self
            .http
            .post(&url)
            .bearer_auth(self.access_token.expose_secret())
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();

        if status.as_u16() == 401 {
            let text = resp.text().await.unwrap_or_default();
            return Err(GitHubError::Unauthorized(text));
        }

        if status.as_u16() == 403 || status.as_u16() == 429 {
            let retry_after_secs = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<u64>().ok());
            return Err(GitHubError::RateLimited { retry_after_secs });
        }

        if !status.is_success() {
            let body_text = resp
                .text()
                .await
                .unwrap_or_default()
                .chars()
                .take(512)
                .collect();
            return Err(GitHubError::HttpStatus {
                status: status.as_u16(),
                body: body_text,
            });
        }

        let payload: CreateIssueResp = resp.json().await?;
        Ok(GitHubCreateIssueOutput {
            id: payload.id,
            number: payload.number,
            html_url: payload.html_url,
        })
    }
}

/// Stable label string for the `kind` axis on the error counter.
fn error_kind(err: &GitHubError) -> &'static str {
    match err {
        GitHubError::Transport(_) => "transport",
        GitHubError::RateLimited { .. } => "rate_limited",
        GitHubError::HttpStatus { .. } => "http_status",
        GitHubError::Unauthorized(_) => "unauthorized",
    }
}

#[allow(dead_code)]
fn _assert_send_sync() {
    fn t<T: Send + Sync>() {}
    t::<GitHubCreateIssueTool>();
    t::<Arc<GitHubCreateIssueTool>>();
}
