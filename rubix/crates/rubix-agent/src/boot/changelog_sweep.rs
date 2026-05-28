//! Per-kind retention sweep for `starter_changes`.
//!
//! Mirror of [`crate::boot::undo_sweep`] but targeting the audit
//! table (`starter_changes`) rather than the operational replay
//! buffer (`undo_snapshots`). Reads `changelog_kind_policy`
//! (provisioned by the `changelog` migration source, seeded by the
//! `changelog_policy` rubix-owned source) and deletes rows older
//! than each kind's `max_age_days`. Kinds with no policy row or a
//! `NULL` curve are skipped entirely — opting into a finite
//! retention window is always explicit.
//!
//! ## Why this exists
//!
//! `starter_changes` is the historical record read by `GET
//! /v1/audit`. Until this module landed, the table had no
//! rubix-side sweep — retention was implicit-unbounded. The
//! audit-log proposal
//! ([`rubix/docs/proposal/audit-log.md`](../../../../docs/proposal/audit-log.md))
//! introduced an explicit per-kind policy so operators can opt
//! some kinds into bounded retention without risking the audit
//! floor on security-relevant kinds (`user`, `team` — both seeded
//! with `NULL` `max_age_days` = keep forever).
//!
//! ## Schedule
//!
//! [`spawn_changelog_sweep`] runs the sweep once at boot (so a
//! fresh deploy that inherits a fat changelog immediately reclaims
//! space for any kind on a finite curve) and then re-runs every
//! 24h until the returned [`tokio::task::JoinHandle`] is dropped.
//! Drop the handle to stop the loop (e.g. in a graceful-shutdown
//! path).
//!
//! ## DSN handling
//!
//! When `database_url` is unset the boot path skips the migration
//! step and there is no `starter_changes` or `changelog_kind_policy`
//! table; this module's [`spawn_changelog_sweep`] no-ops in that
//! case so a laptop boot without Postgres stays clean.

use std::time::Duration;

use anyhow::Result;
use starter_changelog_postgres::{apply_policy, PolicyReport};
use starter_store_postgres::pool::{connect, Pool};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// 24h between sweep ticks. Held as a constant — the cadence is
/// a documented contract, not an operator knob; the *policy* is
/// what operators tune.
const SWEEP_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Run the sweep once and return the per-kind report. Public so
/// the integration test (and a future admin verb) can drive the
/// sweep deterministically without waiting for the 24h tick.
pub async fn sweep_once(pool: &Pool) -> Result<PolicyReport> {
    Ok(apply_policy(pool).await?)
}

/// Spawn the boot-tick + 24h-tick retention loop. Returns the
/// task handle so callers (production `main.rs`, tests) can abort
/// the loop on shutdown. When `dsn` is `None` this returns
/// `Ok(None)` and logs a warn so the laptop boot stays quiet.
pub async fn spawn_changelog_sweep(dsn: Option<&str>) -> Result<Option<JoinHandle<()>>> {
    let Some(dsn) = dsn else {
        warn!(
            target: "rubix.boot",
            "Postgres DSN unset — starter_changes sweep skipped",
        );
        return Ok(None);
    };

    let pool = connect(dsn)
        .await
        .map_err(|e| anyhow::anyhow!("connect for changelog sweep: {e}"))?;

    // Boot-tick sweep runs inline so the agent comes up with the
    // table already inside the retention envelope for kinds on a
    // finite curve. Failures are logged but do not abort the boot
    // — the table existing un-swept is preferable to refusing to
    // start (matches `undo_sweep`).
    match sweep_once(&pool).await {
        Ok(report) => info!(
            target: "rubix.boot",
            kinds_with_policy = report.per_kind.len(),
            deleted = report.total_deleted(),
            "starter_changes sweep (boot tick) complete",
        ),
        Err(e) => warn!(
            target: "rubix.boot",
            error = %e,
            "starter_changes sweep (boot tick) failed; will retry on next 24h tick",
        ),
    }

    let handle = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(SWEEP_INTERVAL);
        // The first `tick` returns immediately by default; we
        // already swept inline above, so consume that initial
        // tick before sleeping for SWEEP_INTERVAL.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            match sweep_once(&pool).await {
                Ok(report) => debug!(
                    target: "rubix.boot.changelog_sweep",
                    kinds_with_policy = report.per_kind.len(),
                    deleted = report.total_deleted(),
                    "starter_changes sweep (24h tick) complete",
                ),
                Err(e) => warn!(
                    target: "rubix.boot.changelog_sweep",
                    error = %e,
                    "starter_changes sweep (24h tick) failed; will retry next tick",
                ),
            }
        }
    });
    Ok(Some(handle))
}
