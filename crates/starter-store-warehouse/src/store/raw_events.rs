//! `raw_events` typed write + read path.
//!
//! W8: every insert through the `clickhouse::Client::insert` API
//! inherits the `async_insert=1` settings baked into `ChClient`,
//! so even one-row writes do not produce one part per row.

use chrono::{DateTime, Utc};
use clickhouse::Row;
use serde::{Deserialize, Serialize};

use crate::client::{ChClient, ChClientError};

/// One row of `raw_events`. `id` is server-side
/// (`generateSnowflakeID()`) — leave it zero on insert; CH fills
/// it. We round-trip the value on read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Row)]
pub struct RawEventRow {
    pub id: u64,
    pub source: String,
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    pub received_at: DateTime<Utc>,
    pub payload: String,
    pub tags: Vec<(String, String)>,
}

/// Insert one or more rows. The server batches under
/// `async_insert=1`; per-row latency is bounded by the
/// `wait_for_async_insert` flush.
pub async fn insert_many(client: &ChClient, rows: &[RawEventRow]) -> Result<(), ChClientError> {
    let mut insert = client.inner().insert("raw_events")?;
    for r in rows {
        insert.write(r).await?;
    }
    insert.end().await?;
    Ok(())
}

/// Read most-recent N rows for a given `source`. Mostly used by
/// tests to demonstrate the async-insert flush bound — for real
/// reads, callers should query `samples` / `events` via the read
/// seam.
pub async fn read_recent(
    client: &ChClient,
    source: &str,
    limit: u64,
) -> Result<Vec<RawEventRow>, ChClientError> {
    let rows = client
        .inner()
        .query(
            "SELECT id, source, received_at, payload, tags \
             FROM raw_events WHERE source = ? \
             ORDER BY received_at DESC LIMIT ?",
        )
        .bind(source)
        .bind(limit)
        .fetch_all::<RawEventRow>()
        .await?;
    Ok(rows)
}
