//! v3 — cold-start warmer.
//!
//! After a deploy, every cache key is cold. SWR doesn't help. The
//! warmer replays the top-N spec ids (by hit count from the previous
//! run's persisted stats) by handing each one to a host-supplied
//! [`WarmEntry::reload`] closure.
//!
//! v3 deliberately keeps the warmer modest: it sweeps the per-spec
//! stats snapshot **after** the cache layer has been told about prior
//! hit counts (either via persisted stats reloaded by the host, or
//! by re-running an explicit list the host hands over). The list of
//! "spec ids worth warming" lives at the host, not the cache — the
//! cache doesn't know how to reload one.
//!
//! Wiring is gated behind `RUBIX_CACHE_WARM_ON_BOOT=<N>`; default
//! off. The host calls [`Warmer::warm_top_n`] once on startup with
//! the N from the env var and a closure that knows how to call the
//! loader for one spec id.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Status surface — what /admin/cache/specs joins onto the per-spec
/// row to surface as `cache.warmer.last_run_at` +
/// `cache.warmer.entries_warmed`.
#[derive(Debug, Default, Clone)]
pub struct WarmerStatus {
    /// Wall-clock instant the last warm pass completed, if any.
    pub last_run_at: Option<std::time::SystemTime>,
    /// How many entries the last pass touched.
    pub entries_warmed: u64,
    /// How long the last pass took.
    pub last_duration: Option<Duration>,
}

/// The host-supplied callback for warming one spec id. Returns
/// `Ok(())` on a successful reload; errors are logged but do not
/// fail the warm pass.
pub type WarmCallback = Arc<
    dyn Fn(
            String,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>>
        + Send
        + Sync,
>;

/// The warmer itself. Cheap to clone — internally `Arc`-shared.
#[derive(Clone)]
pub struct Warmer {
    status: Arc<Mutex<WarmerStatus>>,
}

impl Default for Warmer {
    fn default() -> Self {
        Self::new()
    }
}

impl Warmer {
    /// Build an empty warmer.
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(WarmerStatus::default())),
        }
    }

    /// Snapshot current status (sync) — used by the admin endpoint.
    pub fn snapshot(&self) -> WarmerStatus {
        // try_lock keeps this sync; on contention return defaults.
        match self.status.try_lock() {
            Ok(g) => g.clone(),
            Err(_) => WarmerStatus::default(),
        }
    }

    /// Warm the top-N spec ids. `top_n` is an already-sorted list of
    /// spec ids (the host computes it from the per-spec snapshot,
    /// usually by `hits + misses` descending) and `cb` is called once
    /// per id with the spec id.
    pub async fn warm_top_n(&self, top_n: Vec<String>, cb: WarmCallback) {
        let started = Instant::now();
        let mut warmed: u64 = 0;
        for spec_id in top_n {
            match cb(spec_id.clone()).await {
                Ok(()) => warmed += 1,
                Err(e) => {
                    tracing::warn!("starter-cache: warmer reload failed for {spec_id:?}: {e}");
                }
            }
        }
        let mut g = self.status.lock().await;
        g.last_run_at = Some(std::time::SystemTime::now());
        g.entries_warmed = warmed;
        g.last_duration = Some(started.elapsed());
    }

    /// Read the `RUBIX_CACHE_WARM_ON_BOOT` env var. Returns `None`
    /// when unset, zero, or unparseable — the host treats `None` as
    /// "warmer disabled".
    pub fn n_from_env(prefix: &str) -> Option<usize> {
        let key = format!("{prefix}_WARM_ON_BOOT");
        let raw = std::env::var(&key).ok()?;
        let n: usize = raw.parse().ok()?;
        if n == 0 {
            None
        } else {
            Some(n)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn warmer_calls_back_per_spec_and_records_status() {
        let warmer = Warmer::new();
        let calls = Arc::new(AtomicU32::new(0));
        let calls_cb = calls.clone();
        let cb: WarmCallback = Arc::new(move |_id| {
            let c = calls_cb.clone();
            Box::pin(async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        });
        warmer
            .warm_top_n(vec!["a".into(), "b".into(), "c".into()], cb)
            .await;
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        let snap = warmer.snapshot();
        assert_eq!(snap.entries_warmed, 3);
        assert!(snap.last_run_at.is_some());
        assert!(snap.last_duration.is_some());
    }

    #[tokio::test]
    async fn warmer_records_only_successes() {
        let warmer = Warmer::new();
        let cb: WarmCallback = Arc::new(|id| {
            Box::pin(async move {
                if id == "bad" {
                    Err("oops".into())
                } else {
                    Ok(())
                }
            })
        });
        warmer
            .warm_top_n(vec!["a".into(), "bad".into(), "c".into()], cb)
            .await;
        assert_eq!(warmer.snapshot().entries_warmed, 2);
    }

    #[test]
    fn n_from_env_returns_none_for_zero_or_missing() {
        let prefix = "STARTER_CACHE_TEST_WARMER";
        std::env::remove_var(format!("{prefix}_WARM_ON_BOOT"));
        assert!(Warmer::n_from_env(prefix).is_none());
        std::env::set_var(format!("{prefix}_WARM_ON_BOOT"), "0");
        assert!(Warmer::n_from_env(prefix).is_none());
        std::env::set_var(format!("{prefix}_WARM_ON_BOOT"), "5");
        assert_eq!(Warmer::n_from_env(prefix), Some(5));
        std::env::remove_var(format!("{prefix}_WARM_ON_BOOT"));
    }
}
