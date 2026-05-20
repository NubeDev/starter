//! `SignupMode` — env-driven gate for the signup route.

use crate::role::Role;

/// Controls whether `POST /auth/signup` is mounted and how it
/// behaves. Parsed from the `SIGNUP_MODE` environment variable.
///
/// - `Disabled` (default): the route is not mounted; returns 404.
/// - `Open { default_role }`: anyone with a valid email + password
///   can self-register.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SignupMode {
    /// Signup disabled. Route is not mounted.
    #[default]
    Disabled,
    /// Open signup. New accounts receive `default_role`.
    Open {
        /// Role assigned to self-registered users.
        default_role: Role,
    },
}

/// Parse `SIGNUP_MODE` from environment. Returns [`SignupMode::Disabled`]
/// when the variable is absent or set to `"disabled"`.
///
/// Accepted values:
/// - `disabled` → [`SignupMode::Disabled`]
/// - `open` → [`SignupMode::Open`] with role from `SIGNUP_DEFAULT_ROLE`
///   (default: `reader`)
pub fn parse_signup_mode_env() -> SignupMode {
    let mode = std::env::var("SIGNUP_MODE").unwrap_or_default();
    match mode.to_lowercase().as_str() {
        "open" => {
            let role_str = std::env::var("SIGNUP_DEFAULT_ROLE").unwrap_or_else(|_| "reader".into());
            let default_role = match role_str.to_lowercase().as_str() {
                "admin" => Role::Admin,
                "writer" => Role::Writer,
                _ => Role::Reader,
            };
            SignupMode::Open { default_role }
        }
        _ => SignupMode::Disabled,
    }
}
