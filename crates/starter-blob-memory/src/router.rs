//! Feature-gated axum router that honours `MemoryBlobStore`'s own
//! presigned URLs.
//!
//! # Why this lives in the same crate
//!
//! The SCOPE requires the presign contract to be testable
//! end-to-end without a real S3. That means *some* axum router has
//! to validate and dispatch on the engine's HMAC tokens, and the
//! only entity that knows the HMAC key is the
//! [`MemoryBlobStore`](crate::MemoryBlobStore) instance. Putting
//! the router behind a sibling crate would either force the key
//! onto the public API (B2-adjacent violation) or invent a back
//! channel; neither earns its complexity. The router compiles only
//! under the `axum` feature so consumers who never test the
//! presign loop pay nothing.
//!
//! # Wire shape
//!
//! - `GET /?token=<token>` — verifies the token (which must carry
//!   `PresignOp::Get`) and streams the blob's bytes.
//! - `PUT /?token=<token>` — verifies the token (`PresignOp::Put`)
//!   and writes the request body under the locator.
//!
//! On any verification failure the router returns `403 Forbidden`.
//! On a locator miss for `GET`, `404 Not Found`. The router emits
//! spans under `starter_blob::memory` per the observability
//! contract.

use axum::{
    body::{to_bytes, Body},
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use chrono::Utc;
use serde::Deserialize;
use starter_spi::blob::{BlobMeta, PresignOp};
use tracing::warn;

use crate::presign::{self, PresignClaim, VerifyError};
use crate::store::{Entry, MemoryBlobStore};
use crate::TRACE_TARGET;

/// Build the router bound to `store`. Mount under any path; the
/// presigned URLs are query-string driven, not path-driven.
pub fn router(store: MemoryBlobStore) -> Router {
    Router::new()
        .route("/", get(handle_get).put(handle_put))
        .with_state(store)
}

#[derive(Deserialize)]
struct TokenQuery {
    token: String,
}

fn check(
    store: &MemoryBlobStore,
    token: &str,
    expect_op: PresignOp,
) -> Result<PresignClaim, StatusCode> {
    match presign::verify(store.hmac_key(), token) {
        Ok(claim) if claim.op == expect_op => Ok(claim),
        Ok(_) => {
            warn!(target: TRACE_TARGET, "presign op mismatch");
            Err(StatusCode::FORBIDDEN)
        }
        Err(VerifyError::Expired) => {
            warn!(target: TRACE_TARGET, "presign token expired");
            Err(StatusCode::FORBIDDEN)
        }
        Err(e) => {
            warn!(target: TRACE_TARGET, error = %e, "presign reject");
            Err(StatusCode::FORBIDDEN)
        }
    }
}

async fn handle_get(
    State(store): State<MemoryBlobStore>,
    Query(q): Query<TokenQuery>,
) -> impl IntoResponse {
    let claim = match check(&store, &q.token, PresignOp::Get) {
        Ok(c) => c,
        Err(s) => return s.into_response(),
    };
    let data = store.data().read().await;
    let Some(entry) = data.get(&claim.locator) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mut resp = entry.bytes.clone().into_response();
    if let Some(ct) = entry.meta.content_type.as_deref() {
        if let Ok(v) = ct.parse() {
            resp.headers_mut().insert("content-type", v);
        }
    }
    resp
}

async fn handle_put(
    State(store): State<MemoryBlobStore>,
    Query(q): Query<TokenQuery>,
    body: Body,
) -> impl IntoResponse {
    let claim = match check(&store, &q.token, PresignOp::Put) {
        Ok(c) => c,
        Err(s) => return s,
    };
    let Ok(bytes) = to_bytes(body, usize::MAX).await else {
        return StatusCode::BAD_REQUEST;
    };
    let etag = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(&bytes);
        let d = h.finalize();
        let hex: String = d.iter().take(16).map(|b| format!("{b:02x}")).collect();
        starter_spi::blob::Etag::new(hex)
    };
    let now = Some(Utc::now());
    let meta = BlobMeta::new(bytes.len() as u64, etag)
        .with_created_at(now)
        .with_updated_at(now);
    store
        .data()
        .write()
        .await
        .insert(claim.locator, Entry { bytes, meta });
    StatusCode::NO_CONTENT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryBlobStore;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use bytes::Bytes;
    use http_body_util::BodyExt;
    use starter_spi::blob::{BlobKey, BlobStore, PresignOp, PutOptions};
    use std::time::Duration;
    use tower::ServiceExt;

    fn key(s: &str) -> BlobKey {
        BlobKey::new(s).unwrap()
    }

    #[tokio::test]
    async fn presigned_get_round_trip() {
        let store = MemoryBlobStore::new();
        let r = store
            .put_bytes(
                &key("k"),
                Bytes::from_static(b"hello"),
                PutOptions::with_content_type("text/plain"),
            )
            .await
            .unwrap();
        let url = store
            .presign(&r, PresignOp::Get, Duration::from_secs(30))
            .await
            .unwrap();
        let token = url.url.split("token=").nth(1).unwrap().to_owned();

        let app = router(store);
        let resp = app
            .oneshot(
                Request::get(format!("/?token={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&bytes[..], b"hello");
    }

    #[tokio::test]
    async fn forged_token_is_forbidden() {
        let store = MemoryBlobStore::new();
        let app = router(store);
        let resp = app
            .oneshot(
                Request::get("/?token=bogus.signature")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn presigned_put_then_get() {
        let store = MemoryBlobStore::new();
        // Put against an empty locator: we mint a ref via put_bytes
        // first so the presign carries a known locator, then PUT
        // overwrites.
        let r = store
            .put_bytes(&key("k"), Bytes::from_static(b"old"), PutOptions::default())
            .await
            .unwrap();
        let put_url = store
            .presign(&r, PresignOp::Put, Duration::from_secs(30))
            .await
            .unwrap();
        let get_url = store
            .presign(&r, PresignOp::Get, Duration::from_secs(30))
            .await
            .unwrap();
        let put_tok = put_url.url.split("token=").nth(1).unwrap().to_owned();
        let get_tok = get_url.url.split("token=").nth(1).unwrap().to_owned();

        let app = router(store);
        let resp = app
            .clone()
            .oneshot(
                Request::put(format!("/?token={put_tok}"))
                    .body(Body::from("new-bytes"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        let resp = app
            .oneshot(
                Request::get(format!("/?token={get_tok}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"new-bytes");
    }
}
