//! Errors surfaced from data-root resolution and directory creation.

use std::path::PathBuf;

/// Failures that can arise while resolving or materialising a [`Paths`](crate::Paths) root.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PathsError {
    /// No OS-conventional data dir could be resolved (no `$XDG_DATA_HOME`,
    /// no `~`, no `%LOCALAPPDATA%`) and no override was supplied.
    #[error("no OS data directory for app {app:?}: \
             $XDG_DATA_HOME / Application Support / %LOCALAPPDATA% unavailable")]
    NoDataDir {
        /// The app/binary name that was being resolved.
        app: String,
    },

    /// The app name was empty or contained a path separator. The leaf
    /// segment is appended verbatim to the OS data dir, so any separator
    /// (or empty string) would silently escape into a sibling directory.
    #[error("invalid app name {app:?}: must be non-empty and contain no path separators")]
    InvalidAppName {
        /// The rejected app name.
        app: String,
    },

    /// A [`Paths::subdir`](crate::Paths::subdir) call was given an
    /// absolute path or a path with `..` components. `subdir` is
    /// caller-named and may include nested segments (e.g. `"rubix/warehouse"`),
    /// but it must stay inside the data root.
    #[error("invalid subdir name {name:?}: must be relative, contain no `..`, \
             and not start with `/`")]
    InvalidSubdir {
        /// The rejected subdir.
        name: String,
    },

    /// Creating the root or one of the standard subdirs failed.
    #[error("create directory {path:?}: {source}")]
    Io {
        /// Path that failed to create.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}
