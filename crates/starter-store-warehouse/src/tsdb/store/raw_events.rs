//! `raw_events` typed write path (TimescaleDB / sqlx).
//!
//! Bulk ingest goes through `PgPool::copy_in_raw` because COPY
//! is the only bulk-insert primitive Postgres exposes that wins
//! cleanly over `INSERT ... VALUES (..), (..), ...` past a few
//! hundred rows.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{copy_escape, copy_json, fmt_ts};
use crate::tsdb::client::{WarehouseClient, WarehouseError};

/// One row of `raw_events`. `id` is server-side (`BIGSERIAL`) —
/// the writer leaves it at the default by omitting it from the
/// COPY column list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawEventRow {
    pub tenant_id: String,
    pub source: String,
    pub received_at: DateTime<Utc>,
    pub payload: String,
    pub tags: serde_json::Value,
}

/// Bulk-insert rows via `COPY`. Empty input is a no-op.
pub async fn insert_many(
    client: &WarehouseClient,
    rows: &[RawEventRow],
) -> Result<(), WarehouseError> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut conn = client.pool().acquire().await?;
    let mut copy = conn
        .copy_in_raw(
            "COPY raw_events (tenant_id, source, received_at, payload, tags) \
             FROM STDIN WITH (FORMAT text)",
        )
        .await?;
    let mut buf = String::new();
    for r in rows {
        buf.clear();
        buf.push_str(&copy_escape(&r.tenant_id));
        buf.push('\t');
        buf.push_str(&copy_escape(&r.source));
        buf.push('\t');
        buf.push_str(&fmt_ts(r.received_at));
        buf.push('\t');
        buf.push_str(&copy_escape(&r.payload));
        buf.push('\t');
        buf.push_str(&copy_json(&r.tags));
        buf.push('\n');
        copy.send(buf.as_bytes()).await?;
    }
    copy.finish().await?;
    Ok(())
}
