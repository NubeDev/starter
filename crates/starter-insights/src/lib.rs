//! # starter-insights
//!
//! The Insights capability — one crate per Insights SCOPE R-ins-1.
//!
//! Phase 1 shipped: `RuleRegistry`, `QualityFlagRegistry`,
//! `rule.rust`, `verdict.join`, sqlite verdict-log + tag-index.
//!
//! Phase 2 (this stage) adds:
//!
//! - [`rhai_sandbox`] — the R-ins-4 locked Rhai engine profile.
//! - [`nodes::rule_rhai`] — `starter.flow.rule.rhai` body, with D4
//!   anonymous-id derivation for inline scripts.
//! - [`nodes::windowing`] — `starter.flow.window.tumble` and
//!   `starter.flow.window.slide` with mandatory IANA `tz` config.
//! - [`nodes::rule_sql`] (feature `sqlite`) — `starter.flow.rule.sql`
//!   against the host's primary SQLite store (D2 Phase 1 shape).
//! - [`penalty::apply_derivation_penalty`] — engine-side
//!   `confidence_penalty` enforcement for derivation rules (R-ins-6).
//! - [`backfill`] — D3 100k-row cap + `BackfillTruncated` event.
//! - [`rollups`] (feature `sqlite`) — incremental verdict rollups
//!   (tier 2 materialisation), tag-grouped aggregates (R-ins-8),
//!   and the D5 per-window `rollup_invalidation` watermark seam.
//!
//! Future phases add: `rule.derive`, `align`, derivation cache,
//! `rule.ai-check`, `rule.ai-debug`, skill bundles, and
//! `StreamingDatasetRows`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod ai;
pub mod backfill;
pub mod nodes;
pub mod onboarding;
pub mod penalty;
pub mod prelude;
pub mod registry;
pub mod retroactive;
pub mod rhai_sandbox;
pub mod skills;
pub mod streaming;

#[cfg(feature = "sqlite")]
pub mod cache;
#[cfg(feature = "sqlite")]
pub mod rollups;
#[cfg(feature = "sqlite")]
pub mod sqlite;

pub use registry::{QualityFlagRegistry, RuleRegistry};
