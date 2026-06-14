//! `POST /api/v1/insights/preview` — run an inline script over sample rows.
//!
//! LAYER: transport (REST). Extract → validate → call domain → shape DTO → return.
//! No SQL, no business predicates here. See docs/design/layering/.
//!
//! Powers the authoring workbench's instant edit→result loop: run a draft script
//! against a small sample of rows and return the transformed result shaped like a
//! `QueryResponse`, without saving anything. A *script* error (compile, runtime,
//! or a tripped sandbox limit) is not an HTTP failure — it comes back as
//! `ok: false` with HTTP 200 so the UI shows it inline. Only auth and a malformed
//! request (e.g. too many sample rows) are true HTTP errors.

use std::time::Instant;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_insights::InsightError;
use nexus_spi::dto::insight::{PreviewInsightRequest, PreviewInsightResponse};
use serde_json::Value;
use starter_spi::auth::Principal;

use crate::insights::reshape;
use crate::middleware::tenant::caller;
use crate::state::AppState;

/// The workbench samples small; a larger payload is a client mistake, rejected up
/// front rather than handed to the sandbox.
const MAX_SAMPLE_ROWS: usize = 10_000;

#[utoipa::path(
    post,
    path = "/api/v1/insights/preview",
    tag = "insights",
    operation_id = "preview_insight",
    request_body = PreviewInsightRequest,
    responses(
        (status = 200, description = "Preview result — `ok:true` with the transformed \
            result, or `ok:false` with a script error", body = PreviewInsightResponse),
        (status = 400, description = "Too many sample rows"),
        (status = 401, description = "Unauthenticated"),
    ),
)]
pub async fn preview_insight(
    State(_state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<PreviewInsightRequest>,
) -> axum::response::Response {
    // Auth is kept consistent with the rest of the vertical even though an inline
    // preview touches no tenant data: a valid, tenant-bound principal is required.
    if let Err(resp) = caller(&principal) {
        return resp;
    }

    if req.rows.len() > MAX_SAMPLE_ROWS {
        return (
            StatusCode::BAD_REQUEST,
            format!("too many sample rows (max {MAX_SAMPLE_ROWS})"),
        )
            .into_response();
    }

    let row_count_in = req.rows.len() as u64;
    let params = req.params.unwrap_or(Value::Null);
    let started = Instant::now();

    match nexus_insights::run_insight_rows(req.script, req.rows, params).await {
        Ok(rows) => {
            let elapsed_ms = started.elapsed().as_millis() as u64;
            // Shape identically to the query path so the existing ResultGrid
            // renders the preview with no special-casing. A preview never
            // truncates upstream, so `truncated` is always false here.
            let result = reshape(rows, elapsed_ms, false);
            Json(PreviewInsightResponse::ok(result, row_count_in)).into_response()
        }
        Err(e) => {
            // A script error is an inline result, not a failed request: HTTP 200
            // with `ok:false` so the workbench shows it in the result pane.
            let (kind, message) = classify(e);
            Json(PreviewInsightResponse::err(kind, message)).into_response()
        }
    }
}

/// Map an [`InsightError`] to the workbench's `(kind, message)` pair. Compile and
/// limit faults are surfaced distinctly; everything else (a script's logic error,
/// or an internal engine fault) is a `runtime` catch-all carrying the full
/// message, so the UI always has something actionable to show.
fn classify(err: InsightError) -> (&'static str, String) {
    match err {
        InsightError::Compile(m) => ("compile", m),
        InsightError::LimitExceeded(m) => ("limit", m),
        InsightError::Runtime(m) => ("runtime", m),
        InsightError::Engine(m) => ("runtime", m),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_script_that_does_not_compile_is_ok_false_not_an_error() {
        // `run_insight_rows` is the same entry the handler calls; exercise the
        // error-shaping directly so the test needs no HTTP/auth harness.
        let err = nexus_insights::run_insight_rows(
            "this is ::: not rhai".to_string(),
            vec![],
            Value::Null,
        )
        .await
        .expect_err("a non-compiling script must error");
        let (kind, message) = classify(err);
        assert_eq!(kind, "compile");
        assert!(!message.is_empty());

        let resp = PreviewInsightResponse::err(kind, message);
        match resp {
            PreviewInsightResponse::Err { ok, error } => {
                assert!(!ok);
                assert_eq!(error.kind, "compile");
            }
            PreviewInsightResponse::Ok { .. } => panic!("expected an error variant"),
        }
    }

    #[tokio::test]
    async fn a_passthrough_script_returns_ok_with_the_input_rows() {
        let rows = vec![serde_json::json!({ "a": 1 }), serde_json::json!({ "a": 2 })];
        let out = nexus_insights::run_insight_rows("df".to_string(), rows.clone(), Value::Null)
            .await
            .expect("identity script runs");
        let result = reshape(out, 0, false);
        let resp = PreviewInsightResponse::ok(result, rows.len() as u64);
        match resp {
            PreviewInsightResponse::Ok {
                ok, row_count_in, ..
            } => {
                assert!(ok);
                assert_eq!(row_count_in, 2);
            }
            PreviewInsightResponse::Err { .. } => panic!("expected the ok variant"),
        }
    }
}
