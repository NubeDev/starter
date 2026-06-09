//! The audit-retention policy (WS-12) — how long the ledger keeps a row.
//!
//! A single global horizon, read from the environment at boot. The append-only
//! ledger would grow without bound, so the prune sweep deletes rows older than
//! `retention()`. One number an operator can reason about beats a per-kind matrix
//! nothing yet needs; a per-kind policy is a fast-follow (see
//! `1603_changelog_retention.sql`).
//!
//! Undo reaches only the most recent group an actor authored, so a horizon of
//! months never truncates a reachable undo step — retention bounds the *audit*
//! tail, not undo depth.

use std::time::Duration;

use chrono::{DateTime, Utc};

/// Default retention: 365 days. Long enough that a year-end audit still has the
/// full year; short enough that the ledger does not grow forever. Overridable
/// via `NEXUS_AUDIT_RETENTION_DAYS`.
const DEFAULT_RETENTION_DAYS: i64 = 365;

/// The minimum retention the server will honor. A horizon shorter than a day
/// risks pruning rows mid-session and is almost certainly a misconfiguration, so
/// a smaller value is clamped up rather than silently obeyed.
const MIN_RETENTION_DAYS: i64 = 1;

/// The retention policy: a single horizon. Cheap to clone; carried on the prune
/// task.
#[derive(Clone, Copy, Debug)]
pub struct RetentionPolicy {
    days: i64,
}

impl RetentionPolicy {
    /// Read the horizon from `NEXUS_AUDIT_RETENTION_DAYS`, falling back to the
    /// default. A missing or unparseable value uses the default; a value below
    /// the floor is clamped up.
    pub fn from_env() -> Self {
        let days = std::env::var("NEXUS_AUDIT_RETENTION_DAYS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(DEFAULT_RETENTION_DAYS)
            .max(MIN_RETENTION_DAYS);
        Self { days }
    }

    /// The cutoff instant: rows recorded before this are eligible for pruning.
    /// Computed fresh each sweep so the horizon slides with wall-clock time.
    pub fn cutoff(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        now - chrono::Duration::days(self.days)
    }

    /// The retention horizon as a [`Duration`], for logging/metrics.
    pub fn horizon(&self) -> Duration {
        Duration::from_secs((self.days.max(0) as u64) * 86_400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cutoff_is_horizon_days_before_now() {
        let policy = RetentionPolicy { days: 30 };
        let now = DateTime::parse_from_rfc3339("2026-06-09T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let cutoff = policy.cutoff(now);
        assert_eq!(cutoff, now - chrono::Duration::days(30));
    }

    #[test]
    fn horizon_below_floor_clamps_up() {
        // A zero/negative env value must not produce a sub-day horizon.
        std::env::set_var("NEXUS_AUDIT_RETENTION_DAYS", "0");
        let policy = RetentionPolicy::from_env();
        std::env::remove_var("NEXUS_AUDIT_RETENTION_DAYS");
        assert!(policy.days >= MIN_RETENTION_DAYS);
    }
}
