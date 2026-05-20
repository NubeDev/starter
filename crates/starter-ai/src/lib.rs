//! # starter-ai
//!
//! Unified AI provider runner. Lifted from
//! `codeless-workspace/ai-runner` per SCOPE q7 — the source of truth
//! is now this crate. Types and trait shape live in `starter-spi`
//! (`starter_spi::ai::*`); this crate ships concrete provider impls
//! and a [`Registry`].
//!
//! ## Transports
//!
//! - **CLI subprocess** — `claude-wrapper` (Anthropic Claude), `codex`
//!   (OpenAI), `copilot` (GitHub). Auth managed by the binary itself
//!   or via well-known env vars (`OPENAI_API_KEY` for codex).
//! - **REST HTTP** — `anthropic-ai-sdk`, `async-openai`. API key
//!   passed via [`RestCfg::api_key`](starter_spi::ai::RestCfg) or read
//!   from the standard provider env var as a fallback.
//!
//! ## Provider feature flags
//!
//! Every provider lives behind its own cargo feature, all `default-off`:
//!
//! | Feature | Provider | Transport | Source dep |
//! |---|---|---|---|
//! | `provider-claude` | Claude Code | CLI | `claude-wrapper` (pinned `=0.5.1`) |
//! | `provider-codex` | OpenAI Codex | CLI | `tokio::process` |
//! | `provider-copilot` | GitHub Copilot | CLI | `tokio::process` |
//! | `provider-anthropic` | Anthropic | REST | `anthropic-ai-sdk` |
//! | `provider-openai` | OpenAI | REST | `async-openai` |
//!
//! A binary that needs only Anthropic REST adds
//! `features = ["provider-anthropic"]`; the OpenAI SDK and the three
//! CLI wrappers do not appear in its dependency graph. This is the
//! "headless appliance" guarantee (SCOPE 735–741).
//!
//! ## `claude-wrapper` pin
//!
//! `claude-wrapper`'s `=0.5.1` pin is intentional: the wrapper parses
//! the `claude` binary's stream-json output, which Anthropic does not
//! promise to keep stable across releases. A separate canary CI repo
//! tracks upstream drift (recommendation per SCOPE q6: a separate
//! `starter-ai-canary` repo so a green canary run does not gate normal
//! CI). The canary's home is documented but not yet provisioned.
//!
//! ## Cancellation
//!
//! [`TokenCancel`] wraps a `tokio_util::sync::CancellationToken` and
//! implements `starter_spi::ai::Cancel`. CLI runners spawn their child
//! with `kill_on_drop(true)`; REST runners select against
//! `cancel.cancelled().await` and tear down the HTTP body on cancel.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod cancel;
mod defaults;
mod registry;
mod secret;

pub mod runners;

/// Phase 4 D-F4.11: recording-fake `AiRunner` for smoke tests.
/// Gated behind the `testing` cargo feature so production binaries
/// never carry the fake.
#[cfg(feature = "testing")]
pub mod testing;

pub use cancel::TokenCancel;
pub use defaults::AiDefaults;
pub use registry::{ProviderStatus, Registry};
pub use secret::api_key_for;
