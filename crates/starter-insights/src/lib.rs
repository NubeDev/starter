//! # starter-insights
//!
//! The Insights capability — one crate per Insights SCOPE R-ins-1.
//!
//! Phase 1 (this stage) ships:
//!
//! - [`registry::RuleRegistry`] — `(namespace, name, major)`-keyed
//!   rule registry; rejects duplicate `(namespace, name, major)`.
//! - [`registry::QualityFlagRegistry`] — same shape for the
//!   extensible R-ins-11 quality-flag taxonomy.
//! - [`nodes::rule_rust`] — `starter.flow.rule.rust` node body that
//!   dispatches a registered [`starter_spi::insights::Rule`] by
//!   `RuleId` and converts panics / missing inputs into
//!   `Severity::Error` verdicts (R-ins-6).
//! - [`nodes::verdict_join`] — `starter.flow.verdict.join` node
//!   body implementing `all` / `any` / `weighted` modes plus the
//!   all-Error degenerate case.
//! - [`prelude`] — re-exports for consumers.
//! - [`sqlite`] (feature `sqlite`) — verdict log + tag index over
//!   SQLite.
//!
//! Future phases add: `rule.sql`, `rule.rhai`, `rule.derive`,
//! `rule.ai-check`, `rule.ai-debug`, the Rhai sandbox (R-ins-4),
//! windowing nodes, `align`, derivation cache, verdict rollups,
//! skill bundles, and `StreamingDatasetRows`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod nodes;
pub mod prelude;
pub mod registry;

#[cfg(feature = "sqlite")]
pub mod sqlite;

pub use registry::{QualityFlagRegistry, RuleRegistry};
