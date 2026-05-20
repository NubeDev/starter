//! Per-worker state observed by `starter-ext-server`'s admin route.
//!
//! [`WorkerState`] is a serialisable snapshot — the admin route
//! deserialises it into the `workers:` array on the `GET /extensions/<id>`
//! response body. Keeping it `Serialize` means we never grow a parallel
//! "worker state DTO" inside `starter-ext-server`.

use std::time::SystemTime;

use serde::{Serialize, Serializer};
use starter_ext_spi::ExtensionId;

/// Coarse status surfaced to the admin UI.
///
/// The scheduler manages many small sub-states (running vs sleeping
/// vs backed-off-pending-retry); operators almost always care about
/// the three buckets here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerStatus {
    /// The worker is on its periodic cadence (success path).
    Healthy,
    /// The worker has failed at least once and is currently in
    /// exponential backoff; `attempt > 0` and a retry is scheduled
    /// at `next_due`.
    BackingOff,
    /// The worker hit `on_error.max_attempts` (or `retry: never`
    /// after the first failure) and has stopped scheduling. Admins
    /// re-enable through the admin route, or tests invoke
    /// [`crate::WorkersScheduler::tick_now`].
    Stopped,
}

/// Snapshot of one worker's runtime state.
///
/// Serialised verbatim onto `GET /extensions/<id>`. Time fields are
/// emitted as RFC-3339 strings (UTC) for cross-language readability.
#[derive(Debug, Clone)]
pub struct WorkerState {
    /// `contributes.workers[].id` from the manifest.
    pub worker_id: String,
    /// Owning extension id (handy for the admin response).
    pub extension_id: ExtensionId,
    /// Coarse status.
    pub status: WorkerStatus,
    /// Wall-clock time of the most recent attempt; `None` until the
    /// scheduler has run the handler at least once.
    pub last_run: Option<SystemTime>,
    /// Sticky error from the most recent failing run; cleared on
    /// success.
    pub last_error: Option<String>,
    /// Wall-clock time of the next scheduled run; `None` if the
    /// worker has stopped.
    pub next_due: Option<SystemTime>,
    /// Consecutive failure count (zero after a success).
    pub attempt: u32,
    /// Lifetime total run count (success + failure). Useful for the
    /// "did this worker ever fire?" question.
    pub total_runs: u64,
}

impl Serialize for WorkerState {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct W<'a> {
            worker_id: &'a str,
            extension_id: &'a str,
            status: WorkerStatus,
            #[serde(skip_serializing_if = "Option::is_none")]
            last_run: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            last_error: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            next_due: Option<String>,
            attempt: u32,
            total_runs: u64,
        }
        W {
            worker_id: &self.worker_id,
            extension_id: self.extension_id.as_str(),
            status: self.status,
            last_run: self.last_run.map(format_rfc3339),
            last_error: self.last_error.as_deref(),
            next_due: self.next_due.map(format_rfc3339),
            attempt: self.attempt,
            total_runs: self.total_runs,
        }
        .serialize(ser)
    }
}

fn format_rfc3339(t: SystemTime) -> String {
    // Minimal RFC-3339 UTC formatting without pulling in `chrono` /
    // `time` — we already depend on `std::time` everywhere else.
    let dur = match t.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => d,
        Err(_) => return "1970-01-01T00:00:00Z".into(),
    };
    let secs = dur.as_secs();
    let millis = dur.subsec_millis();
    // Days since the unix epoch.
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let h = rem / 3_600;
    let m = (rem / 60) % 60;
    let s = rem % 60;
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}.{millis:03}Z")
}

// "civil_from_days" — Howard Hinnant's algorithm. ~10 LoC; lets us
// avoid the `time` / `chrono` dep for one timestamp field. The same
// algorithm ships in std::time::SystemTime's nightly formatter.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = (y + if m <= 2 { 1 } else { 0 }) as i32;
    (y, m, d)
}

/// Read-only seam between `starter-ext-workers` and any consumer that
/// wants to surface worker state — chiefly `starter-ext-server`'s
/// `GET /extensions/<id>` handler. Implementations must be
/// `Send + Sync` because the admin router clones the trait object
/// across handlers.
pub trait WorkerStateSource: Send + Sync + 'static {
    /// Return every worker state belonging to the given extension.
    /// An unknown extension id returns an empty vector (not an error
    /// — the admin route always succeeds, and an empty `workers:`
    /// field is meaningful: "no workers for this extension").
    fn workers_for(&self, extension: &ExtensionId) -> Vec<WorkerState>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_state_serialises_with_rfc3339_timestamps() {
        let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        let s = WorkerState {
            worker_id: "com.acme.weather.sync".into(),
            extension_id: ExtensionId::new("com.acme.weather").unwrap(),
            status: WorkerStatus::Healthy,
            last_run: Some(t),
            last_error: None,
            next_due: Some(t + std::time::Duration::from_secs(300)),
            attempt: 0,
            total_runs: 7,
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["worker_id"], "com.acme.weather.sync");
        assert_eq!(v["status"], "healthy");
        assert_eq!(v["total_runs"], 7);
        assert_eq!(v["last_run"], "2023-11-14T22:13:20.000Z");
        assert!(v["next_due"].as_str().unwrap().starts_with("2023-11-14T"));
        assert_eq!(v.get("last_error"), None);
    }
}
