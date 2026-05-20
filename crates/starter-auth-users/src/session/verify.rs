//! Look up a session by cookie value and resolve it to a `Principal`
//! by joining the user record.

use serde_json::Value;
use starter_spi::auth::Principal;

use crate::principal_extras::{NoPrincipalExtras, PrincipalExtrasLookup};
use crate::store::{SessionStore, UserStore, UserStoreError};

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

    Ok(Principal {
        subject: user.id,
        role: user.role,
        scopes: Vec::new(),
        extra,
    })
}
