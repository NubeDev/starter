//! `POST /api/v1/flows/{flow_id}/run` — synchronous human-driven
//! flow invocation.
//!
//! Stage-07 contract: REST for humans, MCP for AI. An operator firing
//! a flow ad-hoc (the same one the dashboard chat tab or the cron
//! scheduler would otherwise fire) hits this endpoint. The body shape
//! mirrors `mcp.tools/call`'s `arguments`:
//!
//! ```json
//! { "input": { "prompt": "build me a disk overview dashboard" } }
//! ```
//!
//! The handler resolves `flow_id` against the shared
//! [`ToolRegistry`] (every bundled flow auto-surfaces as a
//! [`FlowAsTool`] via `boot::mcp::build_tool_registry`), invokes it,
//! and returns the terminal-slot JSON verbatim under `output`.
//!
//! `ai-agent` rooted flows produce an `output` carrying at least the
//! agent's `reply` narration field — stage 07's BANANA test greps
//! that field. Richer per-step events (`Text`, `ToolUse`,
//! `ToolResult` from inside the wrapped CLI run) are not projected
//! through the engine event bus today; that bridging is a separate
//! follow-up.
//!
//! Auth/CSRF: mounted inside `with_principal` in `main.rs` when a
//! database is configured; without a DSN the laptop dev path leaves
//! it ungated alongside the tools router.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use starter_mcp::registry::ToolRegistry;
use starter_flow_spi::flow::FlowId;
use tracing::warn;

/// State threaded into the handler.
#[derive(Clone)]
pub struct FlowRunState {
    /// Shared registry — same `Arc<ToolRegistry>` the MCP surface
    /// dispatches against. Every bundled flow is registered as a
    /// `FlowAsTool` whose `definition().name` is the reverse-DNS
    /// flow id (`com.rubix.dashboard-assistant`, …).
    pub tools: Arc<ToolRegistry>,
}

/// POST body. `input` is forwarded verbatim as the flow's seed payload.
#[derive(Debug, Deserialize, Default)]
pub struct RunBody {
    #[serde(default)]
    pub input: Value,
}

/// Successful response shape.
#[derive(Debug, Serialize)]
pub struct RunResponse {
    pub flow_id: String,
    pub output: Value,
}

/// Build the router. Mount under the app root; the path is fully
/// qualified.
pub fn router(state: FlowRunState) -> Router {
    Router::new()
        .route("/api/v1/flows/{flow_id}/run", post(run))
        .with_state(state)
}

async fn run(
    State(state): State<FlowRunState>,
    Path(flow_id_raw): Path<String>,
    body: Option<Json<RunBody>>,
) -> axum::response::Response {
    // Validate reverse-DNS shape up front — mirrors the SSE
    // `/events` route so a typo lands as a clean 400 instead of a
    // misleading 404 from the registry lookup.
    let flow_id = match FlowId::new(&flow_id_raw) {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("invalid flow_id: {e}") })),
            )
                .into_response();
        }
    };

    let Some(tool) = state.tools.get(flow_id.as_str()) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("no flow registered under `{flow_id}`") })),
        )
            .into_response();
    };

    // `FlowAsTool::invoke` expects the *whole* call payload, not just
    // the `input` sub-object. The MCP tools/call surface forwards
    // `arguments` verbatim; mirror that shape so the seed adapter's
    // `payload.get("input")` lookup keeps working unchanged.
    let payload = body
        .map(|Json(b)| b.input)
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let invoke_input = json!({ "input": payload });

    match tool.invoke(invoke_input).await {
        Ok(output) => Json(RunResponse {
            flow_id: flow_id.to_string(),
            output,
        })
        .into_response(),
        Err(e) => {
            warn!(
                target: "rubix.routes.flow_run",
                flow_id = %flow_id,
                error = %e,
                "flow invocation failed"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string(), "flow_id": flow_id.to_string() })),
            )
                .into_response()
        }
    }
}
