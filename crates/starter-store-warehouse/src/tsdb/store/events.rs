//! `events` typed write path (TimescaleDB / sqlx).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{copy_escape, copy_json, fmt_ts};
use crate::tsdb::client::{WarehouseClient, WarehouseError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventRow {
    pub tenant_id: String,
    pub entity_id: String,
    pub ts: DateTime<Utc>,
    pub kind: String,
    pub payload: String,
    pub tags: serde_json::Value,
}

// v3: unified chokepoint is `starter_cache::DefaultWarehouseWriter`.
// Callers enqueue one `WriteRow { table: "events", ts, dimensions }`
// per row and `commit()`; the writer dedupes.
pub async fn insert_many(
    client: &WarehouseClient,
    rows: &[EventRow],
) -> Result<(), WarehouseError> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut conn = client.pool().acquire().await?;
    let mut copy = conn
        .copy_in_raw(
            "COPY events (tenant_id, entity_id, ts, kind, payload, tags) \
             FROM STDIN WITH (FORMAT text)",
        )
        .await?;
    let mut buf = String::new();
    for r in rows {
        buf.clear();
        buf.push_str(&copy_escape(&r.tenant_id));
        buf.push('\t');
        buf.push_str(&copy_escape(&r.entity_id));
        buf.push('\t');
        buf.push_str(&fmt_ts(r.ts));
        buf.push('\t');
        buf.push_str(&copy_escape(&r.kind));
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
