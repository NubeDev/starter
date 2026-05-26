//! `rubix.analytics.query` — tool dispatch.
//!
//! Looks up a *named* SQL template embedded via `include_dir!`
//! (one per file under [`templates/`](./templates/)), binds the
//! caller-supplied params through ClickHouse's native `{name:Type}`
//! parameter syntax (the official Rust client's `param()` API; no
//! string interpolation), runs it through the shared `ChClient`,
//! and returns the rows as a JSON array.
//!
//! Read-only: there is no `ReversibleTool` impl — this verb never
//! changes warehouse state. See
//! [docs/design/analytics/](../../../../docs/design/analytics/README.md).
//!
//! The template catalogue is closed at compile time. Unknown names
//! yield `rubix.analytics.query.unknown_template`; ClickHouse-side
//! parse/bind errors surface as `rubix.analytics.query.bind_error`
//! to keep raw driver messages off the wire.

use std::sync::Arc;

use async_trait::async_trait;
use include_dir::{include_dir, Dir};
use rubix_spi::dto::analytics::query::{AnalyticsQueryRequest, AnalyticsQueryResponse};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_store_clickhouse::ChClient;

/// All bundled analytics SQL templates, embedded at compile time.
/// One file per named template (no subdirectories). Lookup is by
/// `<name>.sql` filename — keep template names matching the file
/// stem so the lookup is a single `get_file` call.
static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/analytics/templates");

/// Concrete [`Tool`] for `rubix.analytics.query`. Holds the shared
/// `ChClient` so every call goes through the same W8-configured
/// connection (and tests can swap in a testcontainer client).
#[derive(Clone)]
pub struct AnalyticsQueryTool {
    client: Arc<ChClient>,
}

impl AnalyticsQueryTool {
    /// Wrap the shared client.
    pub fn new(client: Arc<ChClient>) -> Self {
        Self { client }
    }

    /// Filenames of every bundled template, sans `.sql`. Stable
    /// order matters for snapshot tests; `include_dir!` preserves
    /// directory iteration order.
    pub fn known_templates() -> Vec<&'static str> {
        TEMPLATES
            .files()
            .filter_map(|f| {
                f.path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .filter(|_| {
                        f.path().extension().and_then(|e| e.to_str()) == Some("sql")
                    })
            })
            .collect()
    }
}

#[async_trait]
impl Tool for AnalyticsQueryTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.analytics.query".to_owned(),
            description: rubix_spi::dto::analytics::query::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name":   { "type": "string", "minLength": 1 },
                    "params": { "type": "object", "additionalProperties": true }
                },
                "required": ["name"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: AnalyticsQueryRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("AnalyticsQueryRequest: {e}"),
            })?;

        let sql = lookup_template(&req.name)?;
        let rows = run_query(&self.client, sql, &req.params).await?;
        let row_count = u32::try_from(rows.len()).unwrap_or(u32::MAX);

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.analytics.query.ran")
                .expect("hard-coded key parses"),
        )
        .with_param("name", DiagnosticParam::String(req.name.clone()))
        .with_param("rows", DiagnosticParam::I64(i64::from(row_count)));

        let response = AnalyticsQueryResponse {
            summary,
            name: req.name,
            rows,
            row_count,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

/// Resolve a template name to its SQL body, or return the
/// `unknown_template` invalid error. Pulled out so the unit tests
/// can assert the closed catalogue without standing up a client.
pub(crate) fn lookup_template(name: &str) -> Result<&'static str> {
    let filename = format!("{name}.sql");
    match TEMPLATES.get_file(&filename) {
        Some(file) => file.contents_utf8().ok_or_else(|| Error::Internal {
            source: format!("template {name} is not valid UTF-8").into(),
        }),
        None => {
            let key = MessageKey::parse("rubix.analytics.query.unknown_template")
                .expect("hard-coded key parses");
            Err(Error::Invalid {
                message: format!("{}: name={name}", key.as_str()),
            })
        }
    }
}

/// Run `sql` with `params` bound through CH's native `{name:Type}`
/// parameter syntax. Failures (parse/bind/connection) surface as
/// `rubix.analytics.query.bind_error` so callers see one stable
/// MessageKey rather than the driver's free-form text.
pub(crate) async fn run_query(
    client: &ChClient,
    sql: &str,
    params: &std::collections::BTreeMap<String, Value>,
) -> Result<Vec<Value>> {
    let mut q = client.inner().query(sql);
    for (k, v) in params {
        q = q.param(k, v);
    }

    // `JSONEachRow` returns one JSON object per row, newline-
    // separated. The driver streams it through `BytesCursor`; we
    // collect once because the row count is bounded by the template
    // (weekly windows over rubix-sized warehouses are kilobytes).
    let bytes = q
        .fetch_bytes("JSONEachRow")
        .map_err(bind_error)?
        .collect()
        .await
        .map_err(bind_error)?;

    let mut rows = Vec::new();
    for line in bytes.split(|b| *b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let row: Value = serde_json::from_slice(line).map_err(|e| Error::Internal {
            source: Box::new(e),
        })?;
        rows.push(row);
    }
    Ok(rows)
}

/// Map a driver-level error into the stable bind-error MessageKey.
/// Takes `impl Display` so this file does not need to depend on
/// `clickhouse` directly — the concrete type travels through
/// `starter-store-clickhouse::ChClient::inner()` and never appears
/// in our signatures.
fn bind_error<E: std::fmt::Display>(err: E) -> Error {
    let key = MessageKey::parse("rubix.analytics.query.bind_error")
        .expect("hard-coded key parses");
    Error::Invalid {
        message: format!("{}: {err}", key.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_templates_contains_all_bundled_names() {
        let mut names = known_templates_sorted();
        names.sort();
        assert_eq!(
            names,
            vec![
                "alert_count_weekly",
                "clickhouse_writes_weekly",
                "disk_history_weekly",
                "flow_run_summary_weekly",
                "meter_kwh_last_24h",
                "meter_litres_last_24h",
                "undo_count_weekly",
                "user_activity_weekly",
            ],
            "Phase C + Stage 05 ship eight named templates",
        );
    }

    #[test]
    fn lookup_template_unknown_name_returns_invalid_with_messagekey() {
        let err = lookup_template("not_a_template").unwrap_err();
        let msg = match err {
            Error::Invalid { message } => message,
            other => panic!("expected Invalid, got {other:?}"),
        };
        assert!(
            msg.contains("rubix.analytics.query.unknown_template"),
            "msg: {msg}"
        );
    }

    #[test]
    fn lookup_template_known_name_returns_non_empty_sql() {
        let sql = lookup_template("disk_history_weekly").unwrap();
        assert!(
            sql.to_ascii_uppercase().contains("SELECT"),
            "template body must be a SELECT; got: {sql}"
        );
    }

    fn known_templates_sorted() -> Vec<&'static str> {
        AnalyticsQueryTool::known_templates()
    }
}
