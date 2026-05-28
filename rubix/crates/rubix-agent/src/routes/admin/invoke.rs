//! `POST /api/v1/admin/registry/tools/{id}/invoke` — admin tool dispatch.
//!
//! LAYER: transport (REST). The handler validates the
//! tenant-required request body, scopes a [`CallerIdentity`] task-
//! local for the dispatch (so extension-backed tools call as the
//! target tenant rather than the admin's own session), then maps
//! the [`Tool::invoke`] result onto an HTTP response.
//!
//! ## Security
//!
//! Mounted under `with_scope("admin:invoke")` in addition to the
//! `Role::Admin` gate that wraps the read surface — an
//! `admin:read`-only principal can browse but cannot fire tools.
//! See [docs/design/admin/](../../../../docs/design/admin/README.md)
//! §"Security model".
//!
//! ## Tenant
//!
//! `body.tenant` is mandatory. The admin's own `Principal.tenant_id`
//! is *not* used as the dispatch tenant — admins are expected to
//! act as a target tenant explicitly. A 400 fires when the field
//! is absent, blank, or whitespace-only.
//!
//! ## Audit
//!
//! Every invoke emits a structured `tracing::info!` line with
//! target `rubix.admin.invoke` carrying the actor, target tenant,
//! tool id, latency, and result status. Persistent audit (writing
//! to `starter-audit` / `starter-changelog`) is wired by mounting
//! this router under `middleware::changelog_layer` in `main.rs`
//! when an `Arc<dyn ChangeRecorder>` is available; the tracing
//! line is the always-on minimum.

use std::time::Instant;

use axum::extract::{Path, State};
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Extension, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use starter_ext_spi::identity::CallerIdentity;
use starter_spi::auth::Principal;
use starter_spi::error::Error;
use tracing::info;

use crate::admin::AdminState;
use crate::routes::{RouteMeta, RouteRegistrar};

/// Build the invoke sub-registrar.
///
/// Mounted separately from the read registrar so a deployment
/// can gate it behind an additional scope (`admin:invoke`)
/// without affecting browse access.
pub fn admin_invoke_registrar(state: AdminState) -> RouteRegistrar {
    RouteRegistrar::new().mount(
        Method::POST,
        "/api/v1/admin/registry/tools/{tool_id}/invoke",
        post(invoke).with_state(state),
        RouteMeta::new()
            .describe(
                "Synchronously dispatch a registered tool as the supplied tenant.",
            )
            .tag("admin")
            .request_schema(json!({
                "type": "object",
                "required": ["tenant"],
                "properties": {
                    "tenant": { "type": "string", "minLength": 1 },
                    "input":  { "description": "Tool-specific input; schema published at GET /api/v1/admin/registry/tools/{id}." }
                }
            })),
    )
}

/// Request body for the admin invoke endpoint.
#[derive(Debug, Deserialize)]
struct InvokeBody {
    /// Target tenant the tool is dispatched as. Required at the
    /// handler level — declared `Option<String>` here only so the
    /// 400 path runs through our explicit `bad_request(...)`
    /// helper instead of bouncing off axum's default 422 for a
    /// missing field. The admin's session principal is the
    /// *actor*, not the tenant scope.
    #[serde(default)]
    tenant: Option<String>,
    /// Raw input forwarded to [`Tool::invoke`]. Schema validation
    /// is the tool's responsibility (the wire schema is published
    /// at `GET /api/v1/admin/registry/tools/{id}`).
    #[serde(default)]
    input: Value,
}

async fn invoke(
    State(state): State<AdminState>,
    Path(tool_id): Path<String>,
    principal: Option<Extension<Principal>>,
    Json(body): Json<InvokeBody>,
) -> Response {
    let tenant = body.tenant.unwrap_or_default().trim().to_owned();
    if tenant.is_empty() {
        return bad_request("`tenant` is required and must be a non-empty string");
    }
    let Some(tool) = state.tools.get(&tool_id).cloned() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "unknown tool", "tool_id": tool_id})),
        )
            .into_response();
    };
    let actor_subject = principal
        .as_ref()
        .map(|Extension(p)| p.subject.clone())
        .unwrap_or_else(|| "anonymous".to_owned());
    let caller = CallerIdentity {
        tenant_id: Some(tenant.clone()),
        user_id: Some(actor_subject.clone()),
        roles: vec!["admin".to_owned()],
        request_id: String::new(),
    };
    let actor = starter_spi::changelog::Actor::User {
        subject: actor_subject.clone(),
    };
    let started = Instant::now();
    let result = starter_ext_supervisor::caller_local::scope(
        caller,
        starter_undo::actor_local::scope(actor, tool.invoke(body.input)),
    )
    .await;
    let latency_ms = started.elapsed().as_millis();
    audit(&actor_subject, &tenant, &tool_id, &result, latency_ms);
    shape_response(result)
}

fn audit(
    actor: &str,
    tenant: &str,
    tool_id: &str,
    result: &Result<Value, Error>,
    latency_ms: u128,
) {
    let status = match result {
        Ok(_) => "ok",
        Err(e) => match e {
            Error::NotFound { .. } => "not_found",
            Error::Invalid { .. } => "invalid",
            Error::Unauthenticated => "unauthenticated",
            Error::Forbidden => "forbidden",
            Error::Conflict { .. } => "conflict",
            Error::Unavailable { .. } => "unavailable",
            Error::Internal { .. } => "internal",
            _ => "other",
        },
    };
    info!(
        target: "rubix.admin.invoke",
        actor = %actor,
        tenant = %tenant,
        tool_id = %tool_id,
        status = %status,
        latency_ms = latency_ms as u64,
        "admin tool invoke",
    );
}

fn shape_response(result: Result<Value, Error>) -> Response {
    match result {
        Ok(body) => (StatusCode::OK, Json(body)).into_response(),
        Err(Error::Unavailable {
            code,
            subject,
            message,
        }) => {
            // Mirrors `routes::tools::shape_response`: surface
            // recoverable supervisor death as 503 with a restart
            // hint instead of an opaque 500 the admin UI cannot act
            // on. The same wire shape is consumed by the chat-side
            // and admin consoles.
            let mut body = json!({
                "error":   message,
                "code":    code,
                "subject": subject,
            });
            if code == "extension.supervisor_unavailable" {
                if let Some(id) = subject.as_deref().filter(|s| !s.is_empty()) {
                    body["restart"] = json!({
                        "method": "POST",
                        "path":   format!("/extensions/{id}/restart"),
                    });
                }
            }
            (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
        }
        Err(e) => {
            let status = match &e {
                Error::NotFound { .. } => StatusCode::NOT_FOUND,
                Error::Invalid { .. } => StatusCode::BAD_REQUEST,
                Error::Unauthenticated => StatusCode::UNAUTHORIZED,
                Error::Forbidden => StatusCode::FORBIDDEN,
                Error::Conflict { .. } => StatusCode::CONFLICT,
                Error::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status, Json(json!({"error": e.to_string()}))).into_response()
        }
    }
}

fn bad_request(message: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error": "bad_request", "message": message})),
    )
        .into_response()
}
