//! Look up a session by cookie value and resolve it to a `Principal`
//! by joining the user record.

use starter_spi::auth::Principal;

use crate::store::{SessionStore, UserStore, UserStoreError};

use super::issue::SessionError;

/// Verify `cookie_value` against the session table and return the
/// matching principal.
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

    Ok(Principal {
        subject: user.id,
        role: user.role,
        scopes: Vec::new(),
        extra: serde_json::Value::Null,
    })
}
