//! Revoke a session by id. `/auth/logout` calls this.

use super::issue::SessionError;
use super::store::SessionStore;

/// Mark the session row revoked. Subsequent requests carrying the
/// cookie return `Unauthenticated`.
pub async fn revoke(_store: &SessionStore, _session_id: &str) -> Result<(), SessionError> {
    todo!("revoke impl lands with the auth migrations")
}
