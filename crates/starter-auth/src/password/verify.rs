//! Verify a plaintext password against a stored hash. Constant-time
//! on the hash comparison side (delegated to the argon2 library).

use super::hash::PasswordError;

/// Return `Ok(true)` if `password` matches `encoded_hash`, `Ok(false)`
/// otherwise. Returns `Err` only for malformed hashes — wrong
/// passwords return `Ok(false)`.
pub fn verify(_password: &str, _encoded_hash: &str) -> Result<bool, PasswordError> {
    todo!("verify impl lands with the auth migrations")
}
