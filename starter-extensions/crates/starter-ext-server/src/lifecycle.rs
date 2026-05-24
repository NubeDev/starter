//! `POST /extensions/install` + `DELETE /extensions/<id>` — Phase D.1.
//!
//! Install accepts a gzipped tar bundle via multipart upload (field
//! name `file`). The handler extracts the bundle into a sibling staging
//! directory under the configured `extensions_dir`, validates the
//! resulting `block.yaml` via the upstream [`Loader`], then atomically
//! renames the staging dir to `<extensions_dir>/<id>`. Persistence
//! records the id as [`EnablementState::Enabled`]. The new extension
//! becomes live at the next boot — hot-mount-after-seal is out of
//! scope for v0.1 (the sealed [`ExtensionRegistry`] forbids
//! post-commit mutation).
//!
//! Install also accepts `application/json` bodies carrying a registry
//! URL. That path is deliberately stubbed with HTTP 501 until the
//! registry-pull pipeline lands; the goal is to reserve the shape of
//! the endpoint so clients don't have to migrate later.
//!
//! Uninstall stops any live supervisor, removes the on-disk bundle
//! directory, and writes [`EnablementState::Disabled`]. Missing bundles
//! surface as HTTP 404 with the `uninstall.not_found` code.
//!
//! Both handlers return small JSON envelopes carrying a stable `code`
//! string the consumer maps to a localised message via its own
//! `MessageKey` catalog (rubix uses the `rubix.extension.*` namespace).

use std::io::Read;
use std::path::{Path, PathBuf};

use axum::extract::{Multipart, Path as AxumPath, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use flate2::read::GzDecoder;
use serde::Serialize;
use starter_ext_host::Loader;
use starter_ext_spi::{ExtensionId, LifecycleState};

use crate::admin::ExtensionAdmin;
use crate::store::EnablementState;

/// JSON envelope returned by both install and uninstall handlers. The
/// `code` field is the stable upstream identifier; consumers map it to
/// their own MessageKey namespace.
#[derive(Debug, Serialize)]
pub(crate) struct LifecycleResponse {
    /// Extension id involved in the operation. Empty when install
    /// fails before the manifest could be parsed.
    pub id: String,
    /// Stable, namespaced status code. One of:
    /// `install.succeeded`, `install.invalid_manifest`,
    /// `uninstall.succeeded`, `uninstall.not_found`.
    pub code: &'static str,
}

// ---------------------------------------------------------------------------
// POST /extensions/install
// ---------------------------------------------------------------------------

/// Multipart-or-JSON install handler. The function is intentionally
/// long: every error path needs to clean its own staging directory.
pub(crate) async fn install(
    State(admin): State<ExtensionAdmin>,
    headers: HeaderMap,
    body: axum::body::Body,
) -> axum::response::Response {
    let Some(extensions_dir) = admin.extensions_dir() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "install endpoint not wired (extensions_dir unset)",
        )
            .into_response();
    };

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // JSON registry-URL path is reserved for a later stage. The
    // response body names the deferral so clients see a clear
    // contract rather than a 404.
    if content_type.starts_with("application/json") {
        return (
            StatusCode::NOT_IMPLEMENTED,
            "registry-URL install is deferred; upload the tarball via multipart instead",
        )
            .into_response();
    }

    // Reconstitute a Multipart from the incoming request. We rebuild a
    // throwaway `axum::extract::Request` so `Multipart::from_request`
    // can parse the boundary headers + body together.
    let mut req = axum::http::Request::new(body);
    *req.headers_mut() = headers;
    let mut multipart = match Multipart::from_request(req, &()).await {
        Ok(m) => m,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("multipart parse: {e}")).into_response()
        }
    };

    // We accept the first field whose name is `file` (or, as a
    // convenience, any field if the client only sent one). The body
    // bytes are buffered fully — extension bundles are small (handful
    // of MiB at most), and streaming straight into the tar reader
    // would make the size-cap awkward to enforce later.
    let mut tarball: Option<Vec<u8>> = None;
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => {
                return (StatusCode::BAD_REQUEST, format!("multipart field: {e}"))
                    .into_response()
            }
        };
        let name = field.name().unwrap_or("").to_owned();
        if !name.is_empty() && name != "file" && tarball.is_some() {
            continue;
        }
        match field.bytes().await {
            Ok(b) => tarball = Some(b.to_vec()),
            Err(e) => {
                return (StatusCode::BAD_REQUEST, format!("read field bytes: {e}"))
                    .into_response()
            }
        }
        if name == "file" {
            break;
        }
    }
    let Some(tarball) = tarball else {
        return (StatusCode::BAD_REQUEST, "missing tarball field").into_response();
    };

    // Stage into a temp directory adjacent to extensions_dir so the
    // final rename stays on the same filesystem (atomic on POSIX).
    if let Err(e) = std::fs::create_dir_all(extensions_dir) {
        tracing::warn!(err = %e, dir = %extensions_dir.display(),
            "extensions_dir create failed");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let staging = match make_staging_dir(extensions_dir) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(err = %e, "staging dir create failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Err(e) = extract_tarball(&tarball, &staging) {
        let _ = std::fs::remove_dir_all(&staging);
        return (StatusCode::BAD_REQUEST, format!("tarball extract: {e}"))
            .into_response();
    }

    // Validate via the upstream loader. We point scan() at the staging
    // *parent* so the loader walks one entry — our staging dir — and
    // returns one record. If the bundle was wrapped in a top-level
    // directory inside the tar we promote that directory's contents.
    let bundle_root = promote_single_subdir(&staging).unwrap_or(staging.clone());
    let scan_parent = match make_solo_scan_root(&bundle_root) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(err = %e, "solo scan root setup failed");
            let _ = std::fs::remove_dir_all(&staging);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let records = Loader::scan(scan_parent.path()).validate_all();
    let validated = records
        .into_iter()
        .find(|r| matches!(r.state, LifecycleState::Validated) && r.id.is_some());
    let Some(record) = validated else {
        let _ = std::fs::remove_dir_all(&staging);
        let _ = std::fs::remove_dir_all(scan_parent.path());
        return (
            StatusCode::BAD_REQUEST,
            Json(LifecycleResponse {
                id: String::new(),
                code: "install.invalid_manifest",
            }),
        )
            .into_response();
    };
    let ext_id: ExtensionId = record.id.clone().expect("validated record has id");
    let final_dir = extensions_dir.join(sanitize_dirname(ext_id.as_str()));

    // Best-effort: if a previous install lives there, blow it away.
    if final_dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&final_dir) {
            tracing::warn!(err = %e, dir = %final_dir.display(),
                "removing previous install dir failed");
            let _ = std::fs::remove_dir_all(&staging);
            let _ = std::fs::remove_dir_all(scan_parent.path());
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    // The validated record's bundle_dir points inside scan_parent;
    // rename it (not staging) into the final slot.
    let source = scan_parent.path().join(
        record.bundle_dir.file_name().unwrap_or_default(),
    );
    if let Err(e) = std::fs::rename(&source, &final_dir) {
        tracing::warn!(err = %e, from = %source.display(), to = %final_dir.display(),
            "promote staging to final failed");
        let _ = std::fs::remove_dir_all(&staging);
        let _ = std::fs::remove_dir_all(scan_parent.path());
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let _ = std::fs::remove_dir_all(&staging);
    let _ = std::fs::remove_dir_all(scan_parent.path());

    if let Err(e) = admin
        .store()
        .set(&ext_id, EnablementState::Enabled)
        .await
    {
        tracing::warn!(err = %e.0, id = %ext_id.as_str(),
            "persist enablement after install failed");
        // Roll back the on-disk install so the next boot doesn't
        // surface a half-installed extension.
        let _ = std::fs::remove_dir_all(&final_dir);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    tracing::info!(
        target: "starter_ext_server::lifecycle",
        id = %ext_id.as_str(),
        dir = %final_dir.display(),
        "extension installed; will surface on next boot",
    );
    (
        StatusCode::OK,
        Json(LifecycleResponse {
            id: ext_id.as_str().to_owned(),
            code: "install.succeeded",
        }),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// DELETE /extensions/<id>
// ---------------------------------------------------------------------------

pub(crate) async fn uninstall(
    State(admin): State<ExtensionAdmin>,
    AxumPath(id): AxumPath<String>,
) -> axum::response::Response {
    let Some(extensions_dir) = admin.extensions_dir() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "uninstall endpoint not wired (extensions_dir unset)",
        )
            .into_response();
    };

    // Stop a live supervisor first so the process doesn't hold open
    // file handles inside the directory we're about to remove.
    if let Ok(ext_id) = ExtensionId::new(&id) {
        if let Some(handle) = admin.replace_supervisor(&ext_id, None) {
            handle.shutdown().await;
        }
        // Persist disabled regardless of on-disk outcome so a future
        // boot doesn't autostart a uninstalled-id ghost.
        if let Err(e) = admin.store().set(&ext_id, EnablementState::Disabled).await {
            tracing::warn!(err = %e.0, id = %id, "persist disabled on uninstall failed");
        }
    }

    let bundle_dir = extensions_dir.join(sanitize_dirname(&id));
    if !bundle_dir.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(LifecycleResponse {
                id,
                code: "uninstall.not_found",
            }),
        )
            .into_response();
    }
    if let Err(e) = std::fs::remove_dir_all(&bundle_dir) {
        tracing::warn!(err = %e, dir = %bundle_dir.display(),
            "remove extension bundle failed");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    tracing::info!(
        target: "starter_ext_server::lifecycle",
        id = %id,
        dir = %bundle_dir.display(),
        "extension uninstalled",
    );
    (
        StatusCode::OK,
        Json(LifecycleResponse {
            id,
            code: "uninstall.succeeded",
        }),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a uniquely-named staging directory directly under `root`.
fn make_staging_dir(root: &Path) -> std::io::Result<PathBuf> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = root.join(format!(".staging-{nanos}"));
    std::fs::create_dir(&path)?;
    Ok(path)
}

/// Build a sibling directory that contains a single child — the
/// `bundle_root` — so `Loader::scan` walks exactly one entry.
struct SoloScanRoot {
    path: PathBuf,
}
impl SoloScanRoot {
    fn path(&self) -> &Path {
        &self.path
    }
}

fn make_solo_scan_root(bundle_root: &Path) -> std::io::Result<SoloScanRoot> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let parent = bundle_root
        .parent()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "bundle_root has no parent"))?;
    let scan_root = parent.join(format!(".scan-{nanos}"));
    std::fs::create_dir(&scan_root)?;
    let entry = scan_root.join(
        bundle_root
            .file_name()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "bundle_root unnamed"))?,
    );
    std::fs::rename(bundle_root, &entry)?;
    Ok(SoloScanRoot { path: scan_root })
}

/// Extract a gzip+tar payload into `dest`, rejecting any entry whose
/// path escapes `dest` via `..` or an absolute path. Skips symlinks.
fn extract_tarball(bytes: &[u8], dest: &Path) -> std::io::Result<()> {
    let gz = GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(gz);
    archive.set_overwrite(false);
    archive.set_preserve_permissions(false);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        if path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unsafe tar entry path: {}", path.display()),
            ));
        }
        let target = dest.join(&path);
        match entry.header().entry_type() {
            tar::EntryType::Directory => {
                std::fs::create_dir_all(&target)?;
            }
            tar::EntryType::Regular | tar::EntryType::Continuous => {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                std::fs::write(&target, &buf)?;
            }
            // Skip symlinks, hardlinks, devices, FIFOs — none belong
            // in an extension bundle.
            _ => continue,
        }
    }
    Ok(())
}

/// If `dir` contains exactly one child which is itself a directory and
/// no `block.yaml`, treat that child as the real bundle root. Bundles
/// authored with `tar czf ext.tgz com.foo.bar/` produce this shape.
fn promote_single_subdir(dir: &Path) -> Option<PathBuf> {
    if dir.join("block.yaml").exists() {
        return None;
    }
    let mut children = Vec::new();
    for entry in std::fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        children.push(entry.path());
    }
    if children.len() == 1 && children[0].is_dir() {
        return Some(children.into_iter().next().unwrap());
    }
    None
}

/// Replace path separators with underscores so an id like
/// `com.rubix.example` lands as that literal dirname on disk and so an
/// adversarial id can't escape `extensions_dir`.
fn sanitize_dirname(id: &str) -> String {
    id.replace(['/', '\\', '\0'], "_")
}

// Pulled into scope only where it's used to keep the public surface
// of the module tight.
use axum::extract::FromRequest;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_path_separators() {
        assert_eq!(sanitize_dirname("com.foo.bar"), "com.foo.bar");
        assert_eq!(sanitize_dirname("../etc/passwd"), ".._etc_passwd");
    }
}
