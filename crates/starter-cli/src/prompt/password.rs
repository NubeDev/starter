//! Read a password from stdin without echoing.

/// Prompt the user for a password. The typed characters are not
/// echoed and never reach scrollback or process listings.
///
/// On a non-tty stdin (pipes, redirected files), `rpassword` falls
/// back to reading the next line as the password — useful for
/// scripted callers but callers shouldn't rely on that behaviour
/// for anything other than tests.
pub fn password(prompt: &str) -> std::io::Result<String> {
    rpassword::prompt_password(prompt)
}
