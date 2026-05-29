//! OS-conventional data-root resolution.
//!
//! Precedence (matches the spec):
//!
//! 1. Explicit override (`--data-root` flag / programmatic).
//! 2. `$<APP>_DATA_ROOT` env var (uppercased app name, `-` → `_`).
//! 3. OS convention:
//!    - Linux: `$XDG_DATA_HOME/<app>/` (falling back to `~/.local/share/<app>/`).
//!    - macOS: `~/Library/Application Support/<app>/`.
//!    - Windows: `%LOCALAPPDATA%/<app>/`.
//!
//! `dirs::data_dir()` already implements the per-OS leg of step 3, so
//! the only thing this module owns is env-var lookup, the `app` leaf
//! join, and validation of the app name.

use std::path::PathBuf;

use crate::error::PathsError;

/// Resolve the absolute data root for `app`, honouring (in order)
/// the caller-supplied override, the `$<APP>_DATA_ROOT` env var, and
/// the OS convention. Does **not** touch the filesystem; use
/// [`crate::Paths::ensure`] to create the directory tree.
pub(crate) fn resolve_root(
    app: &str,
    override_root: Option<PathBuf>,
) -> Result<PathBuf, PathsError> {
    validate_app_name(app)?;

    if let Some(p) = override_root {
        return Ok(p);
    }

    if let Some(p) = env_override(app) {
        return Ok(p);
    }

    dirs::data_dir()
        .map(|d| d.join(app))
        .ok_or_else(|| PathsError::NoDataDir { app: app.into() })
}

fn validate_app_name(app: &str) -> Result<(), PathsError> {
    if app.is_empty() || app.contains('/') || app.contains('\\') {
        return Err(PathsError::InvalidAppName { app: app.into() });
    }
    Ok(())
}

/// `$<APP>_DATA_ROOT`, with `-` in the app name mapped to `_` and the
/// whole thing uppercased. `rubix` → `RUBIX_DATA_ROOT`,
/// `my-app` → `MY_APP_DATA_ROOT`.
fn env_override(app: &str) -> Option<PathBuf> {
    let var = format!("{}_DATA_ROOT", app.replace('-', "_").to_uppercase());
    std::env::var_os(&var).map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_wins() {
        let p = resolve_root("rubix", Some(PathBuf::from("/tmp/foo"))).unwrap();
        assert_eq!(p, PathBuf::from("/tmp/foo"));
    }

    #[test]
    fn empty_app_rejected() {
        let err = resolve_root("", None).unwrap_err();
        assert!(matches!(err, PathsError::InvalidAppName { .. }));
    }

    #[test]
    fn app_with_slash_rejected() {
        let err = resolve_root("foo/bar", Some(PathBuf::from("/tmp"))).unwrap_err();
        assert!(matches!(err, PathsError::InvalidAppName { .. }));
    }

    #[test]
    fn env_var_overrides_os_default() {
        // Pick a name unlikely to collide with anything else the test
        // process touches.
        let app = "starter-paths-test-env";
        let var = "STARTER_PATHS_TEST_ENV_DATA_ROOT";
        // SAFETY: serial; no other test in this module reads this var.
        std::env::set_var(var, "/tmp/from-env");
        let p = resolve_root(app, None).unwrap();
        std::env::remove_var(var);
        assert_eq!(p, PathBuf::from("/tmp/from-env"));
    }

    #[test]
    fn dash_in_app_name_becomes_underscore_in_env() {
        let app = "my-test-app";
        let var = "MY_TEST_APP_DATA_ROOT";
        std::env::set_var(var, "/tmp/dashy");
        let p = resolve_root(app, None).unwrap();
        std::env::remove_var(var);
        assert_eq!(p, PathBuf::from("/tmp/dashy"));
    }

    #[test]
    fn os_default_appends_app_leaf() {
        // We can't assert the exact prefix portably, but we can assert
        // the leaf segment was joined on.
        let p = resolve_root("rubix-leaf-check", None).unwrap();
        assert_eq!(p.file_name().and_then(|s| s.to_str()), Some("rubix-leaf-check"));
    }
}
