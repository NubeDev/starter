//! Anomaly flagging: mark rows whose `col` deviates more than `z_threshold`
//! standard deviations from the column mean. Adds one boolean column; the row
//! count is unchanged.

use super::{from_frame, quote_ident};
use crate::engine::frame::Frame;
use crate::error::{InsightError, InsightResult};

impl Frame {
    /// Add a boolean `<col>_anomaly` column, true where `abs(zscore(col)) >
    /// z_threshold`. A non-positive threshold is a script error. A constant column
    /// (zero stddev) flags nothing rather than dividing by zero.
    pub fn anomalies(&self, col: &str, z_threshold: f64) -> InsightResult<Frame> {
        self.require_column(col)?;
        if !(z_threshold > 0.0) {
            return Err(InsightError::Runtime(
                "anomaly z_threshold must be > 0".into(),
            ));
        }
        let q = quote_ident(col)?;
        let alias = quote_ident(&format!("{col}_anomaly"))?;
        let mean = format!("avg({q}) OVER ()");
        let sd = format!("stddev_pop({q}) OVER ()");
        let flag = format!(
            "CASE WHEN {sd} = 0 THEN false \
             ELSE abs(({q} - {mean}) / {sd}) > {z_threshold} END"
        );
        let existing: Vec<String> = self
            .columns()
            .iter()
            .map(|c| quote_ident(c))
            .collect::<InsightResult<_>>()?;
        self.query(&format!(
            "SELECT {}, {flag} AS {alias} {}",
            existing.join(", "),
            from_frame()
        ))
    }
}
