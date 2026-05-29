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

// TODO(cache-invalidation): scattered warehouse write site —
// starter-cache wants `invalidate_tags(&["table:events"])` here on
// commit, but the unified `WarehouseWriter` chokepoint doesn't
// exist yet (see rubix/docs/sessions/cache-v0-progress.md).
// Best-effort until then.
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
