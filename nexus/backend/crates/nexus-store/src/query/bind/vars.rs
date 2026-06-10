//! Expand `$var` / `${var:fmt}` / `$__sqlIn(var)` and `$param` references into
//! bound arguments.
//!
//! Every variable and param value becomes a `$N` bound arg — the format
//! suffix (`:csv`, `:singlequote`) only decides how many placeholders and what
//! separators surround them, never whether the value is quoted by hand. A
//! `'); DROP …` value therefore lands inert as one bound string. See
//! docs/design/query/.

use super::bound::{BoundQueryBuilder, SqlValue};
use super::context::{BindCtx, ScalarValue, VarValue};
use super::error::BindError;

/// The interpolation format a `${var:fmt}` site requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarFormat {
    /// `$var` / `${var}` — a single bound placeholder (first value if multi).
    Single,
    /// `${var:csv}` — comma-separated bound placeholders, no surrounding parens.
    Csv,
    /// `${var:singlequote}` — same as csv (values are bound, so the historical
    /// "single-quote each" intent is satisfied by binding, which is safer).
    SingleQuote,
    /// `$__sqlIn(var)` — a parenthesised `($1, $2, …)` list for an `IN (...)`.
    SqlIn,
}

/// Reject a value that smuggles a host token in (it must come from the
/// `Principal`, never the request). A defense-in-depth check at expansion time.
fn reject_host_token(value: &ScalarValue) -> Result<(), BindError> {
    if let ScalarValue::Text(s) = value {
        let lowered = s.to_ascii_lowercase();
        if lowered == "$caller_tenant_id" || lowered == "$caller_user_id" {
            return Err(BindError::HostTokenInInput(s.clone()));
        }
    }
    Ok(())
}

/// Bind one scalar as the next `$N` placeholder.
fn bind_scalar(out: &mut BoundQueryBuilder, value: &ScalarValue) -> Result<(), BindError> {
    reject_host_token(value)?;
    out.push_arg(to_sql(value));
    Ok(())
}

/// Lower a context scalar into a bound [`SqlValue`].
fn to_sql(value: &ScalarValue) -> SqlValue {
    match value {
        ScalarValue::Text(s) => SqlValue::Text(s.clone()),
        ScalarValue::Int(i) => SqlValue::Int(*i),
        ScalarValue::Float(f) => SqlValue::Float(*f),
        ScalarValue::Bool(b) => SqlValue::Bool(*b),
    }
}

/// Expand a variable reference at the requested format, appending bound args.
pub fn expand_variable(
    out: &mut BoundQueryBuilder,
    ctx: &BindCtx,
    name: &str,
    format: VarFormat,
) -> Result<(), BindError> {
    let var = ctx
        .variables
        .get(name)
        .ok_or_else(|| BindError::UndefinedVariable(name.to_string()))?;
    let values: Vec<&ScalarValue> = match var {
        VarValue::Single(v) => vec![v],
        VarValue::Multi(vs) => vs.iter().collect(),
    };
    match format {
        VarFormat::Single => {
            // A single placeholder. An empty multi-value or absent first value
            // is a NULL bind, which is inert in the surrounding predicate.
            match values.first() {
                Some(v) => bind_scalar(out, v)?,
                None => out.push_arg(SqlValue::Null),
            }
        }
        VarFormat::Csv | VarFormat::SingleQuote => bind_list(out, &values, false)?,
        VarFormat::SqlIn => bind_list(out, &values, true)?,
    }
    Ok(())
}

/// Bind a list of values as comma-separated placeholders, optionally wrapped in
/// parentheses for `IN (...)`. An empty list with parens emits `(NULL)` so
/// `col IN (NULL)` is a well-formed, always-false guard rather than a syntax
/// error — an "All / nothing selected" multi-select stays inert.
fn bind_list(
    out: &mut BoundQueryBuilder,
    values: &[&ScalarValue],
    parens: bool,
) -> Result<(), BindError> {
    if parens {
        out.push_sql("(");
    }
    if values.is_empty() {
        out.push_arg(SqlValue::Null);
    } else {
        for (i, v) in values.iter().enumerate() {
            if i > 0 {
                out.push_sql(", ");
            }
            bind_scalar(out, v)?;
        }
    }
    if parens {
        out.push_sql(")");
    }
    Ok(())
}

/// Expand a `$param` (kind named param) as a single bound placeholder.
pub fn expand_param(
    out: &mut BoundQueryBuilder,
    ctx: &BindCtx,
    name: &str,
) -> Result<(), BindError> {
    let value = ctx
        .params
        .get(name)
        .ok_or_else(|| BindError::UndefinedParameter(name.to_string()))?;
    bind_scalar(out, value)
}

/// Bind a host token (`$caller_tenant_id` / `$caller_user_id`) from the
/// `Principal`-sourced context. Absent context is a 4xx, not a silent skip —
/// a query that needs the tenant id must get it.
pub fn expand_host_token(
    out: &mut BoundQueryBuilder,
    ctx: &BindCtx,
    token: &str,
) -> Result<(), BindError> {
    let value = match token {
        "caller_tenant_id" => ctx.host_tokens.caller_tenant_id.clone(),
        "caller_user_id" => ctx.host_tokens.caller_user_id.clone(),
        _ => return Err(BindError::UndefinedVariable(token.to_string())),
    };
    match value {
        Some(v) => {
            out.push_arg(SqlValue::Text(v));
            Ok(())
        }
        None => Err(BindError::MissingContext {
            macro_name: token.to_string(),
            missing: "host token".to_string(),
        }),
    }
}
