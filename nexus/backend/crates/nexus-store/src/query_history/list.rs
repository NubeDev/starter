//! Read a user's recent query history, newest first.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::internal;
use super::row::QueryHistoryRow;
use crate::tenant_tx;

/// The `(tenant, user)`'s most recent runs, newest first, capped at `limit`.
/// Starred rows sort first within the same recency so pinned favourites stay
/// at the top of the recall drawer.
pub async fn list_recent(
    pool: &PgPool,
    tenant_id: &str,
    user_id: &str,
    limit: i64,
) -> Result<Vec<QueryHistoryRow>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, user_id, datasource_id, sql, ran_at, elapsed_ms, row_count, error, starred \
         FROM nexus_query_history \
         WHERE user_id = $1 \
         ORDER BY starred DESC, ran_at DESC \
         LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(to_row).collect())
}

/// Map a fetched DB row to the domain record.
fn to_row(r: &sqlx::postgres::PgRow) -> QueryHistoryRow {
    QueryHistoryRow {
        id: r.get::<Uuid, _>("id"),
        user_id: r.get::<String, _>("user_id"),
        datasource_id: r.get::<Option<Uuid>, _>("datasource_id"),
        sql: r.get::<String, _>("sql"),
        ran_at: r.get("ran_at"),
        elapsed_ms: r.get::<Option<i64>, _>("elapsed_ms"),
        row_count: r.get::<Option<i64>, _>("row_count"),
        error: r.get::<Option<String>, _>("error"),
        starred: r.get::<bool, _>("starred"),
    }
}
