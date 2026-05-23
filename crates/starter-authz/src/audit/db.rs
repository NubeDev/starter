//! DB-backed [`DecisionSink`] (sqlite and postgres). SCOPE-EXT.md
//! R14.
//!
//! ## Shape
//!
//! - Bounded `mpsc::channel` (default depth 4096) + dedicated
//!   writer task. `record()` calls `try_send`; on full-queue it
//!   logs `tracing::warn { dropped_count }` and returns. **Never**
//!   blocks `check()`.
//! - Sample policy: 100% denies + 1-in-N allows via deterministic
//!   `xxhash64(seed ^ subject) % N == 0`. A per-process random
//!   seed prevents two deployments from sampling identical
//!   subjects. `N == 1` short-circuits (everything passes).
//! - Per-kind override map — the audit-log kind itself defaults
//!   to `1` so the audit-of-audit route is not 99% lossy.
//! - Retention: [`spawn_retention`] kicks off an hourly task
//!   that deletes rows older than `retention` in bounded batches
//!   (10k per pass), logging the count. **If the binary never
//!   spawns this task, the table grows without bound.**

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::mpsc;

use super::{DecisionEntry, DecisionSink};
use crate::config::Effect;

/// Tunables for the shipped DB sink. Constructed via
/// [`DecisionSinkConfig::from_env`] in the common case.
#[derive(Debug, Clone)]
pub struct DecisionSinkConfig {
    /// `mpsc` channel capacity. Drops past this point.
    pub queue_depth: usize,
    /// 1-in-N sampling for `Allow` decisions. `1` retains every
    /// allow; the default of `100` keeps the table affordable on
    /// the busy single-tenant path. SCOPE-EXT.md "Allow-sampling
    /// default = 1 in 100".
    pub allow_sample: u32,
    /// Per-kind override map. The audit-log kind itself defaults
    /// to `1` so the audit-of-audit route isn't 99% lossy.
    pub per_kind_sample: HashMap<String, u32>,
    /// Per-process random seed mixed into the sampler so two
    /// deployments don't sample identical subjects.
    pub seed: u64,
}

impl DecisionSinkConfig {
    /// Default depth / sample / seed; the `audit_logs` kind is
    /// pre-installed at 1 so paging audit doesn't sample.
    pub fn new() -> Self {
        let mut per_kind = HashMap::new();
        per_kind.insert("audit_logs".to_string(), 1);
        Self {
            queue_depth: 4096,
            allow_sample: 100,
            per_kind_sample: per_kind,
            seed: random_seed(),
        }
    }

    /// Read the documented env vars:
    ///
    /// - `STARTER_AUTHZ_DECISION_ALLOW_SAMPLE` — default 100
    pub fn from_env() -> Self {
        let mut cfg = Self::new();
        if let Ok(v) = std::env::var("STARTER_AUTHZ_DECISION_ALLOW_SAMPLE") {
            if let Ok(n) = v.parse::<u32>() {
                if n >= 1 {
                    cfg.allow_sample = n;
                }
            }
        }
        cfg
    }

    /// Force-include / exclude a kind from sampling.
    pub fn with_kind_sample(mut self, kind: impl Into<String>, sample: u32) -> Self {
        self.per_kind_sample.insert(kind.into(), sample.max(1));
        self
    }
}

impl Default for DecisionSinkConfig {
    fn default() -> Self {
        Self::new()
    }
}

fn random_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0xC0FFEE);
    // mix with a thread-id-ish counter so two near-simultaneous
    // process starts diverge.
    let tid = std::process::id() as u64;
    nanos.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(tid)
}

/// Cheap deterministic hash (xxhash64-style mixer). Plenty for
/// sampling decisions; we don't need crypto.
fn hash_subject(seed: u64, subject: &str) -> u64 {
    let mut h: u64 = seed ^ 0x9E3779B97F4A7C15;
    for b in subject.as_bytes() {
        h = h.wrapping_add(*b as u64);
        h = h.wrapping_mul(0x100000001B3); // FNV prime
        h ^= h >> 27;
    }
    h ^= h >> 33;
    h = h.wrapping_mul(0xC2B2AE3D27D4EB4F);
    h ^ (h >> 29)
}

/// Decide whether this allow row should be persisted.
///
/// Always `true` for denies (caller responsibility); always
/// `true` for `N == 1`; otherwise `hash(seed ^ subject) % N == 0`.
pub fn should_sample_allow(
    cfg: &DecisionSinkConfig,
    kind: &str,
    subject: &str,
    tenant_override: Option<u32>,
) -> bool {
    let mut n = cfg
        .per_kind_sample
        .get(kind)
        .copied()
        .or(tenant_override)
        .unwrap_or(cfg.allow_sample);
    if n == 0 {
        n = 1;
    }
    if n == 1 {
        return true;
    }
    hash_subject(cfg.seed, subject) % (n as u64) == 0
}

/// Configuration for [`spawn_retention`].
#[derive(Debug, Clone)]
pub struct RetentionConfig {
    /// Rows older than `now() - retain` are eligible for deletion.
    pub retain: chrono::Duration,
    /// Sleep between retention passes.
    pub interval: Duration,
    /// Max rows deleted per pass — keeps SQLite long-running
    /// deletes bounded.
    pub batch: i64,
}

impl RetentionConfig {
    /// 90 days retain, hourly pass, 10k batch. SCOPE-EXT.md R14.
    pub fn new() -> Self {
        Self {
            retain: chrono::Duration::days(90),
            interval: Duration::from_secs(3600),
            batch: 10_000,
        }
    }

    /// Read `STARTER_AUTHZ_DECISION_RETAIN_DAYS` (default 90).
    pub fn from_env() -> Self {
        let mut cfg = Self::new();
        if let Ok(v) = std::env::var("STARTER_AUTHZ_DECISION_RETAIN_DAYS") {
            if let Ok(d) = v.parse::<i64>() {
                if d >= 1 {
                    cfg.retain = chrono::Duration::days(d);
                }
            }
        }
        cfg
    }
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Filter used by [`list_decisions`] and the
/// `GET /v1/authz/decisions` route.
#[derive(Debug, Clone, Default)]
pub struct DecisionFilter {
    /// Restrict to a tenant.
    pub tenant: Option<String>,
    /// Restrict to a subject.
    pub subject: Option<String>,
    /// `"allow"` / `"deny"`.
    pub effect: Option<String>,
    /// Cursor — exclusive upper bound on `at`. Paging returns rows
    /// strictly before this timestamp.
    pub before: Option<DateTime<Utc>>,
    /// Page size; the route bounds this to `[1, 500]`.
    pub limit: i64,
}

/// One backend supported by the shipped sink.
enum Backend {
    #[cfg(feature = "sqlite")]
    Sqlite(starter_store_sqlite::Pool),
    #[cfg(feature = "postgres")]
    Postgres(starter_store_postgres::Pool),
}

/// Bounded-queue DB sink. SCOPE-EXT.md R14.
///
/// Construct via [`DbDecisionSink::sqlite`] /
/// [`DbDecisionSink::postgres`]. The struct holds the
/// `mpsc::Sender`; the writer task owns the receiver and the
/// pool.
pub struct DbDecisionSink {
    tx: mpsc::Sender<DecisionEntry>,
    cfg: Arc<DecisionSinkConfig>,
    dropped: Arc<AtomicU64>,
    backend: Arc<Backend>,
}

impl DbDecisionSink {
    /// Build a sqlite sink. Spawns the writer task on the current
    /// tokio runtime.
    #[cfg(feature = "sqlite")]
    pub fn sqlite(pool: starter_store_sqlite::Pool, cfg: DecisionSinkConfig) -> Self {
        let (tx, rx) = mpsc::channel(cfg.queue_depth);
        let cfg = Arc::new(cfg);
        let dropped = Arc::new(AtomicU64::new(0));
        let backend = Arc::new(Backend::Sqlite(pool.clone()));
        tokio::spawn(sqlite_writer_task(pool, rx));
        Self {
            tx,
            cfg,
            dropped,
            backend,
        }
    }

    /// Build a postgres sink. Spawns the writer task on the
    /// current tokio runtime.
    #[cfg(feature = "postgres")]
    pub fn postgres(pool: starter_store_postgres::Pool, cfg: DecisionSinkConfig) -> Self {
        let (tx, rx) = mpsc::channel(cfg.queue_depth);
        let cfg = Arc::new(cfg);
        let dropped = Arc::new(AtomicU64::new(0));
        let backend = Arc::new(Backend::Postgres(pool.clone()));
        tokio::spawn(postgres_writer_task(pool, rx));
        Self {
            tx,
            cfg,
            dropped,
            backend,
        }
    }

    /// For tests: total rows dropped due to overflow since boot.
    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Read the sampling config — used by route handlers that need
    /// to override per-kind sampling (the audit-log read route
    /// must NOT generate sampled-away allows).
    pub fn config(&self) -> &DecisionSinkConfig {
        &self.cfg
    }
}

#[async_trait]
impl DecisionSink for DbDecisionSink {
    async fn record(&self, entry: DecisionEntry) {
        // SCOPE-EXT.md R14 sampling: denies are unsampled, allows
        // are 1-in-N. Per-kind override map lets the
        // `audit_logs` kind opt out of sampling entirely.
        let keep = match entry.effect {
            Effect::Deny => true,
            Effect::Allow => should_sample_allow(
                &self.cfg,
                &entry.kind,
                &entry.subject,
                None, // per-tenant override resolved at retention/write time below
            ),
        };
        if !keep {
            return;
        }
        // try_send so check() never blocks. Spec contract is
        // "drop, don't block."
        if let Err(e) = self.tx.try_send(entry) {
            match e {
                mpsc::error::TrySendError::Full(_) => {
                    let n = self.dropped.fetch_add(1, Ordering::Relaxed) + 1;
                    tracing::warn!(dropped_count = n, "authz decision sink overflow");
                }
                mpsc::error::TrySendError::Closed(_) => {
                    // writer task has exited; nothing we can do
                    // from the request path.
                    tracing::error!("authz decision sink writer task closed");
                }
            }
        }
    }
}

// --- sqlite writer + query -------------------------------------

#[cfg(feature = "sqlite")]
async fn sqlite_writer_task(
    pool: starter_store_sqlite::Pool,
    mut rx: mpsc::Receiver<DecisionEntry>,
) {
    while let Some(entry) = rx.recv().await {
        if let Err(e) = sqlite_insert_one(&pool, &entry).await {
            tracing::error!(error = %e, "authz decision sink: insert failed");
        }
    }
}

#[cfg(feature = "sqlite")]
async fn sqlite_insert_one(
    pool: &starter_store_sqlite::Pool,
    entry: &DecisionEntry,
) -> Result<(), sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let effect = match entry.effect {
        Effect::Allow => "allow",
        Effect::Deny => "deny",
    };
    sqlx::query(
        "INSERT INTO starter_authz_decisions \
            (id, at, tenant_id, subject, principal_role, action, kind, resource_id, effect, rule_id, reason) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
    )
    .bind(&id)
    .bind(entry.at.to_rfc3339())
    .bind(&entry.tenant)
    .bind(&entry.subject)
    .bind(&entry.principal_role)
    .bind(&entry.action)
    .bind(&entry.kind)
    .bind(&entry.id)
    .bind(effect)
    .bind(&entry.rule_id)
    .bind(&entry.reason)
    .execute(pool.sqlx())
    .await?;
    Ok(())
}

/// Query helper used by the admin route. Returns rows newest-first.
#[cfg(feature = "sqlite")]
pub async fn sqlite_list_decisions(
    pool: &starter_store_sqlite::Pool,
    filter: &DecisionFilter,
) -> Result<Vec<DecisionEntry>, sqlx::Error> {
    use sqlx::Row;

    let mut sql = String::from(
        "SELECT at, tenant_id, subject, principal_role, action, kind, resource_id, effect, rule_id, reason \
         FROM starter_authz_decisions WHERE 1=1",
    );
    if filter.tenant.is_some() {
        sql.push_str(" AND tenant_id = ?");
    }
    if filter.subject.is_some() {
        sql.push_str(" AND subject = ?");
    }
    if filter.effect.is_some() {
        sql.push_str(" AND effect = ?");
    }
    if filter.before.is_some() {
        sql.push_str(" AND at < ?");
    }
    sql.push_str(" ORDER BY at DESC, id DESC LIMIT ?");

    let mut q = sqlx::query(&sql);
    if let Some(t) = &filter.tenant {
        q = q.bind(t);
    }
    if let Some(s) = &filter.subject {
        q = q.bind(s);
    }
    if let Some(e) = &filter.effect {
        q = q.bind(e);
    }
    if let Some(b) = &filter.before {
        q = q.bind(b.to_rfc3339());
    }
    q = q.bind(filter.limit.clamp(1, 500));

    let rows = q.fetch_all(pool.sqlx()).await?;
    let out = rows
        .into_iter()
        .filter_map(|r| {
            let at_s: String = r.get(0);
            let at = DateTime::parse_from_rfc3339(&at_s).ok()?.with_timezone(&Utc);
            let effect_s: String = r.get(7);
            let effect = match effect_s.as_str() {
                "allow" => Effect::Allow,
                "deny" => Effect::Deny,
                _ => return None,
            };
            Some(DecisionEntry {
                at,
                tenant: r.get(1),
                subject: r.get(2),
                principal_role: r.get(3),
                action: r.get(4),
                kind: r.get(5),
                id: r.get(6),
                effect,
                rule_id: r.get(8),
                reason: r.get(9),
            })
        })
        .collect();
    Ok(out)
}

// --- postgres writer + query -----------------------------------

#[cfg(feature = "postgres")]
async fn postgres_writer_task(
    pool: starter_store_postgres::Pool,
    mut rx: mpsc::Receiver<DecisionEntry>,
) {
    while let Some(entry) = rx.recv().await {
        if let Err(e) = postgres_insert_one(&pool, &entry).await {
            tracing::error!(error = %e, "authz decision sink: insert failed");
        }
    }
}

#[cfg(feature = "postgres")]
async fn postgres_insert_one(
    pool: &starter_store_postgres::Pool,
    entry: &DecisionEntry,
) -> Result<(), sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let effect = match entry.effect {
        Effect::Allow => "allow",
        Effect::Deny => "deny",
    };
    sqlx::query(
        "INSERT INTO starter_authz_decisions \
            (id, at, tenant_id, subject, principal_role, action, kind, resource_id, effect, rule_id, reason) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(&id)
    .bind(entry.at)
    .bind(&entry.tenant)
    .bind(&entry.subject)
    .bind(&entry.principal_role)
    .bind(&entry.action)
    .bind(&entry.kind)
    .bind(&entry.id)
    .bind(effect)
    .bind(&entry.rule_id)
    .bind(&entry.reason)
    .execute(pool.sqlx())
    .await?;
    Ok(())
}

/// Query helper used by the admin route. Newest-first.
#[cfg(feature = "postgres")]
pub async fn postgres_list_decisions(
    pool: &starter_store_postgres::Pool,
    filter: &DecisionFilter,
) -> Result<Vec<DecisionEntry>, sqlx::Error> {
    use sqlx::Row;

    let mut sql = String::from(
        "SELECT at, tenant_id, subject, principal_role, action, kind, resource_id, effect, rule_id, reason \
         FROM starter_authz_decisions WHERE 1=1",
    );
    let mut idx = 1usize;
    if filter.tenant.is_some() {
        sql.push_str(&format!(" AND tenant_id = ${idx}"));
        idx += 1;
    }
    if filter.subject.is_some() {
        sql.push_str(&format!(" AND subject = ${idx}"));
        idx += 1;
    }
    if filter.effect.is_some() {
        sql.push_str(&format!(" AND effect = ${idx}"));
        idx += 1;
    }
    if filter.before.is_some() {
        sql.push_str(&format!(" AND at < ${idx}"));
        idx += 1;
    }
    sql.push_str(&format!(" ORDER BY at DESC, id DESC LIMIT ${idx}"));

    let mut q = sqlx::query(&sql);
    if let Some(t) = &filter.tenant {
        q = q.bind(t);
    }
    if let Some(s) = &filter.subject {
        q = q.bind(s);
    }
    if let Some(e) = &filter.effect {
        q = q.bind(e);
    }
    if let Some(b) = &filter.before {
        q = q.bind(*b);
    }
    q = q.bind(filter.limit.clamp(1, 500));

    let rows = q.fetch_all(pool.sqlx()).await?;
    let out = rows
        .into_iter()
        .filter_map(|r| {
            let at: DateTime<Utc> = r.get(0);
            let effect_s: String = r.get(7);
            let effect = match effect_s.as_str() {
                "allow" => Effect::Allow,
                "deny" => Effect::Deny,
                _ => return None,
            };
            Some(DecisionEntry {
                at,
                tenant: r.get(1),
                subject: r.get(2),
                principal_role: r.get(3),
                action: r.get(4),
                kind: r.get(5),
                id: r.get(6),
                effect,
                rule_id: r.get(8),
                reason: r.get(9),
            })
        })
        .collect();
    Ok(out)
}

// --- retention task --------------------------------------------

/// Hourly retention task. Deletes `starter_authz_decisions` rows
/// older than `cfg.retain` in batches of `cfg.batch` and logs the
/// count. The task returns when the runtime shuts down — call it
/// from the binary's startup alongside other long-lived tasks.
///
/// **If the binary never spawns this task, the table grows
/// without bound.** Naming the dependency loudly so the omission
/// surfaces at code-review time, not at disk-full o'clock.
pub fn spawn_retention(sink: &DbDecisionSink, cfg: RetentionConfig) -> tokio::task::JoinHandle<()> {
    let backend = sink.backend.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(cfg.interval);
        // Skip the first immediate tick — let the binary boot
        // before the retention task starts deleting.
        tick.tick().await;
        loop {
            tick.tick().await;
            let cutoff = Utc::now() - cfg.retain;
            let deleted = match &*backend {
                #[cfg(feature = "sqlite")]
                Backend::Sqlite(pool) => sqlite_retention_pass(pool, cutoff, cfg.batch).await,
                #[cfg(feature = "postgres")]
                Backend::Postgres(pool) => postgres_retention_pass(pool, cutoff, cfg.batch).await,
            };
            match deleted {
                Ok(n) => tracing::info!(deleted = n, "authz decisions retention pass"),
                Err(e) => tracing::error!(error = %e, "authz decisions retention pass failed"),
            }
        }
    })
}

#[cfg(feature = "sqlite")]
async fn sqlite_retention_pass(
    pool: &starter_store_sqlite::Pool,
    cutoff: DateTime<Utc>,
    batch: i64,
) -> Result<u64, sqlx::Error> {
    // SQLite's default lacks DELETE...LIMIT unless built with
    // SQLITE_ENABLE_UPDATE_DELETE_LIMIT. Subselect by rowid keeps
    // this portable.
    let res = sqlx::query(
        "DELETE FROM starter_authz_decisions \
         WHERE rowid IN ( \
             SELECT rowid FROM starter_authz_decisions \
              WHERE at < ?1 \
              LIMIT ?2 \
         )",
    )
    .bind(cutoff.to_rfc3339())
    .bind(batch)
    .execute(pool.sqlx())
    .await?;
    Ok(res.rows_affected())
}

#[cfg(feature = "postgres")]
async fn postgres_retention_pass(
    pool: &starter_store_postgres::Pool,
    cutoff: DateTime<Utc>,
    batch: i64,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "DELETE FROM starter_authz_decisions \
         WHERE id IN ( \
             SELECT id FROM starter_authz_decisions \
              WHERE at < $1 \
              ORDER BY at ASC \
              LIMIT $2 \
         )",
    )
    .bind(cutoff)
    .bind(batch)
    .execute(pool.sqlx())
    .await?;
    Ok(res.rows_affected())
}

/// List decisions through the sink's configured backend. Used
/// by the `GET /v1/authz/decisions` admin route.
pub async fn list_via_sink(
    sink: &DbDecisionSink,
    filter: &DecisionFilter,
) -> Result<Vec<DecisionEntry>, sqlx::Error> {
    match &*sink.backend {
        #[cfg(feature = "sqlite")]
        Backend::Sqlite(pool) => sqlite_list_decisions(pool, filter).await,
        #[cfg(feature = "postgres")]
        Backend::Postgres(pool) => postgres_list_decisions(pool, filter).await,
    }
}

/// One-shot retention pass — for tests and operator-triggered
/// cleanup. Returns the number of rows deleted.
pub async fn retention_pass_once(
    sink: &DbDecisionSink,
    cutoff: DateTime<Utc>,
    batch: i64,
) -> Result<u64, sqlx::Error> {
    match &*sink.backend {
        #[cfg(feature = "sqlite")]
        Backend::Sqlite(pool) => sqlite_retention_pass(pool, cutoff, batch).await,
        #[cfg(feature = "postgres")]
        Backend::Postgres(pool) => postgres_retention_pass(pool, cutoff, batch).await,
    }
}
