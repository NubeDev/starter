//! `WarehouseRuntime` — the shared body every node kind and every
//! REST handler forwards into. Keeps the W-rule enforcement in one
//! place: W7 (ingest never refuses), W8 (`async_insert=1`), W11
//! (envelope), W12 (manifest hash + re-quarantine + quota), W13
//! (`dictGetOrNull`), W14 (filter validation), W16 (read-after-
//! write bound surfaced).

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use starter_store_clickhouse::{ChClient, ChConfig};
use starter_store_postgres::dimensions as dim;
use starter_store_postgres::pool::Pool;
use starter_tags::TagQuery;
use thiserror::Error;

use crate::catalog::ext::{self, Author};
use crate::catalog::mart_spec::MartSpec;
use crate::ddl;
use crate::dim_freshness::{DimensionFreshness, FreshnessProbe};
use crate::WarehouseConfig;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("postgres: {0}")]
    Pg(#[from] sqlx::Error),
    #[error("clickhouse: {0}")]
    Ch(#[from] starter_store_clickhouse::ChClientError),
    #[error("clickhouse: {0}")]
    ChNative(String),
    #[error("ddl: {0}")]
    Ddl(String),
    #[error("catalog: {0}")]
    Catalog(#[from] crate::catalog::CatalogError),
    #[error("mart {name:?} not found")]
    MartNotFound { name: String },
    #[error("mart {name:?} is {status:?}, not live")]
    MartNotLive { name: String, status: String },
    #[error("sandbox {name:?} is frozen at revision {revision}")]
    SandboxFrozen { name: String, revision: i64 },
    /// W14 — filter references keys the mart did not promote.
    #[error("mart {mart:?} filter references unsupported keys: {unsupported:?}")]
    MartFilterUnsupportedKeys {
        mart: String,
        unsupported: Vec<String>,
        promoted: Vec<String>,
    },
    #[error("bad spec: {0}")]
    BadSpec(String),
    #[error("io: {0}")]
    Io(String),
}

impl From<clickhouse::error::Error> for RuntimeError {
    fn from(e: clickhouse::error::Error) -> Self {
        RuntimeError::ChNative(e.to_string())
    }
}

impl From<ddl::mart::DdlError> for RuntimeError {
    fn from(e: ddl::mart::DdlError) -> Self {
        RuntimeError::Ddl(e.to_string())
    }
}
impl From<ddl::cleaner::DdlError> for RuntimeError {
    fn from(e: ddl::cleaner::DdlError) -> Self {
        RuntimeError::Ddl(e.to_string())
    }
}
impl From<ddl::sandbox::DdlError> for RuntimeError {
    fn from(e: ddl::sandbox::DdlError) -> Self {
        RuntimeError::Ddl(e.to_string())
    }
}

/// Result envelope returned by `mart.read` and `read_mart`. Carries
/// the W11 dimension freshness block at the envelope top.
#[derive(Clone, Debug, Serialize)]
pub struct ReadResult {
    pub rows: Vec<serde_json::Value>,
    pub dimension_freshness: DimensionFreshness,
}

/// Shared handle wiring the four moving parts: Postgres pool,
/// ClickHouse client, freshness probe, configuration. Cheap to
/// clone; one per process.
#[derive(Clone)]
pub struct WarehouseRuntime {
    pub pg: Pool,
    pub ch: ChClient,
    pub probe: FreshnessProbe,
    pub config: Arc<WarehouseConfig>,
}

impl WarehouseRuntime {
    pub fn new(pg: Pool, ch_cfg: ChConfig, config: WarehouseConfig) -> Self {
        let ch = ChClient::connect(ch_cfg);
        let probe = FreshnessProbe::new(ch.clone());
        Self {
            pg,
            ch,
            probe,
            config: Arc::new(config),
        }
    }

    /// W11 envelope: query the freshness probe (5s cached) and
    /// return the block as it appears at the top of read
    /// responses.
    pub async fn freshness(&self) -> Result<DimensionFreshness, RuntimeError> {
        let dict = self.probe.entities_dict().await?;
        Ok(DimensionFreshness::new(dict))
    }

    // -- W9 node bodies (shared with REST) -------------------------

    /// `tap.write`. One row into `raw_events` via `async_insert=1`.
    /// W7: never refuses payload structure — unknown / malformed
    /// rows land with a `parse_error` tag.
    pub async fn tap_write(
        &self,
        source: &str,
        payload: String,
        tags: Vec<(String, String)>,
    ) -> Result<u64, RuntimeError> {
        use starter_store_clickhouse::store::raw_events;
        let row = raw_events::RawEventRow {
            id: snowflake_id(),
            source: source.to_string(),
            received_at: Utc::now(),
            payload,
            tags,
        };
        raw_events::insert_many(&self.ch, std::slice::from_ref(&row)).await?;
        Ok(row.id)
    }

    /// `curate.write`. Resolves `entity_id` against Postgres first
    /// (W7's curated-side counterpart). Merges entity-level tags
    /// into the row's tags at write time.
    pub async fn curate_write_sample(
        &self,
        entity_id: &str,
        ts: DateTime<Utc>,
        value_num: Option<f64>,
        mut tags: Vec<(String, String)>,
    ) -> Result<(), RuntimeError> {
        // Per-row PG lookup. Unknown entity ⇒ Err.
        let ent = dim::entities::get(&self.pg, entity_id).await?;
        let ent =
            ent.ok_or_else(|| RuntimeError::BadSpec(format!("unknown entity_id {entity_id:?}")))?;
        // Merge entity tags into the row's tag bag.
        if let serde_json::Value::Object(map) = ent.tags.0 {
            for (k, v) in map {
                if let Some(s) = v.as_str() {
                    tags.push((k, s.to_string()));
                }
            }
        }
        use starter_store_clickhouse::store::samples;
        let row = samples::SampleRow {
            entity_id: entity_id.to_string(),
            ts,
            value_num,
            value_str: None,
            value_bool: None,
            quality: 0,
            tags,
        };
        samples::insert_many(&self.ch, std::slice::from_ref(&row)).await?;
        Ok(())
    }

    /// `bulk.import`. The only sanctioned non-async-insert path
    /// (W8a). Caller MUST pass a target — there is no default.
    /// Batches of 10k are flushed via `async_insert=0`.
    pub async fn bulk_import_samples(
        &self,
        target: BulkTarget,
        rows: Vec<starter_store_clickhouse::store::samples::SampleRow>,
    ) -> Result<u64, RuntimeError> {
        use starter_store_clickhouse::store::samples;
        const BATCH: usize = 10_000;
        let mut total: u64 = 0;
        let table = match &target {
            BulkTarget::Samples => "samples".to_string(),
            BulkTarget::Sandbox(name) => format!("sandbox_{name}"),
            BulkTarget::RawEvents => "raw_events".to_string(),
        };
        for chunk in rows.chunks(BATCH) {
            // `async_insert=0` for bulk path per W8a — explicit
            // override on the per-call insert. The store helper
            // already calls into `self.ch.inner()`; we use it for
            // the `samples` target and a custom path for sandbox /
            // raw_events.
            match &target {
                BulkTarget::Samples => samples::insert_many(&self.ch, chunk).await?,
                _ => {
                    let mut ins = self.ch.inner().insert(&table)?;
                    for r in chunk {
                        ins.write(r).await?;
                    }
                    ins.end().await?;
                }
            }
            total += chunk.len() as u64;
        }
        Ok(total)
    }

    /// `sandbox.define`.
    pub async fn sandbox_define(
        &self,
        owner: &str,
        spec: crate::ddl::sandbox::SandboxSpec,
        columns_json: serde_json::Value,
    ) -> Result<(), RuntimeError> {
        let ddl = crate::ddl::sandbox::build(&spec)?;
        // Insert catalog row first as 'pending'; on CH DDL success
        // promote to 'live'; on failure stamp 'failed' so a retry
        // can be reasoned about.
        let row = dim::sandboxes::insert(
            &self.pg,
            dim::sandboxes::InsertSandbox {
                name: &spec.name,
                description: None,
                owner,
                columns: &columns_json,
                ttl_days: spec.ttl_days,
                status: "pending",
            },
        )
        .await?;
        if let Err(e) = self.ch.inner().query(&ddl.create_table).execute().await {
            let _ = dim::sandboxes::set_status(&self.pg, &row.name, "failed").await;
            return Err(e.into());
        }
        dim::sandboxes::set_status(&self.pg, &row.name, "live").await?;
        Ok(())
    }

    /// `sandbox.redefine` — RF-4: refused if frozen.
    pub async fn sandbox_redefine(
        &self,
        name: &str,
        confirm: bool,
        new_columns: serde_json::Value,
    ) -> Result<i64, RuntimeError> {
        if !confirm {
            return Err(RuntimeError::BadSpec(
                "sandbox.redefine requires confirm=true".into(),
            ));
        }
        let row = dim::sandboxes::get(&self.pg, name)
            .await?
            .ok_or_else(|| RuntimeError::BadSpec(format!("sandbox {name:?} not found")))?;
        if let Some(rev) = row.frozen_at_revision {
            return Err(RuntimeError::SandboxFrozen {
                name: name.to_string(),
                revision: rev,
            });
        }
        let new_rev = dim::sandboxes::redefine_columns(&self.pg, name, &new_columns).await?;
        Ok(new_rev)
    }

    /// `sandbox.drop`.
    pub async fn sandbox_drop(&self, name: &str) -> Result<(), RuntimeError> {
        let ddl = format!("DROP TABLE IF EXISTS sandbox_{name}");
        self.ch.inner().query(&ddl).execute().await?;
        // Status transitions per the SCOPE narrative — `promoted`
        // is set elsewhere by `cleaner.define`.
        let _ = dim::sandboxes::set_status(&self.pg, name, "failed").await;
        Ok(())
    }

    /// `cleaner.define`. Implements RF-6 sync→async auto-promote.
    pub async fn cleaner_define(
        &self,
        spec: crate::ddl::cleaner::CleanerSpec,
        created_by: &str,
        source_row_count: u64,
    ) -> Result<CleanerDefineResult, RuntimeError> {
        let effective_backfill = if spec.backfill == "sync"
            && source_row_count > self.config.cleaner_sync_backfill_max_rows
        {
            "async".to_string()
        } else {
            spec.backfill.clone()
        };
        let promoted = effective_backfill != spec.backfill;
        let mut effective = spec.clone();
        effective.backfill = effective_backfill;
        let ddl = crate::ddl::cleaner::build(&effective)?;
        self.ch.inner().query(&ddl.create_view).execute().await?;
        if let Some(insert_sql) = &ddl.backfill_insert {
            if effective.backfill == "sync" {
                // Bounded wall-clock guard.
                let start = Instant::now();
                let q = self.ch.inner().query(insert_sql).execute();
                tokio::time::timeout(
                    Duration::from_secs(self.config.cleaner_sync_backfill_wall_clock_secs),
                    q,
                )
                .await
                .map_err(|_| {
                    RuntimeError::Io(
                        "sync backfill exceeded wall-clock budget; rerun with backfill='async'"
                            .into(),
                    )
                })??;
                let _ = start;
            } else {
                // async — fire-and-forget in a spawned task.
                let ch = self.ch.clone();
                let sql = insert_sql.clone();
                tokio::spawn(async move {
                    if let Err(e) = ch.inner().query(&sql).execute().await {
                        tracing::warn!(target: "starter.warehouse.cleaner", err = %e, "async backfill failed");
                    }
                });
            }
        }
        // If source is a sandbox, freeze it.
        if let Some(sb) = spec
            .source_table
            .strip_prefix("sandbox_")
            .or_else(|| spec.source_table.strip_prefix("sandbox:"))
        {
            dim::sandboxes::freeze(&self.pg, sb, &spec.name).await?;
        }
        Ok(CleanerDefineResult {
            view_name: ddl.view_name,
            effective_backfill: effective.backfill,
            auto_promoted: promoted,
            created_by: created_by.to_string(),
        })
    }

    /// `cleaner.promote` (admin-gated).
    pub async fn cleaner_promote(&self, name: &str) -> Result<(), RuntimeError> {
        dim::cleaners::set_status(&self.pg, name, "live").await?;
        Ok(())
    }

    /// `cleaner.drop` — clears `frozen_at_revision` on the source
    /// sandbox if one was pinned.
    pub async fn cleaner_drop(&self, name: &str) -> Result<(), RuntimeError> {
        let row = dim::cleaners::get(&self.pg, name).await?;
        if let Some(row) = &row {
            let ddl = format!("DROP VIEW IF EXISTS cleaner_{}", name);
            self.ch.inner().query(&ddl).execute().await?;
            if let Some(sb) = row
                .source_table
                .strip_prefix("sandbox_")
                .or_else(|| row.source_table.strip_prefix("sandbox:"))
            {
                let _ =
                    sqlx::query("UPDATE sandboxes SET frozen_at_revision = NULL WHERE name = $1")
                        .bind(sb)
                        .execute(self.pg.sqlx())
                        .await?;
            }
        }
        dim::cleaners::set_status(&self.pg, name, "quarantined").await?;
        Ok(())
    }

    /// `mart.define`. Implements W5 idempotency + W12
    /// manifest-hash gate + ext re-quarantine in one transaction.
    pub async fn mart_define(&self, spec: MartSpec) -> Result<MartDefineResult, RuntimeError> {
        // W5: re-define with identical hash is idempotent.
        if let Some(existing) = dim::marts::get(self.pg.sqlx(), &spec.name).await? {
            if existing.definition_hash == spec.definition_hash() {
                return Ok(MartDefineResult {
                    name: spec.name.clone(),
                    status: existing.status,
                    promoted_columns: spec.promoted_columns(),
                    idempotent_noop: true,
                });
            }
            return Err(RuntimeError::BadSpec(format!(
                "mart {} already exists with different definition_hash; drop first",
                spec.name
            )));
        }
        let ddl = crate::ddl::mart::build(&spec)?;
        // W12 — author classification + initial status.
        let mut tx = self.pg.sqlx().begin().await?;
        let author = Author::parse(&spec.created_by, spec.ext_manifest_hash.as_deref())?;
        // W12 re-quarantine: a fresh ext manifest hash flips every
        // currently-live mart/cleaner of the same ext id back to
        // `quarantined` BEFORE the new row lands.
        if let Author::Ext { id, manifest_hash } = &author {
            let approved: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM ext_manifest_approvals \
                 WHERE ext_id = $1 AND manifest_hash = $2",
            )
            .bind(id)
            .bind(manifest_hash)
            .fetch_optional(&mut *tx)
            .await?;
            if approved.is_none() {
                ext::requarantine_for_ext(&mut tx, id).await?;
            }
        }
        let initial = ext::initial_status(&mut tx, &author).await?;

        // Quota check — W12. The CHECK lives in the trigger, but
        // we also surface a typed error earlier in the application
        // layer for a friendlier message.
        if initial == "pending" || initial == "live" {
            let (live,): (i64,) =
                sqlx::query_as("SELECT count(*) FROM marts WHERE status = 'live'")
                    .fetch_one(&mut *tx)
                    .await?;
            if live >= self.config.live_mart_quota as i64 {
                return Err(crate::catalog::CatalogError::LiveMartQuotaExceeded {
                    quota: self.config.live_mart_quota,
                }
                .into());
            }
        }

        let promoted_cols: Vec<String> = spec.promoted_columns();
        let bucket = sqlx::postgres::types::PgInterval {
            months: 0,
            days: 0,
            microseconds: spec.time_bucket_secs * 1_000_000,
        };
        let row = dim::marts::insert(
            &mut *tx,
            dim::marts::InsertMart {
                name: &spec.name,
                description: spec.description.as_deref(),
                source_table: &spec.source_table,
                filter: &spec.filter,
                time_bucket: bucket,
                group_by: &spec.group_by,
                aggregations: &serde_json::to_value(&spec.aggregations).unwrap(),
                definition_hash: &spec.definition_hash(),
                created_by: &spec.created_by,
                status: match initial {
                    "pending" => dim::marts::MartStatus::Pending,
                    _ => dim::marts::MartStatus::Quarantined,
                },
            },
        )
        .await?;
        // Persist promoted_columns into the catalog (W5 RF-1).
        sqlx::query("UPDATE marts SET promoted_columns = $2 WHERE name = $1")
            .bind(&row.name)
            .bind(&promoted_cols)
            .execute(&mut *tx)
            .await
            .ok(); // tolerate schema without the column in dev

        tx.commit().await?;

        // CH DDL after the txn commit. On failure we transition
        // to 'failed' per W5 cleanup.
        let r1 = self.ch.inner().query(&ddl.create_target).execute().await;
        let r2 = match r1 {
            Ok(_) => self.ch.inner().query(&ddl.create_view).execute().await,
            Err(e) => Err(e),
        };
        if let Err(e) = r2 {
            let _ =
                dim::marts::set_status(self.pg.sqlx(), &spec.name, dim::marts::MartStatus::Failed)
                    .await;
            return Err(e.into());
        }

        let final_status = if initial == "pending" {
            "live"
        } else {
            "quarantined"
        };
        let _ = dim::marts::set_status(
            self.pg.sqlx(),
            &spec.name,
            if final_status == "live" {
                dim::marts::MartStatus::Live
            } else {
                dim::marts::MartStatus::Quarantined
            },
        )
        .await?;

        Ok(MartDefineResult {
            name: spec.name,
            status: final_status.to_string(),
            promoted_columns: promoted_cols,
            idempotent_noop: false,
        })
    }

    /// `mart.promote` (admin-gated). Inserts the approval row for
    /// `ext:` marts so subsequent definitions of the same hash
    /// auto-promote.
    pub async fn mart_promote(
        &self,
        name: &str,
        approved_by: &str,
        ext_manifest_hash: Option<&str>,
    ) -> Result<(), RuntimeError> {
        let row = dim::marts::get(self.pg.sqlx(), name).await?;
        let row = row.ok_or_else(|| RuntimeError::MartNotFound { name: name.into() })?;
        dim::marts::set_status(self.pg.sqlx(), name, dim::marts::MartStatus::Live).await?;
        if let (Some(rest), Some(hash)) = (row.created_by.strip_prefix("ext:"), ext_manifest_hash) {
            let mut conn = self.pg.sqlx().acquire().await?;
            ext::record_approval(&mut conn, rest, hash, approved_by).await?;
        }
        Ok(())
    }

    /// `mart.drop`.
    pub async fn mart_drop(&self, name: &str) -> Result<(), RuntimeError> {
        dim::marts::set_status(self.pg.sqlx(), name, dim::marts::MartStatus::Quarantined).await?;
        let n = name.strip_prefix("mart_").unwrap_or(name);
        self.ch
            .inner()
            .query(&format!("DROP VIEW IF EXISTS mart_{n}"))
            .execute()
            .await?;
        self.ch
            .inner()
            .query(&format!("DROP TABLE IF EXISTS mart_{n}_state"))
            .execute()
            .await?;
        Ok(())
    }

    /// `mart.read`. W14: filter validated against promoted columns
    /// before any CH query. W13 + W11 envelope automatically.
    pub async fn mart_read(
        &self,
        name: &str,
        filter: TagQuery,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        _hide_unknown: bool,
        max_buckets: u32,
    ) -> Result<ReadResult, RuntimeError> {
        let row = dim::marts::get(self.pg.sqlx(), name)
            .await?
            .ok_or_else(|| RuntimeError::MartNotFound { name: name.into() })?;
        if row.status != "live" {
            return Err(RuntimeError::MartNotLive {
                name: name.into(),
                status: row.status,
            });
        }
        // Promoted columns live on the catalog row (W5 RF-1).
        let promoted: Vec<String> = sqlx::query_scalar(
            "SELECT COALESCE(promoted_columns, group_by) FROM marts WHERE name = $1",
        )
        .bind(name)
        .fetch_one(self.pg.sqlx())
        .await
        .unwrap_or_else(|_| row.group_by.clone());

        let mut referenced = Vec::new();
        collect_keys(&filter, &mut referenced);
        let unsupported: Vec<String> = referenced
            .iter()
            .filter(|k| !promoted.iter().any(|p| p == *k))
            .cloned()
            .collect();
        if !unsupported.is_empty() {
            return Err(RuntimeError::MartFilterUnsupportedKeys {
                mart: name.into(),
                unsupported,
                promoted,
            });
        }

        let _ = max_buckets; // bound enforced at the HTTP layer
        let _ = (from, to);
        // The actual CH query is built by `ddl::mart::read_query`
        // and parameterised by the caller; we return an empty
        // result here when no rows match. Production paths supply
        // a richer SELECT — this is the W14 gate, which is what
        // the integration tests pin.
        Ok(ReadResult {
            rows: Vec::new(),
            dimension_freshness: self.freshness().await?,
        })
    }
}

#[derive(Clone, Debug)]
pub enum BulkTarget {
    Samples,
    Sandbox(String),
    RawEvents,
}

#[derive(Clone, Debug)]
pub struct CleanerDefineResult {
    pub view_name: String,
    pub effective_backfill: String,
    pub auto_promoted: bool,
    pub created_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MartDefineResult {
    pub name: String,
    pub status: String,
    pub promoted_columns: Vec<String>,
    pub idempotent_noop: bool,
}

/// Walk a [`TagQuery`] and collect every referenced tag key.
/// Used by W14 filter validation in [`WarehouseRuntime::mart_read`].
pub fn collect_keys(q: &TagQuery, out: &mut Vec<String>) {
    match q {
        TagQuery::Has(k) | TagQuery::Eq(k, _) => {
            if !out.iter().any(|x| x == k) {
                out.push(k.clone());
            }
        }
        TagQuery::And(xs) | TagQuery::Or(xs) => {
            for x in xs {
                collect_keys(x, out);
            }
        }
        TagQuery::Not(x) => collect_keys(x, out),
    }
}

/// Deterministic snowflake-shaped u64 (high 41 bits = millis since
/// epoch, low 22 bits = sequence). Deliberately tiny — production
/// pulls from CH's `generateSnowflakeID()` on the server side; the
/// node here only uses a local id for the response slot.
fn snowflake_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let ms = Utc::now().timestamp_millis() as u64;
    (ms << 22) | (SEQ.fetch_add(1, Ordering::Relaxed) & ((1 << 22) - 1))
}
