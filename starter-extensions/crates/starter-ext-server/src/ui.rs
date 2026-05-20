//! `GET /extensions/<id>/ui/*` — Module-Federation bundle serving.
//!
//! Each extension's `contributes.ui.entry` points at a
//! `remoteEntry.js` inside the bundle directory; the chunks live
//! alongside it. We serve the directory rooted at that file's parent
//! (so `contributes.ui.entry == "ui/remoteEntry.js"` means the URL
//! `/extensions/<id>/ui/remoteEntry.js` resolves to
//! `<bundle_dir>/ui/remoteEntry.js`).
//!
//! Bundle responses carry strong ETags (SHA-256 of the file bytes,
//! memoised by canonical path + mtime + size — see [`crate::etag`]).
//! `If-None-Match` short-circuits to `304 Not Modified` so an admin
//! UI's MF host re-validates instead of redownloading.
//!
//! Path safety: we resolve the requested suffix against the bundle's
//! UI dir and refuse any result whose canonical path escapes that
//! directory (`..`, symlink, etc.). The check is `canonicalize`-based;
//! a missing file canonicalizes the existing prefix and is reported as
//! `404`.

use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, HeaderMap, Response, StatusCode};
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::admin::ExtensionAdmin;

#[derive(Debug, Deserialize)]
pub(crate) struct UiParams {
    id: String,
    path: String,
}

pub(crate) async fn ui(
    State(admin): State<ExtensionAdmin>,
    AxumPath(UiParams { id, path: suffix }): AxumPath<UiParams>,
    headers: HeaderMap,
) -> axum::response::Response {
    let rec = match admin.registry().get_by_id_str(&id) {
        Some(r) => r,
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    let manifest = match rec.manifest.as_ref() {
        Some(m) => m,
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    let ui_block = match manifest.contributes.ui.as_ref() {
        Some(u) => u,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let ui_root = match resolve_ui_root(&rec.bundle_dir, &ui_block.entry) {
        Some(root) => root,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let target = match safe_join(&ui_root, &suffix) {
        Some(p) => p,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    let (etag, bytes) = match admin.etag_cache().etag_and_bytes(&target).await {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return StatusCode::NOT_FOUND.into_response()
        }
        Err(e) => {
            tracing::warn!(err = %e, path = %target.display(), "ui asset read failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // `If-None-Match` short-circuit. Match is byte-exact against the
    // quoted ETag.
    if let Some(inm) = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
    {
        if inm.split(',').any(|t| t.trim() == etag) {
            return Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .header(header::ETAG, &etag)
                .body(Body::empty())
                .expect("not modified response is well-formed");
        }
    }

    let mime = guess_mime(&target);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::ETAG, etag)
        .header(header::CACHE_CONTROL, "public, max-age=0, must-revalidate")
        .body(Body::from(bytes))
        .expect("ok response is well-formed")
}

/// Compute the directory the wildcard suffix is resolved against. For
/// `entry == "ui/remoteEntry.js"` and bundle `/x/y`, this is `/x/y/ui`.
fn resolve_ui_root(bundle_dir: &Path, entry: &str) -> Option<PathBuf> {
    let entry_path = bundle_dir.join(entry);
    entry_path.parent().map(|p| p.to_path_buf())
}

/// Join `suffix` against `root` and refuse paths that escape `root`.
///
/// Implementation notes:
/// - We do *component-level* checks before any FS call to reject `..`
///   and absolute paths up-front (faster + works for files that don't
///   exist yet).
/// - We then canonicalize both `root` and the joined path. If `root`
///   itself doesn't canonicalize (no UI dir on disk) we fall through
///   to the lexical result.
fn safe_join(root: &Path, suffix: &str) -> Option<PathBuf> {
    let suffix_path = Path::new(suffix);
    if suffix_path.is_absolute() {
        return None;
    }
    for comp in suffix_path.components() {
        use std::path::Component;
        match comp {
            Component::Normal(_) | Component::CurDir => {}
            _ => return None,
        }
    }
    let joined = root.join(suffix);
    // Best-effort canonical containment check. If either canonicalize
    // fails (file may not exist yet) we fall back to the lexical join,
    // which is already safe because of the component check above.
    if let (Ok(root_c), Ok(joined_c)) = (root.canonicalize(), joined.canonicalize()) {
        if !joined_c.starts_with(&root_c) {
            return None;
        }
        return Some(joined_c);
    }
    Some(joined)
}

/// Tiny mime sniffer for the file extensions an MF bundle ships.
/// Anything unknown becomes `application/octet-stream`.
fn guess_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("js") | Some("mjs") => "application/javascript",
        Some("css") => "text/css",
        Some("json") => "application/json",
        Some("map") => "application/json",
        Some("html") => "text/html; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_join_rejects_parent_escape() {
        let root = Path::new("/tmp/bundle/ui");
        assert!(safe_join(root, "../etc/passwd").is_none());
        assert!(safe_join(root, "a/../../etc").is_none());
        assert!(safe_join(root, "/etc/passwd").is_none());
        assert!(safe_join(root, "chunk.js").is_some());
        assert!(safe_join(root, "./chunk.js").is_some());
    }
}
