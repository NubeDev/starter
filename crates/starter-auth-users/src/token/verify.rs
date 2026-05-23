//! Verify a presented bearer token against the hashed-token table
//! and return the matching principal.

use starter_spi::auth::Principal;

use crate::password;
use crate::store::{TenantStore, TokenStore, UserStore, UserStoreError};

use super::issue::{map_store_err, TokenError, TOKEN_PREFIX};

/// Verify `presented` (the full `sak_<id>.<secret>` plaintext) against
/// the token table.
///
/// On success, returns the principal owning the token and updates
/// `last_used_at` as a side effect — failures of the touch are
/// logged but do not fail the request.
pub async fn verify<T, U>(tokens: &T, users: &U, presented: &str) -> Result<Principal, TokenError>
where
    T: TokenStore + ?Sized,
    U: UserStore + ?Sized,
{
    let rest = presented
        .strip_prefix(TOKEN_PREFIX)
        .ok_or(TokenError::Invalid)?;
    let (id, secret) = rest.split_once('.').ok_or(TokenError::Invalid)?;

    let row = tokens
        .find_active(id)
        .await
        .map_err(map_store_err)?
        .ok_or(TokenError::Revoked)?;

    match password::verify(secret, &row.hashed_token) {
        Ok(true) => {}
        Ok(false) => return Err(TokenError::Invalid),
        Err(_) => return Err(TokenError::Internal),
    }

    if let Err(e) = tokens.touch_last_used(&row.id).await {
        tracing::warn!(
            target: "starter_auth_users",
            error = %e,
            "failed to update token last_used_at",
        );
    }

    let user = users
        .find_by_id(&row.user_id)
        .await
        .map_err(|e| match e {
            UserStoreError::Backend(s) => TokenError::Store(s),
            UserStoreError::NotFound | UserStoreError::Conflict => TokenError::Invalid,
        })?
        .ok_or(TokenError::Invalid)?;

    // Phase 7a (R11) — the token carries its tenant binding.
    // `Some("*")` is the super-admin sentinel reserved for tokens
    // issued by users with global Admin role; the engine treats
    // it as a cross-tenant bypass.
    Ok(Principal {
        subject: user.id,
        role: user.role,
        scopes: row.scopes,
        tenant_id: Some(row.tenant_id),
        teams: Vec::new(),
        extra: serde_json::Value::Null,
    })
}

/// Phase 7b — verify a token **and** populate `Principal.teams`
/// from the team_members join for `(token.tenant_id, user_id)`.
/// The super-admin sentinel `"*"` short-circuits the lookup with
/// an empty team list (cross-tenant admin tokens are role-driven,
/// not team-driven). See `verify_session_with_teams` in
/// `session::verify` for the parallel session-side path.
pub async fn verify_with_teams<T, U, TS>(
    tokens: &T,
    users: &U,
    tenants: &TS,
    presented: &str,
) -> Result<Principal, TokenError>
where
    T: TokenStore + ?Sized,
    U: UserStore + ?Sized,
    TS: TenantStore + ?Sized,
{
    let mut principal = verify(tokens, users, presented).await?;
    if let Some(t) = principal.tenant_id.clone() {
        if t != "*" {
            principal.teams = tenants
                .team_slugs_for_user(&t, &principal.subject)
                .await
                .map_err(|e| TokenError::Store(e.to_string()))?;
        }
    }
    Ok(principal)
}
