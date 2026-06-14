//! The time macros: `$__timeFilter`, `$__timeGroup`, `$__timeFrom`,
//! `$__timeTo`, `$__interval`.
//!
//! Time bounds are always *bound* as `$N` args (a timestamp the driver binds),
//! never formatted into the SQL — the column the bounds compare against is the
//! only text, and it is validated first. The bucket width for `$__timeGroup`
//! comes from the dialect, which renders a server-derived integer interval. See
//! docs/design/query/.

use std::time::Duration;

use super::bound::{BoundQueryBuilder, SqlValue};
use super::context::BindCtx;
use super::dialect::Dialect;
use super::error::BindError;
use super::identifier::validate_identifier;

/// `$__timeFilter(col)` → `col >= $N AND col < $M` with both bounds bound. The
/// column is validated; the instants are bound. Half-open (`>= from`, `< to`)
/// to match Grafana and to avoid double-counting a bucket boundary.
pub fn time_filter(
    out: &mut BoundQueryBuilder,
    ctx: &BindCtx,
    column_arg: &str,
) -> Result<(), BindError> {
    let range = ctx.time_range.ok_or_else(|| BindError::MissingContext {
        macro_name: "timeFilter".to_string(),
        missing: "a time range".to_string(),
    })?;
    let column = validate_identifier(column_arg)?;
    out.push_identifier(&column);
    out.push_sql(" >= ");
    out.push_arg(SqlValue::Timestamp(range.from));
    out.push_sql(" AND ");
    out.push_identifier(&column);
    out.push_sql(" < ");
    out.push_arg(SqlValue::Timestamp(range.to));
    Ok(())
}

/// `$__timeGroup(col, '5m')` / `$__timeGroup(col, $__interval)` → the dialect's
/// bucket expression. Both the column and the resolved width are server-vetted:
/// the column through the identifier allowlist, the width parsed to a
/// `Duration`.
pub fn time_group(
    out: &mut BoundQueryBuilder,
    ctx: &BindCtx,
    dialect: &dyn Dialect,
    column_arg: &str,
    width_arg: &str,
) -> Result<(), BindError> {
    let column = validate_identifier(column_arg)?;
    let width = resolve_width(ctx, width_arg)?;
    let fragment = dialect.time_group(&column, width);
    // The whole bucket expression is dialect-controlled text built from a
    // validated identifier + a server-derived integer, so it is recorded as a
    // validated fragment rather than bound (an expression can't be a bind arg).
    out.push_identifier(&fragment);
    Ok(())
}

/// `$__timeFrom` / `$__timeTo` → a single bound timestamp.
pub fn time_bound(
    out: &mut BoundQueryBuilder,
    ctx: &BindCtx,
    upper: bool,
) -> Result<(), BindError> {
    let range = ctx.time_range.ok_or_else(|| BindError::MissingContext {
        macro_name: if upper { "timeTo" } else { "timeFrom" }.to_string(),
        missing: "a time range".to_string(),
    })?;
    let instant = if upper { range.to } else { range.from };
    out.push_arg(SqlValue::Timestamp(instant));
    Ok(())
}

/// Resolve a `$__timeGroup` width argument: either `$__interval` (the context
/// interval) or a duration literal like `'5m'` / `5m` / `30s` / `1h` / `2d`.
fn resolve_width(ctx: &BindCtx, width_arg: &str) -> Result<Duration, BindError> {
    let trimmed = width_arg.trim();
    if trimmed == "$__interval" {
        return ctx.interval.ok_or_else(|| BindError::MissingContext {
            macro_name: "timeGroup".to_string(),
            missing: "$__interval".to_string(),
        });
    }
    parse_duration_literal(trimmed)
}

/// Resolve the bare `$__interval` macro to its bound-arg-free bucket width and
/// return the [`Duration`]; the scanner uses this to emit the bucket where a
/// raw `$__interval` appears outside `$__timeGroup` (rare, but Grafana allows
/// it as an interval literal). Returns the width so the caller can decide how to
/// render it.
pub fn interval(ctx: &BindCtx) -> Result<Duration, BindError> {
    ctx.interval.ok_or_else(|| BindError::MissingContext {
        macro_name: "interval".to_string(),
        missing: "$__interval".to_string(),
    })
}

/// Parse `<n><unit>` where unit is `s|m|h|d`, optionally wrapped in single
/// quotes. Rejects anything else so a malformed width is a 4xx, not a silent
/// zero bucket.
fn parse_duration_literal(raw: &str) -> Result<Duration, BindError> {
    let unquoted = raw.trim_matches('\'').trim();
    let malformed = || BindError::MalformedMacro {
        macro_name: "timeGroup".to_string(),
        detail: format!("invalid interval literal: {raw}"),
    };
    let (digits, unit) = unquoted.split_at(
        unquoted
            .find(|c: char| !c.is_ascii_digit())
            .ok_or_else(malformed)?,
    );
    let n: u64 = digits.parse().map_err(|_| malformed())?;
    let secs = match unit {
        "s" => n,
        "m" => n * 60,
        "h" => n * 3600,
        "d" => n * 86_400,
        _ => return Err(malformed()),
    };
    Ok(Duration::from_secs(secs.max(1)))
}
