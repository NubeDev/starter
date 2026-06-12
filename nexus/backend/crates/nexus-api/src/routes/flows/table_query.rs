//! `POST /api/v1/flows/{id}/table/query` — query a flow's sink table.
//!
//! LAYER: transport (REST). Extract → resolve the flow's sink connection → run a
//! read-only query through the store guards → return the rows.
//!
//! The flow owns its output table, so the Workbench can show what actually
//! landed without the user resolving a datasource or retyping the table name. We
//! read the flow's `output` config, open a pool on its sink connection, and run
//! the query through the same read-only path as `/api/v1/query` (the
//! `SET TRANSACTION READ ONLY` boundary rejects any write/DDL). A `{table}` token
//! in the SQL expands to the flow's configured table.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::flow::FlowTableQueryRequest;
use nexus_spi::dto::query::QueryResponse;
use nexus_store::flow;
use nexus_store::query::run_query;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::authz::{self, ACTION_VIEW, KIND_FLOW};
use crate::middleware::tenant::caller;
use crate::state::AppState;

/// The hard cap on a preview's row count; the request `limit` is clamped to it.
const PREVIEW_MAX_ROWS: u64 = 500;

#[utoipa::path(
    post,
    path = "/api/v1/flows/{id}/table/query",
    tag = "flows",
    operation_id = "query_flow_table",
    params(("id" = Uuid, Path, description = "Flow id")),
    request_body = FlowTableQueryRequest,
    responses(
        (status = 200, description = "Query result", body = QueryResponse),
        (status = 400, description = "Flow has no queryable table, or the query was rejected", body = nexus_spi::Problem),
        (status = 403, description = "Not allowed to view this flow"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn query_flow_table(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<FlowTableQueryRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let rec = match flow::get(&state.metadata, &tenant, id).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_VIEW,
        KIND_FLOW,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }

    // Resolve a `datasource`-typed sink to its connection material first (audited
    // decrypt), so both a raw `postgres` sink and a datasource-backed one expose
    // a table + connection here. Any other output passes through unchanged.
    let output = match crate::datasource_kinds::resolve_flow_output(
        &state,
        &tenant,
        &caller.subject,
        rec.output.clone(),
    )
    .await
    {
        Ok(o) => o,
        Err(e) => return IntoResponse(e).into_response(),
    };

    let (uri, table) = match sink_table_target(&output) {
        Ok(t) => t,
        Err(message) => return (StatusCode::BAD_REQUEST, message).into_response(),
    };

    // Open a bounded read-only pool on the sink connection. A cache could live
    // here later (mirroring DatasourcePools); a preview is low-frequency, so a
    // small dedicated pool closed at end of request is fine for now.
    let pool = match PgPoolOptions::new().max_connections(2).connect(&uri).await {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("could not connect to the flow's sink: {e}"),
            )
                .into_response()
        }
    };

    let sql = resolve_sql(req.sql.as_deref(), &table, req.limit);
    let mut guards = state.guards;
    guards.max_rows = guards.max_rows.min(req.limit.unwrap_or(PREVIEW_MAX_ROWS));

    let result = run_query(&pool, &sql, guards).await;
    pool.close().await;

    match result {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

/// The sink connection URI + table for a flow whose output writes to Postgres.
/// A raw `postgres` sink carries a flat `uri`; a resolved `datasource` sink
/// carries a `conn` object. Any other sink has no queryable table.
fn sink_table_target(output: &Value) -> Result<(String, String), String> {
    let kind = output.get("type").and_then(Value::as_str).unwrap_or("");
    let table = output
        .get("table")
        .and_then(Value::as_str)
        .ok_or_else(|| "this flow's sink has no table to query".to_string())?
        .to_string();
    match kind {
        "postgres" => {
            let uri = output
                .get("uri")
                .and_then(Value::as_str)
                .ok_or_else(|| "postgres sink is missing its connection uri".to_string())?
                .to_string();
            Ok((uri, table))
        }
        "datasource" => {
            // resolve_flow_output produced a `conn` object for a datasource sink.
            let conn = output
                .get("conn")
                .ok_or_else(|| "resolved datasource sink is missing its connection".to_string())?;
            let uri = conn_to_uri(conn)?;
            Ok((uri, table))
        }
        other => Err(format!(
            "flow sink '{other}' has no queryable table (only postgres/datasource sinks store rows)"
        )),
    }
}

/// Assemble a Postgres URI from a resolved datasource `conn` object.
fn conn_to_uri(conn: &Value) -> Result<String, String> {
    let get = |k: &str| conn.get(k).and_then(Value::as_str);
    let host = get("host").ok_or("conn missing host")?;
    let port = conn
        .get("port")
        .and_then(Value::as_u64)
        .ok_or("conn missing port")?;
    let db = get("database").ok_or("conn missing database")?;
    let user = get("user").ok_or("conn missing user")?;
    let password = get("password").unwrap_or("");
    Ok(format!("postgres://{user}:{password}@{host}:{port}/{db}"))
}

/// The SQL to run: the caller's `sql` with `{table}` expanded, or a default
/// "most recent rows" preview. `{table}` is the flow's own configured table name
/// (a vetted identifier from the flow config, never request input), so expanding
/// it into the text is safe; row values are still bound by the query path.
fn resolve_sql(sql: Option<&str>, table: &str, limit: Option<u64>) -> String {
    let n = limit.unwrap_or(50).min(PREVIEW_MAX_ROWS);
    match sql {
        Some(s) if !s.trim().is_empty() => s.replace("{table}", &quote_ident(table)),
        _ => format!("SELECT * FROM {} LIMIT {n}", quote_ident(table)),
    }
}

/// Double-quote a Postgres identifier, escaping embedded quotes — so a table
/// name is a single safe identifier token even if it contains odd characters.
fn quote_ident(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn postgres_sink_yields_uri_and_table() {
        let out = json!({"type":"postgres","uri":"postgres://u:p@h:5432/db","table":"telemetry"});
        let (uri, table) = sink_table_target(&out).unwrap();
        assert_eq!(uri, "postgres://u:p@h:5432/db");
        assert_eq!(table, "telemetry");
    }

    #[test]
    fn datasource_sink_builds_uri_from_conn() {
        let out = json!({"type":"datasource","table":"t","conn":{
            "host":"127.0.0.1","port":4770,"database":"nexus","user":"nexus","password":"nexus"}});
        let (uri, table) = sink_table_target(&out).unwrap();
        assert_eq!(uri, "postgres://nexus:nexus@127.0.0.1:4770/nexus");
        assert_eq!(table, "t");
    }

    #[test]
    fn non_table_sink_is_rejected() {
        assert!(sink_table_target(&json!({"type":"sse"})).is_err());
        assert!(sink_table_target(&json!({"type":"drop","table":"x"})).is_err());
    }

    #[test]
    fn default_preview_query_uses_quoted_table_and_limit() {
        assert_eq!(
            resolve_sql(None, "telemetry", Some(20)),
            "SELECT * FROM \"telemetry\" LIMIT 20"
        );
        assert_eq!(
            resolve_sql(Some("  "), "t", None),
            "SELECT * FROM \"t\" LIMIT 50"
        );
    }

    #[test]
    fn table_token_expands_in_custom_sql() {
        let sql = resolve_sql(Some("SELECT count(*) FROM {table}"), "telemetry", None);
        assert_eq!(sql, "SELECT count(*) FROM \"telemetry\"");
    }

    #[test]
    fn limit_clamps_to_preview_max() {
        let sql = resolve_sql(None, "t", Some(99_999));
        assert!(sql.ends_with(&format!("LIMIT {PREVIEW_MAX_ROWS}")));
    }
}
