//! The vectorized compute backend: a `Frame` is a handle over Arrow batches that
//! a script transforms by composing curated DataFusion expressions.
//!
//! DataFusion (not Polars) is the engine — see the RW-06 session log for the
//! dep-cost decision. A `Frame` carries the current `Vec<RecordBatch>` plus a
//! shared `SessionContext`; each primitive registers the batches as a one-shot
//! table, applies one expression (projection / window / aggregate / sort /
//! limit), and collects the result back into a new `Frame`. No primitive may
//! increase the row count beyond its input: the surface has no join, and the
//! aggregate/resample primitives only ever reduce. That invariant is what lets
//! the Rhai size limits stay meaningful — the engine can never explode behind
//! the script's back.

mod convert;
mod frame;
mod ops;
mod run_sql;

pub use convert::{batches_to_rows, rows_to_frame};
pub use frame::Frame;
pub use ops::{Agg, FilterValue};
