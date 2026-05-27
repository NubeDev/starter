//! Rubix warehouse verbs — Timescale-backed.
//!
//! PR #44 deleted the previous ClickHouse-backed `rubix.warehouse.*`
//! verbs. The producer + cleaner + rollup flows still reference the
//! verb names, so the absence shows up as a per-tick WARN
//! ("tool_id not registered") and the SDUI `analytics_template`
//! chart sources resolve to empty.
//!
//! This module rebuilds the minimum needed for the bundled
//! `data-flow-site-a` dashboard to render live numbers:
//!
//! - `rubix.warehouse.ingest` — append synth meter readings into the
//!   Timescale `samples` hypertable.
//!
//! The other verbs (`clean_minute`, `rollup_15m`, `mart.create`, …)
//! remain absent; the analytics bridge queries raw `samples` via
//! Timescale's `time_bucket()` so no rollup table is needed.

pub mod ingest;
