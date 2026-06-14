//! REST + SSE surface (P2, DOCS §11). Thin adapters over [`RunService`];
//! all under `/setup`. REST for humans/mobile, MCP for AI.
//!
//! The team check (DOCS §10 step 2) runs in the `run` handler *after* the
//! generic authz gate — it is the data-dependent predicate the condition
//! engine cannot express. Route-level coarse gates are applied by the
//! caller via `starter_authz::with_permission` when mounting; the
//! per-row owner/team checks happen in the handlers.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use starter_flow_spi::flow::RunId;
use starter_spi::auth::Principal;

use crate::authz::team_check;
use crate::import::{export_template_yaml, import_template_yaml};
use crate::service::RunService;
use starter_setup_spi::error::SetupError;
use starter_setup_spi::model::{SemVer, TemplateId, TemplateSource};
use starter_setup_spi::store::{SetupRunFilter, TemplateFilter, TemplateStore};

/// Build the `/setup` router over a shared [`RunService`]. Mount under
/// `/api/v1` in the host. Coarse authz gates (`setup.templates/*`,
/// `setup.runs/*`) are layered by the caller with
/// `starter_authz::with_permission`; this router carries the handlers +
/// the row-level owner/team checks.
pub fn router<S, TS, RS>(service: Arc<RunService<TS, RS>>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    TS: TemplateStore,
    RS: starter_setup_spi::store::SetupRunStore,
{
    Router::new()
        .route("/setup/templates", get(list_templates).post(create_template))
        .route("/setup/templates/import", post(import_template))
        .route(
            "/setup/templates/{id}",
            get(get_template).delete(delete_template),
        )
        .route("/setup/templates/{id}/run", post(run_template))
        .route("/setup/runs", get(list_runs))
        .route("/setup/runs/{id}", get(get_run))
        .route("/setup/runs/{id}/events", get(run_events))
        .route("/setup/runs/{id}/resume", post(resume_run))
        .route("/setup/runs/{id}/cancel", post(cancel_run))
        .with_state(service)
}

// ---- error → response -------------------------------------------------

fn err_response(e: SetupError) -> Response {
    let code = match &e {
        SetupError::NotFound(_) => StatusCode::NOT_FOUND,
        SetupError::Forbidden(_) => StatusCode::FORBIDDEN,
        SetupError::InvalidInput(_)
        | SetupError::InvalidYaml(_)
        | SetupError::InvalidBody(_)
        | SetupError::InvalidBinding(_)
        | SetupError::InvalidVersion(_)
        | SetupError::InvalidRunState(_) => StatusCode::BAD_REQUEST,
        SetupError::Backend(_) => StatusCode::INTERNAL_SERVER_ERROR,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (code, e.to_string()).into_response()
}

fn unauth() -> Response {
    (StatusCode::UNAUTHORIZED, "unauthenticated").into_response()
}

// ---- query params -----------------------------------------------------

#[derive(serde::Deserialize)]
struct FormatQuery {
    format: Option<String>,
    version: Option<String>,
}

// ---- templates --------------------------------------------------------

async fn list_templates<TS, RS>(
    State(svc): State<Arc<RunService<TS, RS>>>,
    principal: Option<Extension<Principal>>,
) -> Response
where
    TS: TemplateStore,
    RS: starter_setup_spi::store::SetupRunStore,
{
    let Some(Extension(p)) = principal else {
        return unauth();
    };
    let filter = TemplateFilter {
        tenant_id: p.tenant_id.clone(),
        category: None,
    };
    match svc.templates().list(filter).await {
        Ok(list) => Json(list).into_response(),
        Err(e) => err_response(e),
    }
}

async fn get_template<TS, RS>(
    State(svc): State<Arc<RunService<TS, RS>>>,
    Path(id): Path<String>,
    Query(q): Query<FormatQuery>,
    principal: Option<Extension<Principal>>,
) -> Response
where
    TS: TemplateStore,
    RS: starter_setup_spi::store::SetupRunStore,
{
    let Some(Extension(p)) = principal else {
        return unauth();
    };
    let version = match q.version.as_deref().map(SemVer::parse).transpose() {
        Ok(v) => v,
        Err(e) => return err_response(e),
    };
    let tid = TemplateId(id);
    match svc.templates().get(p.tenant_id.as_deref(), &tid, version).await {
        Ok(Some(t)) => {
            if q.format.as_deref() == Some("yaml") {
                match export_template_yaml(&t) {
                    Ok(y) => ([("content-type", "application/yaml")], y).into_response(),
                    Err(e) => err_response(e),
                }
            } else {
                Json(t).into_response()
            }
        }
        Ok(None) => err_response(SetupError::NotFound(tid.0)),
        Err(e) => err_response(e),
    }
}

async fn create_template<TS, RS>(
    State(svc): State<Arc<RunService<TS, RS>>>,
    principal: Option<Extension<Principal>>,
    Json(template): Json<starter_setup_spi::model::Template>,
) -> Response
where
    TS: TemplateStore,
    RS: starter_setup_spi::store::SetupRunStore,
{
    let Some(Extension(p)) = principal else {
        return unauth();
    };
    // Bind the template to the caller's tenant + validate kinds/bindings.
    let mut template = template;
    template.access.tenant_id = p.tenant_id.clone();
    if let Err(e) = crate::import::validate_flow_body(&template, svc.engine().kinds()).await {
        return err_response(e);
    }
    if let Err(e) = crate::validate_bindings(&template) {
        return err_response(e);
    }
    match svc.templates().put(template).await {
        Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response(),
        Err(e) => err_response(e),
    }
}

async fn import_template<TS, RS>(
    State(svc): State<Arc<RunService<TS, RS>>>,
    principal: Option<Extension<Principal>>,
    body: String,
) -> Response
where
    TS: TemplateStore,
    RS: starter_setup_spi::store::SetupRunStore,
{
    let Some(Extension(p)) = principal else {
        return unauth();
    };
    let template = match import_template_yaml(
        &body,
        p.tenant_id.clone(),
        TemplateSource::Api,
        svc.engine().kinds(),
    )
    .await
    {
        Ok(t) => t,
        Err(e) => return err_response(e),
    };
    match svc.templates().put(template).await {
        Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response(),
        Err(e) => err_response(e),
    }
}

async fn delete_template<TS, RS>(
    State(svc): State<Arc<RunService<TS, RS>>>,
    Path(id): Path<String>,
    Query(q): Query<FormatQuery>,
    principal: Option<Extension<Principal>>,
) -> Response
where
    TS: TemplateStore,
    RS: starter_setup_spi::store::SetupRunStore,
{
    let Some(Extension(p)) = principal else {
        return unauth();
    };
    let version = match q.version.as_deref().map(SemVer::parse).transpose() {
        Ok(Some(v)) => v,
        Ok(None) => return (StatusCode::BAD_REQUEST, "version required").into_response(),
        Err(e) => return err_response(e),
    };
    match svc
        .templates()
        .delete(p.tenant_id.as_deref(), &TemplateId(id), version)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_response(e),
    }
}

// ---- runs -------------------------------------------------------------

async fn run_template<TS, RS>(
    State(svc): State<Arc<RunService<TS, RS>>>,
    Path(id): Path<String>,
    principal: Option<Extension<Principal>>,
    Json(input): Json<serde_json::Value>,
) -> Response
where
    TS: TemplateStore,
    RS: starter_setup_spi::store::SetupRunStore,
{
    let Some(Extension(p)) = principal else {
        return unauth();
    };
    let tid = TemplateId(id);
    let template = match svc.templates().get(p.tenant_id.as_deref(), &tid, None).await {
        Ok(Some(t)) => t,
        Ok(None) => return err_response(SetupError::NotFound(tid.0)),
        Err(e) => return err_response(e),
    };
    // DOCS §10 step 2 — the setup-layer team check (after the coarse
    // generic gate the route layer already applied).
    if let Err(e) = team_check(&template, &p) {
        return err_response(e);
    }
    match svc.run_template(&template, &p, &input).await {
        Ok(handle) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({ "run_id": handle.run })),
        )
            .into_response(),
        Err(e) => err_response(e),
    }
}

async fn list_runs<TS, RS>(
    State(svc): State<Arc<RunService<TS, RS>>>,
    principal: Option<Extension<Principal>>,
) -> Response
where
    TS: TemplateStore,
    RS: starter_setup_spi::store::SetupRunStore,
{
    let Some(Extension(p)) = principal else {
        return unauth();
    };
    // Own runs only (admins can widen via a dedicated endpoint later).
    let filter = SetupRunFilter {
        owner: Some(p.subject.clone()),
        ..Default::default()
    };
    match svc.runs().list(filter).await {
        Ok(runs) => Json(runs).into_response(),
        Err(e) => err_response(e),
    }
}

fn parse_run_id(s: &str) -> Result<RunId, Response> {
    s.parse::<uuid::Uuid>()
        .map(RunId)
        .map_err(|_| (StatusCode::BAD_REQUEST, "bad run id").into_response())
}

async fn get_run<TS, RS>(
    State(svc): State<Arc<RunService<TS, RS>>>,
    Path(id): Path<String>,
    principal: Option<Extension<Principal>>,
) -> Response
where
    TS: TemplateStore,
    RS: starter_setup_spi::store::SetupRunStore,
{
    let Some(Extension(p)) = principal else {
        return unauth();
    };
    let run_id = match parse_run_id(&id) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    match svc.runs().get(run_id).await {
        Ok(Some(r)) => {
            if !owns_run(&p, &r) {
                return err_response(SetupError::Forbidden("not your run".into()));
            }
            Json(r).into_response()
        }
        Ok(None) => err_response(SetupError::NotFound(id)),
        Err(e) => err_response(e),
    }
}

async fn run_events<TS, RS>(
    State(svc): State<Arc<RunService<TS, RS>>>,
    Path(id): Path<String>,
    principal: Option<Extension<Principal>>,
) -> Response
where
    TS: TemplateStore,
    RS: starter_setup_spi::store::SetupRunStore,
{
    let Some(Extension(p)) = principal else {
        return unauth();
    };
    let run_id = match parse_run_id(&id) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    // Authorize + replay the stored snapshot first.
    let snapshot = match svc.runs().get(run_id).await {
        Ok(Some(r)) if owns_run(&p, &r) => r,
        Ok(Some(_)) => return err_response(SetupError::Forbidden("not your run".into())),
        Ok(None) => return err_response(SetupError::NotFound(id)),
        Err(e) => return err_response(e),
    };

    // Live tail if the run is in-flight here; else snapshot-only stream.
    let snapshot_event = serde_json::json!({
        "done": snapshot.progress.done,
        "total": snapshot.progress.total,
        "current_step": snapshot.progress.current_step,
        "status": snapshot.status.as_str().to_lowercase(),
    });

    match svc.subscribe_live(run_id) {
        Some(rx) => {
            use futures::StreamExt;
            let total = snapshot.progress.total;
            let live = tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(
                move |ev| {
                    let total = total;
                    async move { ev.ok().map(|e| flow_event_to_sse(&e, total)) }
                },
            );
            let head = futures::stream::once(async move { snapshot_event });
            starter_server::sse::from_stream(head.chain(live)).into_response()
        }
        None => {
            // Run not in this process: emit the snapshot once and close.
            let head = futures::stream::once(async move { snapshot_event });
            starter_server::sse::from_stream(head).into_response()
        }
    }
}

async fn resume_run<TS, RS>(
    State(svc): State<Arc<RunService<TS, RS>>>,
    Path(id): Path<String>,
    principal: Option<Extension<Principal>>,
) -> Response
where
    TS: TemplateStore,
    RS: starter_setup_spi::store::SetupRunStore,
{
    let Some(Extension(p)) = principal else {
        return unauth();
    };
    let run_id = match parse_run_id(&id) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    let run = match svc.runs().get(run_id).await {
        Ok(Some(r)) if owns_run(&p, &r) => r,
        Ok(Some(_)) => return err_response(SetupError::Forbidden("not your run".into())),
        Ok(None) => return err_response(SetupError::NotFound(id)),
        Err(e) => return err_response(e),
    };
    let template = match svc
        .templates()
        .get(run.tenant_id.as_deref(), &run.template_id, Some(run.template_version))
        .await
    {
        Ok(Some(t)) => t,
        Ok(None) => return err_response(SetupError::NotFound(run.template_id.0)),
        Err(e) => return err_response(e),
    };
    match svc.resume_run(&template, run_id).await {
        Ok(handle) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({ "run_id": handle.run })),
        )
            .into_response(),
        Err(e) => err_response(e),
    }
}

async fn cancel_run<TS, RS>(
    State(svc): State<Arc<RunService<TS, RS>>>,
    Path(id): Path<String>,
    principal: Option<Extension<Principal>>,
) -> Response
where
    TS: TemplateStore,
    RS: starter_setup_spi::store::SetupRunStore,
{
    let Some(Extension(p)) = principal else {
        return unauth();
    };
    let run_id = match parse_run_id(&id) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    match svc.runs().get(run_id).await {
        Ok(Some(r)) if owns_run(&p, &r) => {}
        Ok(Some(_)) => return err_response(SetupError::Forbidden("not your run".into())),
        Ok(None) => return err_response(SetupError::NotFound(id)),
        Err(e) => return err_response(e),
    }
    let signalled = svc.cancel_live(run_id);
    // Persist the terminal state regardless (the run may be in another
    // process; the durable index is the source of truth).
    let _ = svc
        .runs()
        .mark_finished(
            run_id,
            starter_setup_spi::model::SetupRunStatus::Cancelled,
            chrono::Utc::now().to_rfc3339(),
        )
        .await;
    Json(serde_json::json!({ "cancelled": true, "was_live": signalled })).into_response()
}

fn owns_run(p: &Principal, r: &starter_setup_spi::model::SetupRun) -> bool {
    r.owner == p.subject || p.is_super_admin()
}

fn flow_event_to_sse(ev: &starter_flow_spi::flow::FlowEvent, total: usize) -> serde_json::Value {
    use starter_flow_spi::flow::FlowEvent as E;
    match ev {
        E::NodeStarted { node, .. } => serde_json::json!({
            "event": "step", "current_step": node.to_string(), "total": total, "status": "running"
        }),
        E::NodeFailed { node, error, .. } => serde_json::json!({
            "event": "failed", "current_step": node.to_string(), "error": error, "resumable": true
        }),
        E::RunCompleted { .. } => serde_json::json!({ "event": "completed", "status": "completed" }),
        E::RunFailed { error, .. } => serde_json::json!({ "event": "failed", "error": error, "resumable": true }),
        E::RunCancelled { .. } => serde_json::json!({ "event": "cancelled", "status": "cancelled" }),
        other => serde_json::json!({ "event": "info", "detail": format!("{other:?}") }),
    }
}
