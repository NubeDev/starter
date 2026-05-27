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

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgListener;
use sqlx::Row;
use starter_changelog::ChangeTail;
use starter_spi::changelog::Change;
use starter_spi::{Error, Result};
use starter_store_postgres::Pool;
use tokio::sync::mpsc;

use crate::codec::row_to_change;

/// LISTEN/NOTIFY tail for Postgres. One [`PgListener`] (and thus
/// one dedicated DB connection) per subscriber.
#[derive(Clone)]
pub struct PgListenTail {
    pool: Pool,
    /// Safety re-poll cadence. The trigger is the primary signal;
    /// this just catches missed notifications after reconnects.
    safety_interval: Duration,
    buffer: usize,
}

/// `NOTIFY` channel name; matches the trigger in migration 0002.
const CHANNEL: &str = "starter_changes_new";

impl PgListenTail {
    /// Wrap a pool. Defaults: 30s safety re-poll, 64-message
    /// buffer.
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            safety_interval: Duration::from_secs(30),
            buffer: 64,
        }
    }

    /// Override the safety re-poll interval. Lower values reduce
    /// the worst-case delay after a missed notification; higher
    /// values reduce idle DB load. 30s is a reasonable default for
    /// human-driven workloads.
    pub fn with_safety_interval(mut self, interval: Duration) -> Self {
        self.safety_interval = interval;
        self
    }

    /// Override the channel buffer size.
    pub fn with_buffer(mut self, buffer: usize) -> Self {
        self.buffer = buffer.max(1);
        self
    }
}

#[async_trait]
impl ChangeTail for PgListenTail {
    async fn subscribe(&self) -> Result<mpsc::Receiver<Change>> {
        let (tx, rx) = mpsc::channel(self.buffer);

        // Acquire the dedicated listener connection up-front so
        // construction errors surface to the caller instead of
        // showing up as a silent task crash.
        let mut listener = PgListener::connect_with(self.pool.sqlx())
            .await
            .map_err(internal)?;
        listener.listen(CHANNEL).await.map_err(internal)?;

        // Snapshot the current head AFTER LISTEN is in effect. Any
        // row committed between these two statements will (a)
        // appear in this SELECT and become our cursor, and (b)
        // trigger a notify that we'll then drain into the empty
        // tail of the SELECT — at worst a redundant query, never a
        // missed row.
        let cursor: Option<(DateTime<Utc>, String)> =
            sqlx::query_as("SELECT at, id FROM starter_changes ORDER BY at DESC, id DESC LIMIT 1")
                .fetch_optional(self.pool.sqlx())
                .await
                .map_err(internal)?;

        let pool = self.pool.clone();
        let safety_interval = self.safety_interval;
        let mut cursor = cursor;

        tokio::spawn(async move {
            loop {
                if tx.is_closed() {
                    break;
                }

                // Wait for a NOTIFY or the safety tick, whichever
                // fires first. We don't care about the payload —
                // it's just a wakeup.
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

                if let Err(e) = drain_since(&pool, &mut cursor, &tx).await {
                    tracing::warn!(error = ?e, "changelog listen tail drain failed");
                }
            }
        });

        Ok(rx)
    }
}

/// Pull every row newer than `cursor`, forward it, and advance the
/// cursor. Returns Ok if the subscriber dropped (so the caller can
/// exit cleanly).
async fn drain_since(
    pool: &Pool,
    cursor: &mut Option<(DateTime<Utc>, String)>,
    tx: &mpsc::Sender<Change>,
) -> Result<()> {
    let rows = match cursor.clone() {
        Some((at, id)) => sqlx::query(
            "SELECT * FROM starter_changes \
             WHERE (at, id) > ($1, $2) \
             ORDER BY at ASC, id ASC LIMIT 256",
        )
        .bind(at)
        .bind(id)
        .fetch_all(pool.sqlx())
        .await
        .map_err(internal)?,
        None => sqlx::query("SELECT * FROM starter_changes ORDER BY at ASC, id ASC LIMIT 256")
            .fetch_all(pool.sqlx())
            .await
            .map_err(internal)?,
    };

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
                // Advance the cursor anyway — a poison row would
                // otherwise jam the tail forever.
                *cursor = Some((at, id));
            }
        }
    }
    Ok(())
}

fn internal<E: std::error::Error + Send + Sync + 'static>(e: E) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
