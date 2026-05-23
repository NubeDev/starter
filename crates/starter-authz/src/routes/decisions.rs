//! `GET /v1/authz/decisions` — paged read of the
//! `starter_authz_decisions` audit table. SCOPE-EXT.md R14.
//!
//! - Cursor-paginated by `at` (DESC). `before=<rfc3339>` is the
//!   exclusive upper bound; `limit` is clamped to `[1, 500]`.
//! - Tenant-admins see their own tenant only; the super-admin
//!   (`Principal.tenant_id == Some("*")`) sees everything.
//! - The audit-log read kind itself opts out of allow-sampling
//!   via the per-kind override map on the sink — paging this
//!   route does NOT generate sampled-away allows.

use std::sync::Arc;

use axum::extract::{Extension, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use starter_spi::auth::Principal;

use crate::audit::db::DecisionFilter;
use crate::config::Effect;

use super::state::AuthzRoutesState;

/// Query-string shape.
#[derive(Debug, Default, Deserialize)]
pub struct DecisionsQuery {
    /// Filter by tenant id (super-admin only; tenant-admins are
    /// forced to their own tenant regardless).
    #[serde(default)]
    pub tenant: Option<String>,
    /// Filter by subject id.
    #[serde(default)]
    pub subject: Option<String>,
    /// `"allow"` / `"deny"`.
    #[serde(default)]
    pub effect: Option<String>,
    /// RFC3339 timestamp — return rows strictly before this.
    #[serde(default)]
    pub before: Option<String>,
    /// Page size; clamped to `[1, 500]`. Defaults to 100.
    #[serde(default)]
    pub limit: Option<i64>,
}

/// JSON view of a decision row.
#[derive(Debug, Serialize)]
pub struct DecisionView {
    /// Wall-clock time of the decision.
    pub at: DateTime<Utc>,
    /// Principal tenant binding.
    pub tenant: Option<String>,
    /// Principal subject.
    pub subject: String,
    /// Role at decision time.
    pub principal_role: String,
    /// Action requested.
    pub action: String,
    /// Resource kind.
    pub kind: String,
    /// Resource id (if any).
    pub id: Option<String>,
    /// `"allow"` / `"deny"`.
    pub effect: String,
    /// Matched rule id, if a rule fired.
    pub rule_id: Option<String>,
    /// Engine-supplied reason code (`cross_tenant`,
    /// `no_tenant_binding`, …), if engine semantics drove the
    /// decision.
    pub reason: Option<String>,
}

/// Paged response.
#[derive(Debug, Serialize)]
pub struct DecisionsPage {
    /// Decision rows, newest first.
    pub items: Vec<DecisionView>,
    /// Cursor for the next page — RFC3339; pass back as `before`.
    /// `None` when fewer than `limit` rows came back.
    pub next_before: Option<String>,
}

/// Handler. Returns `404` when the sink isn't configured (Phase
/// 1–6 deployments keep working).
pub async fn list_decisions(
    Extension(state): Extension<Arc<AuthzRoutesState>>,
    Extension(principal): Extension<Principal>,
    Query(q): Query<DecisionsQuery>,
) -> Response {
    let Some(sink) = state.decision_sink.clone() else {
        return (StatusCode::NOT_FOUND, "audit sink not configured").into_response();
    };

    let is_super_admin = principal.is_super_admin();
    let tenant_filter = if is_super_admin {
        q.tenant.clone()
    } else {
        // Tenant-admin: clamp to own tenant regardless of query.
        match &principal.tenant_id {
            Some(t) if t != "*" => Some(t.clone()),
            _ => return (StatusCode::FORBIDDEN, "tenant binding required").into_response(),
        }
    };

    let before = match &q.before {
        None => None,
        Some(s) => match DateTime::parse_from_rfc3339(s) {
            Ok(t) => Some(t.with_timezone(&Utc)),
            Err(_) => return (StatusCode::BAD_REQUEST, "bad `before`").into_response(),
        },
    };

    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let filter = DecisionFilter {
        tenant: tenant_filter,
        subject: q.subject.clone(),
        effect: q.effect.clone(),
        before,
        limit,
    };

    let rows = match query_rows(&sink, &filter).await {
        Ok(rs) => rs,
        Err(e) => {
            tracing::error!(error = %e, "authz decisions query failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, "query failed").into_response();
        }
    };

    let next_before = if (rows.len() as i64) == limit {
        rows.last().map(|r| r.at.to_rfc3339())
    } else {
        None
    };
    let items: Vec<DecisionView> = rows
        .into_iter()
        .map(|e| DecisionView {
            at: e.at,
            tenant: e.tenant,
            subject: e.subject,
            principal_role: e.principal_role,
            action: e.action,
            kind: e.kind,
            id: e.id,
            effect: match e.effect {
                Effect::Allow => "allow".into(),
                Effect::Deny => "deny".into(),
            },
            rule_id: e.rule_id,
            reason: e.reason,
        })
        .collect();
    Json(DecisionsPage { items, next_before }).into_response()
}

#[cfg(any(feature = "sqlite", feature = "postgres"))]
async fn query_rows(
    _sink: &crate::audit::DbDecisionSink,
    _filter: &DecisionFilter,
) -> Result<Vec<crate::audit::DecisionEntry>, String> {
    // Inspect the backend held by the sink. We can't use generics
    // through the trait, so the sink exposes a helper that does
    // the right thing per-backend.
    crate::audit::db::list_via_sink(_sink, _filter)
        .await
        .map_err(|e| e.to_string())
}
