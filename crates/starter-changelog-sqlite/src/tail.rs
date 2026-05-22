//! Polling [`ChangeTail`] for SQLite.
//!
//! SQLite has no `LISTEN/NOTIFY`. The tail polls `starter_changes`
//! ordered by `(at, id)` from the last delivered cursor at a
//! configurable interval. Suitable for low-volume audit / agent-log
//! consumers; Postgres callers should prefer `LISTEN/NOTIFY`.

use std::time::Duration;

use async_trait::async_trait;
use sqlx::Row;
use starter_changelog::ChangeTail;
use starter_spi::changelog::Change;
use starter_spi::Result;
use starter_store_sqlite::Pool;
use tokio::sync::mpsc;

use crate::codec::row_to_change;

/// Polling tail. Each subscriber gets its own task and channel.
#[derive(Clone)]
pub struct SqlitePollingTail {
    pool: Pool,
    interval: Duration,
    buffer: usize,
}

impl SqlitePollingTail {
    /// Wrap a pool. Defaults: 1s poll interval, 64-message buffer.
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            interval: Duration::from_secs(1),
            buffer: 64,
        }
    }

    /// Override the poll interval.
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Override the channel buffer size.
    pub fn with_buffer(mut self, buffer: usize) -> Self {
        self.buffer = buffer.max(1);
        self
    }
}

#[async_trait]
impl ChangeTail for SqlitePollingTail {
    async fn subscribe(&self) -> Result<mpsc::Receiver<Change>> {
        let (tx, rx) = mpsc::channel(self.buffer);
        let pool = self.pool.clone();
        let interval = self.interval;

        // Start the tail at the *current* high-water mark so a fresh
        // subscriber sees only future writes — replays go through
        // `ChangeLog::list`, not the tail.
        let cursor: Option<(String, String)> = sqlx::query_as::<_, (String, String)>(
            "SELECT at, id FROM starter_changes ORDER BY at DESC, id DESC LIMIT 1",
        )
        .fetch_optional(pool.sqlx())
        .await
        .map_err(|e| starter_spi::Error::Internal {
            source: Box::new(e),
        })?;

        let mut cursor = cursor;

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                if tx.is_closed() {
                    break;
                }

                let rows_result = match &cursor {
                    Some((at, id)) => {
                        sqlx::query(
                            "SELECT * FROM starter_changes \
                         WHERE (at, id) > (?1, ?2) \
                         ORDER BY at ASC, id ASC LIMIT 256",
                        )
                        .bind(at.clone())
                        .bind(id.clone())
                        .fetch_all(pool.sqlx())
                        .await
                    }
                    None => {
                        sqlx::query(
                            "SELECT * FROM starter_changes ORDER BY at ASC, id ASC LIMIT 256",
                        )
                        .fetch_all(pool.sqlx())
                        .await
                    }
                };

                let rows = match rows_result {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!(error = %e, "changelog tail poll failed");
                        continue;
                    }
                };

                for row in rows {
                    let at: String = match row.try_get("at") {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!(error = %e, "tail row decode failed");
                            continue;
                        }
                    };
                    let id: String = match row.try_get("id") {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!(error = %e, "tail row decode failed");
                            continue;
                        }
                    };
                    match row_to_change(&row) {
                        Ok(ch) => {
                            if tx.send(ch).await.is_err() {
                                return; // subscriber dropped
                            }
                            cursor = Some((at, id));
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "tail row decode failed");
                        }
                    }
                }
            }
        });

        Ok(rx)
    }
}
