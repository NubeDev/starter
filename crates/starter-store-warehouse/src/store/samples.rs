//! `samples` typed write + read path. W8 discipline.

use chrono::{DateTime, Utc};
use clickhouse::Row;
use serde::{Deserialize, Serialize};

use crate::client::{ChClient, ChClientError};

/// One row of `samples`. The three value columns are mutually
/// exclusive at the application level — exactly one is `Some`
/// per row by convention — but the schema permits multiple for
/// forward compatibility (e.g. a bool measurement that also
/// carries a string label).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Row)]
pub struct SampleRow {
    pub entity_id: String,
    #[serde(with = "clickhouse::serde::chrono::datetime64::millis")]
    pub ts: DateTime<Utc>,
    pub value_num: Option<f64>,
    pub value_str: Option<String>,
    pub value_bool: Option<u8>,
    pub quality: u8,
    pub tags: Vec<(String, String)>,
}

pub async fn insert_many(client: &ChClient, rows: &[SampleRow]) -> Result<(), ChClientError> {
    let mut insert = client.inner().insert("samples")?;
    for r in rows {
        insert.write(r).await?;
    }
    insert.end().await?;
    Ok(())
}

/// Range read for a single entity. Used by the
/// read-after-write assertion in the integration suite
/// (W16 ≤ 1.5 s claim).
pub async fn read_for_entity(
    client: &ChClient,
    entity_id: &str,
    limit: u64,
) -> Result<Vec<SampleRow>, ChClientError> {
    let rows = client
        .inner()
        .query(
            "SELECT entity_id, ts, value_num, value_str, value_bool, quality, tags \
             FROM samples WHERE entity_id = ? \
             ORDER BY ts DESC LIMIT ?",
        )
        .bind(entity_id)
        .bind(limit)
        .fetch_all::<SampleRow>()
        .await?;
    Ok(rows)
}
