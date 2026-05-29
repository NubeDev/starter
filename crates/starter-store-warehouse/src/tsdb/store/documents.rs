//! `documents` typed write path (TimescaleDB / sqlx).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{copy_escape, copy_json, fmt_ts};
use crate::tsdb::client::{WarehouseClient, WarehouseError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentRow {
    pub id: String,
    pub tenant_id: String,
    pub entity_id: String,
    pub ts: DateTime<Utc>,
    pub blob_ref: String,
    pub mime: String,
    pub tags: serde_json::Value,
}

// v3: unified chokepoint is `starter_cache::DefaultWarehouseWriter`.
// Callers enqueue one `WriteRow { table: "documents", ts,
// dimensions }` per row and `commit()`; the writer dedupes.
pub async fn insert_many(
    client: &WarehouseClient,
    rows: &[DocumentRow],
) -> Result<(), WarehouseError> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut conn = client.pool().acquire().await?;
    let mut copy = conn
        .copy_in_raw(
            "COPY documents (id, tenant_id, entity_id, ts, blob_ref, mime, tags) \
             FROM STDIN WITH (FORMAT text)",
        )
        .await?;
    let mut buf = String::new();
    for r in rows {
        buf.clear();
        buf.push_str(&copy_escape(&r.id));
        buf.push('\t');
        buf.push_str(&copy_escape(&r.tenant_id));
        buf.push('\t');
        buf.push_str(&copy_escape(&r.entity_id));
        buf.push('\t');
        buf.push_str(&fmt_ts(r.ts));
        buf.push('\t');
        buf.push_str(&copy_escape(&r.blob_ref));
        buf.push('\t');
        buf.push_str(&copy_escape(&r.mime));
        buf.push('\t');
        buf.push_str(&copy_json(&r.tags));
        buf.push('\n');
        copy.send(buf.as_bytes()).await?;
    }
    copy.finish().await?;
    Ok(())
}
