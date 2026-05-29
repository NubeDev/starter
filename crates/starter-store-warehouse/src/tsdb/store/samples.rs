//! `samples` typed write + read path (TimescaleDB / sqlx).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use super::{copy_escape, copy_json, fmt_ts, NULL};
use crate::tsdb::client::{WarehouseClient, WarehouseError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SampleRow {
    pub tenant_id: String,
    pub entity_id: String,
    pub ts: DateTime<Utc>,
    pub value_num: Option<f64>,
    pub value_str: Option<String>,
    pub value_bool: Option<bool>,
    pub quality: i16,
    pub tags: serde_json::Value,
}

// TODO(cache-invalidation): one of several scattered warehouse write
// sites. starter-cache's tag invalidation expects a
// `cache_layer.invalidator().invalidate_tags(&["table:samples"])`
// at commit time. There is no unified `WarehouseWriter` chokepoint
// yet (see rubix/docs/sessions/cache-v0-progress.md, "Decisions log");
// until one lands, tag invalidation here is best-effort and callers
// must fire it via the layer's invalidator handle.
pub async fn insert_many(
    client: &WarehouseClient,
    rows: &[SampleRow],
) -> Result<(), WarehouseError> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut conn = client.pool().acquire().await?;
    let mut copy = conn
        .copy_in_raw(
            "COPY samples (tenant_id, entity_id, ts, value_num, value_str, \
             value_bool, quality, tags) FROM STDIN WITH (FORMAT text)",
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
        match r.value_num {
            Some(v) => buf.push_str(&v.to_string()),
            None => buf.push_str(NULL),
        }
        buf.push('\t');
        match &r.value_str {
            Some(v) => buf.push_str(&copy_escape(v)),
            None => buf.push_str(NULL),
        }
        buf.push('\t');
        match r.value_bool {
            Some(true) => buf.push('t'),
            Some(false) => buf.push('f'),
            None => buf.push_str(NULL),
        }
        buf.push('\t');
        buf.push_str(&r.quality.to_string());
        buf.push('\t');
        buf.push_str(&copy_json(&r.tags));
        buf.push('\n');
        copy.send(buf.as_bytes()).await?;
    }
    copy.finish().await?;
    Ok(())
}

/// Count samples for one entity inside a time range — the
/// smallest read shape the smoke tests need.
pub async fn count_for_entity(
    client: &WarehouseClient,
    entity_id: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<i64, WarehouseError> {
    let row = sqlx::query(
        "SELECT count(*)::BIGINT AS c FROM samples \
         WHERE entity_id = $1 AND ts >= $2 AND ts < $3",
    )
    .bind(entity_id)
    .bind(from)
    .bind(to)
    .fetch_one(client.pool())
    .await?;
    Ok(row.try_get::<i64, _>("c")?)
}
