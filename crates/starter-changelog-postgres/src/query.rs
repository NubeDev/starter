//! [`ChangeLog`] impl over `starter_changes` (Postgres).

use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Utc};
use sqlx::QueryBuilder;
use starter_changelog::{ChangeFilter, ChangeLog, ChangePage};
use starter_spi::changelog::{Change, ChangeId, GroupId};
use starter_spi::{Error, Result};
use starter_store_postgres::Pool;

use crate::codec::row_to_change;

/// Postgres-backed [`ChangeLog`].
#[derive(Clone)]
pub struct PgChangeLog {
    pool: Pool,
    max_page: u32,
}

impl PgChangeLog {
    /// Wrap a pool. Default max page is 200.
    pub fn new(pool: Pool) -> Self {
        Self { pool, max_page: 200 }
    }

    /// Override the page-size cap.
    pub fn with_max_page(mut self, max: u32) -> Self {
        self.max_page = max.max(1);
        self
    }
}

#[async_trait]
impl ChangeLog for PgChangeLog {
    async fn get(&self, id: &ChangeId) -> Result<Option<Change>> {
        let row = sqlx::query("SELECT * FROM starter_changes WHERE id = $1")
            .bind(&id.0)
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(internal)?;
        row.map(|r| row_to_change(&r)).transpose()
    }

    async fn group(&self, id: &GroupId) -> Result<Vec<Change>> {
        let rows = sqlx::query(
            "SELECT * FROM starter_changes \
             WHERE group_id = $1 \
             ORDER BY at ASC, id ASC",
        )
        .bind(&id.0)
        .fetch_all(self.pool.sqlx())
        .await
        .map_err(internal)?;
        rows.iter().map(row_to_change).collect()
    }

    async fn list(&self, filter: &ChangeFilter) -> Result<ChangePage> {
        let limit = filter
            .limit
            .unwrap_or(self.max_page)
            .min(self.max_page)
            .max(1);

        let mut qb: QueryBuilder<sqlx::Postgres> =
            QueryBuilder::new("SELECT * FROM starter_changes WHERE 1=1");

        if let Some(v) = &filter.actor_kind {
            qb.push(" AND actor_kind = ").push_bind(v.clone());
        }
        if let Some(v) = &filter.actor_id {
            qb.push(" AND actor_id = ").push_bind(v.clone());
        }
        if let Some(v) = &filter.actor_model {
            qb.push(" AND actor_model = ").push_bind(v.clone());
        }
        if let Some(v) = &filter.resource_kind {
            qb.push(" AND resource_kind = ").push_bind(v.clone());
        }
        if let Some(v) = &filter.resource_id {
            qb.push(" AND resource_id = ").push_bind(v.clone());
        }
        if let Some(v) = &filter.group_id {
            qb.push(" AND group_id = ").push_bind(v.0.clone());
        }
        if let Some(v) = &filter.since {
            qb.push(" AND at >= ").push_bind(*v);
        }
        if let Some(v) = &filter.until {
            qb.push(" AND at < ").push_bind(*v);
        }
        if let Some(cur) = &filter.cursor {
            let (cur_at, cur_id) = decode_cursor(cur)?;
            qb.push(" AND (at, id) < (")
                .push_bind(cur_at)
                .push(", ")
                .push_bind(cur_id)
                .push(")");
        }

        qb.push(" ORDER BY at DESC, id DESC LIMIT ")
            .push_bind((limit as i64) + 1);

        let rows = qb
            .build()
            .fetch_all(self.pool.sqlx())
            .await
            .map_err(internal)?;

        let mut items: Vec<Change> = rows.iter().map(row_to_change).collect::<Result<_>>()?;
        let next_cursor = if items.len() > limit as usize {
            let extra = items.pop().expect("len > limit");
            let last = items.last().unwrap_or(&extra);
            Some(encode_cursor(last.at, &last.id.0))
        } else {
            None
        };

        Ok(ChangePage { items, next_cursor })
    }
}

fn encode_cursor(at: DateTime<Utc>, id: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(format!("{}|{id}", at.to_rfc3339()).as_bytes())
}

fn decode_cursor(s: &str) -> Result<(DateTime<Utc>, String)> {
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s.as_bytes())
        .map_err(|e| Error::Invalid {
            message: format!("invalid cursor: {e}"),
        })?;
    let text = std::str::from_utf8(&raw).map_err(|e| Error::Invalid {
        message: format!("invalid cursor: {e}"),
    })?;
    let (at, id) = text.split_once('|').ok_or_else(|| Error::Invalid {
        message: "invalid cursor: missing separator".into(),
    })?;
    let at: DateTime<Utc> = at.parse().map_err(|e: chrono::ParseError| Error::Invalid {
        message: format!("invalid cursor: {e}"),
    })?;
    Ok((at, id.to_owned()))
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
