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
use starter_ext_spi::AuthGate;
use starter_server::auth::{with_role, with_scope};
use starter_spi::auth::{Role, Scope};

use super::router::RestBuildError;

/// Apply the gate's `require_role` + `require_scope` to `router`. Both
/// gates wrap as middleware layers; `with_role` is conventionally the
/// outer of the two ("must be admin AND have scope X" — gate the
/// coarse axis first). If the gate has neither field set, the router is
/// returned unchanged ("inherit the adapter's default").
pub(crate) fn apply_gate<S>(
    router: Router<S>,
    gate: &AuthGate,
    entry_id: &str,
) -> Result<Router<S>, RestBuildError>
where
    S: Clone + Send + Sync + 'static,
{
    let mut router = router;
    if let Some(scope_str) = gate.require_scope.as_deref() {
        router = with_scope(router, Scope::new(scope_str));
    }
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
