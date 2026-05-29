//! The [`Paths`] handle — a single resolved data root plus subdir
//! accessors.

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::PathsError;
use crate::resolve::resolve_root;

/// Resolved data-root handle. Cheap to clone (one `PathBuf` inside).
///
/// Consumers call [`Self::resolve`] once during boot, then ask for
/// the subdirs they need (`config_dir`, `installs_dir`, `subdir(…)`).
/// Nothing here touches the filesystem until [`Self::ensure`] is
/// called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    /// Resolve the data root for `app`. See [`crate::resolve`] for
    /// the full precedence (override > env > OS convention).
    pub fn resolve(app: &str, override_root: Option<PathBuf>) -> Result<Self, PathsError> {
        Ok(Self {
            root: resolve_root(app, override_root)?,
        })
    }

    /// Construct directly from a known root. Primarily for tests and
    /// for callers that have already resolved the path themselves.
    pub fn from_root(root: PathBuf) -> Self {
        Self { root }
    }

    /// The absolute data root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `<root>/config/` — writable config snapshots and runtime config
    /// state. Distinct from `$XDG_CONFIG_HOME` (which holds the static
    /// `agent.toml` consumed by `starter-config`).
    pub fn config_dir(&self) -> PathBuf {
        self.root.join("config")
    }

    /// `<root>/extensions/installed/` — bundles unpacked from uploaded
    /// tarballs. The uninstall path is allowed to `remove_dir_all` here
    /// because nothing in this tree is user-authored.
    pub fn installs_dir(&self) -> PathBuf {
        self.root.join("extensions").join("installed")
    }

    /// `<root>/logs/` — rotating log files (future).
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// Caller-named subdir under the root. Accepts nested segments
    /// (e.g. `"rubix/warehouse"`) but rejects absolute paths and any
    /// path containing `..`.
    pub fn subdir(&self, name: &str) -> Result<PathBuf, PathsError> {
        validate_subdir(name)?;
        Ok(self.root.join(name))
    }

    /// Create the root and the standard subdirs (`config/`,
    /// `extensions/installed/`, `logs/`) if missing. Idempotent.
    pub fn ensure(&self) -> Result<(), PathsError> {
        create(&self.root)?;
        create(&self.config_dir())?;
        create(&self.installs_dir())?;
        create(&self.logs_dir())?;
        Ok(())
    }
}

fn create(p: &Path) -> Result<(), PathsError> {
    fs::create_dir_all(p).map_err(|source| PathsError::Io {
        path: p.to_path_buf(),
        source,
    })
}

fn validate_subdir(name: &str) -> Result<(), PathsError> {
    let p = Path::new(name);
    if p.is_absolute() {
        return Err(PathsError::InvalidSubdir { name: name.into() });
    }
    for comp in p.components() {
        match comp {
            Component::Normal(_) => {}
            // Anything else (RootDir, Prefix, ParentDir, CurDir) is
            // either a separator we already rejected or a way to escape
            // the data root.
            _ => return Err(PathsError::InvalidSubdir { name: name.into() }),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn standard_subdirs_compose_off_root() {
        let p = Paths::from_root(PathBuf::from("/data/rubix"));
        assert_eq!(p.root(), Path::new("/data/rubix"));
        assert_eq!(p.config_dir(), PathBuf::from("/data/rubix/config"));
        assert_eq!(
            p.installs_dir(),
            PathBuf::from("/data/rubix/extensions/installed")
        );
        assert_eq!(p.logs_dir(), PathBuf::from("/data/rubix/logs"));
    }

    #[test]
    fn subdir_accepts_nested_relative_paths() {
        let p = Paths::from_root(PathBuf::from("/data/rubix"));
        assert_eq!(
            p.subdir("rubix/warehouse").unwrap(),
            PathBuf::from("/data/rubix/rubix/warehouse")
        );
    }

    #[test]
    fn subdir_rejects_absolute() {
        let p = Paths::from_root(PathBuf::from("/data/rubix"));
        let err = p.subdir("/etc/passwd").unwrap_err();
        assert!(matches!(err, PathsError::InvalidSubdir { .. }));
    }

    #[test]
    fn subdir_rejects_parent_traversal() {
        let p = Paths::from_root(PathBuf::from("/data/rubix"));
        let err = p.subdir("../escape").unwrap_err();
        assert!(matches!(err, PathsError::InvalidSubdir { .. }));
        let err = p.subdir("rubix/../escape").unwrap_err();
        assert!(matches!(err, PathsError::InvalidSubdir { .. }));
    }

    #[test]
    fn ensure_creates_root_and_standard_subdirs() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("rubix");
        let p = Paths::from_root(root.clone());
        p.ensure().unwrap();
        assert!(root.is_dir());
        assert!(root.join("config").is_dir());
        assert!(root.join("extensions/installed").is_dir());
        assert!(root.join("logs").is_dir());
    }

    #[test]
    fn ensure_is_idempotent() {
        let tmp = tempdir().unwrap();
        let p = Paths::from_root(tmp.path().join("rubix"));
        p.ensure().unwrap();
        p.ensure().unwrap();
    }
}
