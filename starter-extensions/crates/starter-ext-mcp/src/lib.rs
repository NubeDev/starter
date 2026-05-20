//! # starter-ext-mcp
//!
//! The first transport adapter in the R13 set. Reads every validated
//! extension's `contributes.tools[]` block out of an
//! [`starter_ext_host::ExtensionRegistry`], looks up each declared tool
//! in a [`starter_ext_sdk::builtin::BuiltinTable`], and registers a
//! wrapper [`starter_spi::tool::Tool`] implementation into
//! `starter_mcp`'s `ToolRegistry`.
//!
//! Two design rules drive the shape of this crate:
//!
//! - **R3 — manifest is the source of truth.** The adapter walks
//!   `manifest.contributes.tools[]`. The tool's description and input
//!   schema come from the static files the manifest names — never
//!   templated, never re-derived from extension code at runtime. R7's
//!   anti-prompt-injection guarantee depends on this.
//! - **R13 — adapter validation.** Each tool id must be the extension's
//!   id or a dotted descendant (already enforced by
//!   `starter-ext-host`); the adapter additionally refuses to register a
//!   tool whose dispatch closure is missing from the linked
//!   `BuiltinTable`. That mismatch is a host-build error, not a runtime
//!   surprise.
//!
//! Phase 1 only handles builtin-flavour extensions. Process and WASM
//! flavours land alongside their respective host crates
//! (`starter-ext-supervisor`, `starter-ext-wasm`) in later phases; this
//! adapter will gain matching dispatch arms then.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod ctx_stub;
mod register;
mod tool_wrapper;

pub use register::{register_process_tools, register_tools, RegisterError, RegisterOutcome};
pub use tool_wrapper::{ExtensionToolBinding, ProcessExtensionToolBinding};
