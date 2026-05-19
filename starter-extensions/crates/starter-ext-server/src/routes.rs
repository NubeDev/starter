//! Admin route handlers.
//!
//! `list`, `detail`, `enable`, `disable`. The `events` and `ui`
//! handlers live in their own modules because they have meaningfully
//! different shapes (SSE upgrade + ETag-cached file serving).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use starter_ext_host::ExtensionRecord;
use starter_ext_spi::{ExtensionId, LifecycleState, Manifest};

use crate::admin::ExtensionAdmin;
use crate::store::EnablementState;

// ---------------------------------------------------------------------------
// GET /extensions
// ---------------------------------------------------------------------------

/// One row in the `GET /extensions` response. Deliberately flat so a
/// React table renders it without `?.` chains.
#[derive(Debug, Serialize)]
pub(crate) struct ExtensionSummary {
    pub id: String,
    pub version: Option<String>,
    pub display_name: Option<String>,
    pub state: LifecycleState,
    pub runtime_kind: Option<starter_ext_spi::RuntimeKind>,
    /// Cumulative restart count from the supervisor's event ring;
    /// `0` for records with no live supervisor (builtin, disabled).
    pub restart_count: u64,
    /// Capability-violation counter from the supervisor. Surfaced in
    /// the list view so operators see crash-loop-adjacent issues at a
    /// glance.
    pub capability_violations: u64,
    /// Persisted enablement state. Defaults to `Enabled` when the
    /// store has no row for this id yet.
    pub enabled: EnablementState,
}

pub(crate) async fn list(State(admin): State<ExtensionAdmin>) -> impl IntoResponse {
    let mut rows = Vec::with_capacity(admin.registry().list().len());
    for rec in admin.registry().list() {
        let id = rec.id.clone();
        let enabled = match &id {
            Some(eid) => admin
                .store()
                .get(eid)
                .await
                .ok()
                .flatten()
                .unwrap_or(EnablementState::Enabled),
            None => EnablementState::Enabled,
        };
        let (restart_count, capability_violations) = supervisor_metrics(&admin, rec);
        rows.push(ExtensionSummary {
            id: rec.id_hint.clone(),
            version: rec.manifest.as_ref().map(|m| m.version.to_string()),
            display_name: rec.manifest.as_ref().map(|m| m.display_name.clone()),
            state: rec.state,
            runtime_kind: rec.manifest.as_ref().map(|m| m.runtime.kind),
            restart_count,
            capability_violations,
            enabled,
        });
    }
    Json(rows)
}

// ---------------------------------------------------------------------------
// GET /extensions/<id>
// ---------------------------------------------------------------------------

/// Full record. The manifest is surfaced as-deserialised — adapters and
/// admin UIs do their own rendering against the well-known `block.yaml`
/// shape.
#[derive(Debug, Serialize)]
pub(crate) struct ExtensionDetail {
    pub id: String,
    pub state: LifecycleState,
    pub enabled: EnablementState,
    pub manifest: Option<Manifest>,
    pub failure: Option<String>,
    pub restart_count: u64,
    pub capability_violations: u64,
    /// Sequence number the next event will receive — clients use this
    /// as a `?after=<seq>` cursor when polling the events endpoint.
    pub events_cursor: u64,
}

pub(crate) async fn detail(
    State(admin): State<ExtensionAdmin>,
    Path(id): Path<String>,
) -> Result<Json<ExtensionDetail>, StatusCode> {
    let rec = admin
        .registry()
        .get_by_id_str(&id)
        .ok_or(StatusCode::NOT_FOUND)?;
    let parsed_id = ExtensionId::new(&id).ok();
    let enabled = if let Some(eid) = &parsed_id {
        admin
            .store()
            .get(eid)
            .await
            .map_err(|e| {
                tracing::warn!(err = %e.0, ext = %id, "enablement store get failed");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .unwrap_or(EnablementState::Enabled)
    } else {
        EnablementState::Enabled
    };
    let (restart_count, capability_violations) = supervisor_metrics(&admin, rec);
    let events_cursor = parsed_id
        .as_ref()
        .and_then(|eid| admin.supervisor(eid))
        .map(|h| {
            // We compute next_seq from the snapshot length + last seq
            // when reachable; ring exposes next_seq directly via events.
            // The handle doesn't surface the ring, so derive from the
            // events snapshot.
            h.events()
                .last()
                .map(|e| e.seq.wrapping_add(1))
                .unwrap_or(0)
        })
        .unwrap_or(0);

    Ok(Json(ExtensionDetail {
        id: rec.id_hint.clone(),
        state: rec.state,
        enabled,
        manifest: rec.manifest.clone(),
        failure: rec.failure.as_ref().map(|e| e.to_string()),
        restart_count,
        capability_violations,
        events_cursor,
    }))
}

// ---------------------------------------------------------------------------
// POST /extensions/<id>/enable  +  /disable
// ---------------------------------------------------------------------------

/// Response body for the toggle endpoints. Identical shape so a client
/// can `fetch` either and parse with one type.
#[derive(Debug, Serialize)]
pub(crate) struct ToggleResponse {
    pub id: String,
    pub enabled: EnablementState,
    pub state: LifecycleState,
}

pub(crate) async fn enable(
    State(admin): State<ExtensionAdmin>,
    Path(id): Path<String>,
) -> Result<Json<ToggleResponse>, StatusCode> {
    let rec = admin
        .registry()
        .get_by_id_str(&id)
        .ok_or(StatusCode::NOT_FOUND)?
        .clone();
    let eid = rec.id.clone().ok_or(StatusCode::CONFLICT)?;

    // Persist first; if persistence fails we don't want a running
    // supervisor whose row doesn't say "enabled".
    admin
        .store()
        .set(&eid, EnablementState::Enabled)
        .await
        .map_err(|e| {
            tracing::warn!(err = %e.0, ext = %id, "enablement store set(enabled) failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Only spawn if we don't already have a live supervisor — calling
    // enable on an already-enabled extension is idempotent.
    if admin.supervisor(&eid).is_none() {
        match admin.factory().spawn(&rec).await {
            Ok(Some(handle)) => {
                admin.replace_supervisor(&eid, Some(handle));
            }
            Ok(None) => {
                // Builtin/wasm record — nothing to spawn.
            }
            Err(e) => {
                tracing::warn!(err = %e.0, ext = %id, "supervisor spawn on enable failed");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    let state = current_state(&admin, &eid, rec.state);
    Ok(Json(ToggleResponse {
        id,
        enabled: EnablementState::Enabled,
        state,
    }))
}

pub(crate) async fn disable(
    State(admin): State<ExtensionAdmin>,
    Path(id): Path<String>,
) -> Result<Json<ToggleResponse>, StatusCode> {
    let rec = admin
        .registry()
        .get_by_id_str(&id)
        .ok_or(StatusCode::NOT_FOUND)?
        .clone();
    let eid = rec.id.clone().ok_or(StatusCode::CONFLICT)?;

    // Tell any live supervisor to wind down. We pop the handle first
    // so a concurrent `enable` doesn't spawn against a still-running
    // child.
    if let Some(handle) = admin.replace_supervisor(&eid, None) {
        handle.shutdown().await;
    }

    admin
        .store()
        .set(&eid, EnablementState::Disabled)
        .await
        .map_err(|e| {
            tracing::warn!(err = %e.0, ext = %id, "enablement store set(disabled) failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ToggleResponse {
        id,
        enabled: EnablementState::Disabled,
        state: rec.state,
    }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Derive `(restart_count, capability_violations)` from a record's
/// supervisor handle, if one is live. Records with no supervisor (or no
/// validated id) report zeros.
fn supervisor_metrics(admin: &ExtensionAdmin, rec: &ExtensionRecord) -> (u64, u64) {
    let id = match &rec.id {
        Some(id) => id,
        None => return (0, 0),
    };
    let handle = match admin.supervisor(id) {
        Some(h) => h,
        None => return (0, 0),
    };
    let restart_count = handle
        .events()
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                starter_ext_supervisor::EventKind::RestartScheduled { .. }
            )
        })
        .count() as u64;
    (restart_count, handle.capability_violations())
}

/// Best-effort current state for the toggle response. If a supervisor
/// handle is live, ask it; otherwise fall back to the record's
/// load-time state.
fn current_state(
    admin: &ExtensionAdmin,
    id: &ExtensionId,
    fallback: LifecycleState,
) -> LifecycleState {
    admin
        .supervisor(id)
        .map(|h| *h.state().borrow())
        .unwrap_or(fallback)
}
