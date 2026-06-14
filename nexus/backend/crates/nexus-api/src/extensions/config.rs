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
    ///
    /// `extensions_dir` stays the read-only in-repo **seed** pack (env
    /// `NEXUS_EXTENSIONS_DIR`, default `./extensions`). The *writable* dirs
    /// (`installs_dir`, `pidfile_dir`), when not pinned by their own env vars,
    /// resolve under an **external data root** via `starter-paths` — the same
    /// convention rubix uses (`Paths::resolve("rubix", …)`): override-arg >
    /// `$NEXUS_DATA_ROOT` > OS XDG. This keeps uploaded bundles + supervisor
    /// pidfiles out of the repo tree. If that resolution fails, we fall back to
    /// the historical in-repo `.installs`/`.pids` siblings so boot never aborts
    /// over a path-resolution error (logged by the helper).
    pub fn from_env() -> Self {
        let extensions_dir: PathBuf = std::env::var("NEXUS_EXTENSIONS_DIR")
            .unwrap_or_else(|_| "./extensions".into())
            .into();
        let (default_installs, default_pids) = Self::resolve_data_dirs(&extensions_dir);
        let installs_dir: PathBuf = std::env::var("NEXUS_EXTENSIONS_INSTALLS_DIR")
            .map(PathBuf::from)
            .unwrap_or(default_installs);
        let pidfile_dir: PathBuf = std::env::var("NEXUS_EXTENSIONS_PIDFILE_DIR")
            .map(PathBuf::from)
            .unwrap_or(default_pids);
        Self {
            // Absolutise every path. The process-flavour supervisor execs
            // `bundle_dir.join(runtime.bin)` *with* `current_dir(bundle_dir)`
            // set — so a **relative** bundle_dir (e.g. dev's `../extensions`)
            // would be resolved twice and the exec would fail with ENOENT. An
            // absolute `extensions_dir` makes `bundle_dir` (derived from it)
            // absolute, so the spawn is CWD-independent. `absolutize` keeps the
            // value verbatim if it's already absolute or can't be resolved.
            extensions_dir: absolutize(extensions_dir),
            installs_dir: absolutize(installs_dir),
            pidfile_dir: absolutize(pidfile_dir),
        }
    }

    /// Default `(installs_dir, pidfile_dir)` under the external data root
    /// resolved by `starter-paths` for app `"nexus"` (`$NEXUS_DATA_ROOT` +
    /// XDG). Installs land at `<root>/extensions/installed/` (the crate's
    /// canonical `installs_dir()`), pidfiles at `<root>/extensions/pids/` (a
    /// sibling, mirroring rubix's `supervisor-pids/`). On any resolution error,
    /// falls back to the in-repo `<extensions_dir>/.installs` + `.pids` so a
    /// missing/unwritable data root degrades gracefully instead of aborting boot.
    fn resolve_data_dirs(extensions_dir: &std::path::Path) -> (PathBuf, PathBuf) {
        match starter_paths::Paths::resolve("nexus", None) {
            Ok(paths) => match paths.subdir("extensions/pids") {
                Ok(pids) => (paths.installs_dir(), pids),
                Err(e) => {
                    tracing::warn!(
                        target: "nexus.extensions.config",
                        err = %e,
                        "resolve pids subdir failed; falling back to in-repo .installs/.pids"
                    );
                    (
                        extensions_dir.join(".installs"),
                        extensions_dir.join(".pids"),
                    )
                }
            },
            Err(e) => {
                tracing::warn!(
                    target: "nexus.extensions.config",
                    err = %e,
                    "resolve nexus data root failed; falling back to in-repo .installs/.pids"
                );
                (
                    extensions_dir.join(".installs"),
                    extensions_dir.join(".pids"),
                )
            }
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

/// Resolve `p` to an absolute path. Already-absolute paths are returned
/// unchanged. A relative path is joined onto the process CWD (it is *not*
/// canonicalised — the dir may not exist yet, e.g. the installs dir before
/// `ensure_writable_dirs`). On any error (no CWD) the input is returned
/// verbatim so behaviour never gets worse than before.
fn absolutize(p: PathBuf) -> PathBuf {
    if p.is_absolute() {
        return p;
    }
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(p),
        Err(_) => p,
    }
}
