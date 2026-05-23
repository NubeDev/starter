//! `documents` typed write + read path. `id` is caller-supplied
//! (typically a content-addressed digest from the upload pipeline).
//! W8 discipline.

use chrono::{DateTime, Utc};
use clickhouse::Row;
use serde::{Deserialize, Serialize};

use crate::client::{ChClient, ChClientError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Row)]
pub struct DocumentRow {
    pub id: String,
    pub entity_id: String,
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    pub ts: DateTime<Utc>,
    pub blob_ref: String,
    pub mime: String,
    pub tags: Vec<(String, String)>,
}

pub async fn insert_many(client: &ChClient, rows: &[DocumentRow]) -> Result<(), ChClientError> {
    let mut insert = client.inner().insert("documents")?;
    for r in rows {
        insert.write(r).await?;
    }
    insert.end().await?;
    Ok(())
}

pub async fn get(client: &ChClient, id: &str) -> Result<Option<DocumentRow>, ChClientError> {
    let mut rows = client
        .inner()
        .query(
            "SELECT id, entity_id, ts, blob_ref, mime, tags \
             FROM documents WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_all::<DocumentRow>()
        .await?;
    Ok(rows.pop())
}
