//! REST routes: `GET /v1/i18n/manifest` and
//! `GET /v1/i18n/catalogs/{lang}` (plus the content-hashed variant
//! `GET /v1/i18n/catalogs/{lang}-{fingerprint}.json`).
//!
//! Owns: SCOPE.md Phase 3 "API surface" — catalog distribution and
//! the immutable-cache + ETag-revalidate posture.
//!
//! # Manifest
//!
//! `GET /v1/i18n/manifest` returns a flat JSON object mapping every
//! language tag the binary serves to its 16-char fingerprint:
//!
//! ```json
//! { "en": "0123456789abcdef", "es": "fedcba9876543210" }
//! ```
//!
//! Fingerprints are stable across calls for a given bundle (see
//! [`crate::catalog::Catalog::fingerprint`]); the manifest itself
//! carries an `ETag` derived from the sorted manifest bytes so a
//! conditional `If-None-Match` re-request returns `304 Not Modified`.
//!
//! # Catalogs
//!
//! Two URL shapes serve the same JSON body:
//!
//! - **Un-fingerprinted** `GET /v1/i18n/catalogs/{lang}` — supports
//!   plain `ETag` / `If-None-Match` revalidation. `Cache-Control` is
//!   `public, max-age=0, must-revalidate` so a CDN may store the
//!   payload but must revalidate before serving.
//! - **Fingerprinted** `GET /v1/i18n/catalogs/{lang}-{fingerprint}.json`
//!   — when the caller embeds the fingerprint (typically taken from a
//!   previous `/manifest` fetch) the response carries
//!   `Cache-Control: public, max-age=31536000, immutable`. CDNs and
//!   browsers may cache forever; if the catalog changes its
//!   fingerprint also changes, so the URL changes too.
//!
//! When the path embeds a fingerprint that does NOT match the served
//! catalog the route still returns the current bytes (the immutable
//! cache contract is on the URL, not on the response) but downgrades
//! `Cache-Control` to the revalidate posture so a stale fingerprint
//! is not promoted to immutable.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use http::header::{
    HeaderValue, CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH, VARY,
};
use http::StatusCode;
use starter_spi::i18n::LanguageTag;

use crate::bundle::MessageBundle;
use crate::catalog::Catalog;

/// `Cache-Control` value for immutable, content-addressed responses.
const CC_IMMUTABLE: &str = "public, max-age=31536000, immutable";
/// `Cache-Control` value for revalidate-style responses (ETag-driven).
const CC_REVALIDATE: &str = "public, max-age=0, must-revalidate";

/// Build the i18n routes router, attached to a shared
/// [`MessageBundle`]. Mount under whatever prefix the host server
/// wants; the SCOPE-documented paths are bare (`/v1/i18n/manifest` and
/// `/v1/i18n/catalogs/...`) so callers typically mount this at `/`.
pub fn router(bundle: Arc<MessageBundle>) -> Router {
    Router::new()
        .route("/v1/i18n/manifest", get(get_manifest))
        .route("/v1/i18n/catalogs/{spec}", get(get_catalog))
        .with_state(bundle)
}

/// The manifest body: a sorted JSON object mapping language → 16-char
/// fingerprint.
fn manifest_bytes(bundle: &MessageBundle) -> Vec<u8> {
    // BTreeMap gives deterministic key order, which keeps the ETag
    // stable across processes regardless of insertion order.
    let mut map: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for lang in bundle.languages() {
        if let Some(cat) = bundle.catalog(lang) {
            map.insert(lang.as_str().to_string(), cat.fingerprint());
        }
    }
    serde_json::to_vec(&map).expect("manifest always serialises")
}

/// 16-char fingerprint of the manifest bytes. Format mirrors
/// `Catalog::fingerprint` so the wire shape is uniform.
fn manifest_fingerprint(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let hex = format!("{digest:x}");
    hex[..16].to_string()
}

/// Build a quoted ETag value (`"<fingerprint>"`) per RFC 7232 §2.3.
fn etag_value(fingerprint: &str) -> String {
    format!("\"{fingerprint}\"")
}

/// `If-None-Match` revalidation check — returns `true` when the
/// header's value matches `etag` exactly (we do not honour the `*`
/// wildcard here because cache validators only have one resource to
/// match against).
fn if_none_match_hit(headers: &http::HeaderMap, etag: &str) -> bool {
    headers
        .get(IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .map(|raw| {
            // RFC 7232 allows comma-separated multi-tag lists.
            raw.split(',').any(|p| p.trim() == etag)
        })
        .unwrap_or(false)
}

/// Parse the `{spec}` path segment for `GET /v1/i18n/catalogs/{spec}`.
///
/// Two shapes are recognised:
///
/// - `"<lang>"` — un-fingerprinted, e.g. `"en"`, `"en-US"`.
/// - `"<lang>-<fp>.json"` — content-hashed; the fingerprint is the
///   tail before the `.json` extension. The hyphen separating the
///   language tag from the fingerprint is the LAST `-` before
///   `.json`. We extract the fingerprint conservatively: anything that
///   isn't a 16-char lowercase-hex blob means we treat the whole
///   spec as a plain language tag.
///
/// The hyphen-ambiguity story: BCP-47 tags use `-` internally (e.g.
/// `en-US`). The fingerprint suffix is always exactly 16 lowercase hex
/// characters preceded by `-` and followed by `.json`, which is a
/// regular language (no hex-only tag passes BCP-47 validation), so
/// peeling off the tail is unambiguous.
fn parse_catalog_spec(spec: &str) -> CatalogSpec {
    if let Some(stem) = spec.strip_suffix(".json") {
        if let Some((lang_part, fp_part)) = stem.rsplit_once('-') {
            if fp_part.len() == 16 && fp_part.chars().all(|c| c.is_ascii_hexdigit()) {
                return CatalogSpec {
                    lang: lang_part.to_string(),
                    fingerprint: Some(fp_part.to_string()),
                };
            }
        }
    }
    CatalogSpec {
        lang: spec.to_string(),
        fingerprint: None,
    }
}

struct CatalogSpec {
    lang: String,
    fingerprint: Option<String>,
}

/// `GET /v1/i18n/manifest`.
///
/// Returns `{<lang>: <16-char-fingerprint>}` for every shipped
/// catalog, with an `ETag` over the canonical manifest bytes.
/// Conditional `If-None-Match` returns `304 Not Modified`.
async fn get_manifest(
    State(bundle): State<Arc<MessageBundle>>,
    req_headers: http::HeaderMap,
) -> Response {
    let bytes = manifest_bytes(&bundle);
    let etag = etag_value(&manifest_fingerprint(&bytes));

    if if_none_match_hit(&req_headers, &etag) {
        return not_modified(&etag, CC_REVALIDATE);
    }

    let mut resp = Response::new(Body::from(bytes));
    *resp.status_mut() = StatusCode::OK;
    let h = resp.headers_mut();
    h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    if let Ok(v) = HeaderValue::from_str(&etag) {
        h.insert(ETAG, v);
    }
    h.insert(CACHE_CONTROL, HeaderValue::from_static(CC_REVALIDATE));
    h.insert(VARY, HeaderValue::from_static("Accept-Encoding"));
    resp
}

/// `GET /v1/i18n/catalogs/{spec}` — handles both un-fingerprinted and
/// content-hashed shapes.
async fn get_catalog(
    State(bundle): State<Arc<MessageBundle>>,
    Path(spec): Path<String>,
    req_headers: http::HeaderMap,
) -> Response {
    let parsed = parse_catalog_spec(&spec);

    let Ok(tag) = LanguageTag::parse(parsed.lang.clone()) else {
        return (StatusCode::NOT_FOUND, "unknown language tag").into_response();
    };
    let Some(cat) = bundle.catalog(&tag) else {
        return (StatusCode::NOT_FOUND, "no catalog for language").into_response();
    };

    let actual_fp = cat.fingerprint();
    let etag = etag_value(&actual_fp);

    // The fingerprint URL form gets the immutable cache header IF
    // the path-supplied fingerprint matches the current bytes. If
    // the caller embedded a stale fingerprint we still serve the
    // current bytes (the immutable contract lives on the URL) but
    // downgrade caching to the revalidate posture.
    let cache_control = match &parsed.fingerprint {
        Some(fp) if fp == &actual_fp => CC_IMMUTABLE,
        _ => CC_REVALIDATE,
    };

    if if_none_match_hit(&req_headers, &etag) {
        return not_modified(&etag, cache_control);
    }

    let bytes = serde_json::to_vec(cat).expect("Catalog always serialises");
    let mut resp = Response::new(Body::from(bytes));
    *resp.status_mut() = StatusCode::OK;
    let h = resp.headers_mut();
    h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    if let Ok(v) = HeaderValue::from_str(&etag) {
        h.insert(ETAG, v);
    }
    h.insert(CACHE_CONTROL, HeaderValue::from_static(cache_control));
    h.insert(VARY, HeaderValue::from_static("Accept-Encoding"));
    resp
}

/// Build a `304 Not Modified` response carrying the same `ETag` and
/// `Cache-Control` as the would-be `200` body — required by RFC 7232
/// §4.1 so the cache knows the validator and freshness directives
/// still apply.
fn not_modified(etag: &str, cache_control: &'static str) -> Response {
    let mut resp = Response::new(Body::empty());
    *resp.status_mut() = StatusCode::NOT_MODIFIED;
    let h = resp.headers_mut();
    if let Ok(v) = HeaderValue::from_str(etag) {
        h.insert(ETAG, v);
    }
    h.insert(CACHE_CONTROL, HeaderValue::from_static(cache_control));
    resp
}

/// Helper exposed for hosts that already own a `Router` and just want
/// to merge the i18n routes in. Equivalent to `router(bundle)` —
/// kept as a named function for symmetry with
/// `with_accept_units` / `with_request_id` in `starter-server`.
pub fn with_i18n_routes(router: Router, bundle: Arc<MessageBundle>) -> Router {
    router.merge(self::router(bundle))
}

// Re-export `Catalog` here purely for doc-link ergonomics; the route
// module is the natural entry point for "I want to expose i18n over
// HTTP" so keeping the type referenced under `routes::Catalog` makes
// the docs flow. No new public surface.
#[allow(unused_imports)]
use Catalog as _Catalog;

// ---------------------------------------------------------------------
// Integration tests.
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http::{Request as HttpRequest, StatusCode};
    use http_body_util::BodyExt;
    use starter_spi::i18n::MessageKey;
    use tower::ServiceExt;

    fn tag(s: &str) -> LanguageTag {
        LanguageTag::parse(s).expect("test tag must parse")
    }

    fn key(s: &str) -> MessageKey {
        MessageKey::parse(s).expect("test key must parse")
    }

    fn bundle() -> Arc<MessageBundle> {
        let mut b = MessageBundle::new(tag("en"));
        let mut en = std::collections::BTreeMap::new();
        en.insert(key("a.b"), "english".to_string());
        en.insert(key("c.d"), "two".to_string());
        b.insert(tag("en"), Catalog { messages: en });
        let mut es = std::collections::BTreeMap::new();
        es.insert(key("a.b"), "spanish".to_string());
        b.insert(tag("es"), Catalog { messages: es });
        Arc::new(b)
    }

    async fn body_json(resp: Response) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn body_bytes(resp: Response) -> Vec<u8> {
        resp.into_body().collect().await.unwrap().to_bytes().to_vec()
    }

    fn app() -> Router {
        router(bundle())
    }

    // -------- /v1/i18n/manifest --------

    #[tokio::test]
    async fn manifest_lists_every_language_and_is_stable() {
        let r1 = app()
            .oneshot(
                HttpRequest::builder()
                    .uri("/v1/i18n/manifest")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r1.status(), StatusCode::OK);
        let etag1 = r1.headers().get(ETAG).unwrap().to_str().unwrap().to_string();
        let j1 = body_json(r1).await;

        // Repeat — same fingerprints, same ETag.
        let r2 = app()
            .oneshot(
                HttpRequest::builder()
                    .uri("/v1/i18n/manifest")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let etag2 = r2.headers().get(ETAG).unwrap().to_str().unwrap().to_string();
        let j2 = body_json(r2).await;
        assert_eq!(etag1, etag2);
        assert_eq!(j1, j2);

        // Shape: every shipped lang is present with a 16-char fp.
        let obj = j1.as_object().expect("object");
        assert!(obj.contains_key("en"));
        assert!(obj.contains_key("es"));
        for v in obj.values() {
            let s = v.as_str().unwrap();
            assert_eq!(s.len(), 16);
            assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[tokio::test]
    async fn manifest_revalidates_to_304_on_if_none_match() {
        // First fetch to capture the ETag.
        let r1 = app()
            .oneshot(
                HttpRequest::builder()
                    .uri("/v1/i18n/manifest")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let etag = r1.headers().get(ETAG).unwrap().to_str().unwrap().to_string();

        let r2 = app()
            .oneshot(
                HttpRequest::builder()
                    .uri("/v1/i18n/manifest")
                    .header(IF_NONE_MATCH, etag.clone())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r2.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(r2.headers().get(ETAG).unwrap().to_str().unwrap(), etag);
        assert!(body_bytes(r2).await.is_empty());
    }

    // -------- /v1/i18n/catalogs/{lang} (un-fingerprinted) --------

    #[tokio::test]
    async fn un_fingerprinted_catalog_returns_etag_and_revalidate_cache_control() {
        let resp = app()
            .oneshot(
                HttpRequest::builder()
                    .uri("/v1/i18n/catalogs/en")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let cc = resp.headers().get(CACHE_CONTROL).unwrap().to_str().unwrap();
        assert_eq!(cc, CC_REVALIDATE);
        let etag = resp.headers().get(ETAG).expect("ETag present");
        assert!(etag.to_str().unwrap().starts_with('"'));
    }

    #[tokio::test]
    async fn un_fingerprinted_catalog_304_on_revalidate() {
        let r1 = app()
            .oneshot(
                HttpRequest::builder()
                    .uri("/v1/i18n/catalogs/en")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let etag = r1.headers().get(ETAG).unwrap().to_str().unwrap().to_string();

        let r2 = app()
            .oneshot(
                HttpRequest::builder()
                    .uri("/v1/i18n/catalogs/en")
                    .header(IF_NONE_MATCH, etag.clone())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r2.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(r2.headers().get(ETAG).unwrap().to_str().unwrap(), etag);
    }

    #[tokio::test]
    async fn unknown_language_returns_404() {
        let resp = app()
            .oneshot(
                HttpRequest::builder()
                    .uri("/v1/i18n/catalogs/zz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // -------- /v1/i18n/catalogs/{lang}-{fp}.json (fingerprinted) --------

    #[tokio::test]
    async fn fingerprinted_catalog_returns_immutable_cache_control() {
        // First grab the manifest to learn the en fingerprint.
        let m = app()
            .oneshot(
                HttpRequest::builder()
                    .uri("/v1/i18n/manifest")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let manifest = body_json(m).await;
        let fp = manifest["en"].as_str().unwrap().to_string();

        let url = format!("/v1/i18n/catalogs/en-{fp}.json");
        let resp = app()
            .oneshot(HttpRequest::builder().uri(&url).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let cc = resp.headers().get(CACHE_CONTROL).unwrap().to_str().unwrap();
        assert_eq!(cc, CC_IMMUTABLE);
        let etag = resp.headers().get(ETAG).unwrap().to_str().unwrap().to_string();
        assert_eq!(etag, format!("\"{fp}\""));

        // Body matches what the un-fingerprinted endpoint serves.
        let bytes_fp = body_bytes(resp).await;
        let resp_plain = app()
            .oneshot(
                HttpRequest::builder()
                    .uri("/v1/i18n/catalogs/en")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes_plain = body_bytes(resp_plain).await;
        assert_eq!(bytes_fp, bytes_plain);
    }

    #[tokio::test]
    async fn fingerprinted_url_with_stale_fp_downgrades_to_revalidate() {
        // 16-char hex but obviously wrong.
        let url = "/v1/i18n/catalogs/en-deadbeefdeadbeef.json";
        let resp = app()
            .oneshot(HttpRequest::builder().uri(url).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let cc = resp.headers().get(CACHE_CONTROL).unwrap().to_str().unwrap();
        assert_eq!(cc, CC_REVALIDATE);
    }

    #[tokio::test]
    async fn fingerprinted_catalog_for_lang_with_subtag() {
        // Add an en-GB catalog so the lang part of the spec carries a
        // hyphen of its own — exercises the "last hyphen before .json"
        // parsing rule.
        let mut b = MessageBundle::new(tag("en"));
        let mut en_gb = std::collections::BTreeMap::new();
        en_gb.insert(key("a.b"), "british".to_string());
        b.insert(tag("en-GB"), Catalog { messages: en_gb });
        let bundle = Arc::new(b);

        let manifest_bytes = manifest_bytes(&bundle);
        let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes).unwrap();
        let fp = manifest["en-GB"].as_str().unwrap().to_string();

        let app = router(bundle);
        let url = format!("/v1/i18n/catalogs/en-GB-{fp}.json");
        let resp = app
            .oneshot(HttpRequest::builder().uri(&url).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let cc = resp.headers().get(CACHE_CONTROL).unwrap().to_str().unwrap();
        assert_eq!(cc, CC_IMMUTABLE);
    }
}
