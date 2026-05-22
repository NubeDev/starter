//! Example app: two routes (`PUT /attachments/:name`,
//! `GET /attachments/:id`) over a SQLite `attachments` table and
//! an `Arc<dyn BlobStore>`.
//!
//! The point of the example is the *shape*: the row stores the
//! `BlobRef` as opaque serde JSON. The trait surface alone
//! satisfies the SCOPE's B1 (no `put_attachment` method exists on
//! `BlobStore` — the domain word lives in *this* file, on the
//! route, not on the seam).

use std::sync::Arc;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use starter_spi::blob::{BlobKey, BlobRef, BlobStore, PresignOp, PutOptions};
use starter_store_sqlite::Pool;

/// Shared state for the example routes. Both axum extractors and
/// the test harness construct one.
#[derive(Clone)]
pub struct AppState {
    pub db: Pool,
    pub blobs: Arc<dyn BlobStore>,
}

/// Apply the example's tiny schema. Kept inline rather than under
/// `migrations/` because the example DB is in-memory and the
/// schema is one table; a production consumer migrates the same
/// shape via `starter-store-sqlite`'s `MigrationSource`.
pub async fn migrate(db: &Pool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS attachments (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            name          TEXT NOT NULL,
            blob_ref_json TEXT NOT NULL
        )
        "#,
    )
    .execute(db.sqlx())
    .await?;
    Ok(())
}

/// Build the consumer-side router. The blob engine's router is
/// mounted separately in `main`.
pub fn router(db: Pool, blobs: Arc<dyn BlobStore>) -> Router {
    let state = AppState { db, blobs };
    Router::new()
        .route("/attachments/{name}", put(upload))
        .route("/attachments/by-id/{id}", get(fetch))
        .with_state(state)
}

#[derive(Serialize, Deserialize)]
struct UploadResponse {
    id: i64,
}

async fn upload(
    State(state): State<AppState>,
    Path(name): Path<String>,
    body: Bytes,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    let key = BlobKey::new(format!("attachments/{name}"))
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("bad key: {e}")))?;
    let blob_ref = state
        .blobs
        .put_bytes(
            &key,
            body,
            PutOptions::with_content_type("application/octet-stream"),
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("put: {e}")))?;
    let blob_ref_json = serde_json::to_string(&blob_ref).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("encode ref: {e}"),
        )
    })?;
    let id = sqlx::query("INSERT INTO attachments (name, blob_ref_json) VALUES (?1, ?2)")
        .bind(&name)
        .bind(&blob_ref_json)
        .execute(state.db.sqlx())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("insert: {e}")))?
        .last_insert_rowid();
    Ok(Json(UploadResponse { id }))
}

#[derive(Serialize)]
struct FetchResponse {
    name: String,
    presigned_url: String,
    expires_at_unix: u64,
}

async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<FetchResponse>, (StatusCode, String)> {
    let row = sqlx::query("SELECT name, blob_ref_json FROM attachments WHERE id = ?1")
        .bind(id)
        .fetch_optional(state.db.sqlx())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("select: {e}")))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("no attachment {id}")))?;
    let name: String = row.get(0);
    let blob_ref_json: String = row.get(1);
    let blob_ref: BlobRef = serde_json::from_str(&blob_ref_json).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("decode ref: {e}"),
        )
    })?;
    let url = state
        .blobs
        .presign(&blob_ref, PresignOp::Get, Duration::from_secs(60))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("presign: {e}")))?;
    let expires_at_unix = url
        .expires_at
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    Ok(Json(FetchResponse {
        name,
        presigned_url: url.url,
        expires_at_unix,
    }))
}
