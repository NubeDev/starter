//! [`BundleOrigin`] — provenance of an extension bundle on disk.
//!
//! Per the installed-only model
//! (`rubix/docs/scope/extensions/installed-only-model.md`), bundles
//! reach the runtime only by being unpacked into the writable installs
//! dir. The type carries the installs root so uninstall can sanity-
//! check that a path it's about to `remove_dir_all` really lives under
//! the configured installs tree.

use std::path::PathBuf;

/// Where a bundle came from. Today there is exactly one origin; the
/// type is retained as a wrapper so call sites stay forward-compatible
/// if signed-registry installs or quarantine areas land later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleOrigin {
    /// Unpacked from an uploaded tarball into the writable installs dir.
    /// Uninstall removes the bundle directory.
    Installed {
        /// The installs root the loader walked. Records the *parent* of
        /// the bundle dir so the uninstall handler can sanity-check
        /// that a path it's about to delete really lives under the
        /// installs tree.
        installs_dir: PathBuf,
    },
}

impl Default for BundleOrigin {
    /// `Installed { installs_dir: <empty> }`. Used by test fixtures and
    /// other call sites that construct an
    /// [`ExtensionRecord`](crate::ExtensionRecord) directly. Real boot
    /// paths replace this with the actual installs dir via the loader.
    fn default() -> Self {
        Self::Installed {
            installs_dir: PathBuf::new(),
        }
    }
}

impl BundleOrigin {
    /// `true` when uninstall is allowed to `remove_dir_all` the bundle
    /// directory. Always true under the installed-only model — kept as
    /// a method so call sites read intent and stay stable if more
    /// origin variants are added later.
    pub fn is_installed(&self) -> bool {
        matches!(self, BundleOrigin::Installed { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_default_has_empty_installs_dir() {
        let o = BundleOrigin::default();
        assert!(o.is_installed());
        if let BundleOrigin::Installed { installs_dir } = o {
            assert_eq!(installs_dir, PathBuf::new());
        }
    }
}
