//! `GET /extensions/<id>/issues` — consolidated diagnostics.
//!
//! A pure projection: merge the record-level issues
//! ([`ExtensionRecord::issues`], the no-supervisor path — a `Failed`
//! manifest is one `Fatal` issue) with the live supervisor's issues
//! ([`SupervisorHandle::issues`], derived from the event ring + capability
//! violation counter), sort by `at` descending, and apply the optional
//! `?severity=` / `?since=<seq>` filters.
//!
//! There is no rubix code here — the handler folds two starter-owned
//! producers into one ordered [`ExtensionIssue`] list. The wire `code` is
//! the stable `ext.issue.*` string; the consumer maps it onto its own
//! `MessageKey` catalog.
//!
//! [`ExtensionRecord::issues`]: starter_ext_host::ExtensionRecord::issues
//! [`SupervisorHandle::issues`]: starter_ext_supervisor::SupervisorHandle::issues

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use starter_ext_spi::{ExtensionId, ExtensionIssue, Severity};

use crate::admin::ExtensionAdmin;

#[derive(Debug, Default, Deserialize)]
pub(crate) struct IssuesQuery {
    /// Only return issues at or above this severity. Accepts the snake-case
    /// wire form (`info` | `warning` | `error` | `fatal`).
    #[serde(default)]
    pub severity: Option<Severity>,
    /// Only return issues whose originating event `seq` is strictly greater
    /// than this cursor. Record-level issues (no `seq`) are dropped when a
    /// `since` cursor is supplied — they have no position in the event
    /// stream the cursor walks.
    #[serde(default)]
    pub since: Option<u64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct IssuesResponse {
    /// Issues, most recent first.
    pub issues: Vec<ExtensionIssue>,
}

pub(crate) async fn issues(
    State(admin): State<ExtensionAdmin>,
    Path(id): Path<String>,
    Query(q): Query<IssuesQuery>,
) -> Result<Json<IssuesResponse>, StatusCode> {
    // Record-level issues work for every record — builtin, wasm, disabled,
    // and Failed bundles that never had a supervisor.
    let rec = admin
        .registry()
        .get_by_id_str(&id)
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut merged = rec.issues();

    // Live-supervisor issues, when a handle exists for this id.
    if let Ok(parsed_id) = ExtensionId::new(&id) {
        if let Some(handle) = admin.supervisor(&parsed_id) {
            merged.extend(handle.issues());
        }
    }

    // Filters before the sort — cheaper, and the order is irrelevant to a
    // retain.
    if let Some(min) = q.severity {
        merged.retain(|i| i.severity >= min);
    }
    if let Some(cursor) = q.since {
        merged.retain(|i| i.seq.is_some_and(|s| s > cursor));
    }

    // Most recent first.
    merged.sort_by(|a, b| b.at.cmp(&a.at));

    Ok(Json(IssuesResponse { issues: merged }))
}
