//! Create the first-run admin user. Invoked from the CLI:
//! `starter-cli admin create --email … --role admin`.

use uuid::Uuid;

use crate::password;
use crate::role::Role;
use crate::store::{UserStore, UserStoreError};

/// Create a user with the given email, password, and role.
///
/// The caller (the CLI) is responsible for prompting the operator
/// for the password — this function never reads from stdin or env
/// directly. Returns the new user's id.
pub async fn create_admin<U: UserStore + ?Sized>(
    store: &U,
    email: &str,
    password: &str,
    role: Role,
) -> Result<String, AdminError> {
    let hash = password::hash(password).map_err(|_| AdminError::HashFailed)?;
    let id = Uuid::new_v4().to_string();
    store
        .create(&id, email, &hash, role)
        .await
        .map_err(|e| match e {
            UserStoreError::Conflict => AdminError::Conflict,
            UserStoreError::Backend(s) => AdminError::Store(s),
            UserStoreError::NotFound => AdminError::Store("unexpected NotFound on create".into()),
        })?;
    Ok(id)
}

/// Admin-operation failures.
#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    /// A user with that email already exists.
    #[error("user already exists")]
    Conflict,
    /// Underlying store error.
    #[error("store error: {0}")]
    Store(String),
    /// Argon2 hash generation failed.
    #[error("password hashing failed")]
    HashFailed,
}
