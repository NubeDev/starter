//! Read a password from stdin without echoing.

/// Prompt the user for a password.
///
/// Stubbed for v0.1; will use `rpassword` or equivalent so the
/// password never reaches scrollback or process listings.
pub fn password(_prompt: &str) -> std::io::Result<String> {
    todo!("password prompt lands with starter-auth admin create")
}
