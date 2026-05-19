//! # starter-ext-cli — Adapter Phase 6 (SCOPE R13)
//!
//! Surfaces every `contributes.cli` entry in the manifest as a
//! [`clap::Command`] impl wired into [`starter_cli::CommandRegistry`].
//! The consumer's binary then `register`s the adapter-produced commands
//! exactly like a starter-shipped one, so an extension subcommand is
//! indistinguishable from a built-in one at the help text level.
//!
//! Two response shapes (chosen per entry in the manifest's
//! `streaming:` field):
//!
//! - **`streaming: none`** — single JSON document printed on stdout when
//!   the handler returns. Errors print on stderr and the process exits
//!   non-zero.
//! - **`streaming: stdout`** — the extension's `Stream<Item = Event>` is
//!   rendered as newline-delimited JSON (one event per line), flushed
//!   as each event arrives. The adapter installs a `SIGINT` handler;
//!   the first `Ctrl-C` from the terminal fires
//!   [`dispatcher::CancelHandle`], which the kernel forwards to the
//!   extension as a `stream.cancel` notification (SCOPE post-R13).
//!   A second `SIGINT` exits the process immediately.
//!
//! Two dispatch flavours ship in v0.1:
//!
//! - [`BuiltinCliDispatcher`] — host populates a
//!   [`BuiltinCliRegistry`] with one closure per `(extension, cli_id)`
//!   at startup (the proc-macro-generated handlers cover
//!   `contributes.tools`; CLI handlers are registered separately for
//!   v0.1 to avoid widening the per-extension trait surface — see
//!   `examples/hello-cli`). Calls run in-process; no JSON-RPC frame is
//!   ever serialised.
//! - [`ProcessCliDispatcher`] / [`WasmCliDispatcher`] — return
//!   `DispatchError::NotWired`. Both ship the configurable
//!   `request_timeout: Duration` knob so the wiring shape is uniform
//!   when the synchronous JSON-RPC dispatch slice lands in the next
//!   iteration. The trait method takes the timeout so a hand-rolled
//!   dispatcher can plug in without changing the call sites.
//!
//! Matching pattern from `starter-ext-server::rest`: the v0.1 adapter
//! ships builtin end-to-end; process / wasm hang behind the same trait
//! and land additively.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod adapter;
mod command;
mod dispatcher;

pub use adapter::{build_cli_commands, BuildCliError};
pub use command::ExtensionSubcommand;
pub use dispatcher::{
    BuiltinCliDispatcher, BuiltinCliRegistry, CancelHandle, CliDispatcher, CliHandler,
    CliStreamingHandler, DispatchError, NotWiredCliDispatcher, ProcessCliDispatcher,
    StreamResponse, WasmCliDispatcher, DEFAULT_REQUEST_TIMEOUT,
};

// Re-export the event type so callers building handlers don't need to
// thread `starter_ext_sdk::ctx` themselves.
pub use starter_ext_sdk::ctx::Event as StreamEvent;
