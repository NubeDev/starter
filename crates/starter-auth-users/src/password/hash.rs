//! Hash a plaintext password using argon2id (via `password-auth`,
//! which wraps the `argon2` crate with sensible defaults).

use password_auth::generate_hash;

/// Hash `password` and return the encoded PHC string
/// (`$argon2id$v=19$...`).
///
/// The salt is generated internally by `password-auth`. Argon2id
/// parameters follow the library's defaults — currently OWASP-
/// recommended for interactive logins.
pub fn hash(password: &str) -> Result<String, PasswordError> {
    Ok(generate_hash(password.as_bytes()))
}

/// Password-handling failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PasswordError {
    /// Hashing failed (typically an internal argon2 error).
    #[error("password hashing failed")]
    HashFailed,
    /// Verification failed (wrong password OR malformed hash).
    #[error("password verification failed")]
    VerifyFailed,
}
