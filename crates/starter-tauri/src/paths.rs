//! On-disk layout for desktop shells: `~/.<app>/workspaces/<slug>/…`.
//!
//! Each desktop launch scopes to one workspace (a folder on disk). The
//! workspace gets a stable slug — `<leaf>-<hex8>` — so two folders that
//! share a last segment never collide, and the same folder accessed
//! through a symlink resolves to one slug.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum PathsError {
    #[error("could not determine home directory")]
    NoHomeDir,
}

/// Resolved on-disk layout for one workspace of one app.
///
/// `app_root` is `~/.<app>/` and is shared across workspaces — put
/// app-wide state (secrets, user config) there. `workspace_dir` is
/// `~/.<app>/workspaces/<slug>/` and is per-workspace.
#[derive(Debug, Clone)]
pub struct DataPaths {
    pub workspace_root: PathBuf,
    pub app_root: PathBuf,
    pub workspace_dir: PathBuf,
    pub slug: String,
}

impl DataPaths {
    /// Resolve paths for `app_name` (used as `~/.<app_name>/`) scoped to
    /// `workspace`. Symlinks in `workspace` are resolved when possible
    /// so the slug is canonical.
    pub fn resolve(app_name: &str, workspace: &Path) -> Result<Self, PathsError> {
        let home = directories::BaseDirs::new()
            .map(|d| d.home_dir().to_path_buf())
            .ok_or(PathsError::NoHomeDir)?;
        let app_root = home.join(format!(".{app_name}"));

        let canonical = workspace
            .canonicalize()
            .unwrap_or_else(|_| workspace.to_path_buf());
        let slug = workspace_slug(&canonical);
        let workspace_dir = app_root.join("workspaces").join(&slug);

        Ok(Self {
            workspace_root: canonical,
            app_root,
            workspace_dir,
            slug,
        })
    }

    /// Convenience: `<workspace_dir>/<name>`. Use for per-workspace
    /// subdirs (worktrees, sqlite files, attachments).
    pub fn workspace_sub(&self, name: &str) -> PathBuf {
        self.workspace_dir.join(name)
    }

    /// Convenience: `<app_root>/<name>`. Use for app-wide artefacts
    /// (`secrets.toml`, `config.toml`, …).
    pub fn app_sub(&self, name: &str) -> PathBuf {
        self.app_root.join(name)
    }

    /// `mkdir -p` for both the app root and this workspace dir.
    /// Best-effort — errors propagate so callers can decide what is
    /// fatal.
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.app_root)?;
        std::fs::create_dir_all(&self.workspace_dir)?;
        Ok(())
    }
}

/// Stable per-workspace directory name. Leaf prefix keeps the directory
/// human-recognisable; 8-hex hash of the canonical path disambiguates
/// two folders that share a name. Non-alphanumerics in the leaf become
/// `-`.
pub fn workspace_slug(canonical: &Path) -> String {
    let leaf = canonical
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "root".to_owned());
    let leaf_sanitised: String = leaf
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    canonical.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{leaf_sanitised}-{hash:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_deterministic_for_same_path() {
        let p = PathBuf::from("/home/user/code/foo");
        assert_eq!(workspace_slug(&p), workspace_slug(&p));
    }

    #[test]
    fn slug_disambiguates_same_leaf_in_different_parents() {
        let a = workspace_slug(&PathBuf::from("/home/user/code/foo"));
        let b = workspace_slug(&PathBuf::from("/home/user/work/foo"));
        assert!(a.starts_with("foo-"));
        assert!(b.starts_with("foo-"));
        assert_ne!(a, b);
    }

    #[test]
    fn slug_sanitises_non_alphanumeric_leaf_chars() {
        let s = workspace_slug(&PathBuf::from("/tmp/my project (v2)"));
        assert!(s.starts_with("my-project--v2--"), "got {s}");
    }
}
