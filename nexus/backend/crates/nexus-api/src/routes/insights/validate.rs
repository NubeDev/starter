//! Save-time script validation: a thin wrapper over the insight engine's
//! compile check so a stored insight is never persisted un-runnable.

/// Return `Err(message)` if `script` does not compile under the insight sandbox.
pub fn compiles(script: &str) -> Result<(), String> {
    nexus_insights::compile_check(script).map_err(|e| e.to_string())
}
