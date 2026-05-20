//! `GET /extensions/<id>/i18n/<lang>.json` — extension catalog serving.
//!
//! Each extension that contributes UI strings declares its catalogs in
//! the manifest:
//!
//! ```yaml
//! contributes:
//!   i18n:
//!     catalogs:
//!       en: i18n/en.json
//!       es: i18n/es.json
//! ```
//!
//! The host's `IntlProvider` fetches the catalog for the **currently
//! active** language only (D-NP.8 — `examples/notes/user-pref.md`). The
//! same `<lang>.json` URL is reached by the client when the operator
//! flips language at runtime.
//!
//! The handler is intentionally a near-copy of [`crate::ui::ui`]: same
//! safe-join path discipline, same etag short-circuit, same unauthed
//! exposure (catalog strings are public — they ship inside the bundle
//! the operator already approved by enabling the extension).
//!
//! 404 is returned when the extension does not declare the requested
//! language, so the client's lazy-load probe can fall through to its
//! own fallback chain (`es-MX` → `es` → `en` — D-NP.6) without
//! interpreting a 5xx as a transport error.

use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, HeaderMap, Response, StatusCode};
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::admin::ExtensionAdmin;

#[derive(Debug, Deserialize)]
pub(crate) struct I18nParams {
    id: String,
    lang: String,
}

pub(crate) async fn i18n(
    State(admin): State<ExtensionAdmin>,
    AxumPath(I18nParams { id, lang }): AxumPath<I18nParams>,
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
    let i18n_block = match manifest.contributes.i18n.as_ref() {
        Some(b) => b,
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    // Strip the `.json` suffix so callers can use the natural URL form
    // (`en.json`) and the manifest can stay tagged by bare language
    // code (`en`).
    let language = lang.strip_suffix(".json").unwrap_or(lang.as_str());
    let rel = match i18n_block.catalogs.get(language) {
        Some(p) => p.clone(),
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let target = match safe_join_root(&rec.bundle_dir, &rel) {
        Some(p) => p,
        None => return StatusCode::FORBIDDEN.into_response(),
    };

    let (etag, bytes) = match admin.etag_cache().etag_and_bytes(&target).await {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return StatusCode::NOT_FOUND.into_response()
        }
        Err(e) => {
            tracing::warn!(err = %e, path = %target.display(), "i18n catalog read failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

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

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ETAG, etag)
        .header(header::CACHE_CONTROL, "public, max-age=0, must-revalidate")
        .body(Body::from(bytes))
        .expect("ok response is well-formed")
}

/// Resolve `rel` against `bundle_dir`, refusing any escape via `..` or
/// absolute paths. The manifest-declared catalog path is operator-
/// supplied (sort of — through the extension author) so we keep the
/// same defensive component-check + canonical containment as
/// [`crate::ui::safe_join`].
fn safe_join_root(bundle_dir: &Path, rel: &str) -> Option<PathBuf> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() {
        return None;
    }
    for comp in rel_path.components() {
        use std::path::Component;
        match comp {
            Component::Normal(_) | Component::CurDir => {}
            _ => return None,
        }
    }
    let joined = bundle_dir.join(rel_path);
    if let (Ok(root_c), Ok(joined_c)) = (bundle_dir.canonicalize(), joined.canonicalize()) {
        if !joined_c.starts_with(&root_c) {
            return None;
        }
        return Some(joined_c);
    }
    Some(joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_join_rejects_escape() {
        let root = Path::new("/tmp/bundle");
        assert!(safe_join_root(root, "../etc/passwd").is_none());
        assert!(safe_join_root(root, "/etc/passwd").is_none());
        assert!(safe_join_root(root, "i18n/en.json").is_some());
    }
}
