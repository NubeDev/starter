//! Time resampling: bucket `time_col` into fixed `every` windows and aggregate
//! each named column. Implemented as DataFusion `date_bin` + `GROUP BY`, the path
//! Timescale users expect; it only ever reduces rows (one row per bucket).

use super::quote_ident;
use crate::engine::frame::Frame;
use crate::engine::run_sql::TABLE;
use crate::error::{InsightError, InsightResult};

/// One aggregation in a resample: which column, reduced by which function.
pub struct Agg {
    /// The column to aggregate within each time bucket.
    pub col: String,
    /// The reducer: one of `avg`/`min`/`max`/`sum`/`count`.
    pub func: String,
}

impl Frame {
    /// Bucket `time_col` into `every`-wide windows (an interval literal like
    /// `"1 hour"` or `"15 minutes"`) and apply each [`Agg`], producing one row per
    /// bucket ordered by bucket start. The bucket column is named `bucket`; each
    /// aggregate keeps its source column name.
    pub fn resample(&self, time_col: &str, every: &str, aggs: &[Agg]) -> InsightResult<Frame> {
        self.require_column(time_col)?;
        if aggs.is_empty() {
            return Err(InsightError::Runtime(
                "resample needs at least one aggregate".into(),
            ));
        }
        let interval = interval_literal(every)?;
        let time_q = quote_ident(time_col)?;
        let bucket = format!("date_bin(INTERVAL {interval}, {time_q}) AS bucket");

        let mut projection = vec![bucket];
        for agg in aggs {
            self.require_column(&agg.col)?;
            let func = aggregate_fn(&agg.func)?;
            let col_q = quote_ident(&agg.col)?;
            let alias = quote_ident(&agg.col)?;
            projection.push(format!("{func}({col_q}) AS {alias}"));
        }
        let sql = format!(
            "SELECT {} FROM {TABLE} GROUP BY 1 ORDER BY 1",
            projection.join(", ")
        );
        self.query(&sql)
    }
}

/// Render an interval as a single-quoted literal, rejecting an embedded quote so
/// the `INTERVAL '…'` literal stays closed. DataFusion parses the unit words.
fn interval_literal(every: &str) -> InsightResult<String> {
    if every.is_empty() || every.contains('\'') {
        return Err(InsightError::Runtime(format!(
            "invalid resample interval: {every:?}"
        )));
    }
    Ok(format!("'{every}'"))
}

/// Whitelist the aggregate function so a script cannot name an arbitrary SQL
/// function in the resample.
fn aggregate_fn(func: &str) -> InsightResult<&'static str> {
    match func {
        "avg" | "mean" => Ok("avg"),
        "min" => Ok("min"),
        "max" => Ok("max"),
        "sum" => Ok("sum"),
        "count" => Ok("count"),
        other => Err(InsightError::Runtime(format!(
            "unsupported resample aggregate: {other}"
        ))),
    }
}
