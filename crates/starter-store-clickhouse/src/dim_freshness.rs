//! W11 — dimension staleness probe.
//!
//! `status()` queries `system.dictionaries` for `entities_dict`
//! and classifies the result into the four-valued [`Status`] enum.
//!
//! Per W11 the result is cached on the server side for ≤ 5 s so a
//! burst of `mart.read` calls does not re-query
//! `system.dictionaries` per request. The cache lives in
//! [`FreshnessProbe`]; instantiate one per process and share it.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::client::{ChClient, ChClientError};

/// W11 four-value status enum. Surfaced verbatim in the
/// `dimension_freshness` envelope on every read response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// `age_seconds <= lifetime_min`.
    Fresh,
    /// `lifetime_min < age_seconds <= lifetime_max`.
    StaleWithinBound,
    /// `age_seconds > lifetime_max` — the lag bound was missed,
    /// but the previous successful refresh is still in memory.
    StaleBeyondBound,
    /// The last refresh attempt failed (CH `last_exception` is
    /// non-empty). Reads still serve from the prior good snapshot,
    /// but the operator needs to know.
    FailedRefresh,
}

/// The full `dimension_freshness.entities_dict` block per W11.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictFreshness {
    pub name: String,
    pub status: Status,
    pub loaded_at: Option<DateTime<Utc>>,
    pub age_seconds: i64,
    pub lifetime_min_seconds: u32,
    pub lifetime_max_seconds: u32,
    pub last_exception: Option<String>,
}

/// Probe with a 5 s server-side cache. Cheap to clone (`Arc` inner).
#[derive(Clone)]
pub struct FreshnessProbe {
    client: ChClient,
    state: Arc<Mutex<CacheState>>,
    ttl: Duration,
}

struct CacheState {
    last: Option<(Instant, DictFreshness)>,
}

impl FreshnessProbe {
    pub fn new(client: ChClient) -> Self {
        Self {
            client,
            state: Arc::new(Mutex::new(CacheState { last: None })),
            ttl: Duration::from_secs(5),
        }
    }

    /// Override the cache TTL. Tests use this to force a refresh
    /// without sleeping 5 s.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Fetch `entities_dict` freshness, served from the cache when
    /// the last result is within `ttl`.
    pub async fn entities_dict(&self) -> Result<DictFreshness, ChClientError> {
        let mut state = self.state.lock().await;
        if let Some((at, cached)) = &state.last {
            if at.elapsed() < self.ttl {
                return Ok(cached.clone());
            }
        }
        let fresh = self.query_dict("entities_dict").await?;
        state.last = Some((Instant::now(), fresh.clone()));
        Ok(fresh)
    }

    async fn query_dict(&self, name: &str) -> Result<DictFreshness, ChClientError> {
        // `clickhouse::Row` for a `system.dictionaries` sub-select.
        #[derive(Debug, clickhouse::Row, Deserialize)]
        struct Raw {
            name: String,
            #[serde(with = "clickhouse::serde::chrono::datetime::option")]
            loaded: Option<DateTime<Utc>>,
            lifetime_min: u64,
            lifetime_max: u64,
            last_exception: String,
        }

        let mut rows = self
            .client
            .inner()
            .query(
                "SELECT \
                    name, \
                    last_successful_update_time AS loaded, \
                    lifetime_min, \
                    lifetime_max, \
                    last_exception \
                 FROM system.dictionaries WHERE name = ? LIMIT 1",
            )
            .bind(name)
            .fetch_all::<Raw>()
            .await?;
        let raw = rows.pop().ok_or_else(|| {
            ChClientError::Other(format!(
                "dictionary '{name}' not found in system.dictionaries"
            ))
        })?;

        let now = Utc::now();
        let age = raw
            .loaded
            .map(|l| (now - l).num_seconds())
            .unwrap_or(i64::MAX);
        let status = if !raw.last_exception.is_empty() {
            Status::FailedRefresh
        } else if age <= raw.lifetime_min as i64 {
            Status::Fresh
        } else if age <= raw.lifetime_max as i64 {
            Status::StaleWithinBound
        } else {
            Status::StaleBeyondBound
        };

        Ok(DictFreshness {
            name: raw.name,
            status,
            loaded_at: raw.loaded,
            age_seconds: age,
            lifetime_min_seconds: raw.lifetime_min as u32,
            lifetime_max_seconds: raw.lifetime_max as u32,
            last_exception: if raw.last_exception.is_empty() {
                None
            } else {
                Some(raw.last_exception)
            },
        })
    }
}
