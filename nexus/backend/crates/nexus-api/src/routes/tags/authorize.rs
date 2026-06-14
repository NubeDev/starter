//! Resolve a `{kind}/{id}` tag target and enforce the resource's own authz.
//!
//! Tags are **behaviour-affecting inputs** under WS-13 (they drive queries via
//! `PageContext.tags`), so the tag write/read path can no longer be tenant-only
//! (the old `set.rs`/`get.rs` resolved only the tenant — WS-13 §3). Before
//! touching `nexus_tags` a handler must (1) confirm the target entity exists
//! **within the caller's tenant** — `nexus_tags` is polymorphic with no FK, so
//! it would otherwise tag a nonexistent or foreign id — and (2) enforce the same
//! grant the resource's own routes use: `edit` to write tags, `view` to read.
//!
//! This is generic over the taggable kinds that *are* nexus resources
//! (dashboard, datasource, flow, alert_rule), so the same fix also closes the
//! gap for datasource tags. `user`/`team` are identity rows owned by another
//! layer with no per-resource nexus grant; their tags stay tenant-scoped (they
//! are not query inputs), authorised only by tenant membership.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use nexus_spi::dto::tag::TaggableKind;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::authz::{
    self, ACTION_EDIT, ACTION_VIEW, KIND_DASHBOARD, KIND_DATASOURCE, KIND_DETECTION, KIND_FLOW,
};
use crate::state::AppState;

/// Authorize a tag **read** (`GET`) on `{kind}/{id}`: the target must exist
/// in-tenant and the caller must hold `view` on it (for resource kinds). Returns
/// a ready 404/403 response on failure.
#[allow(clippy::result_large_err)]
pub async fn authorize_read(
    state: &AppState,
    principal: &Principal,
    tenant: &str,
    kind: TaggableKind,
    id: &str,
) -> Result<(), Response> {
    authorize(state, principal, tenant, kind, id, ACTION_VIEW).await
}

/// Authorize a tag **write** (`PUT`) on `{kind}/{id}`: the target must exist
/// in-tenant and the caller must hold `edit` on it (for resource kinds). Returns
/// a ready 404/403 response on failure.
#[allow(clippy::result_large_err)]
pub async fn authorize_write(
    state: &AppState,
    principal: &Principal,
    tenant: &str,
    kind: TaggableKind,
    id: &str,
) -> Result<(), Response> {
    authorize(state, principal, tenant, kind, id, ACTION_EDIT).await
}

/// The nexus authz resource kind a taggable kind maps to, or `None` for the
/// identity kinds (user/team) that carry no per-resource grant.
fn resource_kind(kind: TaggableKind) -> Option<&'static str> {
    match kind {
        TaggableKind::Dashboard => Some(KIND_DASHBOARD),
        TaggableKind::Datasource => Some(KIND_DATASOURCE),
        TaggableKind::Flow => Some(KIND_FLOW),
        TaggableKind::Detection => Some(KIND_DETECTION),
        TaggableKind::User | TaggableKind::Team => None,
    }
}

/// Whether `id` names an entity of `kind` that exists within `tenant` (RLS hides
/// other tenants, so a tenant-scoped miss covers absent *and* foreign).
async fn exists_in_tenant(
    state: &AppState,
    tenant: &str,
    kind: TaggableKind,
    id: Uuid,
) -> Result<bool, starter_spi::Error> {
    let found = match kind {
        TaggableKind::Dashboard => nexus_store::dashboard::by_id(&state.metadata, tenant, id)
            .await?
            .is_some(),
        TaggableKind::Datasource => nexus_store::datasource::get(&state.metadata, tenant, id)
            .await?
            .is_some(),
        TaggableKind::Flow => nexus_store::flow::get(&state.metadata, tenant, id)
            .await?
            .is_some(),
        TaggableKind::Detection => nexus_store::detection::get(&state.metadata, tenant, id)
            .await?
            .is_some(),
        // Identity kinds are not validated here (owned by another layer); the
        // caller path treats them as tenant-scoped without an existence probe.
        TaggableKind::User | TaggableKind::Team => true,
    };
    Ok(found)
}

#[allow(clippy::result_large_err)]
async fn authorize(
    state: &AppState,
    principal: &Principal,
    tenant: &str,
    kind: TaggableKind,
    id: &str,
    action: &str,
) -> Result<(), Response> {
    let Some(resource_kind) = resource_kind(kind) else {
        // user/team: no per-resource grant. Tenant membership (already checked by
        // the caller) is the authority; tags here are not query inputs.
        return Ok(());
    };
    // A resource id must be a uuid; a non-uuid can't name a real row, so reject
    // it as not-found rather than letting it tag a bogus id.
    let uuid = Uuid::parse_str(id).map_err(|_| not_found())?;
    // Existence first, so a caller without the grant on a *nonexistent* id still
    // gets a 404 (existence is not leaked by a 403).
    match exists_in_tenant(state, tenant, kind, uuid).await {
        Ok(true) => {}
        Ok(false) => return Err(not_found()),
        Err(e) => return Err(starter_server::error::IntoResponse(e).into_response()),
    }
    authz::require(
        state.engine.as_ref(),
        principal,
        action,
        resource_kind,
        id,
        tenant,
    )
    .await
}

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "not found").into_response()
}
