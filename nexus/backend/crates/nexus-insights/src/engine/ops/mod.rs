//! The vectorized primitives, grouped by shape of transform. Each is a method on
//! [`Frame`] that lowers to exactly one SQL statement over the `frame` table.
//!
//! Identifier safety: a column or alias supplied by a script is validated
//! against the live schema (or, for a new alias, checked to be a plain
//! identifier) and then double-quoted before it reaches SQL. Numeric arguments
//! are formatted by us. No part of the lowered SQL is ever the script's raw text,
//! so a column name can neither inject nor reference a second table.

mod anomaly;
mod filter;
mod project;
mod resample;
mod shape;
mod window;

pub use filter::FilterValue;
pub use resample::Agg;

use crate::error::{InsightError, InsightResult};

/// The table name primitives read from — re-exported so each op file builds the
/// same `FROM` clause.
use super::run_sql::TABLE;

/// Double-quote an identifier for SQL, rejecting an embedded quote so the result
/// is always a single safe identifier. Column names come pre-validated against
/// the schema; this guards the rename/alias case where the script names a *new*
/// identifier that has no schema entry to check against.
pub(super) fn quote_ident(name: &str) -> InsightResult<String> {
    if name.is_empty() || name.contains('"') {
        return Err(InsightError::Runtime(format!(
            "invalid identifier: {name:?}"
        )));
    }
    Ok(format!("\"{name}\""))
}

/// A `SELECT * FROM frame` prefix that ops wrap or extend. Centralised so the
/// table name lives in one place.
pub(super) fn from_frame() -> String {
    format!("FROM {TABLE}")
}
