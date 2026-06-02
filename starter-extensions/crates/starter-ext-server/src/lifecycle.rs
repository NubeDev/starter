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
    /// Reports what happened to the bundle directory itself.
    pub bundle: BundleOutcome,
}

/// What the uninstall handler did with the bundle directory on disk.
///
/// Under the installed-only model every uninstall removes the bundle
/// dir; `will_delete` is therefore always `true`. The field is kept
/// for one release so frontends that still branch on it parse
/// successfully.
#[derive(Debug, Serialize)]
pub(crate) struct BundleOutcome {
    /// Where the bundle lived under `installs_dir`. Empty when the id
    /// was unknown to the registry.
    pub path: String,
    /// Always `true` under the installed-only model. Retained for
    /// one-release frontend compatibility.
    pub will_delete: bool,
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
    let Some(installs_dir) = admin.installs_dir() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "install endpoint not wired (installs_dir unset)",
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
    if let Err(e) = std::fs::create_dir_all(installs_dir) {
        tracing::warn!(err = %e, dir = %installs_dir.display(),
            "installs_dir create failed");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let staging = match make_staging_dir(installs_dir) {
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
    let final_dir = installs_dir.join(sanitize_dirname(ext_id.as_str()));

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

    // Run the consumer-supplied post-install hook (rubix: create the
    // bundle's warehouse tables now, so writes land on a real schema
    // before the restart makes the extension code live). Best-effort:
    // the bundle is already on disk and the boot-time DDL pass is the
    // backstop, so a hook failure logs but does not fail the install.
    if let (Some(hook), Some(manifest)) = (admin.post_install_hook(), record.manifest.as_ref()) {
        match hook.run(&ext_id, manifest).await {
            Ok(summary) => tracing::info!(
                target: "starter_ext_server::lifecycle",
                id = %ext_id.as_str(),
                summary = %summary,
                "post-install hook ran",
            ),
            Err(e) => tracing::warn!(
                target: "starter_ext_server::lifecycle",
                id = %ext_id.as_str(),
                err = %e,
                "post-install hook failed (non-fatal; boot DDL is the backstop)",
            ),
        }
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
    let parsed_id = ExtensionId::new(&id).ok();
    // Decide the bundle's fate before any side-effects: every known id
    // resolves to a bundle dir under installs_dir; unknown ids fall
    // back to the sanitised-id shape for idempotent re-purge. The
    // decision drives both the supervisor-shutdown path and the
    // response envelope.
    let plan = match plan_bundle_action(&admin, &id) {
        Ok(p) => p,
        Err(resp) => return resp,
    };

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

    // `?purge=false` (default) keeps today's behaviour: a missing
    // installed bundle is a `404 uninstall.not_found`. Purge is
    // idempotent (it cleans up leftovers even for already-uninstalled
    // ids).
    if !q.purge && plan.is_missing_installed() {
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

    if let Err(resp) = apply_bundle_removal(&plan) {
        return resp;
    }

    admin.clear_pending_restart(&id);

    tracing::info!(
        target: "starter_ext_server::lifecycle",
        id = %id,
        dir = %plan.bundle_path().display(),
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
            bundle: plan.outcome(),
        }),
    )
        .into_response()
}

/// What `uninstall` is going to do with the bundle directory.
///
/// Computed once at the top of the handler so the supervisor-shutdown
/// path, the 404 path, the remove-or-skip step, and the response
/// envelope all agree on what's happening.
enum BundlePlan {
    /// Installed bundle that currently exists on disk under the
    /// configured installs dir. The handler will `remove_dir_all` it.
    RemoveInstalled { path: PathBuf },
    /// No record found, but the installs_dir is wired — fall back to
    /// the legacy `<installs_dir>/<sanitised id>` shape so an
    /// already-uninstalled id can still report idempotently. Removal
    /// only fires when the path exists.
    LegacyInstalled { path: PathBuf, exists: bool },
}

impl BundlePlan {
    fn bundle_path(&self) -> &Path {
        match self {
            Self::RemoveInstalled { path } | Self::LegacyInstalled { path, .. } => path,
        }
    }

    fn is_missing_installed(&self) -> bool {
        matches!(self, Self::LegacyInstalled { exists: false, .. })
    }

    fn outcome(&self) -> BundleOutcome {
        BundleOutcome {
            path: self.bundle_path().display().to_string(),
            // Installed-only model: every uninstall removes the bundle
            // dir. Field retained for one release so frontends with the
            // dev-badge branch keep parsing successfully.
            will_delete: true,
        }
    }
}

// `axum::response::Response` weighs ~128 bytes which trips clippy's
// `result_large_err` lint. The root starter workspace allows this
// lint workspace-wide; starter-extensions does not. Boxing here would
// force every call-site to unbox before `.into_response()`, which is
// the very next thing we do — net loss in readability.
#[allow(clippy::result_large_err)]
fn plan_bundle_action(
    admin: &ExtensionAdmin,
    id: &str,
) -> Result<BundlePlan, axum::response::Response> {
    // Records carry their bundle_dir from the loader; under the
    // installed-only model that path always lives under installs_dir.
    if let Some(rec) = admin.registry().get_by_id_str(id) {
        return Ok(BundlePlan::RemoveInstalled {
            path: rec.bundle_dir.clone(),
        });
    }
    // No record — id may have already been uninstalled this run, or
    // never existed. Need the installs_dir to compute a fallback path
    // for idempotent re-purge; without it the handler 503s like before.
    let Some(installs_dir) = admin.installs_dir() else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "uninstall endpoint not wired (installs_dir unset)",
        )
            .into_response());
    };
    let path = installs_dir.join(sanitize_dirname(id));
    let exists = path.exists();
    Ok(BundlePlan::LegacyInstalled { path, exists })
}

#[allow(clippy::result_large_err)]
fn apply_bundle_removal(plan: &BundlePlan) -> Result<(), axum::response::Response> {
    let (path, must_exist) = match plan {
        BundlePlan::RemoveInstalled { path } => (path, true),
        BundlePlan::LegacyInstalled { path, exists } => (path, *exists),
    };
    if !must_exist {
        return Ok(());
    }
    if let Err(e) = std::fs::remove_dir_all(path) {
        tracing::warn!(err = %e, dir = %path.display(), "remove extension bundle failed");
        return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }
    Ok(())
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
    /// What the bundle directory itself would do. Always
    /// `will_delete = true` under the installed-only model; retained
    /// for one-release frontend compatibility.
    pub bundle: BundleOutcome,
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
    let bundle = match plan_bundle_action(&admin, &id) {
        Ok(plan) => plan.outcome(),
        // installs_dir unset on a TestApp — surface an empty bundle
        // outcome rather than 503'ing the dry-run, which the
        // toggle-only surface should still be able to render.
        Err(_) => BundleOutcome {
            path: String::new(),
            will_delete: false,
        },
    };
    (
        StatusCode::OK,
        Json(CleanupPreview {
            id,
            items,
            total_bytes,
            bundle,
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
                // We disabled preserve_permissions to avoid trusting the
                // tar for setuid/setgid/world-writable bits, but that also
                // drops +x — which kills child-process bundles whose
                // runtime.bin is an executable. Re-apply the source mode
                // masked to user/group/other rwx only.
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(mode) = entry.header().mode() {
                        let safe = mode & 0o777;
                        let _ = std::fs::set_permissions(
                            &target,
                            std::fs::Permissions::from_mode(safe),
                        );
                    }
                }
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
