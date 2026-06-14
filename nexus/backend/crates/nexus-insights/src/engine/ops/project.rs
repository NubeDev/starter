//! Column selection and renaming — projections that never add rows.

use super::{from_frame, quote_ident};
use crate::engine::frame::Frame;
use crate::error::{InsightError, InsightResult};

impl Frame {
    /// Keep only `cols`, in the given order. An unknown column is a runtime error
    /// so a typo fails loudly rather than silently dropping data.
    pub fn select(&self, cols: &[String]) -> InsightResult<Frame> {
        if cols.is_empty() {
            return Err(InsightError::Runtime(
                "select needs at least one column".into(),
            ));
        }
        let mut quoted = Vec::with_capacity(cols.len());
        for col in cols {
            self.require_column(col)?;
            quoted.push(quote_ident(col)?);
        }
        self.query(&format!("SELECT {} {}", quoted.join(", "), from_frame()))
    }

    /// Rename column `from` to `to`, leaving every other column untouched and in
    /// place. `from` must exist; `to` must be a valid new identifier.
    pub fn rename(&self, from: &str, to: &str) -> InsightResult<Frame> {
        self.require_column(from)?;
        let to_q = quote_ident(to)?;
        let projection: Vec<String> = self
            .columns()
            .iter()
            .map(|c| {
                let q = quote_ident(c)?;
                if c == from {
                    Ok(format!("{q} AS {to_q}"))
                } else {
                    Ok(q)
                }
            })
            .collect::<InsightResult<_>>()?;
        self.query(&format!(
            "SELECT {} {}",
            projection.join(", "),
            from_frame()
        ))
    }
}
