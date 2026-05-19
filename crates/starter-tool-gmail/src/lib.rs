//! # starter-tool-gmail
//!
//! Outbound Gmail integration as a [`Tool`](starter_spi::tool::Tool).
//! Wraps the REST
//! [`users.messages.send`](https://developers.google.com/gmail/api/reference/rest/v1/users.messages/send)
//! endpoint so the consumer can send an email — directly from
//! REST/CLI code or via MCP if the tool is registered into a
//! [`ToolRegistry`].
//!
//! ## What this crate is, and is not
//!
//! v0.1 of `starter-tool-gmail` is **send-only**. Inbound Gmail
//! (`users.watch` + Cloud Pub/Sub, or `history.list` long-poll) is
//! explicitly deferred per the source SCOPE one-line summary on
//! Gmail; there is no companion `starter-service-gmail` yet. That
//! gap is by design — landing send-only first keeps the
//! authentication surface narrow (one bearer token) so the
//! integration can ship before `starter-auth-oauth` lands at Phase 6
//! of the wider plan.
//!
//! ## Authentication
//!
//! `users.messages.send` is a `Authorization: Bearer <token>` call
//! over HTTPS. **This crate does not acquire tokens.** Token
//! acquisition (interactive consent, refresh, service-account
//! exchange) lives in the consumer's `main.rs` — usually behind
//! `starter-auth-oauth` (reserved at Phase 6) or a custom flow.
//! [`GmailConfig::oauth_access_token`] takes an already-resolved
//! [`SecretString`](starter_spi::SecretString) and the crate uses it
//! once per call via [`ExposeSecret`](starter_spi::ExposeSecret).
//! Token refresh on 401 is the consumer's responsibility today; the
//! tool surfaces a 401 as
//! [`starter_spi::Error::Unauthenticated`] so a wrapper can trigger
//! it.
//!
//! ## SCOPE rules this crate honours
//!
//! - **R1** — one integration per crate. Depending on
//!   `starter-tool-gmail` compiles only the Gmail outbound surface;
//!   it pulls no Slack, Telegram, or generic-OAuth code.
//! - **R4** — provider crates do not own domain logic. This crate
//!   knows how to build an RFC 5322 message and POST it to Gmail; it
//!   does not know who the consumer wants to email.
//! - **R5** — credentials come in as
//!   [`SecretString`](starter_spi::SecretString). The crate does not
//!   read env vars or files; the consumer resolves the access token
//!   and hands it in via [`GmailConfig`].
//! - **R7** — observability is required. [`GmailSendTool::new`]
//!   takes the same [`prometheus::Registry`] the consumer hands to
//!   `McpHttpOptions`, registers `starter_tool_gmail_*` metrics on
//!   it, and emits a structured `tracing` event per invocation
//!   carrying a stable `tool.name` field.
//! - **R8** — no transitive vendor SDKs in `starter-spi`. The only
//!   starter-side deps here are `starter-spi` and
//!   `starter-observability`; everything else is `reqwest` + `serde`
//!   + `base64` + `prometheus`. No `google-*` SDK is involved.
//!
//! Implementation mined from
//! `codeless/crates/codeless-tools/src/email/{gmail,message}.rs` per
//! the source SCOPE: the `users.messages.send` call shape and the
//! RFC 5322 builder are lifted verbatim; the `Tool` framing
//! (`ToolDefinition`, `invoke -> SpiResult<Value>`), the prometheus
//! surface, and the input/output types are starter's.
//!
//! ## Quick start
//!
//! ```no_run
//! use std::sync::Arc;
//! use prometheus::Registry;
//! use starter_spi::SecretString;
//! use starter_tool_gmail::{GmailConfig, GmailSendTool};
//!
//! # async fn ex() -> Result<(), Box<dyn std::error::Error>> {
//! let registry = Arc::new(Registry::new());
//! let cfg = GmailConfig {
//!     oauth_access_token: SecretString::from("ya29.…".to_string()),
//!     user_id:            GmailConfig::default_user_id(),
//!     base_url:           GmailConfig::default_base_url(),
//! };
//! let tool = GmailSendTool::new(cfg, &registry)?;
//! // hand `tool` to `ToolRegistry::register(...)` in main.rs.
//! # let _ = tool;
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod config;
mod error;
mod message;
mod metrics;
mod send;

pub use config::GmailConfig;
pub use error::GmailError;
pub use message::{GmailMailbox, GmailMessage, MessageError};
pub use send::{GmailSendInput, GmailSendOutput, GmailSendTool};
