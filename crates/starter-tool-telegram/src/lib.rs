//! # starter-tool-telegram
//!
//! Outbound Telegram integration as a [`Tool`](starter_spi::tool::Tool).
//! Wraps the Bot API
//! [`sendMessage`](https://core.telegram.org/bots/api#sendmessage)
//! method so the consumer can post a message into a chat — directly
//! from REST/CLI code or via MCP if the tool is registered into a
//! [`ToolRegistry`].
//!
//! ## SCOPE rules this crate honours
//!
//! - **R1** — one integration per crate. There is no `starter-tools`
//!   mega-crate; depending on `starter-tool-telegram` compiles only
//!   the Telegram outbound surface.
//! - **R4** — provider crates do not own domain logic. This crate
//!   knows how to call `sendMessage`; it does not know what the
//!   consumer wants to say.
//! - **R5** — credentials come in as
//!   [`SecretString`](starter_spi::SecretString). The crate does not
//!   read env vars or files; the consumer resolves the bot token and
//!   hands it in via [`TelegramConfig`].
//! - **R7** — observability is required. [`TelegramSendMessageTool::new`]
//!   takes the same [`prometheus::Registry`] the consumer hands to
//!   `McpHttpOptions`, registers `starter_tool_telegram_*` metrics on
//!   it, and emits a structured `tracing` event per invocation
//!   carrying a stable `tool.name` field.
//! - **R8** — no transitive vendor SDKs in `starter-spi`. The only
//!   starter-side deps here are `starter-spi` and
//!   `starter-observability`.
//!
//! Implementation mined from
//! `codeless/crates/codeless-telegram::web_api` per the source SCOPE;
//! the architecture (Tool, Registry, prometheus surface) is starter's,
//! not codeless's.
//!
//! ## Quick start
//!
//! ```no_run
//! use std::sync::Arc;
//! use prometheus::Registry;
//! use starter_spi::SecretString;
//! use starter_tool_telegram::{TelegramConfig, TelegramSendMessageTool};
//!
//! # async fn ex() -> Result<(), Box<dyn std::error::Error>> {
//! let registry = Arc::new(Registry::new());
//! let cfg = TelegramConfig {
//!     bot_token: SecretString::from("12345:abc…".to_string()),
//!     base_url:  TelegramConfig::default_base_url(),
//! };
//! let tool = TelegramSendMessageTool::new(cfg, &registry)?;
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
mod send;

pub use config::TelegramConfig;
pub use error::TelegramError;
pub use send::{TelegramSendMessageInput, TelegramSendMessageOutput, TelegramSendMessageTool};
