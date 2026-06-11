//! Row filters — predicates that can only ever reduce the row count.
//!
//! The comparison value is bound as a numeric or string literal we format, never
//! the script's raw text, so a filter cannot inject SQL through its value.

use super::{from_frame, quote_ident};
use crate::engine::frame::Frame;
use crate::error::InsightResult;

/// A scalar comparison value a filter accepts: a number formatted as a literal,
/// or a string single-quoted with internal quotes doubled.
pub enum FilterValue {
    /// A numeric bound, formatted with full `f64` precision.
    Num(f64),
    /// A string bound, escaped into a SQL string literal.
    Str(String),
}

impl FilterValue {
    /// Render as a SQL literal. A string doubles any embedded single quote so the
    /// literal stays closed.
    fn to_sql(&self) -> String {
        match self {
            FilterValue::Num(n) => format!("{n}"),
            FilterValue::Str(s) => format!("'{}'", s.replace('\'', "''")),
        }
    }
}

impl Frame {
    /// Keep rows where `col > value`.
    pub fn filter_gt(&self, col: &str, value: FilterValue) -> InsightResult<Frame> {
        self.filter_cmp(col, ">", value)
    }

    /// Keep rows where `col < value`.
    pub fn filter_lt(&self, col: &str, value: FilterValue) -> InsightResult<Frame> {
        self.filter_cmp(col, "<", value)
    }

    /// Keep rows where `col = value`.
    pub fn filter_eq(&self, col: &str, value: FilterValue) -> InsightResult<Frame> {
        self.filter_cmp(col, "=", value)
    }

    /// Shared comparison filter: validate the column, quote it, format the literal,
    /// and lower to a `WHERE`.
    fn filter_cmp(&self, col: &str, op: &str, value: FilterValue) -> InsightResult<Frame> {
        self.require_column(col)?;
        let q = quote_ident(col)?;
        self.query(&format!(
            "SELECT * {} WHERE {q} {op} {}",
            from_frame(),
            value.to_sql()
        ))
    }
}
