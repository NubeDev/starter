//! Per-entry auth gate application.
//!
//! SCOPE post-R13 "Per-entry auth shape": each contribute entry carries
//! an optional `AuthGate { require_role, require_scope }`. The adapter
//! is responsible for wrapping the entry's handler in the matching
//! middleware so the extension never sees a request that did not pass
//! the gate. The extension cannot weaken or skip auth.
//!
//! `require_role` is parsed against `starter_spi::auth::Role`
//! ("reader" / "writer" / "admin"); an unknown role string is a
//! **load-time error** surfaced through [`RestBuildError`] — typos in
//! the manifest do not silently become permissive routes. `Scope` is a
//! free-form newtype, so any string parses.
//!
//! [`RestBuildError`]: super::RestBuildError

use axum::Router;
use starter_authz::with_permission_owned;
use starter_ext_spi::AuthGate;
use starter_server::auth::{with_role, with_scope};
use starter_spi::auth::{Role, Scope};
use starter_spi::authz::ResourceRegistry;

use super::router::RestBuildError;

/// Apply the gate's `require_role` + `require_scope` + `permission`
/// to `router`. Layer order (SCOPE-EXT R15 — Phase 7d):
///
/// ```text
///   with_role (outer, from require_role)
///     → with_scope (from require_scope)
///       → with_permission (inner, from permission)        ← NEW in Phase 7d
///         → handler
/// ```
///
/// The layers are added innermost-first (`with_permission` first,
/// then `with_scope`, then `with_role`) because axum's
/// `Router::layer` wraps the **existing** router — the most-recent
/// `.layer(...)` becomes the outermost middleware.
///
/// **Audit consequence (intended; do NOT flip the order to "fix"):**
/// a role-denied request is rejected by `with_role` before
/// `with_permission` runs, so the policy engine's `DecisionSink`
/// never records a permission-deny entry for that request. The
/// pre-role rejection lives on the `tracing` side via
/// `with_role`'s log path. Dashboards keyed on the
/// `starter_authz_decisions` table must exclude pre-role
/// rejections from "permission deny rate" panels. Inverting the
/// order to give symmetric audit would lose the ability for a
/// coarse role gate to short-circuit the policy engine entirely —
/// that's the wrong trade for the common case where most requests
/// are role-denied.
///
/// If the gate has no fields set, the router is returned unchanged
/// ("inherit the adapter's default").
pub(crate) fn apply_gate<S>(
    router: Router<S>,
    gate: &AuthGate,
    entry_id: &str,
    resource_registry: Option<&dyn ResourceRegistry>,
) -> Result<Router<S>, RestBuildError>
where
    S: Clone + Send + Sync + 'static,
{
    let mut router = router;
    // Innermost: permission gate (calls into the host PolicyEngine).
    if let Some(perm) = gate.permission.as_ref() {
        // Validate the resource kind against the registry at build
        // time. Symmetric with `UnknownRole`: a typo'd kind is a
        // deploy-time error rather than a runtime 403, and the broken
        // extension refuses to mount while the rest of the host comes
        // up.
        let known = match resource_registry {
            Some(reg) => reg.lookup(&perm.resource).is_some(),
            None => false,
        };
        if !known {
            return Err(RestBuildError::UnknownResource {
                entry: entry_id.to_string(),
                resource: perm.resource.clone(),
            });
        }
        router = with_permission_owned(router, perm.resource.clone(), perm.action.clone());
    }
    // Middle: scope.
    if let Some(scope_str) = gate.require_scope.as_deref() {
        router = with_scope(router, Scope::new(scope_str));
    }
    // Outer: role.
    if let Some(role_str) = gate.require_role.as_deref() {
        let role = parse_role(role_str).ok_or_else(|| RestBuildError::UnknownRole {
            entry: entry_id.to_string(),
            role: role_str.to_string(),
        })?;
        router = with_role(router, role);
    }
    Ok(router)
}

/// Case-insensitive parse of the manifest role string into the typed
/// [`Role`] vocabulary. Kept tolerant on case because operators write
/// `Admin` vs `admin` interchangeably; everything else is a load error.
pub(crate) fn parse_role(s: &str) -> Option<Role> {
    match s.to_ascii_lowercase().as_str() {
        "reader" => Some(Role::Reader),
        "writer" => Some(Role::Writer),
        "admin" => Some(Role::Admin),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_role_is_case_insensitive() {
        assert_eq!(parse_role("admin"), Some(Role::Admin));
        assert_eq!(parse_role("ADMIN"), Some(Role::Admin));
        assert_eq!(parse_role("Writer"), Some(Role::Writer));
        assert_eq!(parse_role("reader"), Some(Role::Reader));
        assert_eq!(parse_role("super"), None);
    }
}
