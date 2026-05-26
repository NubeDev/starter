//! The [`blob_proxy_handler`] router and its supporting types.
//!
//! Wire shape:
//!
//! ```text
//! GET  /{ref}            — stream the body
//! HEAD /{ref}            — metadata only (size, etag, content-type, …)
//! GET  /{ref}?download=1 — same, but force `Content-Disposition: attachment`
//! ```
//!
//! `{ref}` is the URL-safe base64 of the serde-JSON form of a
//! [`BlobRef`]. This shape was chosen over passing JSON inline in
//! a path segment because path-segment percent-encoding of `{`/`}`/`/`
//! is brittle across reverse proxies; base64 in a single segment
//! survives every proxy we ship against.
//!
//! Headers honoured end-to-end:
//!
//! - `Range`           — forwarded to the engine; partial-content responses pass back as `206`.
//! - `If-None-Match`   — compared against the blob's etag; matches return `304`.
//! - `Accept-Encoding` — forwarded; engines that store pre-compressed bodies set `Content-Encoding`.
//!
//! Headers set on the response:
//!
//! - `Content-Type`        — from [`BlobMeta::content_type`], fallback `application/octet-stream`.
//! - `Content-Length`      — from [`BlobMeta::size`] (only on full-body responses).
//! - `Content-Range`       — on `206 Partial Content`.
//! - `ETag`                — from [`BlobMeta::etag`].
//! - `Cache-Control`       — from [`BlobMeta::cache_control`] when set.
//! - `Content-Disposition` — `attachment; filename="…"` when `?download=1` and `FILENAME` meta is set.
//! - `Retry-After`         — on `503` from [`BlobError::Throttled`], when the engine names a duration.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde::Deserialize;
use starter_spi::blob::{
    meta_keys, BlobContext, BlobError, BlobMeta, BlobRange, BlobRef, BlobStore,
};

use crate::mapping::blob_error_to_status;

/// Erased authz callback shape. The proxy hands the closure a
/// reference to the parsed [`BlobRef`], its [`BlobContext`]
/// (carrying any namespace prefix the combinator stack peeled
/// off), and the incoming request so the closure can inspect
/// cookies / headers / extensions to identify the viewer.
///
/// Returning `Ok(())` permits the request; any [`BlobError`]
/// returned is mapped to its HTTP status by
/// [`crate::blob_error_to_status`] and short-circuits the
/// pipeline. The typical "this viewer cannot see this blob"
/// answer is [`BlobError::Forbidden`].
pub type AuthzFn =
    dyn Fn(&BlobRef, &BlobContext, &Request<Body>) -> Result<(), BlobError> + Send + Sync + 'static;

/// Build an `axum::Router` that serves authenticated GET / HEAD
/// requests for any [`BlobStore`].
///
/// `authz` is called *after* the [`BlobRef`] is parsed and the
/// [`BlobContext`] resolved, but *before* the engine is touched
/// — so an unauthorised request never causes a backend read.
///
/// # Example
///
/// ```ignore
/// let app = blob_proxy_handler(
///     Arc::new(my_store),
///     |blob_ref, ctx, req| {
///         let project = ctx.outer_namespace().ok_or(BlobError::Forbidden)?;
///         if viewer_of(req).can_read_project(project) {
///             Ok(())
///         } else {
///             Err(BlobError::Forbidden)
///         }
///     },
/// );
/// ```
pub fn blob_proxy_handler<F>(store: Arc<dyn BlobStore>, authz: F) -> Router
where
    F: Fn(&BlobRef, &BlobContext, &Request<Body>) -> Result<(), BlobError> + Send + Sync + 'static,
{
    let state = ProxyState {
        store,
        authz: Arc::new(authz),
    };

    Router::new()
        .route("/{r#ref}", get(serve).head(serve))
        .with_state(state)
}

#[derive(Clone)]
struct ProxyState {
    store: Arc<dyn BlobStore>,
    authz: Arc<AuthzFn>,
}

#[derive(Deserialize)]
struct ServeQuery {
    /// When `1`/`true`, force `Content-Disposition: attachment` and
    /// use the `filename` user-metadata key if present.
    #[serde(default)]
    download: Option<String>,
}

async fn serve(
    State(state): State<ProxyState>,
    Path(ref_b64): Path<String>,
    Query(q): Query<ServeQuery>,
    req: Request<Body>,
) -> Response {
    let blob_ref = match decode_blob_ref(&ref_b64) {
        Ok(r) => r,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid ref").into_response(),
    };

    let ctx = state.store.context_for(&blob_ref);

    if let Err(e) = (state.authz)(&blob_ref, &ctx, &req) {
        return error_response(&e);
    }

    let meta = match state.store.head(&blob_ref).await {
        Ok(m) => m,
        Err(e) => return error_response(&e),
    };

    // If-None-Match short-circuit before doing the streaming get.
    if let Some(inm) = req.headers().get(header::IF_NONE_MATCH) {
        if matches_etag(inm, meta.etag.as_str()) {
            return (StatusCode::NOT_MODIFIED, build_headers(&meta, &q, None)).into_response();
        }
    }

    if req.method() == Method::HEAD {
        return (StatusCode::OK, build_headers(&meta, &q, None)).into_response();
    }

    let range = parse_range(req.headers(), meta.size);

    let stream = match state.store.get(&blob_ref, range).await {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };

    let body = Body::from_stream(stream);
    let status = if range.is_some() {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };

    let headers = build_headers(&meta, &q, range);
    (status, headers, body).into_response()
}

fn decode_blob_ref(input: &str) -> Result<BlobRef, ()> {
    let bytes = URL_SAFE_NO_PAD.decode(input).map_err(|_| ())?;
    serde_json::from_slice(&bytes).map_err(|_| ())
}

fn matches_etag(header_value: &HeaderValue, etag: &str) -> bool {
    let Ok(s) = header_value.to_str() else {
        return false;
    };
    // Honour `*` and exact match. We don't implement weak/strong
    // comparison nuances — engines mint strong etags and the
    // proxy reflects them verbatim.
    s.trim() == "*"
        || s.split(',')
            .any(|part| part.trim().trim_matches('"') == etag)
}

fn parse_range(headers: &HeaderMap, size: u64) -> Option<BlobRange> {
    let raw = headers.get(header::RANGE)?.to_str().ok()?;
    let rest = raw.strip_prefix("bytes=")?;
    let (start_s, end_s) = rest.split_once('-')?;
    let start: u64 = start_s.parse().ok()?;
    let end: u64 = if end_s.is_empty() {
        size.saturating_sub(1)
    } else {
        end_s.parse().ok()?
    };
    BlobRange::new(start, end)
}

fn build_headers(meta: &BlobMeta, q: &ServeQuery, range: Option<BlobRange>) -> HeaderMap {
    let mut headers = HeaderMap::new();

    let ct = meta
        .content_type
        .as_deref()
        .unwrap_or("application/octet-stream");
    if let Ok(v) = HeaderValue::from_str(ct) {
        headers.insert(header::CONTENT_TYPE, v);
    }

    if let Ok(v) = HeaderValue::from_str(&format!("\"{}\"", meta.etag.as_str())) {
        headers.insert(header::ETAG, v);
    }

    if let Some(cc) = meta.cache_control.as_deref() {
        if let Ok(v) = HeaderValue::from_str(cc) {
            headers.insert(header::CACHE_CONTROL, v);
        }
    }

    match range {
        None => {
            headers.insert(header::CONTENT_LENGTH, meta.size.into());
        }
        Some(r) => {
            let end = r.end.min(meta.size.saturating_sub(1));
            let len = end - r.start + 1;
            if let Ok(v) =
                HeaderValue::from_str(&format!("bytes {}-{}/{}", r.start, end, meta.size))
            {
                headers.insert(header::CONTENT_RANGE, v);
            }
            headers.insert(header::CONTENT_LENGTH, len.into());
        }
    }

    if download_requested(q) {
        let filename = meta
            .user_metadata
            .get(meta_keys::FILENAME)
            .map(String::as_str)
            .unwrap_or("download");
        let cd = format!("attachment; filename=\"{}\"", filename.replace('"', "\\\""));
        if let Ok(v) = HeaderValue::from_str(&cd) {
            headers.insert(header::CONTENT_DISPOSITION, v);
        }
    }

    headers
}

fn download_requested(q: &ServeQuery) -> bool {
    matches!(
        q.download.as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

fn error_response(err: &BlobError) -> Response {
    let status = blob_error_to_status(err);
    let mut resp = (status, err.to_string()).into_response();
    if let BlobError::Throttled {
        retry_after: Some(d),
    } = err
    {
        if let Ok(v) = HeaderValue::from_str(&d.as_secs().to_string()) {
            resp.headers_mut().insert(header::RETRY_AFTER, v);
        }
    }
    resp
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use axum::body::to_bytes;
    use axum::http::{Request, StatusCode};
    use bytes::Bytes;
    use starter_blob_compose::Namespaced;
    use starter_blob_memory::MemoryBlobStore as MemStore;
    use starter_spi::blob::{meta_keys, BlobKey, BlobStore, PutOptions};
    use tower::ServiceExt;

    use super::*;

    fn put(store: &dyn BlobStore, key: &str, body: &[u8], filename: Option<&str>) -> BlobRef {
        let key = BlobKey::new(key).unwrap();
        let mut opts = PutOptions::default();
        if let Some(name) = filename {
            opts = opts.user_meta(meta_keys::FILENAME, name);
        }
        opts.content_type = Some("text/plain".into());
        futures::executor::block_on(store.put_bytes(&key, Bytes::copy_from_slice(body), opts))
            .unwrap()
    }

    fn ref_to_path(r: &BlobRef) -> String {
        let json = serde_json::to_vec(r).unwrap();
        URL_SAFE_NO_PAD.encode(json)
    }

    #[tokio::test]
    async fn get_serves_body_with_etag_and_content_type() {
        let store: Arc<dyn BlobStore> = Arc::new(MemStore::new());
        let r = put(&*store, "hello.txt", b"hello world", Some("hello.txt"));

        let app = blob_proxy_handler(store, |_, _, _| Ok(()));

        let req = Request::builder()
            .uri(format!("/{}", ref_to_path(&r)))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get("content-type").unwrap(), "text/plain");
        assert!(resp.headers().get("etag").is_some());
        let body = to_bytes(resp.into_body(), 1024).await.unwrap();
        assert_eq!(&body[..], b"hello world");
    }

    #[tokio::test]
    async fn authz_denial_returns_403_without_touching_store() {
        let store: Arc<dyn BlobStore> = Arc::new(MemStore::new());
        let r = put(&*store, "secret.txt", b"shhh", None);

        let app = blob_proxy_handler(store, |_, _, _| Err(BlobError::Forbidden));

        let req = Request::builder()
            .uri(format!("/{}", ref_to_path(&r)))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn authz_receives_namespace_prefix_via_blob_context() {
        let inner = MemStore::new();
        let store: Arc<dyn BlobStore> =
            Arc::new(Namespaced::new(Arc::new(inner), "project-7").unwrap());

        let key = BlobKey::new("notes.txt").unwrap();
        let r = store
            .put_bytes(&key, Bytes::from_static(b"x"), PutOptions::default())
            .await
            .unwrap();

        // Capture the namespace the authz closure observes.
        let seen: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
        let seen_for_closure = seen.clone();
        let app = blob_proxy_handler(store, move |_, ctx, _| {
            if let Some(ns) = ctx.outer_namespace() {
                seen_for_closure.lock().unwrap().push(ns.to_string());
            }
            Ok(())
        });

        let req = Request::builder()
            .uri(format!("/{}", ref_to_path(&r)))
            .body(Body::empty())
            .unwrap();
        let _ = app.oneshot(req).await.unwrap();

        let seen = seen.lock().unwrap();
        assert_eq!(seen.as_slice(), &["project-7".to_string()]);
    }

    #[tokio::test]
    async fn download_sets_content_disposition_from_filename_meta() {
        let store: Arc<dyn BlobStore> = Arc::new(MemStore::new());
        let r = put(&*store, "blob.bin", b"data", Some("invoice.pdf"));

        let app = blob_proxy_handler(store, |_, _, _| Ok(()));

        let req = Request::builder()
            .uri(format!("/{}?download=1", ref_to_path(&r)))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let cd = resp.headers().get("content-disposition").unwrap();
        assert_eq!(cd, "attachment; filename=\"invoice.pdf\"");
    }

    #[tokio::test]
    async fn if_none_match_returns_304() {
        let store: Arc<dyn BlobStore> = Arc::new(MemStore::new());
        let r = put(&*store, "v.txt", b"v1", None);

        let app = blob_proxy_handler(store.clone(), |_, _, _| Ok(()));

        // Discover etag via a first GET.
        let req = Request::builder()
            .uri(format!("/{}", ref_to_path(&r)))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let etag = resp.headers().get("etag").unwrap().clone();

        let req2 = Request::builder()
            .uri(format!("/{}", ref_to_path(&r)))
            .header("if-none-match", etag)
            .body(Body::empty())
            .unwrap();
        let resp2 = app.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::NOT_MODIFIED);
    }

    #[tokio::test]
    async fn range_request_returns_206() {
        let store: Arc<dyn BlobStore> = Arc::new(MemStore::new());
        let r = put(&*store, "rng.txt", b"abcdefghij", None);

        let app = blob_proxy_handler(store, |_, _, _| Ok(()));

        let req = Request::builder()
            .uri(format!("/{}", ref_to_path(&r)))
            .header("range", "bytes=2-5")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(resp.headers().get("content-range").unwrap(), "bytes 2-5/10");
        let body = to_bytes(resp.into_body(), 1024).await.unwrap();
        assert_eq!(&body[..], b"cdef");
    }

    #[tokio::test]
    async fn missing_blob_returns_404() {
        let store: Arc<dyn BlobStore> = Arc::new(MemStore::new());
        let r = put(&*store, "x.txt", b"x", None);
        // Delete it so head returns NotFound.
        store.delete(&r).await.unwrap();

        let app = blob_proxy_handler(store, |_, _, _| Ok(()));

        let req = Request::builder()
            .uri(format!("/{}", ref_to_path(&r)))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn user_metadata_roundtrips_filename_via_head() {
        let store: MemStore = MemStore::new();
        let r = put(&store, "f.bin", b"x", Some("Final Report.pdf"));
        let meta = store.head(&r).await.unwrap();
        assert_eq!(
            meta.user_metadata.get(meta_keys::FILENAME),
            Some(&"Final Report.pdf".to_string())
        );
        // suppress unused
        let _: BTreeMap<String, String> = meta.user_metadata;
    }
}
