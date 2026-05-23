//! `events` typed write + read path. W8 discipline.

use chrono::{DateTime, Utc};
use clickhouse::Row;
use serde::{Deserialize, Serialize};

use crate::client::{ChClient, ChClientError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Row)]
pub struct EventRow {
    pub id: u64,
    pub entity_id: String,
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    pub ts: DateTime<Utc>,
    pub kind: String,
    pub payload: String,
    pub tags: Vec<(String, String)>,
}

pub async fn insert_many(client: &ChClient, rows: &[EventRow]) -> Result<(), ChClientError> {
    let mut insert = client.inner().insert("events")?;
    for r in rows {
        insert.write(r).await?;
    }
    insert.end().await?;
    Ok(())
}

pub async fn read_for_entity_kind(
    client: &ChClient,
    kind: &str,
    entity_id: &str,
    limit: u64,
) -> Result<Vec<EventRow>, ChClientError> {
    let rows = client
        .inner()
        .query(
            "SELECT id, entity_id, ts, kind, payload, tags \
             FROM events WHERE kind = ? AND entity_id = ? \
             ORDER BY ts DESC LIMIT ?",
        )
        .bind(kind)
        .bind(entity_id)
        .bind(limit)
        .fetch_all::<EventRow>()
        .await?;
    Ok(rows)
}
