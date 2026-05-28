//! `LISTEN/NOTIFY`-driven [`ChangeTail`] for Postgres.
//!
//! Companion of [`crate::PgPollingTail`] — same trait, lower
//! latency, fewer wasted queries. The trigger that powers it is
//! installed by migration `0002_listen_notify.sql`.
//!
//! ## Why we still re-query after each notification
//!
//! `pg_notify` payloads are not a delivery guarantee. They can be
//! coalesced under load, dropped on connection loss, or arrive in
//! bursts. The notification is therefore only used as a *wakeup
//! signal*; the actual change set comes from an incremental SELECT
//! `WHERE (at, id) > cursor` so missed notifications never cause
//! missed rows. A periodic safety tick re-polls even if no
//! notifications arrive (covers the window where the listener
//! reconnected without being told about an in-flight commit).
//!
//! ## Fan-out: one listener per `PgListenTail`, N subscribers
//!
//! The naïve "one [`PgListener`] per [`ChangeTail::subscribe`] call"
//! shape pinned one PG connection per subscriber for the lifetime
//! of the subscription. With long-lived consumers (dashboard SSE,
//! liveness streams) that scaled badly: every browser tab held a
//! connection slot until disconnect, and the pool sized for "max
//! concurrent tabs" instead of "actual notify throughput."
//!
//! This implementation collapses N subscribers onto ONE shared
//! listener task. The task owns the [`PgListener`], drains the
//! `starter_changes` table on every notify, and broadcasts each
//! [`Change`] over a [`tokio::sync::broadcast`] channel. Each
//! `subscribe()` call returns a fresh `mpsc::Receiver<Change>` fed
//! by a small per-subscribe bridge task that:
//!
//! 1. Snapshots a `(at, id)` cursor inside `subscribe()` so any
//!    row committed *before* subscribe completes is filtered out
//!    (preserves the "seed-before-subscribe is not delivered"
//!    contract that the existing tests pin down).
//! 2. Forwards every broadcast item past that cursor to the
//!    subscriber's `mpsc`.
//! 3. On `broadcast::error::RecvError::Lagged`, re-runs the same
//!    incremental SELECT the shared listener uses, so a slow
//!    consumer recovers without missing rows.
//!
//! Connection cost: exactly one pinned listener connection per
//! `PgListenTail` instance, regardless of subscriber count. Plus
//! transient `&Pool` checkouts for the initial cursor snapshot and
//! any `Lagged` gap-fills.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgListener;
use sqlx::Row;
use starter_changelog::ChangeTail;
use starter_spi::changelog::Change;
use starter_spi::{Error, Result};
use starter_store_postgres::Pool;
use tokio::sync::{broadcast, mpsc, Mutex};

use crate::codec::row_to_change;

/// LISTEN/NOTIFY tail for Postgres. One [`PgListener`] (and thus
/// one dedicated DB connection) per `PgListenTail`, fanned out to
/// N subscribers via [`tokio::sync::broadcast`].
#[derive(Clone)]
pub struct PgListenTail {
    inner: Arc<PgListenTailInner>,
}

struct PgListenTailInner {
    pool: Pool,
    /// Safety re-poll cadence. The trigger is the primary signal;
    /// this just catches missed notifications after reconnects.
    safety_interval: Duration,
    /// Per-subscriber `mpsc` buffer.
    buffer: usize,
    /// Broadcast ring capacity. Sized for the worst-case burst
    /// between subscriber polls; on overflow the slowest subscriber
    /// gets `Lagged` and falls back to a SELECT-based gap-fill.
    broadcast_capacity: usize,
    /// Lazily initialised on first `subscribe()`. The async `Mutex`
    /// serialises concurrent first-subscribers so we don't open
    /// multiple `PgListener`s. Re-initialised on next `subscribe()`
    /// if a prior listener task ever exits (the task clears this
    /// slot before returning).
    shared: Mutex<Option<Arc<SharedListener>>>,
}

/// State shared by every subscriber: the broadcast sender for live
/// rows, plus the pool used for `Lagged` gap-fill queries.
struct SharedListener {
    tx: broadcast::Sender<Change>,
    pool: Pool,
}

/// `NOTIFY` channel name; matches the trigger in migration 0002.
const CHANNEL: &str = "starter_changes_new";

impl PgListenTail {
    /// Wrap a pool. Defaults: 30s safety re-poll, 64-message
    /// per-subscriber buffer, 256-message broadcast ring.
    pub fn new(pool: Pool) -> Self {
        Self {
            inner: Arc::new(PgListenTailInner {
                pool,
                safety_interval: Duration::from_secs(30),
                buffer: 64,
                broadcast_capacity: 256,
                shared: Mutex::new(None),
            }),
        }
    }

    /// Override the safety re-poll interval. Lower values reduce
    /// the worst-case delay after a missed notification; higher
    /// values reduce idle DB load. 30s is a reasonable default for
    /// human-driven workloads.
    pub fn with_safety_interval(mut self, interval: Duration) -> Self {
        Arc::get_mut(&mut self.inner)
            .expect("with_safety_interval called after clone")
            .safety_interval = interval;
        self
    }

    /// Override the per-subscriber `mpsc` buffer size.
    pub fn with_buffer(mut self, buffer: usize) -> Self {
        Arc::get_mut(&mut self.inner)
            .expect("with_buffer called after clone")
            .buffer = buffer.max(1);
        self
    }

    /// Override the broadcast ring capacity. Should be ≥ the
    /// largest burst expected between subscriber polls. Subscribers
    /// that fall behind by more than this surface `Lagged` and
    /// recover via gap-fill; sizing this generously trades a bit
    /// of memory for fewer gap-fill round-trips under bursty load.
    pub fn with_broadcast_capacity(mut self, capacity: usize) -> Self {
        Arc::get_mut(&mut self.inner)
            .expect("with_broadcast_capacity called after clone")
            .broadcast_capacity = capacity.max(1);
        self
    }

    /// Return the shared listener, starting it on first call (and
    /// re-starting if a prior listener has exited). The async
    /// `Mutex` serialises concurrent first-subscribers across the
    /// `PgListener::connect_with` `.await` so we never open two
    /// listener connections.
    async fn shared(&self) -> Result<Arc<SharedListener>> {
        let mut guard = self.inner.shared.lock().await;
        if let Some(live) = guard.as_ref() {
            return Ok(Arc::clone(live));
        }
        let listener = start_shared_listener(self, Arc::clone(&self.inner)).await?;
        *guard = Some(Arc::clone(&listener));
        Ok(listener)
    }
}

/// Open the dedicated `PgListener`, snapshot the cursor, and spawn
/// the shared drain/broadcast loop. Returns the `SharedListener`
/// handle the caller stores in `inner.shared`.
async fn start_shared_listener(
    tail: &PgListenTail,
    inner: Arc<PgListenTailInner>,
) -> Result<Arc<SharedListener>> {
    let mut listener = PgListener::connect_with(tail.inner.pool.sqlx())
        .await
        .map_err(internal)?;
    listener.listen(CHANNEL).await.map_err(internal)?;

    // Snapshot the current head AFTER LISTEN is in effect. Any row
    // committed between these two statements will (a) appear in
    // this SELECT and become our cursor, and (b) trigger a notify
    // that we'll then drain into the empty tail of the SELECT —
    // at worst a redundant query, never a missed row.
    let cursor: Option<(DateTime<Utc>, String)> =
        sqlx::query_as("SELECT at, id FROM starter_changes ORDER BY at DESC, id DESC LIMIT 1")
            .fetch_optional(tail.inner.pool.sqlx())
            .await
            .map_err(internal)?;

    let (tx, _rx0) = broadcast::channel::<Change>(tail.inner.broadcast_capacity);
    let shared = Arc::new(SharedListener {
        tx: tx.clone(),
        pool: tail.inner.pool.clone(),
    });

    let pool = tail.inner.pool.clone();
    let safety_interval = tail.inner.safety_interval;
    let shared_weak = Arc::downgrade(&shared);

    tokio::spawn(async move {
        // Liveness rule: the shared loop runs as long as the parent
        // `PgListenTail` is alive AND nothing has cleared the slot.
        // We hold `shared_weak` (not a strong `Arc`) so this task
        // doesn't itself keep the `SharedListener` alive — that way
        // the `PgListenTail` being dropped is detectable here by
        // upgrade failure.
        //
        // We do NOT preemptively shut down when `tx.receiver_count()`
        // hits 0: a momentary gap between two subscribers would
        // otherwise tear down and rebuild the listener, defeating
        // the whole point.
        let mut cursor = cursor;
        loop {
            if shared_weak.upgrade().is_none() {
                // Parent `PgListenTail` was dropped.
                break;
            }

            let wake = tokio::select! {
                n = listener.try_recv() => n.map(|_| ()),
                _ = tokio::time::sleep(safety_interval) => Ok(()),
            };

            match wake {
                Ok(()) => {}
                Err(e) => {
                    // PgListener auto-reconnects on `recv`; we
                    // log and continue. The safety tick will
                    // still fire and pick up anything we
                    // missed during the reconnect window.
                    tracing::warn!(error = ?e, "changelog listen tail recv failed");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            }

            if let Err(e) = drain_since_broadcast(&pool, &mut cursor, &tx).await {
                tracing::warn!(error = ?e, "changelog listen tail drain failed");
            }
        }

        // Clear the slot so the next `subscribe()` re-initialises.
        let mut guard = inner.shared.lock().await;
        *guard = None;
    });

    Ok(shared)
}

#[async_trait]
impl ChangeTail for PgListenTail {
    async fn subscribe(&self) -> Result<mpsc::Receiver<Change>> {
        // Snapshot subscribe-time cursor BEFORE attaching to the
        // broadcast, so any in-flight row the shared listener
        // delivers to us right after `bcast_rx = tx.subscribe()`
        // can be deduped against this watermark. Matches the
        // semantics of the per-subscriber implementation: rows
        // committed before `subscribe()` returns are NOT
        // delivered.
        let snapshot_cursor: Option<(DateTime<Utc>, String)> =
            sqlx::query_as("SELECT at, id FROM starter_changes ORDER BY at DESC, id DESC LIMIT 1")
                .fetch_optional(self.inner.pool.sqlx())
                .await
                .map_err(internal)?;

        let shared = self.shared().await?;
        let mut bcast_rx = shared.tx.subscribe();
        let (tx, rx) = mpsc::channel::<Change>(self.inner.buffer);
        let pool = shared.pool.clone();

        tokio::spawn(async move {
            let mut subscribe_cursor = snapshot_cursor;
            loop {
                match bcast_rx.recv().await {
                    Ok(change) => {
                        let key = (change.at, change.id.0.clone());
                        if let Some(ref c) = subscribe_cursor {
                            if key <= *c {
                                // Pre-subscribe row that the shared
                                // listener already broadcast (or a
                                // duplicate from a gap-fill); skip.
                                continue;
                            }
                        }
                        if tx.send(change).await.is_err() {
                            return; // subscriber dropped
                        }
                        subscribe_cursor = Some(key);
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(
                            skipped = n,
                            "changelog subscriber lagged broadcast ring; gap-filling via SELECT",
                        );
                        if let Err(e) =
                            drain_since_mpsc(&pool, &mut subscribe_cursor, &tx).await
                        {
                            tracing::warn!(error = ?e, "changelog gap-fill SELECT failed");
                        }
                        if tx.is_closed() {
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        // Shared listener task exited. The watcher
                        // in `shared()` will reset the init gate so
                        // the next `subscribe()` call rebuilds it.
                        // This subscriber's stream ends here; the
                        // caller can re-subscribe.
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }
}

/// Drain rows after `cursor` and broadcast them. Used by the shared
/// listener task. Advances `cursor` in place.
async fn drain_since_broadcast(
    pool: &Pool,
    cursor: &mut Option<(DateTime<Utc>, String)>,
    tx: &broadcast::Sender<Change>,
) -> Result<()> {
    let rows = fetch_rows_since(pool, cursor.as_ref()).await?;
    for row in rows {
        let at: DateTime<Utc> = row.try_get("at").map_err(internal)?;
        let id: String = row.try_get("id").map_err(internal)?;
        match row_to_change(&row) {
            Ok(ch) => {
                // `send` only errors when there are zero receivers;
                // that's fine — we still advance the cursor so a
                // future subscriber doesn't re-receive this row via
                // the next safety tick.
                let _ = tx.send(ch);
                *cursor = Some((at, id));
            }
            Err(e) => {
                tracing::warn!(error = %e, "tail row decode failed");
                *cursor = Some((at, id));
            }
        }
    }
    Ok(())
}

/// Drain rows after `cursor` and forward them through a per-subscriber
/// `mpsc`. Used for the `Lagged` recovery path.
async fn drain_since_mpsc(
    pool: &Pool,
    cursor: &mut Option<(DateTime<Utc>, String)>,
    tx: &mpsc::Sender<Change>,
) -> Result<()> {
    let rows = fetch_rows_since(pool, cursor.as_ref()).await?;
    for row in rows {
        let at: DateTime<Utc> = row.try_get("at").map_err(internal)?;
        let id: String = row.try_get("id").map_err(internal)?;
        match row_to_change(&row) {
            Ok(ch) => {
                if tx.send(ch).await.is_err() {
                    return Ok(());
                }
                *cursor = Some((at, id));
            }
            Err(e) => {
                tracing::warn!(error = %e, "tail row decode failed");
                *cursor = Some((at, id));
            }
        }
    }
    Ok(())
}

async fn fetch_rows_since(
    pool: &Pool,
    cursor: Option<&(DateTime<Utc>, String)>,
) -> Result<Vec<sqlx::postgres::PgRow>> {
    match cursor {
        Some((at, id)) => sqlx::query(
            "SELECT * FROM starter_changes \
             WHERE (at, id) > ($1, $2) \
             ORDER BY at ASC, id ASC LIMIT 256",
        )
        .bind(at)
        .bind(id)
        .fetch_all(pool.sqlx())
        .await
        .map_err(internal),
        None => sqlx::query("SELECT * FROM starter_changes ORDER BY at ASC, id ASC LIMIT 256")
            .fetch_all(pool.sqlx())
            .await
            .map_err(internal),
    }
}

fn internal<E: std::error::Error + Send + Sync + 'static>(e: E) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
