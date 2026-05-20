//! Email and password validation for signup (and admin-create).
//!
//! Exported so `admin::create_admin` enforces the same policy.

use super::blocklist;

/// Minimum password length. Configurable via `SIGNUP_PASSWORD_MIN_LEN`
/// env var at startup; defaults to 12.
pub const DEFAULT_PASSWORD_MIN_LEN: usize = 12;

/// Maximum password length (argon2 practical upper bound).
pub const PASSWORD_MAX_LEN: usize = 4096;

/// Maximum email length per RFC 5321.
pub const EMAIL_MAX_LEN: usize = 254;

/// Validation failure returned from [`validate_signup_input`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Email failed basic structural checks.
    InvalidEmail(String),
    /// Password does not meet strength requirements.
    WeakPassword(String),
}

/// Validate email and password for signup.
///
/// Rules:
/// - Email must contain exactly one `@`, local and domain parts must
///   be non-empty, total length ≤ 254.
/// - Password length ≥ `min_len` and ≤ 4096.
/// - Password is not equal to the email local-part.
/// - Password is not in the compile-time top-100 blocklist.
pub fn validate_signup_input(
    email: &str,
    password: &str,
    min_len: usize,
) -> Result<(), ValidationError> {
    // --- Email validation ---
    if email.len() > EMAIL_MAX_LEN {
        return Err(ValidationError::InvalidEmail(
            "email exceeds 254 characters".into(),
        ));
    }
    let at_pos = match email.rfind('@') {
        Some(pos) => pos,
        None => {
            return Err(ValidationError::InvalidEmail(
                "email must contain '@'".into(),
            ));
        }
    };
    let local = &email[..at_pos];
    let domain = &email[at_pos + 1..];
    if local.is_empty() {
        return Err(ValidationError::InvalidEmail(
            "email local part is empty".into(),
        ));
    }
    if domain.is_empty() || !domain.contains('.') {
        return Err(ValidationError::InvalidEmail(
            "email domain is invalid".into(),
        ));
    }

    // --- Password validation ---
    if password.len() < min_len {
        return Err(ValidationError::WeakPassword(format!(
            "password must be at least {} characters",
            min_len,
        )));
    }
    if password.len() > PASSWORD_MAX_LEN {
        return Err(ValidationError::WeakPassword(
            "password exceeds maximum length".into(),
        ));
    }
    // Password must not equal the email local-part.
    if password == local {
        return Err(ValidationError::WeakPassword(
            "password must not match the email local part".into(),
        ));
    }
    // Password must not be in the top-100 blocklist.
    if blocklist::is_blocked(password) {
        return Err(ValidationError::WeakPassword(
            "password is too common".into(),
        ));
    }

    Ok(())
}

/// Read the configured minimum password length from the environment,
/// falling back to [`DEFAULT_PASSWORD_MIN_LEN`].
pub fn password_min_len_from_env() -> usize {
    std::env::var("SIGNUP_PASSWORD_MIN_LEN")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PASSWORD_MIN_LEN)
}
