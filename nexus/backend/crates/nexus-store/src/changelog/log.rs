//! Tenant-scoped [`ChangeLog`] over `nexus_changes`.
//!
//! Reads run inside a [`crate::tenant_tx`] so RLS filters every query to the
//! pinned tenant — the audit API and the [`starter_undo::UndoService`] both go
//! through this, so cross-tenant reads are impossible by construction, not by a
//! `WHERE tenant_id =` the caller could forget. Pagination is keyset on
//! `(at, id)` so a long audit history pages without OFFSET drift.

use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, QueryBuilder};
use starter_changelog::{ChangeFilter, ChangeLog, ChangePage};
use starter_spi::changelog::{Change, ChangeId, GroupId};
use starter_spi::{Error, Result};

use super::codec::row_to_change;
use crate::tenant_tx;

/// Maximum rows returned for one audit page. The audit UI pages; an unbounded
/// query would let one request scan an entire tenant's history.
const MAX_PAGE: u32 = 200;

/// Postgres-backed, tenant-pinned change log. Cheap to construct per request.
#[derive(Clone)]
pub struct NexusChangeLog {
    pool: PgPool,
    tenant_id: String,
}

impl NexusChangeLog {
    /// Build a log pinned to `tenant_id`.
    pub fn new(pool: PgPool, tenant_id: impl Into<String>) -> Self {
        Self {
            pool,
            tenant_id: tenant_id.into(),
        }
    }
}

#[async_trait]
impl ChangeLog for NexusChangeLog {
    async fn get(&self, id: &ChangeId) -> Result<Option<Change>> {
        let mut tx = tenant_tx::begin(&self.pool, &self.tenant_id).await?;
        let row = sqlx::query("SELECT * FROM nexus_changes WHERE id = $1")
            .bind(&id.0)
            .fetch_optional(&mut *tx)
            .await
            .map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        row.map(|r| row_to_change(&r)).transpose()
    }

    async fn group(&self, id: &GroupId) -> Result<Vec<Change>> {
        let mut tx = tenant_tx::begin(&self.pool, &self.tenant_id).await?;
        let rows = sqlx::query(
            "SELECT * FROM nexus_changes \
             WHERE group_id = $1 \
             ORDER BY at ASC, id ASC",
        )
        .bind(&id.0)
        .fetch_all(&mut *tx)
        .await
        .map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        rows.iter().map(row_to_change).collect()
    }

    async fn list(&self, filter: &ChangeFilter) -> Result<ChangePage> {
        let limit = filter.limit.unwrap_or(MAX_PAGE).clamp(1, MAX_PAGE);

        let mut qb: QueryBuilder<sqlx::Postgres> =
            QueryBuilder::new("SELECT * FROM nexus_changes WHERE 1=1");
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

        let mut tx = tenant_tx::begin(&self.pool, &self.tenant_id).await?;
        let rows = qb.build().fetch_all(&mut *tx).await.map_err(internal)?;
        tx.commit().await.map_err(internal)?;

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
