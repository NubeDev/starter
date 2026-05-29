//! `GET /extensions/<id>/metrics` — the merged "how is it doing?" view.
//!
//! Folds two sources, neither of which this handler computes itself:
//!
//! - **Adapter counters** from the shared
//!   [`starter_ext_metrics::MetricsRegistry`] the transport adapters bump
//!   (tool calls/errors, REST requests, worker runs/failures).
//! - **Process gauges** from the live
//!   [`starter_ext_supervisor::SupervisorHandle`]: sampled
//!   [`ProcessStats`](starter_ext_spi::ProcessStats), lifecycle state,
//!   cumulative restarts, capability violations, and event-ring evictions.
//!
//! The projection itself lives in `starter-ext-metrics`
//! ([`MetricsRegistry::merged`]) so the dependency arrows stay one-way
//! (adapters → metrics ← supervisor) and there is no rubix-specific logic
//! here. An unknown id is a plain `404`; every known extension (builtin,
//! wasm, stopped, never-spawned) gets a metrics document — the counters are
//! always meaningful and the process gauges degrade to `null` / zero.
//!
//! [`MetricsRegistry::merged`]: starter_ext_metrics::MetricsRegistry::merged

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use starter_ext_metrics::ProcessGauges;
use starter_ext_spi::ExtensionId;

use crate::admin::ExtensionAdmin;

pub(crate) async fn metrics(
    State(admin): State<ExtensionAdmin>,
    Path(id): Path<String>,
) -> Response {
    // The record must exist for any answer; an unknown id is a plain 404.
    let Some(rec) = admin.registry().get_by_id_str(&id) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    // The live supervisor handle (process-flavour, currently running)
    // supplies the process gauges; everything else degrades gracefully.
    let handle = ExtensionId::new(&id)
        .ok()
        .and_then(|x| admin.supervisor(&x));

    let gauges = match &handle {
        Some(h) => ProcessGauges {
            process: h.process_stats(),
            lifecycle_state: h.lifecycle_state(),
            restarts_total: h.restarts_total(),
            capability_violations_total: h.capability_violations(),
            events_dropped_total: h.events_dropped(),
        },
        // No live supervisor (builtin/wasm, or disabled/stopped): fall
        // back to the record's load-time state and zero gauges. The
        // counters from the registry are still meaningful.
        None => ProcessGauges {
            process: None,
            lifecycle_state: rec.state,
            restarts_total: 0,
            capability_violations_total: 0,
            events_dropped_total: 0,
        },
    };

    let id = match ExtensionId::new(&id) {
        Ok(id) => id,
        // `get_by_id_str` matched, so the id is valid; this is unreachable
        // in practice, but stay total rather than unwrap.
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    Json(admin.metrics().merged(&id, gauges)).into_response()
}
