//! `POST /v1/authz/check` — dry-run an authorization check. The
//! request describes a (principal, action, resource) tuple and
//! the response surfaces what the engine would decide *right
//! now*. Lets the admin UI preview rule edits before saving and
//! powers the SCOPE.md Phase 3 "dry-run-matches-real-check"
//! invariant.

use std::sync::Arc;

use axum::extract::Extension;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use starter_spi::auth::{Principal, Role};
use starter_spi::authz::{Decision, PolicyEngine, ResourceRef};

use super::state::AuthzRoutesState;

/// Body of `POST /v1/authz/check`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRequest {
    /// Principal to simulate. The admin may dry-run an arbitrary
    /// principal — that's the whole point — so we trust the body
    /// instead of pulling from the request extension.
    pub principal: SimulatedPrincipal,
    /// Action to test (`"read"`, `"create"`, …).
    pub action: String,
    /// Resource the check applies to.
    pub resource: SimulatedResource,
}

/// Strictly-validated subset of [`Principal`]. We don't accept
/// `extra` claims here because their shape is consumer-defined;
/// dry-run callers wanting OAuth attributes can either pass a
/// full `Principal` via a future extension or wait for the live
/// `/me`-aware variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedPrincipal {
    /// Subject id.
    pub subject: String,
    /// Coarse role.
    pub role: Role,
    /// Optional `extra` claims. Pass `null` for the no-OAuth path;
    /// the engine merges this exactly like the real flow.
    #[serde(default)]
    pub extra: serde_json::Value,
}

/// Body of the resource leg of [`CheckRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedResource {
    /// Resource kind (must be registered, or the engine returns
    /// `Decision::Deny { reason: "unknown_resource" }`).
    pub kind: String,
    /// Optional row id — present for row-scoped checks.
    #[serde(default)]
    pub id: Option<String>,
    /// Optional owner subject — populates the engine's `owner`
    /// magic condition.
    #[serde(default)]
    pub owner: Option<String>,
}

/// Response of `POST /v1/authz/check`.
#[derive(Debug, Serialize)]
pub struct CheckResponse {
    /// `"allow"` or `"deny"`.
    pub decision: &'static str,
    /// Reason code on deny (`"no_matching_rule"`,
    /// `"unknown_resource"`, `"not_owner"`, `"explicit_deny"`).
    /// `None` on allow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Id of the rule that produced the decision, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_rule: Option<String>,
}

pub(super) async fn check_handler(
    Extension(state): Extension<Arc<AuthzRoutesState>>,
    Json(req): Json<CheckRequest>,
) -> Response {
    let principal = Principal {
        subject: req.principal.subject,
        role: req.principal.role,
        scopes: Vec::new(),
        tenant_id: None,
        teams: Vec::new(),
        extra: req.principal.extra,
    };
    let resource = match (req.resource.id.as_deref(), req.resource.owner.as_deref()) {
        (Some(id), Some(owner)) => ResourceRef::row(&req.resource.kind, id).with_owner(owner),
        (Some(id), None) => ResourceRef::row(&req.resource.kind, id),
        (None, Some(owner)) => ResourceRef::collection(&req.resource.kind).with_owner(owner),
        (None, None) => ResourceRef::collection(&req.resource.kind),
    };

    let decision = state.engine.check(&principal, &req.action, &resource).await;
    let body = match decision {
        Decision::Allow { matched_rule } => CheckResponse {
            decision: "allow",
            reason: None,
            matched_rule,
        },
        Decision::Deny {
            reason,
            matched_rule,
        } => CheckResponse {
            decision: "deny",
            reason: Some(reason),
            matched_rule,
        },
    };
    Json(body).into_response()
}
