//! The SQL-dialect seam for time-bucket macros.
//!
//! Only Postgres exists today, but `$__timeGroup` renders differently per
//! database (Postgres `date_bin`, ClickHouse `toStartOfInterval`, …). Keeping a
//! tiny [`Dialect`] trait means a WS-08 connector adds its own bucket syntax
//! without touching the scanner. The trait emits *text* (a vetted fragment), so
//! every implementation must keep its output free of caller-controlled input —
//! the bucket width arrives as an already-validated [`std::time::Duration`].

use std::time::Duration;

/// Renders the dialect-specific fragments the binder needs. Implementations
/// produce only fixed SQL keywords plus the validated identifier/duration the
/// scanner hands them — never raw caller text.
pub trait Dialect: Send + Sync {
    /// A time-bucket expression grouping `column` into `width`-wide buckets.
    /// `column` is an already-validated identifier; `width` is server-derived.
    /// The result is recorded as a validated fragment by the caller.
    fn time_group(&self, column: &str, width: Duration) -> String;
}

/// Postgres dialect: `date_bin` for fixed-width time bucketing.
#[derive(Debug, Clone, Copy, Default)]
pub struct Postgres;

impl Dialect for Postgres {
    fn time_group(&self, column: &str, width: Duration) -> String {
        // `date_bin('<n> seconds', col, 'epoch')` snaps `col` to fixed buckets
        // anchored at the Unix epoch. The interval literal is built from a
        // server-derived integer, so it carries no caller input; the epoch
        // anchor keeps buckets stable across queries (and so cache-aligned).
        let secs = width.as_secs().max(1);
        format!("date_bin('{secs} seconds', {column}, TIMESTAMPTZ 'epoch')")
    }
}
