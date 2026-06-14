//! Setup / Automation Builder — run service and YAML import/export.
//!
//! Composes [`starter_flow`] (engine, checkpoints, events) with the
//! [`starter_setup_spi`] domain. See `DOCS/setup-automation-builder.md`.
//!
//! Phase map:
//! - [`import`] — YAML envelope → validated [`Template`] (P0, DOCS §6).
//! - [`service`] — run service: validate input, seed trusted identity
//!   slots, launch, project progress, resume (P1/P1a, DOCS §7–§9).

pub mod authz;
pub mod extension;
pub mod import;
pub mod service;

#[cfg(feature = "rest")]
pub mod rest;

#[cfg(feature = "mcp")]
pub mod mcp;

pub use import::{slot_node, validate_bindings};
pub use service::{RunService, RunServiceConfig, SetupEngine};

pub use starter_setup_spi::{
    InputBinding, OutputBinding, Progress, SemVer, SetupError, SetupResult, SetupRun,
    SetupRunStatus, Template, TemplateAccess, TemplateId, TemplateSource, TemplateSummary,
};
