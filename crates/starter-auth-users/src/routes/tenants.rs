//! Phase 7a — `/v1/tenants/*` admin REST surface (SCOPE-EXT.md).
//!
//! The handlers expect the consumer to gate the router with
//! `with_role(Admin)` on the host side; they perform no further
//! authentication, only validate the request body and translate
//! [`TenantStoreError`] to HTTP status codes.
//!
//! Routes mounted by [`tenants_router`]:
//!
//! ```text
//! POST   /v1/tenants
//! GET    /v1/tenants
//! GET    /v1/tenants/{id}
//! PATCH  /v1/tenants/{id}
//! POST   /v1/tenants/{id}/members
//! GET    /v1/tenants/{id}/members
//! PATCH  /v1/tenants/{id}/members/{user_id}
//! DELETE /v1/tenants/{id}/members/{user_id}
//! ```
//!
//! There is **no** `DELETE /v1/tenants/{id}` route — tenant
//! deletion is deferred (ADR-tenant-deletion).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};
use starter_spi::auth::Principal;

use crate::admin::{create_admin, AdminError};
use crate::role::Role;
use crate::store::{
    is_reserved_slug, MembershipRecord, TeamRecord, TenantRecord, TenantStore, TenantStoreError,
    UserStore,
};

/// Wire shape for creating a tenant.
#[derive(Debug, Deserialize)]
pub struct CreateTenantBody {
    /// URL slug. Refused when on the reserved list (HTTP 400).
    pub slug: String,
    /// Display name (free-form).
    pub display_name: String,
    /// Parent tenant id (ADR-tenant-hierarchy). Omit / `null` to
    /// create a root tenant — allowed only for the `"*"` super-admin
    /// (box operator). When set, the caller must administer the
    /// parent (be `"*"` or have it in their subtree) or the request
    /// is `403 forbidden`.
    #[serde(default)]
    pub parent_id: Option<String>,
}

/// Wire shape for patching a tenant. Both fields optional; the
/// slug is immutable per SCOPE-EXT.md and therefore not listed.
#[derive(Debug, Deserialize)]
pub struct PatchTenantBody {
    /// New display name. `null` / absent leaves it unchanged.
    #[serde(default)]
    pub display_name: Option<String>,
    /// `Some(Some(n))` sets the override; `Some(None)` clears
    /// it back to the global default. Absent means "no change".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_allow_sample: Option<Option<i32>>,
}

/// Wire shape for `POST /v1/tenants/{id}/members`.
#[derive(Debug, Deserialize)]
pub struct AddMemberBody {
    /// User id to add.
    pub user_id: String,
    /// One of `"reader" | "writer" | "admin"`.
    pub role: String,
}

/// Wire shape for `PATCH /v1/tenants/{id}/members/{user_id}`.
#[derive(Debug, Deserialize)]
pub struct PatchMemberBody {
    /// New role for the membership.
    pub role: String,
}

/// Wire shape for `POST /v1/tenants/{id}/users` — create a brand-new user
/// account and add them to the tenant in one step.
#[derive(Debug, Deserialize)]
pub struct CreateUserBody {
    /// The new user's login email. Validated + lower-cased server-side.
    pub email: String,
    /// Initial password. Validated against the same strength rules as signup.
    pub password: String,
    /// Tenant role for the new member: `"reader" | "writer" | "admin"`.
    pub role: String,
}

/// Cloneable state for the create-user endpoint, which needs both the user store
/// (to create the account) and the tenant store (to add the membership). Kept
/// separate from the plain `Arc<dyn TenantStore>` the other tenant routes use so
/// their handlers stay unchanged.
#[derive(Clone)]
struct TenantUsersState {
    tenants: Arc<dyn TenantStore>,
    users: Arc<dyn UserStore>,
}

/// JSON view of a tenant returned by the handlers.
#[derive(Debug, Serialize)]
pub struct TenantView {
    /// Stable id.
    pub id: String,
    /// URL slug.
    pub slug: String,
    /// Display name.
    pub display_name: String,
    /// Per-tenant override of the audit-log allow-sample rate.
    pub audit_allow_sample: Option<i32>,
    /// Parent tenant id (ADR-tenant-hierarchy). `null` for a root.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

impl From<TenantRecord> for TenantView {
    fn from(r: TenantRecord) -> Self {
        Self {
            id: r.id,
            slug: r.slug,
            display_name: r.display_name,
            audit_allow_sample: r.audit_allow_sample,
            parent_id: r.parent_id,
        }
    }
}

/// JSON view of a membership.
#[derive(Debug, Serialize)]
pub struct MembershipView {
    /// Tenant id.
    pub tenant_id: String,
    /// User id.
    pub user_id: String,
    /// Role.
    pub role: String,
    /// The member's email, present on the list endpoint (which joins the users
    /// table) and omitted on add/patch responses. A human label for pickers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

impl From<MembershipRecord> for MembershipView {
    fn from(r: MembershipRecord) -> Self {
        Self {
            tenant_id: r.tenant_id,
            user_id: r.user_id,
            role: r.role,
            email: r.email,
        }
    }
}

/// Build the `/v1/tenants/*` router. The consumer is expected to
/// wrap it with the appropriate role gate.
pub fn tenants_router<S>(tenants: Arc<dyn TenantStore>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/v1/tenants", post(create_tenant_h).get(list_tenants_h))
        .route("/v1/tenants/{id}", get(get_tenant_h).patch(patch_tenant_h))
        // ADR-tenant-hierarchy — tree navigation for admin UIs.
        .route("/v1/tenants/{id}/children", get(list_children_h))
        .route("/v1/tenants/{id}/subtree", get(list_subtree_h))
        .route(
            "/v1/tenants/{id}/members",
            post(add_member_h).get(list_members_h),
        )
        .route(
            "/v1/tenants/{id}/members/{user_id}",
            patch(patch_member_h).delete(remove_member_h),
        )
        // Phase 7b — teams CRUD (R13).
        .route(
            "/v1/tenants/{id}/teams",
            post(create_team_h).get(list_teams_h),
        )
        .route(
            "/v1/tenants/{id}/teams/{team_id}",
            axum::routing::delete(delete_team_h),
        )
        .route(
            "/v1/tenants/{id}/teams/{team_id}/members",
            post(add_team_member_h),
        )
        .route(
            "/v1/tenants/{id}/teams/{team_id}/members/{user_id}",
            axum::routing::delete(remove_team_member_h),
        )
        .with_state(tenants)
}

/// Build the create-user route: `POST /v1/tenants/{id}/users`. Mounted alongside
/// [`tenants_router`]; kept separate because it needs the user store too. Gate it
/// the same way (admin / tenant-admin) on the host side.
pub fn tenant_users_router<S>(
    tenants: Arc<dyn TenantStore>,
    users: Arc<dyn UserStore>,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/v1/tenants/{id}/users", post(create_user_h))
        .with_state(TenantUsersState { tenants, users })
}

/// Create a new user account (validated + argon2-hashed via [`create_admin`],
/// the same path the CLI and signup use) and add them to the tenant as a member
/// in one step. Returns the new membership (including the email) on success; a
/// `409` if the email is already taken, `400` on a weak password / bad email /
/// invalid role.
async fn create_user_h(
    State(state): State<TenantUsersState>,
    Path(tenant_id): Path<String>,
    Json(body): Json<CreateUserBody>,
) -> Response {
    let Some(role) = parse_role_str(&body.role) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_role"})),
        )
            .into_response();
    };
    let email = body.email.trim().to_lowercase();

    let user_id = match create_admin(state.users.as_ref(), &email, &body.password, role).await {
        Ok(id) => id,
        Err(AdminError::Conflict) => {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": "email_taken"})),
            )
                .into_response();
        }
        Err(AdminError::Validation(msg)) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid_input", "detail": msg})),
            )
                .into_response();
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    // The account exists now; add them to the tenant. The membership read later
    // joins the email, but we already know it, so return it directly.
    let row = MembershipRecord {
        tenant_id,
        user_id,
        role: body.role,
        email: Some(email),
    };
    match state.tenants.add_member(&row).await {
        Ok(()) => (StatusCode::CREATED, Json(MembershipView::from(row))).into_response(),
        Err(e) => map_err(e),
    }
}

/// Parse a wire role string into the [`Role`] enum. `None` for anything outside
/// the `reader | writer | admin` vocabulary.
fn parse_role_str(s: &str) -> Option<Role> {
    match s {
        "reader" => Some(Role::Reader),
        "writer" => Some(Role::Writer),
        "admin" => Some(Role::Admin),
        _ => None,
    }
}

async fn create_tenant_h(
    State(tenants): State<Arc<dyn TenantStore>>,
    Extension(principal): Extension<Principal>,
    Json(body): Json<CreateTenantBody>,
) -> Response {
    if is_reserved_slug(&body.slug) {
        return reserved_slug(&body.slug);
    }

    // ADR-tenant-hierarchy provisioning gate. The host already gates
    // this router with `with_role(Admin)`; here we additionally
    // enforce *where in the tree* this admin may create.
    match &body.parent_id {
        // Root tenants (no parent) may only be minted by the box
        // operator — the `"*"` super-admin sentinel.
        None => {
            if !principal.is_super_admin() {
                return forbidden("root_tenant_requires_super_admin");
            }
        }
        // A child tenant requires the caller to administer the
        // parent: either super-admin, or the parent is in the
        // caller's subtree.
        Some(parent) => {
            if !principal.is_super_admin() {
                match tenants
                    .is_ancestor(
                        principal.tenant_id.as_deref().unwrap_or_default(),
                        parent,
                    )
                    .await
                {
                    Ok(true) => {}
                    Ok(false) => return forbidden("not_parent_administrator"),
                    Err(e) => return map_err(e),
                }
            }
        }
    }

    let row = TenantRecord {
        id: uuid::Uuid::new_v4().to_string(),
        slug: body.slug,
        display_name: body.display_name,
        audit_allow_sample: None,
        parent_id: body.parent_id,
    };
    match tenants.create_tenant(&row).await {
        Ok(()) => (StatusCode::CREATED, Json(TenantView::from(row))).into_response(),
        Err(e) => map_err(e),
    }
}

async fn list_children_h(
    State(tenants): State<Arc<dyn TenantStore>>,
    Path(id): Path<String>,
) -> Response {
    match tenants.list_children(&id).await {
        Ok(rows) => {
            Json(rows.into_iter().map(TenantView::from).collect::<Vec<_>>()).into_response()
        }
        Err(e) => map_err(e),
    }
}

async fn list_subtree_h(
    State(tenants): State<Arc<dyn TenantStore>>,
    Path(id): Path<String>,
) -> Response {
    match tenants.list_subtree(&id).await {
        Ok(rows) => {
            Json(rows.into_iter().map(TenantView::from).collect::<Vec<_>>()).into_response()
        }
        Err(e) => map_err(e),
    }
}

async fn list_tenants_h(State(tenants): State<Arc<dyn TenantStore>>) -> Response {
    match tenants.list_tenants().await {
        Ok(rows) => {
            Json(rows.into_iter().map(TenantView::from).collect::<Vec<_>>()).into_response()
        }
        Err(e) => map_err(e),
    }
}

async fn get_tenant_h(
    State(tenants): State<Arc<dyn TenantStore>>,
    Path(id): Path<String>,
) -> Response {
    match tenants.get_tenant(&id).await {
        Ok(Some(t)) => Json(TenantView::from(t)).into_response(),
        Ok(None) => not_found(&id),
        Err(e) => map_err(e),
    }
}

async fn patch_tenant_h(
    State(tenants): State<Arc<dyn TenantStore>>,
    Path(id): Path<String>,
    Json(body): Json<PatchTenantBody>,
) -> Response {
    match tenants
        .patch_tenant(&id, body.display_name.as_deref(), body.audit_allow_sample)
        .await
    {
        Ok(()) => match tenants.get_tenant(&id).await {
            Ok(Some(t)) => Json(TenantView::from(t)).into_response(),
            Ok(None) => not_found(&id),
            Err(e) => map_err(e),
        },
        Err(e) => map_err(e),
    }
}

async fn add_member_h(
    State(tenants): State<Arc<dyn TenantStore>>,
    Path(id): Path<String>,
    Json(body): Json<AddMemberBody>,
) -> Response {
    if !["reader", "writer", "admin"].contains(&body.role.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_role"})),
        )
            .into_response();
    }
    let row = MembershipRecord {
        tenant_id: id,
        user_id: body.user_id,
        role: body.role,
        email: None,
    };
    match tenants.add_member(&row).await {
        Ok(()) => (StatusCode::CREATED, Json(MembershipView::from(row))).into_response(),
        Err(e) => map_err(e),
    }
}

async fn patch_member_h(
    State(tenants): State<Arc<dyn TenantStore>>,
    Path((id, user_id)): Path<(String, String)>,
    Json(body): Json<PatchMemberBody>,
) -> Response {
    if !["reader", "writer", "admin"].contains(&body.role.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_role"})),
        )
            .into_response();
    }
    match tenants.patch_member_role(&id, &user_id, &body.role).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => map_err(e),
    }
}

async fn list_members_h(
    State(tenants): State<Arc<dyn TenantStore>>,
    Path(id): Path<String>,
) -> Response {
    match tenants.members_of_tenant(&id).await {
        Ok(rows) => Json(
            rows.into_iter()
                .map(MembershipView::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(e) => map_err(e),
    }
}

async fn remove_member_h(
    State(tenants): State<Arc<dyn TenantStore>>,
    Path((id, user_id)): Path<(String, String)>,
) -> Response {
    // SCOPE-EXT.md R12 — cascades to token revoke inside the
    // store's own transaction.
    match tenants.remove_member(&id, &user_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => map_err(e),
    }
}

// ------------------------------------------------------------- teams (7b)

/// Wire shape for `POST /v1/tenants/{id}/teams`.
#[derive(Debug, Deserialize)]
pub struct CreateTeamBody {
    /// Rule-stable slug. Immutable after create (DB-level trigger).
    pub slug: String,
    /// Human-readable display name.
    pub display_name: String,
}

/// Wire shape for `POST /v1/tenants/{id}/teams/{team_id}/members`.
#[derive(Debug, Deserialize)]
pub struct AddTeamMemberBody {
    /// User id to add.
    pub user_id: String,
}

/// JSON view of a team.
#[derive(Debug, Serialize)]
pub struct TeamView {
    /// Stable id.
    pub id: String,
    /// Tenant id.
    pub tenant_id: String,
    /// Rule-stable slug.
    pub slug: String,
    /// Display name.
    pub display_name: String,
}

impl From<TeamRecord> for TeamView {
    fn from(r: TeamRecord) -> Self {
        Self {
            id: r.id,
            tenant_id: r.tenant_id,
            slug: r.slug,
            display_name: r.display_name,
        }
    }
}

async fn create_team_h(
    State(tenants): State<Arc<dyn TenantStore>>,
    Path(tenant_id): Path<String>,
    Json(body): Json<CreateTeamBody>,
) -> Response {
    // Slug validation: same url-safe shape as tenant slugs would
    // be too restrictive (teams aren't routed), but we do refuse
    // empty / whitespace-only slugs at the boundary so they never
    // reach the DB.
    if body.slug.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_slug"})),
        )
            .into_response();
    }
    let row = TeamRecord {
        id: uuid::Uuid::new_v4().to_string(),
        tenant_id,
        slug: body.slug,
        display_name: body.display_name,
    };
    match tenants.create_team(&row).await {
        Ok(()) => (StatusCode::CREATED, Json(TeamView::from(row))).into_response(),
        Err(e) => map_err(e),
    }
}

async fn list_teams_h(
    State(tenants): State<Arc<dyn TenantStore>>,
    Path(tenant_id): Path<String>,
) -> Response {
    match tenants.list_teams(&tenant_id).await {
        Ok(rows) => Json(rows.into_iter().map(TeamView::from).collect::<Vec<_>>()).into_response(),
        Err(e) => map_err(e),
    }
}

async fn delete_team_h(
    State(tenants): State<Arc<dyn TenantStore>>,
    Path((_tenant_id, team_id)): Path<(String, String)>,
) -> Response {
    match tenants.delete_team(&team_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => map_err(e),
    }
}

async fn add_team_member_h(
    State(tenants): State<Arc<dyn TenantStore>>,
    Path((_tenant_id, team_id)): Path<(String, String)>,
    Json(body): Json<AddTeamMemberBody>,
) -> Response {
    match tenants.add_team_member(&team_id, &body.user_id).await {
        Ok(()) => StatusCode::CREATED.into_response(),
        Err(e) => map_err(e),
    }
}

async fn remove_team_member_h(
    State(tenants): State<Arc<dyn TenantStore>>,
    Path((_tenant_id, team_id, user_id)): Path<(String, String, String)>,
) -> Response {
    match tenants.remove_team_member(&team_id, &user_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => map_err(e),
    }
}

fn reserved_slug(slug: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": "reserved_slug",
            "slug": slug,
        })),
    )
        .into_response()
}

fn not_found(id: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "not_found", "id": id})),
    )
        .into_response()
}

fn forbidden(reason: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({"error": "forbidden", "reason": reason})),
    )
        .into_response()
}

fn map_err(e: TenantStoreError) -> Response {
    match e {
        TenantStoreError::NotFound(id) => not_found(&id),
        TenantStoreError::SlugConflict(s) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "conflict", "slug": s})),
        )
            .into_response(),
        TenantStoreError::ReservedSlug(s) => reserved_slug(&s),
        TenantStoreError::ParentNotFound(id) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "parent_not_found", "id": id})),
        )
            .into_response(),
        TenantStoreError::MaxDepthExceeded(id) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "max_depth_exceeded", "id": id})),
        )
            .into_response(),
        TenantStoreError::Backend(msg) => {
            tracing::error!(target: "starter_auth_users", error = %msg, "tenant store backend error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "backend"})),
            )
                .into_response()
        }
    }
}
