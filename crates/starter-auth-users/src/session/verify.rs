//! Look up a session by cookie value and resolve it to a `Principal`
//! by joining the user record.

use serde_json::Value;
use starter_spi::auth::Principal;

use crate::principal_extras::{NoPrincipalExtras, PrincipalExtrasLookup};
use crate::store::{SessionStore, TenantStore, UserStore, UserStoreError};

use super::issue::SessionError;

/// Verify `cookie_value` against the session table and return the
/// matching principal.
///
/// `Principal.extra` is `Value::Null` — wire a
/// [`PrincipalExtrasLookup`] via [`verify_session_with_extras`] for
/// the authz attribute bus (`oauth.*` etc., see
/// `DOCS/auth/authz/SCOPE.md` R8).
///
/// Returns `SessionError::NotFound` for missing / expired / revoked
/// sessions and for sessions whose owning user has been removed.
pub async fn verify_session<S, U>(
    sessions: &S,
    users: &U,
    cookie_value: &str,
) -> Result<Principal, SessionError>
where
    S: SessionStore + ?Sized,
    U: UserStore + ?Sized,
{
    verify_session_with_extras(sessions, users, &NoPrincipalExtras, cookie_value).await
}

/// Verify a session and merge `extras.extras_for(user_id)` into
/// `Principal.extra` before returning.
///
/// Merge rule (deliberately conservative):
///
/// - `Value::Null` from the lookup → `Principal.extra =
///   Value::Null` (the pre-extras behaviour).
/// - `Value::Object` from the lookup → that object becomes
///   `Principal.extra`. Per `DOCS/auth/authz/SCOPE.md` R8 each
///   provider crate owns a reserved top-level namespace
///   (`oauth.*` for the OAuth crate) to avoid collisions with
///   consumer-defined fields.
/// - Any other shape is treated as `Value::Null` — the lookup is
///   supposed to return an object.
pub async fn verify_session_with_extras<S, U, E>(
    sessions: &S,
    users: &U,
    extras: &E,
    cookie_value: &str,
) -> Result<Principal, SessionError>
where
    S: SessionStore + ?Sized,
    U: UserStore + ?Sized,
    E: PrincipalExtrasLookup + ?Sized,
{
    let session = sessions
        .find_active(cookie_value)
        .await
        .map_err(|crate::store::SessionStoreError::Backend(s)| SessionError::Store(s))?
        .ok_or(SessionError::NotFound)?;
    let user = users
        .find_by_id(&session.user_id)
        .await
        .map_err(|e| match e {
            UserStoreError::Backend(s) => SessionError::Store(s),
            UserStoreError::NotFound | UserStoreError::Conflict => SessionError::NotFound,
        })?
        .ok_or(SessionError::NotFound)?;

    let extra = match extras.extras_for(&user.id).await {
        Ok(Value::Object(o)) => Value::Object(o),
        Ok(_) => Value::Null,
        Err(e) => return Err(SessionError::Store(e.to_string())),
    };

    // Phase 7a — the session carries its tenant binding (set
    // at login from the user's membership row).
    Ok(Principal {
        subject: user.id,
        role: user.role,
        scopes: Vec::new(),
        tenant_id: session.tenant_id,
        teams: Vec::new(),
        extra,
    })
}

/// Phase 7b — verify a session **and** populate `Principal.teams`
/// from `starter_auth_users_team_members` joined to
/// `starter_auth_users_teams.slug` for the session's
/// `(user_id, tenant_id)`. A tenantless session yields an empty
/// team list (rules referencing `principal.teams` simply do not
/// match, per R13). Errors from the tenant store are surfaced as
/// `SessionError::Store` — a sink-side hiccup is not allowed to
/// silently shrink a principal's team set, because that would
/// silently widen access (no team match → no team-grant allow,
/// which is the opposite of the conservative default).
pub async fn verify_session_with_teams<S, U, T>(
    sessions: &S,
    users: &U,
    tenants: &T,
    cookie_value: &str,
) -> Result<Principal, SessionError>
where
    S: SessionStore + ?Sized,
    U: UserStore + ?Sized,
    T: TenantStore + ?Sized,
{
    verify_session_with_teams_and_extras(sessions, users, tenants, &NoPrincipalExtras, cookie_value)
        .await
}

/// Phase 7b — combined variant: extras + team lookup. Mirrors
/// [`verify_session_with_extras`] so consumers wiring both don't
/// need two separate paths.
pub async fn verify_session_with_teams_and_extras<S, U, T, E>(
    sessions: &S,
    users: &U,
    tenants: &T,
    extras: &E,
    cookie_value: &str,
) -> Result<Principal, SessionError>
where
    S: SessionStore + ?Sized,
    U: UserStore + ?Sized,
    T: TenantStore + ?Sized,
    E: PrincipalExtrasLookup + ?Sized,
{
    let mut principal = verify_session_with_extras(sessions, users, extras, cookie_value).await?;
    if let Some(tenant_id) = &principal.tenant_id {
        // Super-admin sentinel "*" intentionally yields no team
        // memberships — cross-tenant admins are role-driven, not
        // team-driven.
        if tenant_id != "*" {
            let teams = tenants
                .team_slugs_for_user(tenant_id, &principal.subject)
                .await
                .map_err(|e| SessionError::Store(e.to_string()))?;
            principal.teams = teams;
        }
    }
    Ok(principal)
}
