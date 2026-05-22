//! Feature-gated axum router that honours `FsBlobStore`'s presigned URLs.
//!
//! Shipped in the same crate as the engine for the same reason
//! `starter-blob-memory` does: only the engine instance holds the
//! [`PresignKey`](crate::PresignKey), and the router has to verify
//! against that key. Compiles only under `--features axum`.
//!
//! # Wire shape
//!
//! - `GET /?token=<token>` — verifies `PresignOp::Get`, streams the
//!   file from disk via `tokio_util::io::ReaderStream`.
//! - `PUT /?token=<token>` — verifies `PresignOp::Put`, writes the
//!   request body to a tempfile alongside the destination and
//!   atomically renames into place. Same durability story as the
//!   trait-level `put_bytes`.

use std::path::Path;

use axum::{
    body::{to_bytes, Body},
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use starter_spi::blob::PresignOp;
use tempfile::NamedTempFile;
use tracing::warn;

use crate::presign::{self, PresignClaim, VerifyError};
use crate::store::FsBlobStore;
use crate::TRACE_TARGET;

/// Build the router bound to `store`.
pub fn router(store: FsBlobStore) -> Router {
    Router::new()
        .route("/", get(handle_get).put(handle_put))
        .with_state(store)
}

#[derive(Deserialize)]
struct TokenQuery {
    token: String,
}

fn check(
    store: &FsBlobStore,
    token: &str,
    expect_op: PresignOp,
) -> Result<PresignClaim, StatusCode> {
    match presign::verify(store.presign_key(), token) {
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

fn data_path(root: &Path, locator: &str) -> std::path::PathBuf {
    root.join(locator)
}

async fn handle_get(State(store): State<FsBlobStore>, Query(q): Query<TokenQuery>) -> Response {
    let claim = match check(&store, &q.token, PresignOp::Get) {
        Ok(c) => c,
        Err(s) => return s.into_response(),
    };
    let path = data_path(store.root(), &claim.locator);
    let file = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return StatusCode::NOT_FOUND.into_response();
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let stream = tokio_util::io::ReaderStream::new(file);
    let body = Body::from_stream(stream);
    let mut resp = body.into_response();
    if let Some(meta) = store.read_meta(&claim.locator).await {
        if let Some(ct) = meta.content_type.as_deref() {
            if let Ok(v) = ct.parse() {
                resp.headers_mut().insert("content-type", v);
            }
        }
    }
    resp
}

async fn handle_put(
    State(store): State<FsBlobStore>,
    Query(q): Query<TokenQuery>,
    body: Body,
) -> StatusCode {
    let claim = match check(&store, &q.token, PresignOp::Put) {
        Ok(c) => c,
        Err(s) => return s,
    };
    let dst = data_path(store.root(), &claim.locator);
    if let Some(parent) = dst.parent() {
        if tokio::fs::create_dir_all(parent).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }
    let Ok(bytes) = to_bytes(body, usize::MAX).await else {
        return StatusCode::BAD_REQUEST;
    };
    let parent = dst.parent().unwrap_or_else(|| Path::new("."));
    let Ok(tmp) = NamedTempFile::new_in(parent) else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };
    {
        use std::io::Write;
        let f: &std::fs::File = tmp.as_file();
        let mut bw = std::io::BufWriter::new(f);
        if bw.write_all(&bytes).is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
        if bw.flush().is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }
    if tmp.as_file().sync_all().is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    if tmp.persist(&dst).is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::NO_CONTENT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FsBlobStore, PresignKey};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use bytes::Bytes;
    use http_body_util::BodyExt;
    use starter_spi::blob::{BlobKey, BlobStore, PresignOp, PutOptions};
    use std::time::Duration;
    use tower::ServiceExt;

    fn k(s: &str) -> BlobKey {
        BlobKey::new(s).unwrap()
    }

    #[tokio::test]
    async fn presigned_get_streams_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsBlobStore::open(dir.path(), PresignKey::ephemeral()).unwrap();
        let r = store
            .put_bytes(
                &k("dir/file.txt"),
                Bytes::from_static(b"on-disk"),
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
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"on-disk");
    }

    #[tokio::test]
    async fn forged_token_forbidden() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsBlobStore::open(dir.path(), PresignKey::ephemeral()).unwrap();
        let app = router(store);
        let resp = app
            .oneshot(
                Request::get("/?token=garbage.signature")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
