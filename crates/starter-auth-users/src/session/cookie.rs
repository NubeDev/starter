//! Cookie naming. Centralised so every callsite agrees.

/// The cookie name carrying the opaque session id.
pub const SESSION_COOKIE: &str = "starter_session";

/// Convenience accessor — useful in tests where macros can't see
/// the const directly.
pub fn cookie_name() -> &'static str {
    SESSION_COOKIE
}
