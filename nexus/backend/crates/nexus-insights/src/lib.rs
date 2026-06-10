//! Sandboxed post-query insight compute.
//!
//! Dashboard insights need more than SQL — rolling windows with custom logic,
//! anomaly scoring, reshaping. This crate is the post-query stage: a tenant Rhai
//! script *orchestrates*, and a curated DataFusion-backed surface *computes*.
//! User code never loops over rows; it composes vetted vectorized primitives
//! (`resample`, `zscore`, `anomalies`, …) that the engine guarantees can never
//! increase the row count. That split is what makes the stage both fast and
//! sandboxable: the Rhai engine caps operations/depth/size/wall-clock and removes
//! file/network/`import`/`eval`, and the curated surface removes any way to
//! explode a result behind the script's back.
//!
//! Entry points: [`run_insight_rows`] for the JSON-rows boundary the query path
//! uses, and [`run_insight`] for callers holding an Arrow [`Frame`].

mod api;
mod engine;
mod error;
mod limits;
mod run;
mod sandbox;

pub use engine::Frame;
pub use error::{InsightError, InsightResult};
pub use limits::Limits;
pub use run::{compile_check, run_insight, run_insight_rows, run_insight_rows_with_limits};
