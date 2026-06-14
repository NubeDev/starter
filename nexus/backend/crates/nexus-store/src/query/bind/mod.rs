//! The query binder (C2): rewrite macro/variable SQL into a prepared query.
//!
//! `bind()` is the project's single injection + tenant-isolation boundary. It is
//! **one engine with two front doors** — raw-SQL macros (`$__timeFilter`, `$var`)
//! and kind named-param binding (`$param`, host tokens) both flow through it —
//! and it returns a [`BoundQuery`] (placeholders + bound args + the vetted
//! identifiers it inserted), never a finished SQL string. The runner executes
//! that as a prepared statement, so values are bound by the driver and can never
//! be concatenated into the query text. See docs/design/query/.

mod bound;
mod context;
mod dialect;
mod error;
mod identifier;
mod scan;
mod time_macros;
mod vars;

pub use bound::{BoundQuery, SqlValue};
pub use context::{BindCtx, HostTokens, ParamValue, ScalarValue, TimeRange, VarValue};
pub use dialect::{Dialect, Postgres};
pub use error::BindError;

/// Rewrite `sql` against `ctx` into a [`BoundQuery`] for the Postgres dialect.
///
/// Raw SQL with no macros, variables, or params binds nothing and passes through
/// unchanged (empty `args`, empty `validated_identifiers`). Any value the query
/// references — time bounds, variable values, `$__sqlIn` elements, kind params,
/// host tokens — becomes a bound `$N` argument; the only text ever inserted is a
/// validated identifier or dialect fragment.
pub fn bind(sql: &str, ctx: &BindCtx) -> Result<BoundQuery, BindError> {
    bind_with(sql, ctx, &Postgres)
}

/// Like [`bind`] but with an explicit [`Dialect`], for WS-08 connectors that
/// render time buckets differently.
pub fn bind_with(sql: &str, ctx: &BindCtx, dialect: &dyn Dialect) -> Result<BoundQuery, BindError> {
    let mut out = BoundQuery::builder();
    scan::scan(sql, ctx, dialect, &mut out)?;
    Ok(out.finish())
}
