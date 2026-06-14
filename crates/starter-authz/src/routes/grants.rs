//! `/v1/authz/grants/*` — sugar over `/rules` that writes one
//! rule row per grant and marks it `source="grant"`. See
//! `crate::grants` for the underlying [`GrantStore`].

use std::sync::Arc;

use axum::extract::{Extension, Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use starter_spi::auth::{Principal, Role};
use starter_spi::authz::{PolicyEngine, ResourceRef};

use crate::grants::{GrantError, GrantFilter, GrantStore, NewGrant};
use crate::instances::{PermissionTier, ShareScope};

use super::router::check_csrf;
use super::rules::store_err;
use super::state::AuthzRoutesState;

fn grant_store(state: &AuthzRoutesState) -> GrantStore {
    GrantStore::new(state.engine.store().clone())
}

/// May `principal` manage who has access to `(kind, resource_id)` in `tenant`?
///
/// A tenant admin always may. A non-admin may only if they hold the **Manage**
/// tier on that specific resource — modelled as the `delete` action, the top of
/// the view⊂edit⊂delete ladder (`acl::actions_for_tier(Manage)`). This is the
/// Grafana model: a dashboard's manager can share it without being a tenant
/// admin. Returns a ready `403` otherwise. `resource_id` is `None` for a
/// tenant-wide (kind-level) grant, which only an admin may write.
async fn require_manage(
    state: &AuthzRoutesState,
    principal: &Principal,
    kind: &str,
    resource_id: Option<&str>,
    tenant: &str,
) -> Result<(), Response> {
    if principal.role == Role::Admin {
        return Ok(());
    }
    // Only a concrete instance can be delegated to a non-admin; a kind-wide grant
    // (no resource_id) is an admin-only operation.
    let Some(id) = resource_id else {
        return Err(StatusCode::FORBIDDEN.into_response());
    };
    let object = ResourceRef::row(kind, id).with_tenant(tenant);
    if state.engine.check(principal, "delete", &object).await.is_allow() {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN.into_response())
    }
}

fn grant_err(e: GrantError) -> Response {
    match e {
        GrantError::Store(s) => store_err(s),
        GrantError::UnsupportedKind { kind } => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "error": "unsupported_kind",
                "kind": kind,
            })),
        )
            .into_response(),
    }
}

/// Gate a patch/delete that names a grant by id: resolve the grant's
/// `(kind, resource_id, tenant)` from the store, then apply [`require_manage`].
/// A missing grant id is treated as forbidden (not found is not disclosed to a
/// non-admin); admins skip the lookup.
async fn require_manage_for_grant(
    state: &AuthzRoutesState,
    principal: &Principal,
    grant_id: &str,
) -> Result<(), Response> {
    if principal.role == Role::Admin {
        return Ok(());
    }
    let rules = state
        .engine
        .store()
        .list_rules()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let Some(rule) = rules.into_iter().find(|r| r.id == grant_id) else {
        return Err(StatusCode::FORBIDDEN.into_response());
    };
    let tenant = rule.tenant_id.as_deref().unwrap_or_default();
    require_manage(
        state,
        principal,
        &rule.resource,
        rule.resource_id.as_deref(),
        tenant,
    )
    .await
}

pub(super) async fn create_grant(
    Extension(state): Extension<Arc<AuthzRoutesState>>,
    Extension(principal): Extension<Principal>,
    headers: HeaderMap,
    Json(body): Json<NewGrant>,
) -> Response {
    if let Err(r) = check_csrf(&headers) {
        return r;
    }
    if let Err(r) = require_manage(
        &state,
        &principal,
        &body.resource_kind,
        body.resource_id.as_deref(),
        &body.tenant_id,
    )
    .await
    {
        return r;
    }
    let gs = grant_store(&state);
    match gs.create(body, &principal.subject).await {
        Ok(g) => match state.engine.reload().await {
            Ok(()) => (StatusCode::CREATED, Json(serde_json::json!({ "grant": g })))
                .into_response(),
            Err(e) => {
                tracing::error!(target: "starter_authz", error = %e, "reload after grant insert failed");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Err(e) => grant_err(e),
    }
}

pub(super) async fn delete_grant(
    Extension(state): Extension<Arc<AuthzRoutesState>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Err(r) = check_csrf(&headers) {
        return r;
    }
    if let Err(r) = require_manage_for_grant(&state, &principal, &id).await {
        return r;
    }
    let gs = grant_store(&state);
    match gs.delete(&id).await {
        Ok(()) => match state.engine.reload().await {
            Ok(()) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => {
                tracing::error!(target: "starter_authz", error = %e, "reload after grant delete failed");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Err(e) => grant_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct ListQuery {
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    resource_kind: Option<String>,
    #[serde(default)]
    resource_id: Option<String>,
    #[serde(default)]
    tenant_id: Option<String>,
}

pub(super) async fn list_grants(
    Extension(state): Extension<Arc<AuthzRoutesState>>,
    Extension(principal): Extension<Principal>,
    Query(q): Query<ListQuery>,
) -> Response {
    let gs = grant_store(&state);
    // Pin non-super-admins to their tenant.
    let tenant_id = if principal.tenant_id.as_deref() == Some("*") {
        q.tenant_id
    } else {
        principal.tenant_id.clone().or(q.tenant_id)
    };
    let filter = GrantFilter {
        subject: q.subject,
        resource_kind: q.resource_kind,
        resource_id: q.resource_id,
        tenant_id,
    };
    match gs.list(filter).await {
        Ok(grants) => Json(serde_json::json!({ "grants": grants })).into_response(),
        Err(e) => grant_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct PatchBody {
    tier: PermissionTier,
}

pub(super) async fn patch_grant(
    Extension(state): Extension<Arc<AuthzRoutesState>>,
    Extension(principal): Extension<Principal>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<PatchBody>,
) -> Response {
    if let Err(r) = check_csrf(&headers) {
        return r;
    }
    if let Err(r) = require_manage_for_grant(&state, &principal, &id).await {
        return r;
    }
    let gs = grant_store(&state);
    match gs.patch_tier(&id, body.tier, &principal.subject).await {
        Ok(g) => match state.engine.reload().await {
            Ok(()) => Json(serde_json::json!({ "grant": g })).into_response(),
            Err(e) => {
                tracing::error!(target: "starter_authz", error = %e, "reload after grant patch failed");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Err(e) => grant_err(e),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct ShareScopeBody {
    scope: ShareScope,
    /// Optional override for super-admins; everyone else is pinned
    /// to their principal's tenant.
    #[serde(default)]
    tenant_id: Option<String>,
}

pub(super) async fn set_share_scope(
    Extension(state): Extension<Arc<AuthzRoutesState>>,
    Extension(principal): Extension<Principal>,
    Path((kind, resource_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<ShareScopeBody>,
) -> Response {
    if let Err(r) = check_csrf(&headers) {
        return r;
    }
    let tenant_id = if principal.tenant_id.as_deref() == Some("*") {
        match body.tenant_id {
            Some(t) => t,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "tenant_id required for super-admin"})),
                )
                    .into_response();
            }
        }
    } else {
        match principal.tenant_id.clone() {
            Some(t) => t,
            None => return StatusCode::FORBIDDEN.into_response(),
        }
    };
    if let Err(r) =
        require_manage(&state, &principal, &kind, Some(&resource_id), &tenant_id).await
    {
        return r;
    }
    let gs = grant_store(&state);
    match gs
        .set_share_scope(&kind, &resource_id, &tenant_id, body.scope, &principal.subject)
        .await
    {
        Ok(()) => match state.engine.reload().await {
            Ok(()) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => {
                tracing::error!(target: "starter_authz", error = %e, "reload after share-scope failed");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Err(e) => grant_err(e),
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    //! E2E: create a grant, then verify the engine resolves an
    //! Allow for a team-member principal and a Deny for everyone
    //! else.

    use std::sync::Arc;

    use crate::grants::{GrantStore, NewGrant};
    use crate::instances::PermissionTier;
    use crate::store::SqlitePolicyStore;
    use crate::{DbPolicyEngine, StaticRegistry};
    use starter_spi::auth::{Principal, Role, Scope};
    use starter_spi::authz::{Decision, Ownership, PolicyEngine, ResourceRef, ResourceSpec};
    use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral};

    async fn setup() -> (Arc<DbPolicyEngine>, GrantStore) {
        let pool = ephemeral().await;
        migrate(&pool)
            .with_source(MigrationSource {
                name: "starter_authz",
                migrator: &crate::store::AUTHZ_SQLITE_MIGRATOR,
            })
            .run()
            .await
            .unwrap();
        let store: Arc<dyn crate::store::PolicyStore> =
            Arc::new(SqlitePolicyStore::new(pool));
        let registry = Arc::new(StaticRegistry::new());
        registry.register_spec(ResourceSpec::from_static_tenant_scoped(
            "rubix.dashboard.page",
            &["view", "edit", "delete"],
            Ownership::Subject,
            "Page",
            "test",
        ));
        let engine = Arc::new(
            DbPolicyEngine::new(store.clone(), registry, false)
                .await
                .unwrap(),
        );
        let gs = GrantStore::new(store);
        (engine, gs)
    }

    fn p(sub: &str, teams: Vec<String>) -> Principal {
        Principal {
            subject: sub.into(),
            role: Role::Reader,
            scopes: vec![Scope("reader".into())],
            tenant_id: Some("t1".into()),
            teams,
            tenant_scope: Vec::new(),
            extra: serde_json::Value::Null,
        }
    }

    fn page_ref() -> ResourceRef {
        ResourceRef {
            kind: "rubix.dashboard.page".into(),
            id: Some("dash_x".into()),
            owner: Some("owner@example.com".into()),
            tenant: Some("t1".into()),
        }
    }

    #[tokio::test]
    async fn grant_lets_team_member_edit_but_denies_outsider() {
        let (engine, gs) = setup().await;
        gs.create(
            NewGrant {
                subject: crate::grants::GrantSubject::Team {
                    slug: "hvac-ops".into(),
                },
                resource_kind: "rubix.dashboard.page".into(),
                resource_id: Some("dash_x".into()),
                tier: PermissionTier::Edit,
                tenant_id: "t1".into(),
            },
            "admin",
        )
        .await
        .unwrap();
        engine.reload().await.unwrap();

        let member = p("u-member", vec!["hvac-ops".into()]);
        let outsider = p("u-outsider", vec![]);

        let d_member = engine.check(&member, "edit", &page_ref()).await;
        assert!(matches!(d_member, Decision::Allow { .. }), "got {d_member:?}");
        let d_outsider = engine.check(&outsider, "edit", &page_ref()).await;
        assert!(matches!(d_outsider, Decision::Deny { .. }), "got {d_outsider:?}");
    }

    /// The Manage-gate: who may share a resource. A tenant admin always may; a
    /// non-admin may only if they hold the Manage tier (delete action) on the
    /// specific resource; a non-admin without it is forbidden. This is the
    /// "tenant admins + resource Manage" delegation model.
    #[tokio::test]
    async fn require_manage_admits_admin_and_resource_manager_only() {
        use super::require_manage;
        use crate::routes::state::AuthzRoutesState;

        let (engine, gs) = setup().await;
        // Grant the Manage tier (view+edit+delete) on dash_x to team:hvac-ops.
        gs.create(
            NewGrant {
                subject: crate::grants::GrantSubject::Team {
                    slug: "hvac-ops".into(),
                },
                resource_kind: "rubix.dashboard.page".into(),
                resource_id: Some("dash_x".into()),
                tier: PermissionTier::Manage,
                tenant_id: "t1".into(),
            },
            "admin",
        )
        .await
        .unwrap();
        engine.reload().await.unwrap();

        let registry = Arc::new(StaticRegistry::new());
        let state = AuthzRoutesState::new(engine, registry);

        let mut admin = p("an-admin", vec![]);
        admin.role = Role::Admin;
        let manager = p("u-manager", vec!["hvac-ops".into()]);
        let bystander = p("u-bystander", vec![]);

        // Admin: allowed regardless of grants.
        assert!(require_manage(&state, &admin, "rubix.dashboard.page", Some("dash_x"), "t1")
            .await
            .is_ok());
        // Non-admin holding Manage on the resource: allowed.
        assert!(
            require_manage(&state, &manager, "rubix.dashboard.page", Some("dash_x"), "t1")
                .await
                .is_ok()
        );
        // Non-admin without Manage: forbidden.
        assert!(
            require_manage(&state, &bystander, "rubix.dashboard.page", Some("dash_x"), "t1")
                .await
                .is_err()
        );
        // Even a manager cannot write a kind-wide (no resource_id) grant.
        assert!(
            require_manage(&state, &manager, "rubix.dashboard.page", None, "t1")
                .await
                .is_err()
        );
    }
}
