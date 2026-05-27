//! # starter-ext-host
//!
//! Manifest loader, two-phase validator, and immutable `ExtensionRegistry`.
//!
//! Per SCOPE.md "What each crate / package owns — `starter-ext-host`":
//!
//! - `Loader::scan(root: &Path)` walks the configured extensions root one
//!   level deep, parses every `<bundle>/block.yaml`, and collects errors
//!   **per-extension without short-circuiting** (a single broken manifest
//!   never poisons the load).
//! - Two-phase commit: [`Loader::validate_all`] runs every check (schema,
//!   R4 namespace ownership, R6 capability compatibility, id uniqueness)
//!   on every candidate before any state lands in the registry;
//!   [`Loader::commit`] then registers all candidates — passing ones at
//!   `Validated`, failing ones at `Failed` — in a single shot.
//! - [`ExtensionRegistry`] is the read-only view a host wires into
//!   adapters. After `commit` the registry is immutable.
//!
//! This crate has **no I/O beyond reading bundle files at load time** and
//! no flavour-specific code. Process spawning (`starter-ext-supervisor`),
//! WASI instantiation (`starter-ext-wasm`), and the per-transport adapters
//! (`starter-ext-mcp`, future REST/CLI/gRPC/UI adapters) live elsewhere
//! and consume the registry types defined here.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod loader;
pub mod record;
pub mod registry;
pub mod validate;
pub mod warehouse;

pub use loader::{Loader, LoaderOutcome};
pub use record::ExtensionRecord;
pub use registry::ExtensionRegistry;
pub use warehouse::TemplateRegistry;
