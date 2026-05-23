//! Typed CRUD for `ext_manifest_approvals` (W12 trust seam).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::pool::Pool;

/// One row of `ext_manifest_approvals`.
#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApprovalRow {
    pub ext_id: String,
    pub manifest_hash: String,
    pub approved_at: DateTime<Utc>,
    pub approved_by: String,
}

/// Result alias.
pub type Result<T> = std::result::Result<T, sqlx::Error>;

/// Record an approval. ON CONFLICT DO NOTHING — re-approval of the
/// same (ext_id, manifest_hash) pair is a no-op.
pub async fn approve(
    pool: &Pool,
    ext_id: &str,
    manifest_hash: &str,
    approved_by: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO ext_manifest_approvals (ext_id, manifest_hash, approved_by) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (ext_id, manifest_hash) DO NOTHING",
    )
    .bind(ext_id)
    .bind(manifest_hash)
    .bind(approved_by)
    .execute(pool.sqlx())
    .await?;
    Ok(())
}

/// True when a given (ext_id, manifest_hash) pair is approved.
/// `mart.define` reads this to decide whether ext-authored marts
/// land `pending` (approved hash) or `quarantined` (new hash).
pub async fn is_approved(pool: &Pool, ext_id: &str, manifest_hash: &str) -> Result<bool> {
    let row: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM ext_manifest_approvals \
         WHERE ext_id = $1 AND manifest_hash = $2",
    )
    .bind(ext_id)
    .bind(manifest_hash)
    .fetch_optional(pool.sqlx())
    .await?;
    Ok(row.is_some())
}

/// List approvals for an extension (ordered newest-first).
pub async fn list(pool: &Pool, ext_id: &str) -> Result<Vec<ApprovalRow>> {
    sqlx::query_as::<_, ApprovalRow>(
        "SELECT ext_id, manifest_hash, approved_at, approved_by \
         FROM ext_manifest_approvals \
         WHERE ext_id = $1 \
         ORDER BY approved_at DESC",
    )
    .bind(ext_id)
    .fetch_all(pool.sqlx())
    .await
}
