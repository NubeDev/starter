//! Revoke a session by id. `/auth/logout` calls this.

use crate::store::{SessionStore, SessionStoreError};

use super::issue::SessionError;

/// Mark the session row revoked. Subsequent requests carrying the
/// cookie return `Unauthenticated`. Idempotent — revoking a missing
/// or already-revoked session is not an error.
pub async fn revoke<S: SessionStore + ?Sized>(
    store: &S,
    session_id: &str,
) -> Result<(), SessionError> {
    store
        .revoke(session_id)
        .await
        .map_err(|SessionStoreError::Backend(s)| SessionError::Store(s))
}
