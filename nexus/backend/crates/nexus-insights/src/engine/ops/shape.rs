//! Frame-shaping primitives: head, tail, sort, fill_null, describe. All reduce
//! or preserve the row count (describe collapses to one row).

use super::{from_frame, quote_ident};
use crate::engine::frame::Frame;
use crate::error::{InsightError, InsightResult};

impl Frame {
    /// Keep the first `n` rows in input order.
    pub fn head(&self, n: i64) -> InsightResult<Frame> {
        let n = require_non_negative(n, "head")?;
        self.query(&format!("SELECT * {} LIMIT {n}", from_frame()))
    }

    /// Keep the last `n` rows. Implemented by reversing a scan-order ordinal,
    /// limiting, then restoring order — so it works without an explicit key.
    pub fn tail(&self, n: i64) -> InsightResult<Frame> {
        let n = require_non_negative(n, "tail")?;
        let sql = format!(
            "WITH t AS (SELECT *, row_number() OVER () AS __ord {}) \
             SELECT * EXCLUDE (__ord) FROM \
             (SELECT * FROM t ORDER BY __ord DESC LIMIT {n}) ORDER BY __ord",
            from_frame()
        );
        self.query(&sql)
    }

    /// Sort by `col`. `ascending` picks the direction.
    pub fn sort(&self, col: &str, ascending: bool) -> InsightResult<Frame> {
        self.require_column(col)?;
        let q = quote_ident(col)?;
        let dir = if ascending { "ASC" } else { "DESC" };
        self.query(&format!("SELECT * {} ORDER BY {q} {dir}", from_frame()))
    }

    /// Replace NULLs in `col` per `strategy`: `"zero"` → 0, `"forward"` → last
    /// non-null in scan order, `"mean"` → the column mean. The column is replaced
    /// in place; no rows are added or removed.
    pub fn fill_null(&self, col: &str, strategy: &str) -> InsightResult<Frame> {
        self.require_column(col)?;
        let q = quote_ident(col)?;
        let filled = match strategy {
            "zero" => format!("coalesce({q}, 0)"),
            "mean" => format!("coalesce({q}, avg({q}) OVER ())"),
            "forward" => format!(
                "coalesce({q}, last_value({q} IGNORE NULLS) OVER \
                 (ORDER BY __fn_ord ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW))"
            ),
            other => {
                return Err(InsightError::Runtime(format!(
                    "unknown fill_null strategy: {other}"
                )))
            }
        };
        let projection: Vec<String> = self
            .columns()
            .iter()
            .map(|c| {
                let cq = quote_ident(c)?;
                if c == col {
                    Ok(format!("{filled} AS {cq}"))
                } else {
                    Ok(cq)
                }
            })
            .collect::<InsightResult<_>>()?;
        // forward-fill needs a scan-order ordinal; the others ignore it.
        let sql = format!(
            "WITH t AS (SELECT *, row_number() OVER () AS __fn_ord {}) \
             SELECT {} FROM t ORDER BY __fn_ord",
            from_frame(),
            projection.join(", ")
        );
        self.query(&sql)
    }

    /// Summary statistics per numeric column — count, mean, stddev, min, max — as
    /// a one-row-per-statistic frame (a `statistic` label column plus one column
    /// per input column). Collapses the frame to a fixed small size.
    pub fn describe(&self) -> InsightResult<Frame> {
        let cols = self.columns();
        if cols.is_empty() {
            return Err(InsightError::Runtime("describe on an empty frame".into()));
        }
        let stats = [
            ("count", "count"),
            ("mean", "avg"),
            ("std", "stddev_pop"),
            ("min", "min"),
            ("max", "max"),
        ];
        let mut unions = Vec::with_capacity(stats.len());
        for (label, func) in stats {
            let mut projection = vec![format!("'{label}' AS statistic")];
            for c in &cols {
                let cq = quote_ident(c)?;
                // count is type-agnostic; the rest cast to double so a non-numeric
                // column yields NULL instead of failing the whole describe.
                let expr = if func == "count" {
                    format!("count({cq})")
                } else {
                    format!("{func}(try_cast({cq} AS DOUBLE))")
                };
                projection.push(format!("{expr} AS {cq}"));
            }
            unions.push(format!("SELECT {} {}", projection.join(", "), from_frame()));
        }
        self.query(&unions.join(" UNION ALL "))
    }
}

/// Row counts for head/tail must be non-negative.
fn require_non_negative(n: i64, what: &str) -> InsightResult<i64> {
    if n < 0 {
        return Err(InsightError::Runtime(format!("{what} count must be >= 0")));
    }
    Ok(n)
}
