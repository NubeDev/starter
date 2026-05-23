//! CRUD over `starter_authz_rules`.

use std::sync::Arc;

use axum::extract::{Extension, Path};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use starter_spi::auth::Principal;

use crate::store::{PolicyStoreError, StoredRule};

use super::router::check_csrf;
use super::state::AuthzRoutesState;

/// Wire shape for rule create/update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleBody {
    /// Optional on create (server generates a UUID); ignored on
    /// update (path id wins).
    #[serde(default)]
    pub id: Option<String>,
    /// Role this rule applies to.
    pub role: String,
    /// Resource kind, must be registered (`*` allowed).
    pub resource: String,
    /// Action list. `["*"]` for "any action".
    pub actions: Vec<String>,
    /// Optional condition (mini-language or `"owner"`).
    #[serde(default)]
    pub condition: Option<String>,
    /// `"allow"` or `"deny"`.
    pub effect: String,
    /// Higher first; defaults to `0`.
    #[serde(default)]
    pub priority: i32,
    /// Phase 7a — tenant scope; `None` is a global rule.
    #[serde(default)]
    pub tenant_id: Option<String>,
}

/// JSON view of a stored rule.
#[derive(Debug, Serialize)]
pub struct RuleView {
    /// Primary key.
    pub id: String,
    /// Role.
    pub role: String,
    /// Resource kind.
    pub resource: String,
    /// Action list.
    pub actions: Vec<String>,
    /// Optional condition.
    pub condition: Option<String>,
    /// `"allow"` or `"deny"`.
    pub effect: String,
    /// Priority; higher first.
    pub priority: i32,
    /// Subject id of the admin who created the row.
    pub created_by: String,
    /// Tenant scope; `None` for global rules.
    pub tenant_id: Option<String>,
}

impl From<StoredRule> for RuleView {
    fn from(s: StoredRule) -> Self {
        Self {
            id: s.id,
            role: s.role,
            resource: s.resource,
            actions: s.actions,
            condition: s.condition,
            effect: s.effect,
            priority: s.priority,
            created_by: s.created_by,
            tenant_id: s.tenant_id,
        }
    }
}

pub(super) async fn list_rules(Extension(state): Extension<Arc<AuthzRoutesState>>) -> Response {
    match state.engine.store().list_rules().await {
        Ok(rows) => Json(serde_json::json!({
            "rules": rows.into_iter().map(RuleView::from).collect::<Vec<_>>(),
        }))
        .into_response(),
        Err(e) => store_err(e),
    }
}

pub(super) async fn create_rule(
    Extension(state): Extension<Arc<AuthzRoutesState>>,
    Extension(principal): Extension<Principal>,
    headers: HeaderMap,
    Json(body): Json<RuleBody>,
) -> Response {
    if let Err(r) = check_csrf(&headers) {
        return r;
    }
    let creator = principal.subject.clone();
    if let Err(r) = validate_effect(&body.effect) {
        return r;
    }
    let row = StoredRule {
        id: body.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        role: body.role,
        resource: body.resource,
        actions: body.actions,
        condition: body.condition,
        effect: body.effect,
        priority: body.priority,
        created_by: creator,
        tenant_id: body.tenant_id,
    };
    match state.engine.store().insert_rule(&row).await {
        Ok(()) => {
            // Reload cache so the next `check()` sees the new
            // rule (SCOPE.md Phase 3, "rule-write-invalidates-cache").
            if let Err(e) = state.engine.reload().await {
                tracing::error!(target: "starter_authz", error = %e, "reload after rule insert failed");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            (
                StatusCode::CREATED,
                Json(serde_json::json!({"rule": RuleView::from(row)})),
            )
                .into_response()
        }
        Err(e) => store_err(e),
    }
}

pub(super) async fn update_rule(
    Extension(state): Extension<Arc<AuthzRoutesState>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<RuleBody>,
) -> Response {
    if let Err(r) = check_csrf(&headers) {
        return r;
    }
    let creator = principal.subject.clone();
    if let Err(r) = validate_effect(&body.effect) {
        return r;
    }
    let row = StoredRule {
        id,
        role: body.role,
        resource: body.resource,
        actions: body.actions,
        condition: body.condition,
        effect: body.effect,
        priority: body.priority,
        created_by: creator,
        tenant_id: body.tenant_id,
    };
    match state.engine.store().update_rule(&row).await {
        Ok(()) => {
            if let Err(e) = state.engine.reload().await {
                tracing::error!(target: "starter_authz", error = %e, "reload after rule update failed");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            Json(serde_json::json!({"rule": RuleView::from(row)})).into_response()
        }
        Err(e) => store_err(e),
    }
}

pub(super) async fn delete_rule(
    Extension(state): Extension<Arc<AuthzRoutesState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Err(r) = check_csrf(&headers) {
        return r;
    }
    match state.engine.store().delete_rule(&id).await {
        Ok(()) => {
            if let Err(e) = state.engine.reload().await {
                tracing::error!(target: "starter_authz", error = %e, "reload after rule delete failed");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => store_err(e),
    }
}

#[allow(clippy::result_large_err)] // `axum::Response` size is fixed by axum.
fn validate_effect(s: &str) -> Result<(), Response> {
    if s == "allow" || s == "deny" {
        Ok(())
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_effect"})),
        )
            .into_response())
    }
}

pub(super) fn store_err(e: PolicyStoreError) -> Response {
    match e {
        PolicyStoreError::Conflict(msg) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "conflict", "message": msg})),
        )
            .into_response(),
        PolicyStoreError::NotFound(msg) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "not_found", "message": msg})),
        )
            .into_response(),
        PolicyStoreError::Malformed(msg) => {
            tracing::error!(target: "starter_authz", error = %msg, "malformed authz row");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "malformed"})),
            )
                .into_response()
        }
        PolicyStoreError::Backend(msg) => {
            tracing::error!(target: "starter_authz", error = %msg, "authz backend error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "backend"})),
            )
                .into_response()
        }
    }
}
