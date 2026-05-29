//! [`BundleOrigin`] — provenance of an extension bundle on disk.
//!
//! The data-root-and-safe-uninstall scope (rubix/docs/scope/extensions/)
//! splits one path into two:
//!
//! - **Dev source trees** are scanned in-place. Their contents are
//!   author-edited and tracked in git; the runtime must never
//!   `remove_dir_all` them.
//! - **Installed bundles** are unpacked from uploaded tarballs into
//!   the writable installs dir. The runtime owns these — uninstall
//!   removes them.
//!
//! Records carry their origin from the moment the loader records them
//! so the uninstall handler can refuse to delete a dev tree without
//! re-deriving the distinction from path heuristics.

use std::path::PathBuf;

/// Where a bundle came from — drives whether uninstall may delete its
/// directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleOrigin {
    /// Loaded in-place from a developer source tree. Uninstall does
    /// **not** delete the bundle directory.
    Dev {
        /// The dev source root the loader walked to find this bundle.
        /// (Not the bundle dir itself — that lives on the record.)
        source_dir: PathBuf,
    },
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
    /// `Installed { installs_dir: <empty> }`. Preserves pre-split
    /// behaviour for tests and other call-sites that construct an
    /// [`ExtensionRecord`](crate::ExtensionRecord) directly: every
    /// record produced without an explicit origin is treated as
    /// installable-and-deletable (the original behaviour before the
    /// dev/installed split). Real boot paths replace this with the
    /// actual installs dir via the loader.
    fn default() -> Self {
        Self::Installed {
            installs_dir: PathBuf::new(),
        }
    }
}

impl BundleOrigin {
    /// `true` when this bundle's source files are owned by the user,
    /// not by the runtime.
    pub fn is_dev(&self) -> bool {
        matches!(self, BundleOrigin::Dev { .. })
    }

    /// `true` when uninstall is allowed to `remove_dir_all` the bundle
    /// directory.
    pub fn is_installed(&self) -> bool {
        matches!(self, BundleOrigin::Installed { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_and_installed_are_distinguishable() {
        let dev = BundleOrigin::Dev {
            source_dir: PathBuf::from("/repo/extensions"),
        };
        let installed = BundleOrigin::Installed {
            installs_dir: PathBuf::from("/var/lib/rubix/extensions/installed"),
        };
        assert!(dev.is_dev());
        assert!(!dev.is_installed());
        assert!(installed.is_installed());
        assert!(!installed.is_dev());
    }
}
