//! Extension-runtime configuration: where bundles live, where uploaded
//! tarballs are unpacked, and where supervisor pidfiles are written.
//!
//! Mirrors the WS-10 `NEXUS_KINDS_DIR` pattern (and carries the same dev-CWD +
//! Docker-copy lesson): the read-only in-repo pack dir must resolve under
//! `cd backend && cargo run` *and* be `COPY`'d into the runtime image. WS-14 §9
//! Q3 settles on **both** a read-only in-repo pack dir and a writable installs
//! dir — uploaded tarballs land in the latter, the former ships with the deploy.

use std::path::PathBuf;

/// Resolved extension-runtime paths. Built once at boot from the environment.
#[derive(Debug, Clone)]
pub struct ExtensionsConfig {
    /// Directory scanned at boot for extension bundles. Read-only in-repo pack
    /// **plus** the installs dir below are both scanned (the loader walks one
    /// level), so a deployment ships built-in extensions here and accepts
    /// uploads into `installs_dir`. Env `NEXUS_EXTENSIONS_DIR`, default
    /// `./extensions`. A missing dir means no extensions are registered (the
    /// loader treats it as empty, like the kinds pack).
    pub extensions_dir: PathBuf,
    /// Writable root where `POST /extensions/install` unpacks uploaded
    /// tarballs and `DELETE …?purge=true` removes them. Env
    /// `NEXUS_EXTENSIONS_INSTALLS_DIR`, default `<extensions_dir>/.installs` so
    /// it sits beside the pack and is scanned by the same boot walk. When unset
    /// and the default is used, install/uninstall are still enabled.
    pub installs_dir: PathBuf,
    /// Directory holding one pidfile per supervised process-flavour extension.
    /// The boot reaper `killpg`s stale groups recorded here from a prior crash
    /// before any new supervisor spawns. Env `NEXUS_EXTENSIONS_PIDFILE_DIR`,
    /// default `<extensions_dir>/.pids`.
    pub pidfile_dir: PathBuf,
}

impl ExtensionsConfig {
    /// Resolve from the environment, applying defaults derived from
    /// `extensions_dir` so a single env var is enough for a dev setup.
    pub fn from_env() -> Self {
        let extensions_dir: PathBuf = std::env::var("NEXUS_EXTENSIONS_DIR")
            .unwrap_or_else(|_| "./extensions".into())
            .into();
        let installs_dir: PathBuf = std::env::var("NEXUS_EXTENSIONS_INSTALLS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| extensions_dir.join(".installs"));
        let pidfile_dir: PathBuf = std::env::var("NEXUS_EXTENSIONS_PIDFILE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| extensions_dir.join(".pids"));
        Self {
            extensions_dir,
            installs_dir,
            pidfile_dir,
        }
    }

    /// Create the writable dirs (`installs_dir`, `pidfile_dir`) if absent. The
    /// read-only `extensions_dir` is *not* created — a missing pack dir is a
    /// valid "no built-in extensions" state, and silently creating it would mask
    /// a deploy that forgot to COPY the bundles in. Best-effort: a failure here
    /// is logged by the caller and does not abort boot (install/reaper degrade
    /// gracefully).
    pub fn ensure_writable_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.installs_dir)?;
        std::fs::create_dir_all(&self.pidfile_dir)?;
        Ok(())
    }
}
