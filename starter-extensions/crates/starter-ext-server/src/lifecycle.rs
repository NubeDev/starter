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

use axum::extract::{Multipart, Path as AxumPath, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use starter_ext_host::Loader;
use starter_ext_spi::{ExtensionId, LifecycleState};
use starter_spi::auth::Principal;

use crate::admin::{ExtensionAdmin, PendingInstall};
use crate::cleanup::CleanupItem;
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
    /// `Some(true)` on a successful install: the extension surfaces on
    /// next boot (the sealed registry forbids hot-mount). Omitted from the
    /// JSON otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_restart: Option<bool>,
}

/// JSON envelope returned by `DELETE /extensions/<id>?purge=true`. Carries
/// the items actually removed so the UI can show the operator exactly what
/// was reclaimed. Idempotent: an already-uninstalled id returns
/// `cleanup.succeeded` with whatever leftovers were found (possibly empty).
#[derive(Debug, Serialize)]
pub(crate) struct CleanupResponse {
    /// Extension id purged.
    pub id: String,
    /// Always `cleanup.succeeded`.
    pub code: &'static str,
    /// Resources actually removed.
    pub removed: Vec<CleanupItem>,
}

/// Query string for `DELETE /extensions/<id>`. `?purge=true` runs the full
/// data cleanup after uninstall; the default (`false`) keeps today's
/// behaviour (stop, remove bundle, flip the row to `Disabled`).
#[derive(Debug, Default, Deserialize)]
pub(crate) struct UninstallQuery {
    #[serde(default)]
    pub purge: bool,
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
                return (StatusCode::BAD_REQUEST, format!("multipart field: {e}")).into_response()
            }
        };
        let name = field.name().unwrap_or("").to_owned();
        if !name.is_empty() && name != "file" && tarball.is_some() {
            continue;
        }
        match field.bytes().await {
            Ok(b) => tarball = Some(b.to_vec()),
            Err(e) => {
                return (StatusCode::BAD_REQUEST, format!("read field bytes: {e}")).into_response()
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
        return (StatusCode::BAD_REQUEST, format!("tarball extract: {e}")).into_response();
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
                pending_restart: None,
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
    let source = scan_parent
        .path()
        .join(record.bundle_dir.file_name().unwrap_or_default());
    if let Err(e) = std::fs::rename(&source, &final_dir) {
        tracing::warn!(err = %e, from = %source.display(), to = %final_dir.display(),
            "promote staging to final failed");
        let _ = std::fs::remove_dir_all(&staging);
        let _ = std::fs::remove_dir_all(scan_parent.path());
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let _ = std::fs::remove_dir_all(&staging);
    let _ = std::fs::remove_dir_all(scan_parent.path());

    if let Err(e) = admin.store().set(&ext_id, EnablementState::Enabled).await {
        tracing::warn!(err = %e.0, id = %ext_id.as_str(),
            "persist enablement after install failed");
        // Roll back the on-disk install so the next boot doesn't
        // surface a half-installed extension.
        let _ = std::fs::remove_dir_all(&final_dir);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    // Track as pending-restart so the list projection can badge it; the
    // sealed registry won't surface it live until next boot. The validated
    // record carries the manifest we summarise here.
    admin.mark_pending_restart(
        ext_id.as_str(),
        PendingInstall {
            version: record.manifest.as_ref().map(|m| m.version.to_string()),
            display_name: record.manifest.as_ref().map(|m| m.display_name.clone()),
            runtime_kind: record.manifest.as_ref().map(|m| m.runtime.kind),
        },
    );

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
            pending_restart: Some(true),
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
    Query(q): Query<UninstallQuery>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let Some(extensions_dir) = admin.extensions_dir() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "uninstall endpoint not wired (extensions_dir unset)",
        )
            .into_response();
    };

    let bundle_dir = extensions_dir.join(sanitize_dirname(&id));
    let parsed_id = ExtensionId::new(&id).ok();

    // Stop a live supervisor first so the process doesn't hold open
    // file handles inside the directory we're about to remove.
    if let Some(ext_id) = &parsed_id {
        if let Some(handle) = admin.replace_supervisor(ext_id, None) {
            handle.shutdown().await;
        }
        // Non-purge: persist disabled regardless of on-disk outcome so a
        // future boot doesn't autostart an uninstalled-id ghost. Purge
        // skips this — its `EnablementRowProvider` deletes the row
        // outright, and re-writing `Disabled` here would just resurrect
        // the row for purge to delete again (so a second idempotent purge
        // would never report an empty `removed`).
        if !q.purge {
            if let Err(e) = admin.store().set(ext_id, EnablementState::Disabled).await {
                tracing::warn!(err = %e.0, id = %id, "persist disabled on uninstall failed");
            }
        }
    }

    let bundle_existed = bundle_dir.exists();

    // `?purge=false` (default) keeps today's behaviour: a missing bundle
    // is a `404 uninstall.not_found`. Purge is idempotent and never 404s
    // (it cleans up leftovers — e.g. a ghost enablement row — even for an
    // already-uninstalled id).
    if !q.purge && !bundle_existed {
        return (
            StatusCode::NOT_FOUND,
            Json(LifecycleResponse {
                id,
                code: "uninstall.not_found",
                pending_restart: None,
            }),
        )
            .into_response();
    }

    if bundle_existed {
        if let Err(e) = std::fs::remove_dir_all(&bundle_dir) {
            tracing::warn!(err = %e, dir = %bundle_dir.display(),
                "remove extension bundle failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    // The bundle is gone (or was already), so it no longer surfaces on
    // next boot — drop any pending-restart badge.
    admin.clear_pending_restart(&id);

    tracing::info!(
        target: "starter_ext_server::lifecycle",
        id = %id,
        dir = %bundle_dir.display(),
        purge = q.purge,
        "extension uninstalled",
    );

    if !q.purge {
        return (
            StatusCode::OK,
            Json(LifecycleResponse {
                id,
                code: "uninstall.succeeded",
                pending_restart: None,
            }),
        )
            .into_response();
    }

    // Purge: run every registered cleanup provider after uninstall. The
    // in-memory registry record (sealed at boot) still carries the manifest
    // even though the on-disk bundle is gone, so cache/warehouse providers
    // can resolve their targets.
    let caller = principal
        .map(|Extension(p)| p.subject)
        .unwrap_or_else(|| "anonymous".to_owned());
    let manifest = admin
        .registry()
        .get_by_id_str(&id)
        .and_then(|rec| rec.manifest.clone());

    let removed = match &parsed_id {
        Some(ext_id) => {
            admin
                .purge_cleanup(ext_id, manifest.as_ref(), &caller)
                .await
        }
        None => Vec::new(),
    };

    (
        StatusCode::OK,
        Json(CleanupResponse {
            id,
            code: "cleanup.succeeded",
            removed,
        }),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// GET /extensions/<id>/cleanup — dry-run manifest
// ---------------------------------------------------------------------------

/// JSON body of the dry-run endpoint: what `?purge=true` *would* remove,
/// plus a best-effort total byte size for the operator's confirmation.
#[derive(Debug, Serialize)]
pub(crate) struct CleanupPreview {
    /// Extension id.
    pub id: String,
    /// Every resource the registered providers would reclaim.
    pub items: Vec<CleanupItem>,
    /// Sum of the `bytes` fields that are known.
    pub total_bytes: u64,
}

/// `GET /extensions/<id>/cleanup` — run every provider's `discover` (only)
/// and return the manifest so the operator sees exactly what a purge would
/// drop before confirming. Unknown ids that were never loaded this boot are
/// a plain `404`; a known (even already-uninstalled) id returns its
/// leftovers.
pub(crate) async fn cleanup_preview(
    State(admin): State<ExtensionAdmin>,
    AxumPath(id): AxumPath<String>,
) -> axum::response::Response {
    let Ok(ext_id) = ExtensionId::new(&id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let manifest = admin
        .registry()
        .get_by_id_str(&id)
        .and_then(|rec| rec.manifest.clone());
    let items = admin.discover_cleanup(&ext_id, manifest.as_ref()).await;
    let total_bytes = items.iter().filter_map(|i| i.bytes).sum();
    (
        StatusCode::OK,
        Json(CleanupPreview {
            id,
            items,
            total_bytes,
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
        .ok_or_else(|| std::io::Error::other("bundle_root has no parent"))?;
    let scan_root = parent.join(format!(".scan-{nanos}"));
    std::fs::create_dir(&scan_root)?;
    let entry = scan_root.join(
        bundle_root
            .file_name()
            .ok_or_else(|| std::io::Error::other("bundle_root unnamed"))?,
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
