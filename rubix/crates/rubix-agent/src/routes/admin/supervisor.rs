//! `GET /api/v1/admin/supervisor/health` — boot-reaper telemetry.
//!
//! Surfaces the result of the startup orphan reap: process groups left
//! alive by a previously `SIGKILL`ed agent instance that this boot
//! reclaimed (see [`starter_ext_supervisor::reaper`] and the boot wiring in
//! [`crate::boot::extensions`]). This is the one piece of supervisor health
//! that is *not* already on `GET /api/v1/extensions/overview`: the boot
//! reap happens once, before any per-extension supervisor exists, so it has
//! no per-extension row to hang off.
//!
//! Per-extension live gauges (lifecycle, restarts, `group_kills_total`,
//! capability violations, …) are served by the upstream
//! `GET /api/v1/extensions/overview`; the admin UI fetches both and renders
//! the reaper card from here and the per-extension table from there.

use axum::extract::State;
use axum::http::Method;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Json;
use serde::Serialize;
use starter_ext_supervisor::ReapReport;

use crate::admin::AdminState;
use crate::routes::{RouteMeta, RouteRegistrar};

/// Response body for `GET /api/v1/admin/supervisor/health`.
#[derive(Debug, Serialize)]
struct SupervisorHealth {
    /// The boot-time orphan reap. `reaped.groups` is every stale pidfile
    /// processed; each entry's `was_alive` flags whether the group was
    /// still running (and thus got `SIGKILL`ed) versus already gone. The UI
    /// derives `total` / `killed` counts from this list. A non-empty list
    /// with live groups over successive boots is the signal that the agent
    /// is being hard-killed without tearing its children down.
    reaped: ReapReport,
}

pub(super) fn registrar(state: AdminState) -> RouteRegistrar {
    RouteRegistrar::new().mount(
        Method::GET,
        "/api/v1/admin/supervisor/health",
        get(handler).with_state(state),
        RouteMeta::new()
            .describe(
                "Boot-reaper telemetry: child process groups left alive by a \
                 previously killed agent instance and reclaimed at startup.",
            )
            .tag("admin"),
    )
}

async fn handler(State(state): State<AdminState>) -> Response {
    // No reaper wired (extension host disabled) → an empty report renders
    // as "clean boot" in the UI rather than an error.
    let reaped = state
        .supervisor_reaped
        .as_ref()
        .map(|r| (**r).clone())
        .unwrap_or_default();
    Json(SupervisorHealth { reaped }).into_response()
}
