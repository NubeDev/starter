//! `GET /extensions/overview` — one shot, all extensions.
//!
//! The console's `/extensions` list page polls every 5s for live lifecycle
//! state plus per-row process gauges (uptime / mem / cpu) and counters
//! (tool calls, restarts, capability violations). Hitting that with the
//! per-id endpoints means `1 + 2N` requests per tick (list + process/<id> +
//! metrics/<id> for every row). This handler folds the three reads into a
//! single response so the table polls one URL regardless of how many
//! extensions are installed.
//!
//! Per row the payload is a [`ExtensionOverviewRow`]:
//!
//! - The same identity/state/enabled fields the list endpoint already
//!   surfaces (so the table can render directly from this response and
//!   skip the separate `useExtensionsList` query entirely).
//! - The full [`ExtensionMetrics`] projection — identical bytes to
//!   `GET /extensions/<id>/metrics` — so the existing chip code that
//!   reads `uptime`, `rss_bytes`, `tool_calls_total`, etc. keeps working.
//! - The `ContributesSummary` block reused from `routes::list` for the
//!   "contributes" pills + Load-UI button.
//!
//! Pending-but-not-yet-live records (freshly installed, awaiting reboot)
//! are surfaced the same way the list does, just with zero gauges and
//! `lifecycle_state = Validated`.

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use starter_ext_host::ExtensionRecord;
use starter_ext_metrics::ProcessGauges;
use starter_ext_spi::{ExtensionMetrics, LifecycleState, RuntimeKind};

use crate::admin::ExtensionAdmin;
use crate::routes::ContributesSummary;
use crate::store::EnablementState;

/// One row in the `GET /extensions/overview` response.
///
/// Field layout deliberately mirrors `routes::ExtensionSummary` for the
/// identity/state half and the merged `ExtensionMetrics` document for the
/// telemetry half, so the React table can stop calling
/// `useExtensionsList` / `useExtensionProcess` / `useExtensionMetrics`
/// per-row and read everything off this single response.
#[derive(Debug, Serialize)]
pub(crate) struct ExtensionOverviewRow {
    // ---- identity (same shape as `list`) ----
    pub id: String,
    pub version: Option<String>,
    pub display_name: Option<String>,
    pub runtime_kind: Option<RuntimeKind>,
    pub enabled: EnablementState,
    pub restart_required: bool,
    /// Purged this run but still in the sealed registry until next boot —
    /// see `ExtensionSummary::uninstalled`. Renders as dead/stale.
    pub uninstalled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributes: Option<ContributesSummary>,

    // ---- live telemetry (same shape as `/metrics`) ----
    /// Merged process + counters projection — identical bytes to the
    /// per-id `/metrics` response.
    #[serde(flatten)]
    pub metrics: ExtensionMetrics,
}

pub(crate) async fn overview(State(admin): State<ExtensionAdmin>) -> impl IntoResponse {
    let mut rows: Vec<ExtensionOverviewRow> = Vec::with_capacity(admin.registry().list().len());

    for rec in admin.registry().list() {
        let row = build_row(&admin, rec).await;
        rows.push(row);
    }

    // Append rows for extensions installed during this run that the sealed
    // registry has not surfaced yet — mirrors `routes::list`. These have
    // no live supervisor, so gauges are zero and the lifecycle is
    // `Validated`.
    let known: std::collections::HashSet<String> =
        rows.iter().map(|r| r.id.clone()).collect();
    let pending: Vec<_> = admin
        .pending_rows()
        .into_iter()
        .filter(|(id, _)| !known.contains(id))
        .collect();
    for (id, p) in pending {
        rows.push(ExtensionOverviewRow {
            id,
            version: p.version,
            display_name: p.display_name,
            runtime_kind: p.runtime_kind,
            enabled: EnablementState::Enabled,
            restart_required: true,
            uninstalled: false,
            contributes: None,
            metrics: ExtensionMetrics {
                process: None,
                lifecycle_state: LifecycleState::Validated,
                restarts_total: 0,
                capability_violations_total: 0,
                tool_calls_total: 0,
                tool_errors_total: 0,
                rest_requests_total: 0,
                worker_runs_total: 0,
                worker_failures_total: 0,
                events_dropped_total: 0,
                group_kills_total: 0,
            },
        });
    }

    Json(rows)
}

async fn build_row(admin: &ExtensionAdmin, rec: &ExtensionRecord) -> ExtensionOverviewRow {
    let id_str = rec.id_hint.clone();
    let parsed = rec.id.clone();

    let enabled = match &parsed {
        Some(eid) => admin
            .store()
            .get(eid)
            .await
            .ok()
            .flatten()
            .unwrap_or(EnablementState::Enabled),
        None => EnablementState::Enabled,
    };

    // Match `/metrics`: a live supervisor supplies the gauges; otherwise
    // fall back to the record's load-time state and zeros. Counters come
    // from the shared metrics registry either way.
    let handle = parsed.as_ref().and_then(|eid| admin.supervisor(eid));
    let gauges = match &handle {
        Some(h) => ProcessGauges {
            process: h.process_stats(),
            lifecycle_state: h.lifecycle_state(),
            restarts_total: h.restarts_total(),
            capability_violations_total: h.capability_violations(),
            events_dropped_total: h.events_dropped(),
            group_kills_total: h.group_kills_total(),
        },
        None => ProcessGauges {
            process: None,
            lifecycle_state: rec.state,
            restarts_total: 0,
            capability_violations_total: 0,
            events_dropped_total: 0,
            group_kills_total: 0,
        },
    };

    let metrics = match &parsed {
        Some(eid) => admin.metrics().merged(eid, gauges),
        // No validated id (parse failure); synthesise a zeroed view so
        // the row still renders. Should be unreachable in practice
        // because the registry refuses to store records without an id.
        None => ExtensionMetrics {
            process: gauges.process,
            lifecycle_state: gauges.lifecycle_state,
            restarts_total: gauges.restarts_total,
            capability_violations_total: gauges.capability_violations_total,
            tool_calls_total: 0,
            tool_errors_total: 0,
            rest_requests_total: 0,
            worker_runs_total: 0,
            worker_failures_total: 0,
            events_dropped_total: gauges.events_dropped_total,
            group_kills_total: gauges.group_kills_total,
        },
    };

    ExtensionOverviewRow {
        id: id_str.clone(),
        version: rec.manifest.as_ref().map(|m| m.version.to_string()),
        display_name: rec.manifest.as_ref().map(|m| m.display_name.clone()),
        runtime_kind: rec.manifest.as_ref().map(|m| m.runtime.kind),
        enabled,
        restart_required: admin.is_pending_restart(&id_str) || admin.is_uninstalled(&id_str),
        uninstalled: admin.is_uninstalled(&id_str),
        contributes: rec.manifest.as_ref().map(ContributesSummary::from_manifest),
        metrics,
    }
}
