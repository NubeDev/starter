//! Windowed primitives: rolling aggregates, lag, diff, pct_change, zscore.
//!
//! Each adds one derived column over the rows in input order and never changes
//! the row count. Row order is fixed by a synthetic ordinal (`__ord`) assigned in
//! scan order over the single-partition in-memory frame, so a rolling window is
//! deterministic without the script having to name an order column. The ordinal
//! is dropped from the output — only the requested derived column is added.

use super::{from_frame, quote_ident};
use crate::engine::frame::Frame;
use crate::error::{InsightError, InsightResult};

/// The synthetic, scan-order ordinal that fixes window order. Never surfaced.
const ORD: &str = "__ord";

impl Frame {
    /// Rolling mean of `col` over the trailing `window` rows (inclusive of the
    /// current row), as a new `<col>_roll_mean` column.
    pub fn rolling_mean(&self, col: &str, window: i64) -> InsightResult<Frame> {
        self.rolling(col, window, "avg", "roll_mean")
    }

    /// Rolling minimum over the trailing `window` rows → `<col>_roll_min`.
    pub fn rolling_min(&self, col: &str, window: i64) -> InsightResult<Frame> {
        self.rolling(col, window, "min", "roll_min")
    }

    /// Rolling maximum over the trailing `window` rows → `<col>_roll_max`.
    pub fn rolling_max(&self, col: &str, window: i64) -> InsightResult<Frame> {
        self.rolling(col, window, "max", "roll_max")
    }

    /// Rolling sum over the trailing `window` rows → `<col>_roll_sum`.
    pub fn rolling_sum(&self, col: &str, window: i64) -> InsightResult<Frame> {
        self.rolling(col, window, "sum", "roll_sum")
    }

    /// `col` shifted down by `n` rows (NULL for the first `n`), as `<col>_lag`.
    pub fn lag(&self, col: &str, n: i64) -> InsightResult<Frame> {
        self.require_column(col)?;
        let positive = require_positive(n, "lag")?;
        let q = quote_ident(col)?;
        let alias = quote_ident(&format!("{col}_lag"))?;
        let expr = format!("lag({q}, {positive}) OVER (ORDER BY {ORD})");
        self.windowed(&format!("{expr} AS {alias}"))
    }

    /// First difference: `col - lag(col, 1)`, as `<col>_diff`.
    pub fn diff(&self, col: &str) -> InsightResult<Frame> {
        self.require_column(col)?;
        let q = quote_ident(col)?;
        let alias = quote_ident(&format!("{col}_diff"))?;
        let expr = format!("({q} - lag({q}, 1) OVER (ORDER BY {ORD}))");
        self.windowed(&format!("{expr} AS {alias}"))
    }

    /// Percent change from the previous row: `(col - prev) / prev`, as
    /// `<col>_pct_change`. A zero previous value yields NULL (no divide-by-zero).
    pub fn pct_change(&self, col: &str) -> InsightResult<Frame> {
        self.require_column(col)?;
        let q = quote_ident(col)?;
        let alias = quote_ident(&format!("{col}_pct_change"))?;
        let prev = format!("lag({q}, 1) OVER (ORDER BY {ORD})");
        let expr = format!("CASE WHEN {prev} = 0 THEN NULL ELSE ({q} - {prev}) / {prev} END");
        self.windowed(&format!("{expr} AS {alias}"))
    }

    /// Standard score of `col` against the whole frame's mean/stddev, as
    /// `<col>_zscore`. A zero stddev (constant column) yields NULL.
    pub fn zscore(&self, col: &str) -> InsightResult<Frame> {
        self.require_column(col)?;
        let q = quote_ident(col)?;
        let alias = quote_ident(&format!("{col}_zscore"))?;
        let mean = format!("avg({q}) OVER ()");
        let sd = format!("stddev_pop({q}) OVER ()");
        let expr = format!("CASE WHEN {sd} = 0 THEN NULL ELSE ({q} - {mean}) / {sd} END");
        self.windowed(&format!("{expr} AS {alias}"))
    }

    /// Shared rolling-aggregate lowering: `<agg>(col) OVER (ORDER BY ord ROWS
    /// BETWEEN window-1 PRECEDING AND CURRENT ROW)`.
    fn rolling(&self, col: &str, window: i64, agg: &str, suffix: &str) -> InsightResult<Frame> {
        self.require_column(col)?;
        let w = require_positive(window, "rolling window")?;
        let q = quote_ident(col)?;
        let alias = quote_ident(&format!("{col}_{suffix}"))?;
        let preceding = w - 1;
        let expr = format!(
            "{agg}({q}) OVER (ORDER BY {ORD} ROWS BETWEEN {preceding} PRECEDING AND CURRENT ROW)"
        );
        self.windowed(&format!("{expr} AS {alias}"))
    }

    /// Append one window expression to every existing column, ordered by the
    /// synthetic ordinal and then dropping that ordinal from the result.
    fn windowed(&self, derived: &str) -> InsightResult<Frame> {
        let existing: Vec<String> = self
            .columns()
            .iter()
            .map(|c| quote_ident(c))
            .collect::<InsightResult<_>>()?;
        let with_ord = format!(
            "WITH t AS (SELECT *, row_number() OVER () AS {ORD} {}) ",
            from_frame()
        );
        // The outer select re-projects the original columns plus the derived one;
        // the ordinal stays only inside the CTE.
        let sql = format!(
            "{with_ord}SELECT {}, {derived} FROM t ORDER BY {ORD}",
            existing.join(", ")
        );
        self.query(&sql)
    }
}

/// Window sizes / lags must be positive — a zero or negative value is a script
/// error, not a silently-empty window.
fn require_positive(n: i64, what: &str) -> InsightResult<i64> {
    if n < 1 {
        return Err(InsightError::Runtime(format!(
            "{what} must be >= 1, got {n}"
        )));
    }
    Ok(n)
}
