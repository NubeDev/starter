//! `TagQuery` → Postgres predicate (T8a).
//!
//! Produces a parameterised WHERE-clause fragment that uses **only**
//! `<column> @> '…'::jsonb`, `NOT`, `AND`, `OR`. No `->>`, no array
//! operators, no `jsonb_path_query`. This is what keeps the GIN
//! `jsonb_path_ops` index efficient.

use serde_json::json;

use crate::query::TagQuery;
use crate::set::TagValue;

/// Options for [`compile_to_pg`].
#[derive(Clone, Debug)]
pub struct PgCompileOptions<'a> {
    /// Column name (e.g. `"tags"`) holding the JSONB tag bag.
    pub column: &'a str,
    /// First placeholder number — the compiler emits `$first_bind`,
    /// `$first_bind+1`, etc. so the caller can splice the fragment
    /// inside an existing prepared statement.
    pub first_bind: usize,
}

/// SQL fragment produced by [`compile_to_pg`] / [`crate::compile_ch::compile_to_ch`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlFragment {
    pub sql: String,
    pub binds: Vec<serde_json::Value>,
}

/// Compile a [`TagQuery`] to a Postgres `WHERE`-clause fragment.
pub fn compile_to_pg(q: &TagQuery, opts: PgCompileOptions<'_>) -> SqlFragment {
    let mut binds: Vec<serde_json::Value> = Vec::new();
    let sql = render(q, opts.column, opts.first_bind, &mut binds);
    SqlFragment { sql, binds }
}

fn render(q: &TagQuery, col: &str, first: usize, binds: &mut Vec<serde_json::Value>) -> String {
    match q {
        TagQuery::Has(k) => render_eq(col, k, &json!(true), first, binds),
        TagQuery::Eq(k, v) => {
            let jv = match v {
                TagValue::Bool(b) => json!(b),
                TagValue::Str(s) => json!(s),
            };
            render_eq(col, k, &jv, first, binds)
        }
        TagQuery::And(xs) => join(xs, "AND", col, first, binds),
        TagQuery::Or(xs) => join(xs, "OR", col, first, binds),
        TagQuery::Not(x) => {
            let inner = render(x, col, first, binds);
            format!("(NOT {inner})")
        }
    }
}

fn render_eq(
    col: &str,
    key: &str,
    value: &serde_json::Value,
    first: usize,
    binds: &mut Vec<serde_json::Value>,
) -> String {
    let bind_idx = first + binds.len();
    let mut obj = serde_json::Map::new();
    obj.insert(key.to_owned(), value.clone());
    binds.push(serde_json::Value::Object(obj));
    format!("({col} @> ${bind_idx}::jsonb)")
}

fn join(
    xs: &[TagQuery],
    op: &str,
    col: &str,
    first: usize,
    binds: &mut Vec<serde_json::Value>,
) -> String {
    if xs.is_empty() {
        // empty AND ≡ TRUE, empty OR ≡ FALSE. Defensive.
        return if op == "AND" {
            "(TRUE)".to_owned()
        } else {
            "(FALSE)".to_owned()
        };
    }
    let mut parts = Vec::with_capacity(xs.len());
    for x in xs {
        parts.push(render(x, col, first, binds));
    }
    format!("({})", parts.join(&format!(" {op} ")))
}
