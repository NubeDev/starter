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
use starter_ext_spi::{ContributeUi, ExtensionId, LifecycleState, Manifest};

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
    /// `true` for a freshly-installed extension that has not yet surfaced
    /// live — the sealed registry forbids hot-mount, so a newly-uploaded
    /// bundle only becomes active on next boot. The UI badges these rows.
    pub restart_required: bool,
    /// Compact view of the manifest's `contributes` block — counts per
    /// kind plus the UI `entry`/`exposes` (needed by the host to register
    /// the remote bundle). Lets the list view skip a per-row
    /// `GET /extensions/<id>` fetch for the contributes pills and
    /// Load-UI button. `None` for rows with no parsed manifest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributes: Option<ContributesSummary>,
}

/// Counts + UI block from the manifest's `contributes`. Skips the heavier
/// payload (REST paths, schemas, etc.) which the row UI does not render.
#[derive(Debug, Serialize)]
pub(crate) struct ContributesSummary {
    pub tools: usize,
    pub cli: usize,
    pub rest: usize,
    pub grpc: usize,
    pub workers: usize,
    pub nodes: usize,
    pub skills: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui: Option<ContributeUi>,
}

impl ContributesSummary {
    fn from_manifest(m: &Manifest) -> Self {
        let c = &m.contributes;
        Self {
            tools: c.tools.len(),
            cli: c.cli.len(),
            rest: c.rest.len(),
            grpc: c.grpc.len(),
            workers: c.workers.len(),
            nodes: c.nodes.len(),
            skills: c.skills.len(),
            ui: c.ui.clone(),
        }
    }
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
            // A record already in the registry is live; it only needs a
            // restart if it was reinstalled-over this run.
            restart_required: admin.is_pending_restart(&rec.id_hint),
            contributes: rec.manifest.as_ref().map(ContributesSummary::from_manifest),
        });
    }

    // Append rows for extensions installed during this run that the sealed
    // registry has not surfaced yet — they go live on next boot.
    let known: std::collections::HashSet<&str> = rows
        .iter()
        .map(|r| r.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let pending: Vec<_> = admin
        .pending_rows()
        .into_iter()
        .filter(|(id, _)| !known.contains(id.as_str()))
        .collect();
    for (id, p) in pending {
        rows.push(ExtensionSummary {
            id,
            version: p.version,
            display_name: p.display_name,
            state: LifecycleState::Validated,
            runtime_kind: p.runtime_kind,
            restart_count: 0,
            capability_violations: 0,
            enabled: EnablementState::Enabled,
            restart_required: true,
            // Pending rows have no parsed manifest yet (they go live on
            // next boot); the row will gain `contributes` then.
            contributes: None,
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
    /// Per-worker state from the periodic-worker adapter (Adapter
    /// Phase 7). Empty when no `WorkerStatesFn` is wired on the
    /// admin builder, or when the extension contributes no workers.
    /// Each element is one serialised `WorkerState` (see
    /// `starter_ext_workers::WorkerState`).
    pub workers: Vec<serde_json::Value>,
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

    let workers = match &parsed_id {
        Some(eid) => admin.worker_states(eid),
        None => Vec::new(),
    };

    Ok(Json(ExtensionDetail {
        id: rec.id_hint.clone(),
        state: rec.state,
        enabled,
        manifest: rec.manifest.clone(),
        failure: rec.failure.as_ref().map(|e| e.to_string()),
        restart_count,
        capability_violations,
        events_cursor,
        workers,
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
// POST /extensions/<id>/restart
// ---------------------------------------------------------------------------

/// Force-restart a single extension: shut the live supervisor down (if
/// any) and spawn a fresh one through the same factory the toggle
/// endpoints use. Idempotent — a no-op when the record has no
/// associated supervisor flavour (builtin / wasm).
///
/// The endpoint exists so a UI surfaced [`Error::Unavailable`] with
/// `code = "extension.supervisor_unavailable"` can offer a single-
/// click recovery action; the same handler is also useful for
/// operators forcing a clean reload after a manifest swap.
///
/// Distinct from `disable + enable` because it preserves the
/// persisted [`EnablementState`] — restarting a disabled extension
/// would be a bug. A `409 Conflict` fires when the record is
/// `Disabled`; the operator must `enable` it explicitly.
pub(crate) async fn restart(
    State(admin): State<ExtensionAdmin>,
    Path(id): Path<String>,
) -> Result<Json<ToggleResponse>, StatusCode> {
    let rec = admin
        .registry()
        .get_by_id_str(&id)
        .ok_or(StatusCode::NOT_FOUND)?
        .clone();
    let eid = rec.id.clone().ok_or(StatusCode::CONFLICT)?;

    // Don't resurrect a disabled extension by accident.
    if matches!(
        admin.store().get(&eid).await.ok().flatten(),
        Some(EnablementState::Disabled)
    ) {
        return Err(StatusCode::CONFLICT);
    }

    // Pop the existing handle first so a concurrent enable/disable
    // doesn't race with our spawn.
    if let Some(handle) = admin.replace_supervisor(&eid, None) {
        handle.shutdown().await;
    }

    match admin.factory().spawn(&rec).await {
        Ok(Some(handle)) => {
            admin.replace_supervisor(&eid, Some(handle));
        }
        Ok(None) => {
            // Builtin/wasm — nothing to spawn. The 200 response
            // still communicates "we tried"; the lifecycle field
            // tells the caller the runtime kind has no process to
            // restart.
        }
        Err(e) => {
            tracing::warn!(err = %e.0, ext = %id, "supervisor spawn on restart failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    let state = current_state(&admin, &eid, rec.state);
    Ok(Json(ToggleResponse {
        id,
        enabled: EnablementState::Enabled,
        state,
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
