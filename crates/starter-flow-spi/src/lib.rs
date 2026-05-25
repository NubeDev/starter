//! # starter-flow-spi
//!
//! Contracts crate for the `starter-flow` engine. Per
//! `DOCS/flow/scope/SCOPE.md`'s "What lands in `starter-flow-spi`"
//! block: traits, ids, value enums, and event types — no runtime
//! logic, no I/O, no tokio runtime in the dep tree beyond what
//! `starter-spi` already pulls.
//!
//! Every other `starter-flow*` crate depends on this one; this one
//! depends only on `starter-spi`. Same posture as `starter-spi` has
//! with the rest of the workspace.
//!
//! `#[non_exhaustive]` is applied to every public enum and every
//! public config struct per the SCOPE "What lands in
//! `starter-flow-spi`" block — adding a variant or a field is not a
//! breaking change.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod agent_session;
pub mod ai_runner;
pub mod definition;
pub mod event_dto;
pub mod flow;
pub mod graph;
pub mod node;
pub mod settings;
pub mod skill;
pub mod state;

/// Re-export of [`starter_spi::ai::Cancel`].
///
/// SCOPE rule R13: cancellation across the flow engine reuses the
/// existing `Cancel` seam. Adapters flip this on client disconnect; the
/// propagator stops scheduling; in-flight `ai-agent` nodes cancel their
/// `AiRunner` calls. No new cancellation primitive.
pub use starter_spi::ai::Cancel;

/// Re-export of [`starter_spi::auth::Principal`].
///
/// SCOPE rule R3: nodes carry an `auth` config slot whose value is the
/// [`Principal`] required to invoke them. The adapter on the boundary
/// (REST, MCP, CLI, JSON-RPC) applies the check before the engine sees
/// the call — "adapters apply auth, not extensions".
pub use starter_spi::auth::Principal;

/// Re-export of [`starter_spi::SecretString`].
///
/// SCOPE R5 (inherited from `starter-spi`): node-kind bodies that need
/// credentials take them as [`SecretString`], never as plaintext. The
/// `secrecy` crate name appears in exactly one workspace `Cargo.toml`
/// (`starter-spi`); every other crate, including this one, re-exports.
pub use starter_spi::SecretString;
