//! `TagQuery` → ClickHouse predicate (T8b).
//!
//! Produces a parameterised WHERE-clause fragment that uses **only**
//! `<column>['k'] = ?`, `NOT`, `AND`, `OR`. No `mapContains` with
//! nesting, no `JSONExtract`, no `LIKE`. The bloom-filter skip index on
//! `Map(String, String)` prunes equality predicates only.

pub use crate::compile_pg::SqlFragment;
use crate::query::TagQuery;
use crate::set::{tag_value_to_ch_string, TagValue};

/// Options for [`compile_to_ch`].
#[derive(Clone, Debug)]
pub struct ChCompileOptions<'a> {
    /// Column name (e.g. `"tags"`) holding the `Map(String, String)`.
    pub column: &'a str,
    /// First placeholder number — the compiler emits `$1`, `$2`, …
    /// starting at `first_bind`.
    pub first_bind: usize,
}

/// Compile a [`TagQuery`] to a ClickHouse `WHERE`-clause fragment.
pub fn compile_to_ch(q: &TagQuery, opts: ChCompileOptions<'_>) -> SqlFragment {
    let mut binds: Vec<serde_json::Value> = Vec::new();
    let sql = render(q, opts.column, opts.first_bind, &mut binds);
    SqlFragment { sql, binds }
}

fn render(q: &TagQuery, col: &str, first: usize, binds: &mut Vec<serde_json::Value>) -> String {
    match q {
        TagQuery::Has(k) => emit_eq(col, k, &TagValue::Bool(true), first, binds),
        TagQuery::Eq(k, v) => emit_eq(col, k, v, first, binds),
        TagQuery::And(xs) => join(xs, "AND", col, first, binds),
        TagQuery::Or(xs) => join(xs, "OR", col, first, binds),
        TagQuery::Not(x) => {
            let inner = render(x, col, first, binds);
            format!("(NOT {inner})")
        }
    }
}

fn emit_eq(
    col: &str,
    key: &str,
    value: &TagValue,
    first: usize,
    binds: &mut Vec<serde_json::Value>,
) -> String {
    let key_idx = first + binds.len();
    binds.push(serde_json::Value::String(key.to_owned()));
    let val_idx = first + binds.len();
    // tag_value_to_ch_string is THE single conversion (T2).
    binds.push(serde_json::Value::String(tag_value_to_ch_string(value)));
    format!("({col}[${key_idx}] = ${val_idx})")
}

fn join(
    xs: &[TagQuery],
    op: &str,
    col: &str,
    first: usize,
    binds: &mut Vec<serde_json::Value>,
) -> String {
    if xs.is_empty() {
        return if op == "AND" {
            "(1=1)".to_owned()
        } else {
            "(1=0)".to_owned()
        };
    }
    let mut parts = Vec::with_capacity(xs.len());
    for x in xs {
        parts.push(render(x, col, first, binds));
    }
    format!("({})", parts.join(&format!(" {op} ")))
}
