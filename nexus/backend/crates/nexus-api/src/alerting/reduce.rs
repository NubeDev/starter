//! Reduce a query result set to a single scalar before comparison.
//!
//! A multi-condition rule's condition runs a query that may return many rows;
//! the reducer collapses the first numeric column across those rows to one value
//! the threshold comparison can use. Pure and unit-tested: it takes the already
//! fetched rows (JSON objects from the guarded query path) and never does I/O.

use serde_json::Value;

/// How a condition collapses its query's rows to a single value. `Last` mirrors
/// the legacy single-scalar behaviour (first row, first column), so a rule
/// without an explicit reducer behaves exactly as before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reducer {
    Last,
    Min,
    Max,
    Avg,
    Sum,
    Count,
}

impl Reducer {
    /// Parse the stored string form. An unknown value falls back to `Last` so a
    /// malformed reducer degrades to the legacy behaviour rather than erroring.
    pub fn parse(s: &str) -> Self {
        match s {
            "min" => Reducer::Min,
            "max" => Reducer::Max,
            "avg" => Reducer::Avg,
            "sum" => Reducer::Sum,
            "count" => Reducer::Count,
            _ => Reducer::Last,
        }
    }

    /// The stored string form.
    pub fn as_str(self) -> &'static str {
        match self {
            Reducer::Last => "last",
            Reducer::Min => "min",
            Reducer::Max => "max",
            Reducer::Avg => "avg",
            Reducer::Sum => "sum",
            Reducer::Count => "count",
        }
    }
}

/// Reduce `rows` to a single value per `reducer`. Returns `None` when there is no
/// data to reduce — except `Count`, which is defined on an empty set (zero rows
/// is a meaningful count of 0). The first column of each row is the series; a row
/// whose first cell is non-numeric is skipped (it cannot contribute to min/avg).
pub fn reduce(rows: &[Value], reducer: Reducer) -> Option<f64> {
    if reducer == Reducer::Count {
        return Some(rows.len() as f64);
    }
    let values: Vec<f64> = rows.iter().filter_map(first_numeric_cell).collect();
    match reducer {
        Reducer::Last => values.last().copied(),
        Reducer::Min => values.iter().copied().reduce(f64::min),
        Reducer::Max => values.iter().copied().reduce(f64::max),
        Reducer::Sum => {
            if values.is_empty() {
                None
            } else {
                Some(values.iter().sum())
            }
        }
        Reducer::Avg => {
            if values.is_empty() {
                None
            } else {
                Some(values.iter().sum::<f64>() / values.len() as f64)
            }
        }
        Reducer::Count => unreachable!("handled above"),
    }
}

/// The first column's value of a row as f64, if the row is an object whose first
/// cell is numeric.
fn first_numeric_cell(row: &Value) -> Option<f64> {
    row.as_object()?.values().next()?.as_f64()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rows(vals: &[f64]) -> Vec<Value> {
        vals.iter().map(|v| json!({ "v": v })).collect()
    }

    #[test]
    fn reducers_collapse_the_set() {
        let r = rows(&[2.0, 8.0, 5.0]);
        assert_eq!(reduce(&r, Reducer::Last), Some(5.0));
        assert_eq!(reduce(&r, Reducer::Min), Some(2.0));
        assert_eq!(reduce(&r, Reducer::Max), Some(8.0));
        assert_eq!(reduce(&r, Reducer::Sum), Some(15.0));
        assert_eq!(reduce(&r, Reducer::Avg), Some(5.0));
        assert_eq!(reduce(&r, Reducer::Count), Some(3.0));
    }

    #[test]
    fn empty_set_is_no_data_except_count() {
        let empty: Vec<Value> = Vec::new();
        assert_eq!(reduce(&empty, Reducer::Last), None);
        assert_eq!(reduce(&empty, Reducer::Avg), None);
        assert_eq!(reduce(&empty, Reducer::Sum), None);
        assert_eq!(reduce(&empty, Reducer::Count), Some(0.0));
    }

    #[test]
    fn non_numeric_first_cells_are_skipped() {
        let r = vec![json!({ "v": "x" }), json!({ "v": 4.0 })];
        assert_eq!(reduce(&r, Reducer::Sum), Some(4.0));
        assert_eq!(reduce(&r, Reducer::Last), Some(4.0));
    }

    #[test]
    fn unknown_reducer_parses_to_last() {
        assert_eq!(Reducer::parse("bogus"), Reducer::Last);
        assert_eq!(Reducer::parse("min").as_str(), "min");
    }
}
