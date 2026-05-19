//! Verify a plaintext password against a stored hash. Constant-time
//! on the hash comparison side (delegated to the argon2 library).

use password_auth::{verify_password, VerifyError};

use super::hash::PasswordError;

/// Return `Ok(true)` if `password` matches `encoded_hash`, `Ok(false)`
/// for a wrong password. Returns `Err` only for malformed / invalid
/// hashes — a wrong password is not an error.
pub fn verify(password: &str, encoded_hash: &str) -> Result<bool, PasswordError> {
    match verify_password(password.as_bytes(), encoded_hash) {
        Ok(()) => Ok(true),
        Err(VerifyError::PasswordInvalid) => Ok(false),
        Err(VerifyError::Parse(_)) => Err(PasswordError::VerifyFailed),
    }
}
