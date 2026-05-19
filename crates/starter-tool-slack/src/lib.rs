//! # starter-tool-slack
//!
//! Outbound Slack integration as a [`Tool`](starter_spi::tool::Tool).
//! Wraps Slack's [`chat.postMessage`] Web API method so the consumer
//! can post a message into a channel — directly from REST/CLI code or
//! via MCP if the tool is registered into a [`ToolRegistry`].
//!
//! [`chat.postMessage`]: https://api.slack.com/methods/chat.postMessage
//!
//! ## SCOPE rules this crate honours
//!
//! - **R1** — one integration per crate. There is no `starter-tools`
//!   mega-crate; depending on `starter-tool-slack` compiles only the
//!   Slack outbound surface.
//! - **R4** — provider crates do not own domain logic. This crate
//!   knows how to call `chat.postMessage`; it does not know what the
//!   consumer wants to say.
//! - **R5** — credentials come in as [`SecretString`]. The crate does
//!   not read env vars or files; the consumer resolves the bot token
//!   (and signing secret, for the inbound service) and hands it in
//!   via [`SlackConfig`].
//! - **R7** — observability is required. [`SlackPostTool::new`] takes
//!   the same [`prometheus::Registry`] the consumer hands to
//!   `McpHttpOptions`, registers `starter_tool_slack_*` metrics on it,
//!   and emits a structured `tracing` event per invocation carrying a
//!   stable `tool.name` field.
//! - **R8** — no transitive vendor SDKs in `starter-spi`. The only
//!   starter-side deps here are `starter-spi` and
//!   `starter-observability`.
//!
//! Implementation mined from `codeless/crates/codeless-slack` per the
//! source SCOPE; the architecture (Tool, Registry, prometheus surface)
//! is starter's, not codeless's.
//!
//! ## Quick start
//!
//! ```no_run
//! use std::sync::Arc;
//! use prometheus::Registry;
//! use starter_spi::SecretString;
//! use starter_tool_slack::{SlackConfig, SlackPostTool};
//!
//! # async fn ex() -> Result<(), Box<dyn std::error::Error>> {
//! let registry = Arc::new(Registry::new());
//! let cfg = SlackConfig {
//!     bot_token:      SecretString::from("xoxb-…".to_string()),
//!     signing_secret: SecretString::from("…".to_string()),
//!     base_url:       SlackConfig::default_base_url(),
//! };
//! let tool = SlackPostTool::new(cfg, &registry)?;
//! // hand `tool` to `ToolRegistry::register(...)` in main.rs.
//! # let _ = tool;
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod config;
mod error;
mod metrics;
mod post;

pub use config::SlackConfig;
pub use error::SlackError;
pub use post::{SlackPostInput, SlackPostOutput, SlackPostTool};
