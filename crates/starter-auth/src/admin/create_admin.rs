//! Create the first-run admin user. Invoked from the CLI:
//! `starter-cli admin create --email … --role admin`.

use crate::role::Role;

/// Create an admin user with the given email and role.
///
/// Reads the password from stdin (prompted by the CLI; never an
/// argument). Returns the new user's id.
pub async fn create_admin(_email: &str, _role: Role) -> Result<String, AdminError> {
    todo!("create_admin lands with the auth migrations")
}

/// Admin-operation failures.
#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    /// A user with that email already exists.
    #[error("user already exists")]
    Conflict,
    /// Underlying store error.
    #[error("store error")]
    Store,
}
