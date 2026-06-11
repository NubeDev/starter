//! Detection notification handlers: channels, silences, and the notify-event
//! history. These are the delivery surface of an "alert-type" detection — the
//! pieces the old standalone alert subsystem owned, re-homed under
//! `/api/v1/detections/*`.
//!
//! LAYER: transport (REST). Channels and silences are tenant-global config, so
//! they're authorized by tenant membership alone (RLS isolates the rows),
//! matching the former alert channel/silence posture. A channel's secret config
//! is redacted on read.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::detection::{
    ChannelDetail, CreateChannelRequest, CreateSilenceRequest, NotifyEvent, SilenceDetail,
};
use nexus_store::notify::{self, NewChannel, NewSilence};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::detecting::notify::redact_config;
use crate::middleware::tenant::{caller, tenant_of};
use crate::state::AppState;

// ── Channels ────────────────────────────────────────────────────────────────

#[utoipa::path(get, path = "/api/v1/detections/channels", tag = "detections", operation_id = "list_channels",
    responses((status = 200, description = "Channels", body = [ChannelDetail])))]
pub async fn list_channels(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match notify::channel::list(&state.metadata, &tenant).await {
        Ok(cs) => Json(cs.iter().map(channel_to_detail).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/detections/channels", tag = "detections", operation_id = "create_channel",
    request_body = CreateChannelRequest,
    responses((status = 200, description = "Created", body = ChannelDetail)))]
pub async fn create_channel(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateChannelRequest>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let new = NewChannel {
        name: req.name,
        kind: req.kind,
        config: req.config,
    };
    match notify::channel::insert(&state.metadata, &tenant, &new).await {
        Ok(c) => Json(channel_to_detail(&c)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(delete, path = "/api/v1/detections/channels/{id}", tag = "detections", operation_id = "delete_channel",
    params(("id" = Uuid, Path, description = "Channel id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")))]
pub async fn delete_channel(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match notify::channel::delete(&state.metadata, &tenant, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

// ── Silences ──────────────────────────────────────────────────────────────��─

#[utoipa::path(get, path = "/api/v1/detections/silences", tag = "detections", operation_id = "list_silences",
    responses((status = 200, description = "Silences", body = [SilenceDetail])))]
pub async fn list_silences(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match notify::silence::list(&state.metadata, &tenant).await {
        Ok(ss) => Json(ss.iter().map(silence_to_detail).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/detections/silences", tag = "detections", operation_id = "create_silence",
    request_body = CreateSilenceRequest,
    responses((status = 200, description = "Created", body = SilenceDetail)))]
pub async fn create_silence(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateSilenceRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let new = NewSilence {
        detection_id: req.detection_id,
        starts_at: req.starts_at,
        ends_at: req.ends_at,
        reason: req.reason,
        created_by: caller.subject.clone(),
    };
    match notify::silence::insert(&state.metadata, &tenant, &new).await {
        Ok(s) => Json(silence_to_detail(&s)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(delete, path = "/api/v1/detections/silences/{id}", tag = "detections", operation_id = "delete_silence",
    params(("id" = Uuid, Path, description = "Silence id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")))]
pub async fn delete_silence(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match notify::silence::delete(&state.metadata, &tenant, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

// ── Notify events ─────────────────────────────────────────────────────────────

/// Server-side cap on a notify-events page, matching the findings browse cap.
const EVENTS_LIMIT: i64 = 200;

#[utoipa::path(get, path = "/api/v1/detections/notify-events", tag = "detections", operation_id = "list_notify_events",
    responses((status = 200, description = "Recent notification events", body = [NotifyEvent])))]
pub async fn list_notify_events(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match notify::event::list(&state.metadata, &tenant, EVENTS_LIMIT).await {
        Ok(es) => Json(es.iter().map(event_to_dto).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

// ── Conversions ─────────────────────────────────────────────────────────────

fn channel_to_detail(r: &notify::ChannelRecord) -> ChannelDetail {
    ChannelDetail {
        id: r.id,
        name: r.name.clone(),
        kind: r.kind.clone(),
        // Never echo secrets on read.
        config: redact_config(&r.kind, &r.config),
    }
}

fn silence_to_detail(r: &notify::SilenceRecord) -> SilenceDetail {
    SilenceDetail {
        id: r.id,
        detection_id: r.detection_id,
        starts_at: r.starts_at,
        ends_at: r.ends_at,
        reason: r.reason.clone(),
    }
}

fn event_to_dto(r: &notify::NotifyEventRecord) -> NotifyEvent {
    NotifyEvent {
        id: r.id,
        detection_id: r.detection_id,
        finding_id: r.finding_id,
        at: r.at,
        transition: r.transition.clone(),
        value: r.value,
        silenced: r.silenced,
        notified: r.notified,
        detail: r.detail.clone(),
    }
}
