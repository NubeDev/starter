//! Left-to-right scan that rewrites macro/variable tokens into bound args.
//!
//! The scanner copies literal SQL verbatim and, on each `$`, recognises one
//! token form and dispatches to the right expander. It never interprets the
//! surrounding SQL — only the tokens — so the author's query text stays the
//! existing guarded path and the binder only ever *adds* bound `$N` args and
//! vetted identifiers. See docs/design/query/.
//!
//! The scan is **comment- and string-aware**: a `$token` inside a `--` line
//! comment, a `/* */` block comment (nested, per Postgres), a `'...'` string
//! literal, or a `$tag$...$tag$` dollar-quoted body is plain text, not a token.
//! Without this, a kind whose *documentation header* mentions
//! `$caller_tenant_id` demands the host token at bind time (and the rewritten
//! `$N` inside the comment then desyncs Postgres's supplied-vs-referenced
//! param count), and a literal like `'price in $USD'` 4xxes as an undefined
//! variable.

use super::bound::{BoundQueryBuilder, SqlValue};
use super::context::BindCtx;
use super::dialect::Dialect;
use super::error::BindError;
use super::time_macros;
use super::vars::{self, VarFormat};

/// Walk `sql`, expanding tokens against `ctx` using `dialect` for time buckets.
pub fn scan(
    sql: &str,
    ctx: &BindCtx,
    dialect: &dyn Dialect,
    out: &mut BoundQueryBuilder,
) -> Result<(), BindError> {
    let bytes = sql.as_bytes();
    let mut i = 0;
    let mut literal_start = 0;
    while i < bytes.len() {
        match bytes[i] {
            // Comments and string literals are copied verbatim with no token
            // expansion — advance `i` past them; the literal run keeps growing
            // and is flushed at the next real token (or the end).
            b'\'' => i = skip_string(bytes, i),
            b'-' if bytes.get(i + 1) == Some(&b'-') => i = skip_line_comment(bytes, i),
            b'/' if bytes.get(i + 1) == Some(&b'*') => i = skip_block_comment(bytes, i),
            b'$' => {
                // A dollar-quoted body (`$$...$$` / `$tag$...$tag$`) is a
                // string literal too — skip it whole before token dispatch.
                if let Some(end) = skip_dollar_quote(sql, i) {
                    i = end;
                    continue;
                }
                // Flush the literal run before this token, then expand it.
                out.push_sql(&sql[literal_start..i]);
                i = dispatch(sql, i, ctx, dialect, out)?;
                literal_start = i;
            }
            _ => i += 1,
        }
    }
    out.push_sql(&sql[literal_start..]);
    Ok(())
}

/// Advance past a `'...'` string literal starting at `at` (`bytes[at] == '\''`).
/// A doubled `''` is the SQL escape for a quote inside the literal. An
/// unterminated literal runs to the end — the scanner copies it verbatim and
/// Postgres reports the syntax error.
fn skip_string(bytes: &[u8], at: usize) -> usize {
    let mut i = at + 1;
    while i < bytes.len() {
        if bytes[i] == b'\'' {
            if bytes.get(i + 1) == Some(&b'\'') {
                i += 2; // escaped quote — still inside the literal
                continue;
            }
            return i + 1;
        }
        i += 1;
    }
    bytes.len()
}

/// Advance past a `--` line comment starting at `at` (to past the newline, or
/// the end of input).
fn skip_line_comment(bytes: &[u8], at: usize) -> usize {
    let mut i = at + 2;
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    (i + 1).min(bytes.len())
}

/// Advance past a `/* ... */` block comment starting at `at`. Nested per the
/// Postgres rule (`/* /* */ */` is one comment). Unterminated runs to the end.
fn skip_block_comment(bytes: &[u8], at: usize) -> usize {
    let mut depth = 1usize;
    let mut i = at + 2;
    while i + 1 < bytes.len() {
        if bytes[i] == b'/' && bytes[i + 1] == b'*' {
            depth += 1;
            i += 2;
        } else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            i += 1;
        }
    }
    bytes.len()
}

/// If `sql[at..]` opens a dollar-quoted string (`$$` or `$tag$` where `tag` is
/// an identifier), return the index just past its closing delimiter (or the end
/// of input when unterminated). `None` when the `$` is not a dollar-quote
/// opener — i.e. it's a candidate macro/variable/param token.
fn skip_dollar_quote(sql: &str, at: usize) -> Option<usize> {
    let rest = &sql[at + 1..];
    let tag = ident_after(rest);
    // Opener is `$<tag>$`; a bare `$name` with no closing `$` is a token, and
    // `$__macro` is never a dollar-quote (Postgres tags can't start with a
    // digit; ours can't be macros).
    if !rest[tag.len()..].starts_with('$') {
        return None;
    }
    let delim_len = tag.len() + 2; // $tag$
    let delim = &sql[at..at + delim_len];
    let body_start = at + delim_len;
    match sql[body_start..].find(delim) {
        Some(rel) => Some(body_start + rel + delim_len),
        None => Some(sql.len()),
    }
}

/// Recognise the token starting at `at` (`sql[at] == '$'`), expand it, and
/// return the byte index just past it. A `$` that begins no known token (e.g. a
/// dollar-quote or a stray `$$`) is copied through verbatim.
fn dispatch(
    sql: &str,
    at: usize,
    ctx: &BindCtx,
    dialect: &dyn Dialect,
    out: &mut BoundQueryBuilder,
) -> Result<usize, BindError> {
    let rest = &sql[at..];
    if rest.starts_with("${") {
        return braced_variable(sql, at, ctx, out);
    }
    if rest.starts_with("$__") {
        return macro_token(sql, at, ctx, dialect, out);
    }
    // `$name` — a bare variable, kind param, or host token. `$$` / `$1` (a
    // literal placeholder the author wrote) fall through as verbatim text.
    let name = ident_after(&rest[1..]);
    if name.is_empty() {
        out.push_sql("$");
        return Ok(at + 1);
    }
    expand_bare(out, ctx, name)?;
    Ok(at + 1 + name.len())
}

/// Expand a bare `$name`: a host token first (so the caller can never shadow
/// it with a same-named variable), then a kind param, then a dashboard
/// variable. A name that matches none is an undefined-variable 4xx.
fn expand_bare(out: &mut BoundQueryBuilder, ctx: &BindCtx, name: &str) -> Result<(), BindError> {
    if name == "caller_tenant_id" || name == "caller_user_id" {
        return vars::expand_host_token(out, ctx, name);
    }
    if ctx.params.contains_key(name) {
        return vars::expand_param(out, ctx, name);
    }
    vars::expand_variable(out, ctx, name, VarFormat::Single)
}

/// Expand `${var}` / `${var:csv}` / `${var:singlequote}`.
fn braced_variable(
    sql: &str,
    at: usize,
    ctx: &BindCtx,
    out: &mut BoundQueryBuilder,
) -> Result<usize, BindError> {
    let open = at + 2; // past `${`
    let close = sql[open..]
        .find('}')
        .map(|rel| open + rel)
        .ok_or(BindError::Unterminated(at))?;
    let inner = &sql[open..close];
    let (name, format) = match inner.split_once(':') {
        Some((n, "csv")) => (n.trim(), VarFormat::Csv),
        Some((n, "singlequote")) => (n.trim(), VarFormat::SingleQuote),
        Some((_, other)) => {
            return Err(BindError::MalformedMacro {
                macro_name: "variable".to_string(),
                detail: format!("unknown format `{other}`"),
            })
        }
        None => (inner.trim(), VarFormat::Single),
    };
    vars::expand_variable(out, ctx, name, format)?;
    Ok(close + 1)
}

/// Expand a `$__...` macro. Macros split into the call forms (`name(args)`) and
/// the bare forms (`$__timeFrom`, `$__timeTo`, `$__interval`).
fn macro_token(
    sql: &str,
    at: usize,
    ctx: &BindCtx,
    dialect: &dyn Dialect,
    out: &mut BoundQueryBuilder,
) -> Result<usize, BindError> {
    let after = at + 3; // past `$__`
    let name = ident_after(&sql[after..]);
    let name_end = after + name.len();
    let has_call = sql[name_end..].trim_start().starts_with('(');
    if has_call {
        let (args, end) = call_args(sql, name_end, at)?;
        expand_call_macro(out, ctx, dialect, name, &args)?;
        Ok(end)
    } else {
        expand_bare_macro(out, ctx, name)?;
        Ok(name_end)
    }
}

/// Dispatch a call-form macro (`$__timeFilter(col)`, `$__timeGroup(col, w)`,
/// `$__sqlIn(var)`).
fn expand_call_macro(
    out: &mut BoundQueryBuilder,
    ctx: &BindCtx,
    dialect: &dyn Dialect,
    name: &str,
    args: &[String],
) -> Result<(), BindError> {
    match name {
        "timeFilter" => {
            expect_arity(name, args, 1)?;
            time_macros::time_filter(out, ctx, &args[0])
        }
        "timeGroup" => {
            expect_arity(name, args, 2)?;
            time_macros::time_group(out, ctx, dialect, &args[0], &args[1])
        }
        "sqlIn" => {
            expect_arity(name, args, 1)?;
            let var = args[0].trim().trim_start_matches('$');
            vars::expand_variable(out, ctx, var, VarFormat::SqlIn)
        }
        _ => Err(BindError::UnknownMacro(name.to_string())),
    }
}

/// Dispatch a bare-form macro (`$__timeFrom`, `$__timeTo`, `$__interval`).
fn expand_bare_macro(
    out: &mut BoundQueryBuilder,
    ctx: &BindCtx,
    name: &str,
) -> Result<(), BindError> {
    match name {
        "timeFrom" => time_macros::time_bound(out, ctx, false),
        "timeTo" => time_macros::time_bound(out, ctx, true),
        "interval" => {
            // A standalone `$__interval` renders as a bound seconds integer — a
            // value, so it binds rather than inlines.
            let width = time_macros::interval(ctx)?;
            out.push_arg(SqlValue::Int(width.as_secs().max(1) as i64));
            Ok(())
        }
        _ => Err(BindError::UnknownMacro(name.to_string())),
    }
}

/// Reject a macro called with the wrong number of arguments.
fn expect_arity(name: &str, args: &[String], want: usize) -> Result<(), BindError> {
    if args.len() == want {
        Ok(())
    } else {
        Err(BindError::MalformedMacro {
            macro_name: name.to_string(),
            detail: format!("expected {want} argument(s), got {}", args.len()),
        })
    }
}

/// Parse a parenthesised, comma-separated argument list starting at the `(`
/// found from `name_end`. Returns the trimmed args and the index past the `)`.
/// Splits on top-level commas only (no nesting is expected in macro args, but a
/// quoted comma is respected).
fn call_args(sql: &str, name_end: usize, token_start: usize) -> Result<(Vec<String>, usize), BindError> {
    let open = name_end + sql[name_end..].find('(').unwrap();
    let close = sql[open..]
        .find(')')
        .map(|rel| open + rel)
        .ok_or(BindError::Unterminated(token_start))?;
    let inner = &sql[open + 1..close];
    let args = split_top_level(inner);
    Ok((args, close + 1))
}

/// Split on commas that are not inside single quotes. Macro args are simple
/// (an identifier or a quoted interval), so single-quote tracking is enough.
fn split_top_level(inner: &str) -> Vec<String> {
    if inner.trim().is_empty() {
        return Vec::new();
    }
    let mut args = Vec::new();
    let mut start = 0;
    let mut in_quote = false;
    for (i, c) in inner.char_indices() {
        match c {
            '\'' => in_quote = !in_quote,
            ',' if !in_quote => {
                args.push(inner[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
    }
    args.push(inner[start..].trim().to_string());
    args
}

/// The leading `[A-Za-z_][A-Za-z0-9_]*` run of `s` (an identifier/macro name),
/// or `""` if `s` does not start with one.
fn ident_after(s: &str) -> &str {
    let end = s
        .char_indices()
        .take_while(|(i, c)| {
            if *i == 0 {
                c.is_ascii_alphabetic() || *c == '_'
            } else {
                c.is_ascii_alphanumeric() || *c == '_'
            }
        })
        .count();
    &s[..end]
}
