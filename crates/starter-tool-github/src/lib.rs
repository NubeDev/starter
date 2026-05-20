//! # starter-tool-github
//!
//! Outbound GitHub integration as a [`Tool`](starter_spi::tool::Tool).
//! Wraps the GitHub REST API
//! [`POST /repos/{owner}/{repo}/issues`](https://docs.github.com/en/rest/issues/issues#create-an-issue)
//! endpoint so the consumer can create issues — directly from
//! REST/CLI code or via MCP if the tool is registered into a
//! [`ToolRegistry`].
//!
//! ## What this crate is, and is not
//!
//! v0.1 of `starter-tool-github` is **create-issue only**. Additional
//! endpoints (list issues, create PR, add comment) are deferred to
//! future versions. Landing a single action first keeps the
//! authentication surface narrow (one bearer token with `repo` scope).
//!
//! ## Authentication
//!
//! All GitHub REST calls use `Authorization: Bearer <token>` over
//! HTTPS. **This crate does not acquire tokens.** Token acquisition
//! (OAuth consent, PAT generation) lives in the consumer's `main.rs`
//! — usually behind `starter-auth-oauth` (the GitHub provider in
//! `starter-auth-oauth/src/providers/github.rs` already handles the
//! OAuth flow). [`GitHubConfig::access_token`] takes an
//! already-resolved [`SecretString`](starter_spi::SecretString) and
//! the crate uses it once per call via
//! [`ExposeSecret`](starter_spi::ExposeSecret).
//!
//! ## SCOPE rules this crate honours
//!
//! - **R1** — one integration per crate. Depending on
//!   `starter-tool-github` compiles only the GitHub outbound surface;
//!   it pulls no Slack, Telegram, or Gmail code.
//! - **R4** — provider crates do not own domain logic. This crate
//!   knows how to POST to the issues endpoint; it does not decide
//!   what the consumer wants to file.
//! - **R5** — credentials come in as
//!   [`SecretString`](starter_spi::SecretString). The crate does not
//!   read env vars or files; the consumer resolves the access token
//!   and hands it in via [`GitHubConfig`].
//! - **R7** — observability is required. [`GitHubCreateIssueTool::new`]
//!   takes the same [`prometheus::Registry`] the consumer hands to
//!   `McpHttpOptions`, registers `starter_tool_github_*` metrics on
//!   it, and emits a structured `tracing` event per invocation
//!   carrying a stable `tool.name` field.
//! - **R8** — no transitive vendor SDKs in `starter-spi`. The only
//!   starter-side deps here are `starter-spi` and
//!   `starter-observability`; everything else is `reqwest` + `serde`
//!   + `prometheus`. No `octocrab` or `octokit` SDK is involved.
//!
//! ## Quick start
//!
//! ```no_run
//! use std::sync::Arc;
//! use prometheus::Registry;
//! use starter_spi::SecretString;
//! use starter_tool_github::{GitHubConfig, GitHubCreateIssueTool};
//!
//! # async fn ex() -> Result<(), Box<dyn std::error::Error>> {
//! let registry = Arc::new(Registry::new());
//! let cfg = GitHubConfig {
//!     access_token: SecretString::from("ghp_…".to_string()),
//!     base_url:     GitHubConfig::default_base_url(),
//! };
//! let tool = GitHubCreateIssueTool::new(cfg, &registry)?;
//! // hand `tool` to `ToolRegistry::register(...)` in main.rs.
//! # let _ = tool;
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod config;
mod create_issue;
mod error;
mod metrics;

pub use config::GitHubConfig;
pub use create_issue::{GitHubCreateIssueInput, GitHubCreateIssueOutput, GitHubCreateIssueTool};
pub use error::GitHubError;
