//! `POST /api/v1/ai/assist` — synchronous, task-typed AI assistance.
//!
//! Turns a plain-English intent (plus optional datasource schema grounding and
//! existing SQL) into a single concrete artifact: a SQL string, a panel spec, or
//! a dashboard spec. Unlike agent sessions this does not stream or persist — it
//! is one inference round-trip returning structured JSON, backing the query
//! editor's "write SQL for me" and the dashboard builder's "suggest panels".
//!
//! Grounding: when `datasource_id` is set the caller must be able to `view` it
//! (same gate as the schema route); its table/column catalogue is introspected
//! and fed to the model so generated SQL references real columns. The model is
//! asked to answer as JSON only; the reply is parsed leniently (fenced code
//! blocks tolerated) and the raw text is returned alongside for fallback.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_ai::ModelRef;
use nexus_spi::dto::ai::{AssistRequest, AssistResponse, AssistTask};
use serde_json::{json, Value};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::authz::{self, ACTION_VIEW, KIND_DATASOURCE};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/ai/assist",
    tag = "ai",
    operation_id = "ai_assist",
    request_body = AssistRequest,
    responses(
        (status = 200, description = "Structured assistance", body = AssistResponse),
        (status = 400, description = "Empty prompt"),
        (status = 403, description = "Not allowed to view the grounding datasource"),
        (status = 502, description = "The model call failed"),
    ),
)]
pub async fn ai_assist(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<AssistRequest>,
) -> axum::response::Response {
    let (caller_principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if req.prompt.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "prompt must not be empty").into_response();
    }

    // Ground with the datasource schema when one is named — gated on `view`.
    let schema_text = match req.datasource_id.as_deref() {
        Some(id_str) => match ground_schema(&state, caller_principal, &tenant, id_str).await {
            Ok(text) => Some(text),
            Err(resp) => return resp,
        },
        None => None,
    };

    let system = system_prompt(req.task, schema_text.as_deref(), req.current_sql.as_deref());
    let model = req
        .model
        .as_deref()
        .map(parse_model)
        .unwrap_or_else(ModelRef::medium);
    // Default to the CLI tier so assist works without a provider key wherever a
    // coding-agent CLI is logged in (the project's primary mode).
    let backend = req.backend.as_deref().unwrap_or("claude");

    let reply = match state
        .sessions
        .chat_once(backend, model, Some(system), req.prompt.clone())
        .await
    {
        Ok(text) => text,
        Err(e) => return (StatusCode::BAD_GATEWAY, format!("model call failed: {e}")).into_response(),
    };

    let result = parse_result(req.task, &reply);
    Json(AssistResponse {
        task: req.task,
        result,
        raw: Some(reply),
    })
    .into_response()
}

/// Introspect the named datasource (gated on `view`) into a compact
/// `table(col type, …)` listing for the model. Returns a ready `Response` on the
/// error paths so the handler can early-return it.
#[allow(clippy::result_large_err)]
async fn ground_schema(
    state: &AppState,
    caller_principal: &Principal,
    tenant: &str,
    id_str: &str,
) -> Result<String, axum::response::Response> {
    let id = Uuid::parse_str(id_str)
        .map_err(|_| (StatusCode::BAD_REQUEST, "malformed datasource_id").into_response())?;
    let rec = match nexus_store::datasource::get(&state.metadata, tenant, id).await {
        Ok(Some(rec)) => rec,
        Ok(None) => return Err((StatusCode::NOT_FOUND, "datasource not found").into_response()),
        Err(e) => return Err(IntoResponse(e).into_response()),
    };
    authz::require(
        state.engine.as_ref(),
        caller_principal,
        ACTION_VIEW,
        KIND_DATASOURCE,
        &rec.id.to_string(),
        tenant,
    )
    .await?;

    let pool = state
        .datasource_pools
        .get_or_connect(
            &state.metadata,
            &state.envelope,
            tenant,
            &caller_principal.subject,
            &rec,
        )
        .await
        .map_err(|e| IntoResponse(e).into_response())?;
    let schema = nexus_store::introspect(&pool, state.guards)
        .await
        .map_err(|e| IntoResponse(e).into_response())?;

    // Render a token-frugal schema: one line per table, columns inline. The AI
    // assist prompt only needs table/column names, so FK relations are ignored.
    let mut out = String::new();
    for t in &schema.tables {
        let cols = t
            .columns
            .iter()
            .map(|c| format!("{} {}", c.name, c.data_type))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("{}({})\n", t.name, cols));
    }
    Ok(out)
}

/// Task-specific system instructions. Each task pins the output to a strict JSON
/// shape so [`parse_result`] can lift it; the schema and existing SQL, when
/// present, are appended as grounding.
fn system_prompt(task: AssistTask, schema: Option<&str>, current_sql: Option<&str>) -> String {
    let mut s = String::new();
    match task {
        AssistTask::Sql => {
            s.push_str(
                "You are a SQL assistant for a PostgreSQL analytics database. Given the user's \
                 request, return ONLY a JSON object of the form {\"sql\": \"<query>\"}. The SQL \
                 must be a single read-only SELECT. Do not include explanation outside the JSON.",
            );
        }
        AssistTask::Panel => {
            s.push_str(
                "You design dashboard panels over a PostgreSQL analytics database. Return ONLY a \
                 JSON object {\"title\": \"…\", \"viz\": \"line|area|bar|stat|gauge|table|pie\", \
                 \"sql\": \"<read-only SELECT>\", \"x\": \"<time/category column or null>\", \
                 \"value\": \"<numeric column>\"}. No prose outside the JSON.",
            );
        }
        AssistTask::Dashboard => {
            s.push_str(
                "You design dashboards over a PostgreSQL analytics database. Return ONLY a JSON \
                 object {\"name\": \"…\", \"panels\": [{\"title\", \"viz\", \"sql\", \"x\", \
                 \"value\"}, …]} with 2–6 panels covering the user's request. Each sql is a \
                 single read-only SELECT. No prose outside the JSON.",
            );
        }
    }
    if let Some(schema) = schema.filter(|s| !s.trim().is_empty()) {
        s.push_str("\n\nDatabase schema (table(column type, …)):\n");
        s.push_str(schema.trim_end());
    }
    if let Some(sql) = current_sql.filter(|s| !s.trim().is_empty()) {
        s.push_str("\n\nThe user's current SQL to edit/improve:\n");
        s.push_str(sql.trim());
    }
    s
}

/// Lift the model's structured JSON from its reply. Tolerates a ```json fenced
/// block and surrounding whitespace. On a parse miss, falls back to a best-effort
/// shape so the UI always gets something usable (for `Sql`, the raw text as the
/// query; otherwise an empty object).
fn parse_result(task: AssistTask, reply: &str) -> Value {
    if let Some(v) = extract_json(reply) {
        return v;
    }
    match task {
        AssistTask::Sql => json!({ "sql": strip_fences(reply).trim() }),
        _ => json!({}),
    }
}

/// Find the first JSON object in `text`, tolerating a fenced code block.
fn extract_json(text: &str) -> Option<Value> {
    let cleaned = strip_fences(text);
    // Try the whole cleaned string first, then the first {...} span.
    if let Ok(v) = serde_json::from_str::<Value>(cleaned.trim()) {
        return Some(v);
    }
    let start = cleaned.find('{')?;
    let end = cleaned.rfind('}')?;
    if end > start {
        serde_json::from_str::<Value>(&cleaned[start..=end]).ok()
    } else {
        None
    }
}

/// Strip a leading ```lang fence and trailing ``` if present.
fn strip_fences(text: &str) -> String {
    let t = text.trim();
    if let Some(rest) = t.strip_prefix("```") {
        // Drop the language tag up to the first newline, and the trailing fence.
        let rest = rest.split_once('\n').map(|x| x.1).unwrap_or(rest);
        rest.trim_end().strip_suffix("```").unwrap_or(rest).to_string()
    } else {
        t.to_string()
    }
}

/// A size alias resolves to that tier; anything else is a concrete model id.
fn parse_model(model: &str) -> ModelRef {
    match model {
        "small" => ModelRef::small(),
        "medium" => ModelRef::medium(),
        "large" => ModelRef::large(),
        other => ModelRef::concrete(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_plain_json() {
        let v = extract_json(r#"{"sql":"SELECT 1"}"#).unwrap();
        assert_eq!(v["sql"], "SELECT 1");
    }

    #[test]
    fn extracts_fenced_json() {
        let reply = "```json\n{\"sql\": \"SELECT 2\"}\n```";
        let v = extract_json(reply).unwrap();
        assert_eq!(v["sql"], "SELECT 2");
    }

    #[test]
    fn extracts_json_with_surrounding_prose() {
        let reply = "Here you go:\n{\"sql\": \"SELECT 3\"}\nHope that helps.";
        let v = extract_json(reply).unwrap();
        assert_eq!(v["sql"], "SELECT 3");
    }

    #[test]
    fn sql_falls_back_to_raw_when_unparseable() {
        let v = parse_result(AssistTask::Sql, "SELECT 4 FROM t");
        assert_eq!(v["sql"], "SELECT 4 FROM t");
    }
}
