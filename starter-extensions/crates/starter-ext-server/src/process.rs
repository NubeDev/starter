//! `GET /extensions/<id>/process` — live pid + sampled process stats.
//!
//! Process-flavour only. A builtin extension runs inside the host (its
//! "pid" would be the host's, which is misleading) and a wasm component is
//! instantiated in-process — both report **no** process and the UI hides
//! the tab. For a process-flavour extension that is not currently
//! `Running` (stopped, failed, or never spawned) there is likewise no live
//! process. Every one of these cases is a `404` carrying the stable code
//! `ext.process.not_running`; the consumer maps it onto its own catalog.
//!
//! On success the body is the [`ProcessStats`] shape from
//! `starter-ext-spi`: pid, started_at, uptime, best-effort rss/cpu, and the
//! restart count. There is no rubix code here — the handler is a pure
//! projection of [`SupervisorHandle::process_stats`].
//!
//! [`SupervisorHandle::process_stats`]: starter_ext_supervisor::SupervisorHandle::process_stats

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use starter_ext_spi::{ExtensionId, ProcessFlavour, ProcessStats};

use crate::admin::ExtensionAdmin;

/// Stable code returned with the `404` when there is no live process to
/// report (builtin, wasm, stopped, failed, or never spawned).
const NOT_RUNNING: &str = "ext.process.not_running";

#[derive(Debug, Serialize)]
struct NotRunning {
    /// Stable, non-localised code. Always `ext.process.not_running`.
    code: &'static str,
}

fn not_running() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(NotRunning { code: NOT_RUNNING }),
    )
        .into_response()
}

pub(crate) async fn process(
    State(admin): State<ExtensionAdmin>,
    Path(id): Path<String>,
) -> Response {
    // The record must exist for any answer; an unknown id is a plain 404.
    let Some(rec) = admin.registry().get_by_id_str(&id) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    // Builtin / wasm have no host-visible child — report not-running so the
    // UI hides the tab. The host pid is never reported for builtin.
    let flavour = rec
        .manifest
        .as_ref()
        .map(|m| ProcessFlavour::from(m.runtime.kind));
    if !matches!(flavour, Some(f) if f.reports_process_stats()) {
        return not_running();
    }

    // Process-flavour: stats come from the live supervisor handle, and
    // only while `Running` (the handle gates this internally).
    let stats: Option<ProcessStats> = ExtensionId::new(&id)
        .ok()
        .and_then(|pid| admin.supervisor(&pid))
        .and_then(|handle| handle.process_stats());

    match stats {
        Some(stats) => Json(stats).into_response(),
        None => not_running(),
    }
}
