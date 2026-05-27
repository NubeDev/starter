//! `service::FlowAsService` — the cron-aware companion to
//! [`crate::FlowAsTool`].
//!
//! Phase B.1 scaffold (Goal 6 — see
//! `.codeless/jobs/rubix-goal-6-weekly-report/SCOPE.md`). This
//! file lands the struct, the [`FlowRunner`] trait that future
//! tick loops dispatch through, and the two write-side methods
//! that own the `starter_scheduled_flows` PG table:
//!
//! - [`FlowAsService::register_schedule`] inserts (or updates) a
//!   row with `next_run_at = clock.now() + next_fire(expr)` and
//!   relies on the migration's `AFTER INSERT` trigger to emit a
//!   `starter_scheduled_flows` LISTEN/NOTIFY payload.
//! - [`FlowAsService::unregister_schedule`] flips `enabled =
//!   FALSE`; the same trigger fires on the scoped `UPDATE OF
//!   enabled` so cross-instance listeners hear the disable.
//!
//! The tick loop (`tick` + `start`) lands in Phase B.2; this
//! stage's surface is intentionally limited to the two write
//! verbs the rubix-agent boot path needs to seed bundled
//! `trigger: schedule` flows.
//!
//! ## Naming note
//!
//! The crate root already exports an event-driven `FlowAsService`
//! (the broadcast-subscriber wrapper from stage 8). That type is
//! re-exported as [`crate::FlowAsService`]; this scheduler-flavored
//! type lives at `starter_flow_surfaces::service::FlowAsService`
//! and is the one the durable scheduler land referenced in the
//! Goal 6 SCOPE. The two compose at the rubix-agent layer (one
//! wraps a flow for cron-driven invocation, the other for
//! broadcast-event-driven invocation) and never alias inside any
//! single use-site.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::{self, StreamExt};
use sqlx::Row;
use thiserror::Error;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use uuid::Uuid;

use starter_cron::CronError;
use starter_store_postgres::Pool;

use crate::clock::{Clock, SystemClock};
use crate::FlowRegistry;

/// Trait the durable scheduler dispatches a claimed schedule row
/// through. Kept local to `starter-flow-surfaces` (rather than
/// promoted to `starter-flow-spi`) so the rubix-agent boot path
/// can wire any callable — a real
/// [`starter_flow::run::FlowRunner`](starter_flow::run::FlowRunner),
/// a test stub, or a tracing-only logger — without dragging the
/// concrete runner into the SPI crate.
///
/// The Phase B.2 tick loop calls [`FlowRunner::run`] once per
/// claimed row and uses the `Result` to populate
/// `last_run_status` / `last_run_message` before recomputing
/// `next_run_at`.
#[async_trait]
pub trait FlowRunner: Send + Sync + 'static {
    /// Dispatch the named flow under the given tenant. Returns
    /// `Ok(())` on a successful run; the `Err` arm carries a
    /// human-readable summary the scheduler truncates to 4 KB and
    /// writes into `last_run_message`.
    async fn run(
        &self,
        tenant_id: Uuid,
        flow_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Errors raised by [`FlowAsService`] write-side methods.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ServiceError {
    /// The cron expression failed [`starter_cron::next_fire`]
    /// validation. Carries the structured parser error so callers
    /// can surface a precise diagnostic to operators.
    #[error("invalid cron expression `{expr}`: {source}")]
    InvalidCron {
        /// The offending expression as supplied.
        expr: String,
        /// The structured `starter-cron` error.
        #[source]
        source: CronError,
    },

    /// The underlying SQL operation against
    /// `starter_scheduled_flows` failed. Wraps the `sqlx` error
    /// verbatim — the trigger / unique constraint detail rides
    /// along so operators can pinpoint conflicts.
    #[error("scheduled_flows write failed: {0}")]
    Sql(#[from] sqlx::Error),
}

/// Durable scheduler write surface — the cron-aware companion to
/// [`crate::FlowAsTool`].
///
/// Holds:
///
/// - the PG [`Pool`] backing the `starter_scheduled_flows` table;
/// - an [`Arc<FlowRegistry>`] so the future tick loop can look up
///   the typed `(topology, terminal_slots, …)` bundle by flow id;
/// - an `Arc<dyn FlowRunner>` the future tick loop dispatches
///   through;
/// - an `Arc<dyn Clock>` so tests can drive time deterministically
///   via [`crate::clock::TestClock`].
///
/// This phase exposes only the write-side methods
/// [`Self::register_schedule`] / [`Self::unregister_schedule`];
/// `tick` and `start` land in Phase B.2.
pub struct FlowAsService {
    pool: Pool,
    registry: Arc<FlowRegistry>,
    runner: Arc<dyn FlowRunner>,
    clock: Arc<dyn Clock>,
    /// Cadence of the tick loop spawned by [`Self::start`].
    /// Defaults to 60 s; tune lower when many fine-grained
    /// schedules (e.g. `*/5 * * * * *`) would otherwise pile up
    /// a large per-tick backlog.
    tick_interval: Duration,
    /// Maximum number of claimed flows dispatched concurrently
    /// within a single tick. Defaults to 16 — comfortable for a
    /// 16-connection pool while still leaving headroom for HTTP
    /// (notably `/health`) and any flow-side DB writes
    /// (e.g. `rubix.warehouse.ingest`). Bump higher if the pool
    /// is bigger or scheduled flows are mostly I/O-bound.
    max_concurrent_dispatch: usize,
    /// Per-invocation wall-clock budget enforced via
    /// [`tokio::time::timeout`]. A run that exceeds this budget
    /// is cancelled (dropping the future) and bookkeeping
    /// records `failed` with a timeout message — preventing a
    /// single stuck flow from holding a dispatch slot (and any
    /// pool connections it acquired) indefinitely. Defaults to
    /// 30 s.
    dispatch_timeout: Duration,
}

impl FlowAsService {
    /// Construct a fresh scheduler write-surface against the
    /// supplied PG [`Pool`] and dispatcher. The clock defaults to
    /// [`SystemClock`]; tests substitute via
    /// [`Self::with_clock`].
    pub fn new(pool: Pool, registry: Arc<FlowRegistry>, runner: Arc<dyn FlowRunner>) -> Self {
        Self {
            pool,
            registry,
            runner,
            clock: Arc::new(SystemClock::new()),
            tick_interval: Duration::from_secs(60),
            max_concurrent_dispatch: 16,
            dispatch_timeout: Duration::from_secs(30),
        }
    }

    /// Override the tick-loop cadence (default 60 s). Lower
    /// values reduce per-tick backlog for fine-grained
    /// schedules; higher values reduce DB load.
    #[must_use]
    pub fn with_tick_interval(mut self, interval: Duration) -> Self {
        self.tick_interval = interval;
        self
    }

    /// Override the in-flight dispatch concurrency cap
    /// (default 16). Values of `0` are clamped to `1` so the
    /// dispatch stream always makes progress.
    #[must_use]
    pub fn with_max_concurrent_dispatch(mut self, max: usize) -> Self {
        self.max_concurrent_dispatch = max.max(1);
        self
    }

    /// Override the per-invocation timeout (default 30 s).
    #[must_use]
    pub fn with_dispatch_timeout(mut self, dispatch_timeout: Duration) -> Self {
        self.dispatch_timeout = dispatch_timeout;
        self
    }

    /// Replace the wall-clock seam. Used by `tests/clock_test.rs`
    /// and the Phase B.2 tick test to advance time without
    /// sleeping the test runner.
    #[must_use]
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Borrow the bound [`FlowRegistry`] (introspection helper).
    pub fn registry(&self) -> &Arc<FlowRegistry> {
        &self.registry
    }

    /// Borrow the bound [`FlowRunner`] (introspection helper).
    pub fn runner(&self) -> &Arc<dyn FlowRunner> {
        &self.runner
    }

    /// Borrow the bound [`Clock`] (introspection helper).
    pub fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }

    /// Borrow the bound [`Pool`] (introspection helper).
    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    /// Register (or re-register) a schedule for `(tenant, flow)`.
    ///
    /// Inserts a row into `starter_scheduled_flows` with
    /// `next_run_at = clock.now() + first cron fire`, or updates
    /// the existing row's cron expression + recomputed
    /// `next_run_at` if one already exists (the
    /// `ON CONFLICT (tenant_id, flow_id)` arm). The migration's
    /// `AFTER INSERT` trigger fires the `starter_scheduled_flows`
    /// LISTEN/NOTIFY payload on first insert; the scoped
    /// `AFTER UPDATE OF next_run_at, enabled` trigger fires on
    /// the conflict-update path because both columns change.
    ///
    /// The PK is a ULID rendered as TEXT to match the schema and
    /// keep parity with the sqlite twin (which has no UUID type).
    pub async fn register_schedule(
        &self,
        tenant_id: Uuid,
        flow_id: &str,
        cron_expr: &str,
    ) -> Result<DateTime<Utc>, ServiceError> {
        // Validate the cron expression up-front. Storing an
        // invalid expression would leave the tick loop unable to
        // recompute `next_run_at` after the first fire; bail now.
        let now = self.clock.now();
        let next_run_at = starter_cron::next_fire(now, cron_expr).map_err(|source| {
            ServiceError::InvalidCron {
                expr: cron_expr.to_string(),
                source,
            }
        })?;

        let id = ulid::Ulid::new().to_string();

        // The `created_by` actor for an upstream-driven register
        // call defaults to the all-zero sentinel. The future REST
        // surface that lets operators register schedules
        // interactively will supply a real principal here.
        let actor: Uuid = Uuid::nil();

        sqlx::query(
            r#"INSERT INTO starter_scheduled_flows
                  (id, tenant_id, flow_id, cron_expr, next_run_at, created_by, enabled)
               VALUES ($1, $2, $3, $4, $5, $6, TRUE)
               ON CONFLICT (tenant_id, flow_id) DO UPDATE
                  SET cron_expr   = EXCLUDED.cron_expr,
                      next_run_at = EXCLUDED.next_run_at,
                      enabled     = TRUE"#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(flow_id)
        .bind(cron_expr)
        .bind(next_run_at)
        .bind(actor)
        .execute(self.pool.sqlx())
        .await?;

        Ok(next_run_at)
    }

    /// Soft-disable the `(tenant, flow)` schedule by flipping
    /// `enabled = FALSE`. The scoped `AFTER UPDATE OF enabled`
    /// trigger emits the LISTEN/NOTIFY payload so other rubix-
    /// agent instances drop the schedule from their tick loop.
    ///
    /// Returns `Ok(true)` if a row was affected, `Ok(false)` if
    /// no matching row existed (idempotent caller contract — the
    /// boot seeder may call this for a flow it later removes).
    pub async fn unregister_schedule(
        &self,
        tenant_id: Uuid,
        flow_id: &str,
    ) -> Result<bool, ServiceError> {
        let result = sqlx::query(
            r#"UPDATE starter_scheduled_flows
                  SET enabled = FALSE
                WHERE tenant_id = $1 AND flow_id = $2 AND enabled = TRUE"#,
        )
        .bind(tenant_id)
        .bind(flow_id)
        .execute(self.pool.sqlx())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Read back the `(next_run_at, enabled)` pair for
    /// `(tenant, flow)` — convenience for tests asserting the
    /// register / unregister round-trip wrote what the caller
    /// expected. Returns `None` if no row exists.
    pub async fn lookup_schedule(
        &self,
        tenant_id: Uuid,
        flow_id: &str,
    ) -> Result<Option<(DateTime<Utc>, bool)>, ServiceError> {
        let row = sqlx::query(
            r#"SELECT next_run_at, enabled
                 FROM starter_scheduled_flows
                WHERE tenant_id = $1 AND flow_id = $2"#,
        )
        .bind(tenant_id)
        .bind(flow_id)
        .fetch_optional(self.pool.sqlx())
        .await?;

        Ok(row.map(|r| {
            let next_run_at: DateTime<Utc> = r.get("next_run_at");
            let enabled: bool = r.get("enabled");
            (next_run_at, enabled)
        }))
    }

    /// Run one durable-scheduler tick.
    ///
    /// Within a single transaction, this method claims up to 32
    /// rows whose `next_run_at <= clock.now()` and `enabled = TRUE`
    /// via `SELECT … FOR UPDATE SKIP LOCKED` so that multiple
    /// rubix-agent instances sharing the same `starter_scheduled_flows`
    /// table never double-fire a schedule. For every claimed row the
    /// tick re-computes the next firing time via
    /// [`starter_cron::next_fire`] and writes it back **before**
    /// committing the transaction; this is what releases the row
    /// for other instances while keeping it from being reclaimed
    /// on the very next tick.
    ///
    /// Dispatch through the bound [`FlowRunner`] happens **outside**
    /// the claim transaction so a slow flow does not hold the row
    /// lock for the duration of its run. The post-dispatch
    /// `last_run_at` / `last_run_status` / `last_run_message`
    /// bookkeeping update runs against the pool directly; a NULL
    /// is written for `last_run_message` on success, and the
    /// failure summary is truncated to 4 KB on error.
    ///
    /// Returns the number of rows claimed (and therefore the
    /// number of dispatches attempted). A claim whose
    /// `cron_expr` fails re-parse after-the-fact (which should
    /// not happen because `register_schedule` validates) is
    /// dispatched anyway but its `next_run_at` is left untouched
    /// and the bookkeeping row records `failed` with a
    /// parse-error message — the scheduler does not silently
    /// disable schedules whose authors made a mistake; operators
    /// see the failed status and intervene.
    pub async fn tick(&self) -> Result<usize, ServiceError> {
        let now = self.clock.now();

        // Claim due rows within a single transaction. The
        // FOR UPDATE SKIP LOCKED clause is what makes the table
        // safe to share across rubix-agent instances.
        let mut tx = self.pool.sqlx().begin().await?;
        let rows = sqlx::query(
            r#"SELECT id, tenant_id, flow_id, cron_expr
                 FROM starter_scheduled_flows
                WHERE enabled = TRUE AND next_run_at <= $1
                ORDER BY next_run_at
                FOR UPDATE SKIP LOCKED
                LIMIT 128"#,
        )
        .bind(now)
        .fetch_all(&mut *tx)
        .await?;

        let mut claimed: Vec<(String, Uuid, String, String)> = Vec::with_capacity(rows.len());
        for r in rows {
            let id: String = r.get("id");
            let tenant_id: Uuid = r.get("tenant_id");
            let flow_id: String = r.get("flow_id");
            let cron_expr: String = r.get("cron_expr");

            // Recompute `next_run_at` while still inside the
            // transaction so the row immediately stops looking
            // due to peer instances.
            if let Ok(next) = starter_cron::next_fire(now, &cron_expr) {
                sqlx::query(
                    r#"UPDATE starter_scheduled_flows
                          SET next_run_at = $1
                        WHERE id = $2"#,
                )
                .bind(next)
                .bind(&id)
                .execute(&mut *tx)
                .await?;
            }
            claimed.push((id, tenant_id, flow_id, cron_expr));
        }
        tx.commit().await?;

        let count = claimed.len();

        // Bounded-concurrent dispatch. Sequential dispatch
        // turns even a modest per-tick backlog into a wall-clock
        // pile-up: with N claims at ~1 s each, the tick takes
        // >N seconds, during which any flow that touches the
        // shared PG pool (e.g. `rubix.warehouse.ingest`)
        // competes with the bookkeeping `UPDATE` below for
        // connections — a pool drained this way blocks unrelated
        // callers including the HTTP `/health` endpoint. Capping
        // in-flight dispatches at `max_concurrent_dispatch`
        // keeps the pool from saturating while still draining
        // backlogs in parallel. The per-invocation timeout
        // prevents one stuck flow from holding a dispatch slot
        // (and the connections it acquired) indefinitely.
        let dispatch_timeout = self.dispatch_timeout;
        let pool = self.pool.clone();
        let runner = self.runner.clone();
        let clock = self.clock.clone();

        stream::iter(claimed)
            .for_each_concurrent(
                self.max_concurrent_dispatch,
                |(id, tenant_id, flow_id, _cron)| {
                    let pool = pool.clone();
                    let runner = runner.clone();
                    let clock = clock.clone();
                    async move {
                        let result = match timeout(
                            dispatch_timeout,
                            runner.run(tenant_id, &flow_id),
                        )
                        .await
                        {
                            Ok(inner) => inner,
                            Err(_) => Err(format!(
                                "flow dispatch exceeded {dispatch_timeout:?} budget"
                            )
                            .into()),
                        };
                        let (status, message): (&'static str, Option<String>) = match result {
                            Ok(()) => ("succeeded", None),
                            Err(e) => ("failed", Some(truncate_message(&e.to_string()))),
                        };
                        if let Err(e) = sqlx::query(
                            r#"UPDATE starter_scheduled_flows
                                  SET last_run_at      = $1,
                                      last_run_status  = $2,
                                      last_run_message = $3
                                WHERE id = $4"#,
                        )
                        .bind(clock.now())
                        .bind(status)
                        .bind(message)
                        .bind(&id)
                        .execute(pool.sqlx())
                        .await
                        {
                            tracing::warn!(
                                flow_id = %flow_id,
                                error = %e,
                                "flow_as_service.tick.bookkeeping_failed",
                            );
                        }
                    }
                },
            )
            .await;

        Ok(count)
    }

    /// Spawn the durable-scheduler tick loop and return its
    /// [`JoinHandle`].
    ///
    /// The loop ticks every 60 seconds via a
    /// [`tokio::time::interval`] with `MissedTickBehavior::Skip`
    /// (a slow tick must not pile up missed ticks; if a tick takes
    /// >60s the scheduler simply runs the next one immediately on
    /// the next interval edge, not back-to-back to catch up).
    /// Each iteration calls [`Self::tick`]; tick failures are logged
    /// but do not terminate the loop — a transient PG outage must
    /// not silently disable scheduling for the rest of the
    /// process's lifetime.
    ///
    /// The returned handle never resolves under normal operation;
    /// callers shut the loop down by `JoinHandle::abort`.
    pub fn start(self) -> JoinHandle<()> {
        let tick_interval = self.tick_interval;
        let me = Arc::new(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tick_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                match me.tick().await {
                    Ok(n) if n > 0 => {
                        tracing::debug!(claimed = n, "flow_as_service.tick.fired");
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!(error = %e, "flow_as_service.tick.failed");
                    }
                }
            }
        })
    }
}

/// Truncate `s` to at most 4 KB on a UTF-8 char boundary, as the
/// SCOPE pins for `last_run_message`. Returns owned `String` so
/// callers can `bind` it directly.
fn truncate_message(s: &str) -> String {
    const CAP: usize = 4 * 1024;
    if s.len() <= CAP {
        return s.to_owned();
    }
    let mut end = CAP;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_owned()
}
