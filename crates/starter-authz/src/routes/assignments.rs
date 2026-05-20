//! CRUD over `starter_authz_assignments`. Smaller surface than
//! rules — assignments are pure bindings, no condition expression.

use std::sync::Arc;

use axum::extract::{Extension, Path};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use starter_spi::auth::Principal;

use crate::store::StoredAssignment;

use super::router::check_csrf;
use super::rules::store_err;
use super::state::AuthzRoutesState;

/// Body for `POST /v1/authz/assignments`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAssignmentRequest {
    /// Optional client-provided id; server assigns a UUID when
    /// absent.
    #[serde(default)]
    pub id: Option<String>,
    /// Subject id or single-trailing-`*` glob.
    pub subject: String,
    /// Role name.
    pub role: String,
}

/// JSON view of one stored assignment.
#[derive(Debug, Serialize)]
pub struct AssignmentView {
    /// Primary key.
    pub id: String,
    /// Subject id / glob.
    pub subject: String,
    /// Role name.
    pub role: String,
    /// Subject id of the admin who created the row.
    pub created_by: String,
}

impl From<StoredAssignment> for AssignmentView {
    fn from(s: StoredAssignment) -> Self {
        Self {
            id: s.id,
            subject: s.subject,
            role: s.role,
            created_by: s.created_by,
        }
    }
}

pub(super) async fn list_assignments(
    Extension(state): Extension<Arc<AuthzRoutesState>>,
) -> Response {
    match state.engine.store().list_assignments().await {
        Ok(rows) => Json(serde_json::json!({
            "assignments": rows
                .into_iter()
                .map(AssignmentView::from)
                .collect::<Vec<_>>(),
        }))
        .into_response(),
        Err(e) => store_err(e),
    }
}

pub(super) async fn create_assignment(
    Extension(state): Extension<Arc<AuthzRoutesState>>,
    Extension(principal): Extension<Principal>,
    headers: HeaderMap,
    Json(body): Json<CreateAssignmentRequest>,
) -> Response {
    if let Err(r) = check_csrf(&headers) {
        return r;
    }
    let row = StoredAssignment {
        id: body.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        subject: body.subject,
        role: body.role,
        created_by: principal.subject.clone(),
    };
    match state.engine.store().insert_assignment(&row).await {
        Ok(()) => {
            if let Err(e) = state.engine.reload().await {
                tracing::error!(target: "starter_authz", error = %e, "reload after assignment insert failed");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            (
                StatusCode::CREATED,
                Json(serde_json::json!({"assignment": AssignmentView::from(row)})),
            )
                .into_response()
        }
        Err(e) => store_err(e),
    }
}

pub(super) async fn delete_assignment(
    Extension(state): Extension<Arc<AuthzRoutesState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Err(r) = check_csrf(&headers) {
        return r;
    }
    match state.engine.store().delete_assignment(&id).await {
        Ok(()) => {
            if let Err(e) = state.engine.reload().await {
                tracing::error!(target: "starter_authz", error = %e, "reload after assignment delete failed");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => store_err(e),
    }
}
