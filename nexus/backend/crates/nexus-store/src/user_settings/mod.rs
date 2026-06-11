//! Per-user freeform settings persistence.
//!
//! A single `jsonb` bag per `(tenant_id, user_id)` for nexus-side UI state the
//! frontend owns (starred dashboards, collapsed groups, …). Unlike the typed
//! `starter-prefs` rows, the shape is opaque to this crate. Every function runs
//! inside a tenant-bound transaction so RLS isolates the rows — see
//! [`crate::tenant_tx`]; the route layer additionally pins `user_id` to the
//! caller's subject so one user can't reach another's row in a shared tenant.
//!
//! Schema: `migrations/nexus/2301_user_settings.sql`.

use serde_json::{json, Value as JsonValue};
use sqlx::{PgPool, Row};
use starter_spi::Error;

use crate::tenant_tx;

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}

/// The caller's settings bag, or an empty object (`{}`) when the user has no row
/// yet. Returning `{}` rather than `None` lets the caller treat "never saved"
/// and "saved empty" alike — there is no meaningful difference for UI state.
pub async fn get(pool: &PgPool, tenant_id: &str, user_id: &str) -> Result<JsonValue, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query("SELECT settings FROM nexus_user_settings WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row
        .map(|r| r.get::<JsonValue, _>("settings"))
        .unwrap_or_else(|| json!({})))
}

/// Replace the caller's settings bag with `settings` (upsert). A full replace,
/// like the tag editor: the frontend reads-modifies-writes the whole bag, so the
/// store never merges. Bumps `updated_at`.
pub async fn set(
    pool: &PgPool,
    tenant_id: &str,
    user_id: &str,
    settings: &JsonValue,
) -> Result<(), Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    sqlx::query(
        "INSERT INTO nexus_user_settings (tenant_id, user_id, settings) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (tenant_id, user_id) \
         DO UPDATE SET settings = EXCLUDED.settings, updated_at = now()",
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(settings)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(())
}
