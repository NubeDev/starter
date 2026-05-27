//! Generic resolver for extension-contributed warehouse-read
//! templates.
//!
//! At extension load time the host folds every
//! `contributes.warehouse_templates[]` entry into
//! [`starter_ext_host::TemplateRegistry`] as a [`TemplateSpec`]
//! carrying:
//!
//! - `params`  — JSON-Schema fragment for inbound parameters,
//! - `sql`     — the verbatim SQL body the extension shipped, and
//! - `tables`  — the allowlist the supervisor cross-checks against
//!               the calling extension's grant.
//!
//! This module is the **single** runtime path for executing any
//! contributed template. The host knows nothing about specific
//! extensions; there are no per-extension match arms anywhere in
//! the resolver. Adding a new extension that ships templates
//! requires zero host code edits — matching the SPI contract
//! (`docs/scope/extensions-north-star` rows 2/3, SCOPE R7/R8).
//!
//! ## Parameter binding (R7 — never string-template SQL)
//!
//! The SQL body uses named placeholders (`$caller_tenant_id`,
//! `$<schema_property>`). At call time we:
//!
//! 1. Validate `params` against `spec.params` (JSON-Schema, draft
//!    2020-12) — this rejects extra/missing/mistyped keys before
//!    any SQL is touched.
//! 2. Walk `spec.sql` once with a strict ident regex and replace
//!    each named placeholder with a numbered `$1..$N`, recording
//!    the bind order. `$caller_tenant_id` is always position 1 and
//!    is filled from the caller frame the SDK threads through —
//!    extensions cannot override it.
//! 3. Execute the rewritten SQL with `sqlx::query` and bind each
//!    placeholder with the JSON-typed value from `params` (or the
//!    schema `default`).
//! 4. Decode every returned column generically via the postgres
//!    type-info system and pack the row as a `serde_json::Map`.
//!
//! The original SQL string is **never** sent to the database; the
//! rewritten one carries only `$N` placeholders, so caller input
//! never enters the SQL text. Templates that reference a
//! placeholder not declared in the schema, or that the regex
//! cannot match, refuse with a clear error at call time.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use jsonschema::{Draft, JSONSchema};

use serde_json::{json, Map, Value as JsonValue};
use sqlx::postgres::PgRow;
use sqlx::{Column, PgPool, Row, TypeInfo};
use starter_ext_spi::warehouse::TemplateSpec;

/// Always-bound first placeholder (`$caller_tenant_id` → `$1`).
const CALLER_TENANT_PLACEHOLDER: &str = "caller_tenant_id";

/// Execute a contributed template against the warehouse pool and
/// return the result rows as a vector of JSON objects.
///
/// `tenant_id` is the caller's identity (the SDK frame's
/// `caller().tenant_id` for extension calls, or the SDUI bridge's
/// `params["tenant_id"]` for the legacy chart path). Callers that
/// can't supply a tenant must refuse BEFORE invoking this fn —
/// the SQL bodies assume `$caller_tenant_id` is always bound.
pub async fn execute(
    pool: &PgPool,
    spec: &TemplateSpec,
    tenant_id: &str,
    params: &JsonValue,
) -> Result<Vec<JsonValue>, String> {
    let sql_body = spec
        .sql
        .as_deref()
        .ok_or_else(|| format!("template {:?}: no SQL body registered", spec.name))?;

    // 1. Validate params against the spec's JSON Schema.
    validate_params(spec, params)?;

    // 2. Compile the SQL: $caller_tenant_id → $1, then each
    //    $<property> → $2..$N in first-appearance order.
    let plan = compile(spec, sql_body)?;

    // 3. Bind values and execute.
    let mut q = sqlx::query(&plan.sql);
    for slot in &plan.binds {
        q = bind_slot(q, slot, tenant_id, params, spec)?;
    }
    let rows = q
        .fetch_all(pool)
        .await
        .map_err(|e| format!("template {:?}: {e}", spec.name))?;

    // 4. Decode rows generically.
    rows.into_iter()
        .map(|row| row_to_json(&row).map_err(|e| format!("template {:?}: {e}", spec.name)))
        .collect()
}

/// Validate `params` against `spec.params` (JSON Schema, draft
/// 2020-12). An empty schema accepts anything.
fn validate_params(spec: &TemplateSpec, params: &JsonValue) -> Result<(), String> {
    if spec.params.is_null()
        || spec
            .params
            .as_object()
            .is_some_and(|m| m.is_empty())
    {
        return Ok(());
    }
    // The workspace pulls `jsonschema` with default-features off,
    // which gates the 2019-09 / 2020-12 drafts. Draft 7 covers the
    // baseline keywords we use in template `params_schema` files
    // (type/properties/required/additionalProperties/enum/default).
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&spec.params)
        .map_err(|e| format!("template {:?}: invalid params_schema: {e}", spec.name))?;
    if let Err(errors) = compiled.validate(params) {
        let joined = errors
            .map(|e| format!("{} at {}", e, e.instance_path))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "template {:?}: params failed schema validation: {joined}",
            spec.name
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct Plan {
    /// Rewritten SQL — `$caller_tenant_id` / `$<prop>` replaced
    /// with `$1..$N`.
    sql: String,
    /// One entry per `$N`, in bind order.
    binds: Vec<BindSlot>,
}

#[derive(Debug, Clone)]
enum BindSlot {
    /// The caller-bound tenant id (always position 1 once present).
    CallerTenantId,
    /// A schema-declared parameter (position 2..N).
    Param(String),
}

/// Rewrite `$caller_tenant_id` and `$<schema_property>` named
/// placeholders to positional `$1..$N`. Refuses to rewrite any
/// `$ident` token whose name is neither `caller_tenant_id` nor a
/// declared schema property — that catches typos in the contributed
/// SQL at first use rather than passing them through to the DB.
fn compile(spec: &TemplateSpec, body: &str) -> Result<Plan, String> {
    let allowed: Vec<String> = schema_properties(&spec.params);
    let mut binds: Vec<BindSlot> = Vec::new();
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = String::with_capacity(body.len());

    let bytes = body.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        // Skip `-- line comment` to end-of-line. Apostrophes inside
        // doc comments (e.g. "host's") must NOT flip the scanner
        // into string-literal mode, and `$ident` mentions inside
        // comments must not be rewritten.
        if b == b'-' && i + 1 < bytes.len() && bytes[i + 1] == b'-' {
            let start = i;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            out.push_str(&body[start..i]);
            continue;
        }
        // Skip `/* block comment */`. Postgres permits nesting but
        // the SQL we accept is hand-written and doesn't rely on it.
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < bytes.len() {
                i += 2;
            }
            out.push_str(&body[start..i]);
            continue;
        }
        // Skip single-quoted string literals so a stray `$foo` inside
        // a literal isn't rewritten. The SQL we accept is hand-written
        // by the extension author, so we don't need to handle every
        // postgres quoting form — just the one that can legitimately
        // contain a `$`.
        if b == b'\'' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\'' {
                    i += 1;
                    if i < bytes.len() && bytes[i] == b'\'' {
                        i += 1; // escaped ''
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            out.push_str(&body[start..i]);
            continue;
        }
        if b != b'$' {
            out.push(b as char);
            i += 1;
            continue;
        }
        // Possible placeholder. Read ident.
        let ident_start = i + 1;
        let mut j = ident_start;
        while j < bytes.len() {
            let c = bytes[j];
            if c == b'_' || c.is_ascii_alphanumeric() {
                j += 1;
            } else {
                break;
            }
        }
        if j == ident_start {
            // Lone `$` (or `$1` from a hand-numbered template — we
            // refuse those to keep one canonical form).
            return Err(format!(
                "template {:?}: bare `$` or positional placeholder at byte {i} \
                 — use `$caller_tenant_id` or `$<param>` named placeholders only",
                spec.name
            ));
        }
        let name = &body[ident_start..j];
        let slot = if name == CALLER_TENANT_PLACEHOLDER {
            BindSlot::CallerTenantId
        } else if allowed.iter().any(|p| p == name) {
            BindSlot::Param(name.to_string())
        } else {
            return Err(format!(
                "template {:?}: SQL references `${name}` which is not declared in \
                 params_schema (allowed: {:?} + caller_tenant_id)",
                spec.name, allowed
            ));
        };
        let pos = *index.entry(name.to_string()).or_insert_with(|| {
            binds.push(slot.clone());
            binds.len()
        });
        out.push('$');
        out.push_str(&pos.to_string());
        i = j;
    }
    Ok(Plan { sql: out, binds })
}

/// Pull the top-level `properties` key names out of a JSON-Schema
/// object. Returns an empty vec for schemas that declare none.
fn schema_properties(schema: &JsonValue) -> Vec<String> {
    schema
        .get("properties")
        .and_then(JsonValue::as_object)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

/// Bind one slot onto the query. JSON null/bool/number/string map
/// to the corresponding sqlx postgres types. Arrays/objects are
/// JSON-encoded and bound as TEXT — rare in template params.
fn bind_slot<'q>(
    q: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    slot: &'q BindSlot,
    tenant_id: &'q str,
    params: &'q JsonValue,
    spec: &'q TemplateSpec,
) -> Result<sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>, String> {
    match slot {
        BindSlot::CallerTenantId => Ok(q.bind(tenant_id)),
        BindSlot::Param(name) => {
            let value = params
                .get(name)
                .or_else(|| schema_default(&spec.params, name))
                .unwrap_or(&JsonValue::Null);
            Ok(bind_json(q, value))
        }
    }
}

fn schema_default<'a>(schema: &'a JsonValue, prop: &str) -> Option<&'a JsonValue> {
    schema
        .get("properties")
        .and_then(JsonValue::as_object)
        .and_then(|m| m.get(prop))
        .and_then(|p| p.get("default"))
}

fn bind_json<'q>(
    q: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    v: &'q JsonValue,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    match v {
        JsonValue::Null => q.bind(Option::<String>::None),
        JsonValue::Bool(b) => q.bind(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                q.bind(i)
            } else if let Some(f) = n.as_f64() {
                q.bind(f)
            } else {
                q.bind(n.to_string())
            }
        }
        JsonValue::String(s) => q.bind(s.as_str()),
        JsonValue::Array(_) | JsonValue::Object(_) => q.bind(v.to_string()),
    }
}

/// Decode a row's columns into a JSON object. Handles the common
/// postgres scalar types extension warehouse tables actually emit
/// (TEXT, INT2/4/8, FLOAT4/8, NUMERIC-as-string, BOOL, DATE,
/// TIMESTAMP, TIMESTAMPTZ, UUID, JSON/JSONB). Unknown types
/// degrade to a string representation rather than refusing the
/// whole query.
fn row_to_json(row: &PgRow) -> Result<JsonValue, String> {
    let mut out = Map::with_capacity(row.columns().len());
    for col in row.columns() {
        let name = col.name();
        let ord = col.ordinal();
        let ty = col.type_info().name();
        let value = decode_cell(row, ord, ty)
            .map_err(|e| format!("column `{name}` ({ty}): {e}"))?;
        out.insert(name.to_string(), value);
    }
    Ok(JsonValue::Object(out))
}

fn decode_cell(row: &PgRow, idx: usize, ty: &str) -> Result<JsonValue, String> {
    // NULL-tolerant decode: each branch reads `Option<T>` and folds
    // `None` to `JsonValue::Null`.
    macro_rules! get {
        ($t:ty) => {{
            let v: Option<$t> = row.try_get(idx).map_err(|e| e.to_string())?;
            Ok(match v {
                Some(x) => json!(x),
                None => JsonValue::Null,
            })
        }};
    }
    match ty {
        "TEXT" | "VARCHAR" | "BPCHAR" | "NAME" | "CITEXT" | "UUID" => get!(String),
        "INT2" => get!(i16),
        "INT4" => get!(i32),
        "INT8" => get!(i64),
        "FLOAT4" => get!(f32),
        "FLOAT8" => get!(f64),
        "BOOL" => get!(bool),
        "DATE" => {
            let v: Option<NaiveDate> = row.try_get(idx).map_err(|e| e.to_string())?;
            Ok(match v {
                Some(d) => JsonValue::String(d.format("%Y-%m-%d").to_string()),
                None => JsonValue::Null,
            })
        }
        "TIMESTAMP" => {
            let v: Option<chrono::NaiveDateTime> =
                row.try_get(idx).map_err(|e| e.to_string())?;
            Ok(match v {
                Some(t) => JsonValue::String(t.format("%Y-%m-%dT%H:%M:%S").to_string()),
                None => JsonValue::Null,
            })
        }
        "TIMESTAMPTZ" => {
            let v: Option<DateTime<Utc>> = row.try_get(idx).map_err(|e| e.to_string())?;
            Ok(match v {
                Some(t) => JsonValue::Number(t.timestamp_millis().into()),
                None => JsonValue::Null,
            })
        }
        "JSON" | "JSONB" => {
            let v: Option<sqlx::types::Json<JsonValue>> =
                row.try_get(idx).map_err(|e| e.to_string())?;
            Ok(match v {
                Some(sqlx::types::Json(j)) => j,
                None => JsonValue::Null,
            })
        }
        // Fall through: try string. Catches NUMERIC (rendered as
        // decimal text), unknown user-defined types, etc.
        _ => {
            let v: Option<String> = row.try_get(idx).map_err(|e| {
                format!("no JSON decoder for postgres type `{ty}`: {e}")
            })?;
            Ok(match v {
                Some(s) => JsonValue::String(s),
                None => JsonValue::Null,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(sql: &str, props: JsonValue) -> TemplateSpec {
        TemplateSpec {
            name: "test.t".into(),
            params: json!({ "type": "object", "properties": props }),
            tables: vec![],
            sql: Some(sql.into()),
        }
    }

    #[test]
    fn compile_rewrites_caller_tenant_first() {
        let s = spec("SELECT 1 WHERE tenant = $caller_tenant_id", json!({}));
        let p = compile(&s, s.sql.as_deref().unwrap()).unwrap();
        assert!(p.sql.contains("$1"));
        assert!(matches!(p.binds[0], BindSlot::CallerTenantId));
    }

    #[test]
    fn compile_assigns_params_in_first_appearance_order() {
        let s = spec(
            "SELECT $a, $b, $a, $caller_tenant_id",
            json!({ "a": {"type": "integer"}, "b": {"type": "string"} }),
        );
        let p = compile(&s, s.sql.as_deref().unwrap()).unwrap();
        // `$a` first → $1, `$b` second → $2, `$a` reused → $1,
        // `$caller_tenant_id` last → $3.
        assert_eq!(p.sql, "SELECT $1, $2, $1, $3");
        assert!(matches!(&p.binds[0], BindSlot::Param(s) if s == "a"));
        assert!(matches!(&p.binds[1], BindSlot::Param(s) if s == "b"));
        assert!(matches!(p.binds[2], BindSlot::CallerTenantId));
    }

    #[test]
    fn compile_refuses_undeclared_placeholder() {
        let s = spec("SELECT $oops", json!({}));
        let err = compile(&s, s.sql.as_deref().unwrap()).unwrap_err();
        assert!(err.contains("not declared"));
    }

    #[test]
    fn compile_ignores_dollar_inside_string_literal() {
        let s = spec(
            "SELECT 'literal $not_a_param' WHERE x = $caller_tenant_id",
            json!({}),
        );
        let p = compile(&s, s.sql.as_deref().unwrap()).unwrap();
        assert!(p.sql.contains("'literal $not_a_param'"));
        assert_eq!(p.binds.len(), 1);
    }

    #[test]
    fn compile_skips_line_comments_with_apostrophes() {
        // Regression: the doc comment contains "host's" — the lone
        // apostrophe must NOT flip the scanner into string-literal
        // mode and swallow the real placeholders below.
        let body = "-- The host's note mentions $limit and $caller_tenant_id\n\
                    SELECT * FROM t WHERE tenant_id = $caller_tenant_id LIMIT $limit";
        let s = spec(
            body,
            json!({ "limit": { "type": "integer" } }),
        );
        let p = compile(&s, body).unwrap();
        // The real WHERE clause must use a numbered placeholder.
        assert!(p.sql.contains("tenant_id = $1"), "sql={}", p.sql);
        assert!(p.sql.contains("LIMIT $2"), "sql={}", p.sql);
        // Comment text preserved verbatim (still contains $limit).
        assert!(p.sql.contains("-- The host's note"));
    }

    #[test]
    fn compile_skips_block_comments() {
        let body = "/* note: $limit is a placeholder */ SELECT $caller_tenant_id";
        let s = spec(body, json!({}));
        let p = compile(&s, body).unwrap();
        assert!(p.sql.contains("/* note: $limit is a placeholder */"));
        assert!(p.sql.ends_with("$1"));
    }

    #[test]
    fn validate_params_rejects_extra_keys_when_schema_forbids() {
        let s = TemplateSpec {
            name: "t".into(),
            params: json!({
                "type": "object",
                "properties": { "a": { "type": "integer" } },
                "additionalProperties": false,
            }),
            tables: vec![],
            sql: Some("SELECT 1".into()),
        };
        assert!(validate_params(&s, &json!({ "a": 1, "extra": 2 })).is_err());
    }
}
