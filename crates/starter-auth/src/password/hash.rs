//! Hash a plaintext password. Uses argon2id with library defaults
//! (currently OWASP-recommended parameters).

/// Hash `password` and return the encoded PHC string (`$argon2id$...`).
///
/// Stubbed for v0.1 — the real implementation pulls in `password-auth`
/// or `argon2` crate. Public surface is locked.
pub fn hash(_password: &str) -> Result<String, PasswordError> {
    todo!("hash impl lands with the auth migrations")
}

/// Password-handling failures.
#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    /// Hashing failed (typically an internal argon2 error).
    #[error("password hashing failed")]
    HashFailed,
    /// Verification failed (wrong password OR malformed hash).
    #[error("password verification failed")]
    VerifyFailed,
}
