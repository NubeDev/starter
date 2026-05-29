//! Live-process bookkeeping + `/proc` sampling for the process surface.
//!
//! The supervisor stores the current child's pid (and the data needed to
//! build a [`ProcessStats`]) in a [`ProcessCell`] shared with the
//! [`SupervisorHandle`](crate::SupervisorHandle). The cell is populated
//! next to the `EventKind::Spawned` push and cleared on exit, so
//! `handle.pid()` / `handle.process_stats()` read it lock-free of the
//! supervisor task.
//!
//! RSS / CPU are **sampled on the existing health tick** (plan rule 4 — no
//! new collector thread) by reading `/proc/<pid>/stat` + `/proc/<pid>/statm`
//! on Linux. Other platforms leave `rss_bytes` / `cpu_pct` as `None`. The
//! parsers are split out as free functions so they can be unit-tested
//! against captured `/proc` text without a live process.

use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime};

use starter_ext_spi::ProcessStats;

/// `USER_HZ` — the number of scheduler ticks per second the kernel reports
/// `utime` / `stime` in via `/proc/<pid>/stat`. 100 on every mainstream
/// Linux build; we assume it rather than pull in `libc` for `sysconf`
/// (R2 — starter stays dependency-light). A wrong value only scales the
/// best-effort `cpu_pct` gauge.
const USER_HZ: f64 = 100.0;

/// Page size in bytes, used to turn `/proc/<pid>/statm` resident *pages*
/// into bytes. 4096 on every platform we target.
const PAGE_SIZE: u64 = 4096;

/// State of the current child, shared between the supervisor task (writer)
/// and the [`SupervisorHandle`](crate::SupervisorHandle) (reader).
#[derive(Debug, Clone)]
pub(crate) struct LiveProcess {
    /// OS pid of the current child.
    pub pid: u32,
    /// Wall-clock spawn time of the current child.
    pub started_at: SystemTime,
    /// Monotonic spawn instant — the basis for `uptime` and the CPU
    /// sampling window (immune to wall-clock jumps).
    pub started_instant: Instant,
    /// Total restarts so far (carried across respawns).
    pub restarts: u64,
    /// Last sampled RSS in bytes; `None` until the first `/proc` read.
    pub rss_bytes: Option<u64>,
    /// Last computed CPU percentage; `None` until two samples exist.
    pub cpu_pct: Option<f32>,
    /// `utime + stime` in ticks at the previous sample, for the CPU delta.
    pub last_cpu_ticks: Option<u64>,
    /// Instant of the previous sample, for the CPU-window denominator.
    pub last_sample_at: Option<Instant>,
}

impl LiveProcess {
    /// A freshly-spawned child with no samples yet.
    pub(crate) fn new(pid: u32, restarts: u64, now: Instant) -> Self {
        Self {
            pid,
            started_at: SystemTime::now(),
            started_instant: now,
            restarts,
            rss_bytes: None,
            cpu_pct: None,
            last_cpu_ticks: None,
            last_sample_at: None,
        }
    }

    /// Project the bookkeeping into the wire [`ProcessStats`] shape.
    /// `now` is the read instant used for `uptime`.
    pub(crate) fn to_stats(&self, now: Instant) -> ProcessStats {
        ProcessStats {
            pid: self.pid,
            started_at: self.started_at,
            uptime: now.saturating_duration_since(self.started_instant),
            rss_bytes: self.rss_bytes,
            cpu_pct: self.cpu_pct,
            restarts: self.restarts,
        }
    }

    /// Fold a fresh `/proc` reading into the running CPU/RSS gauges.
    /// `total_ticks` is `utime + stime`; `cpu_pct` is computed from the
    /// delta against the previous sample over the elapsed window.
    pub(crate) fn apply_sample(
        &mut self,
        now: Instant,
        rss_bytes: Option<u64>,
        total_ticks: Option<u64>,
    ) {
        if rss_bytes.is_some() {
            self.rss_bytes = rss_bytes;
        }
        if let Some(ticks) = total_ticks {
            if let (Some(prev_ticks), Some(prev_at)) = (self.last_cpu_ticks, self.last_sample_at) {
                let elapsed = now.saturating_duration_since(prev_at).as_secs_f64();
                let dticks = ticks.saturating_sub(prev_ticks) as f64;
                if elapsed > 0.0 {
                    let pct = (dticks / USER_HZ) / elapsed * 100.0;
                    self.cpu_pct = Some(pct as f32);
                }
            }
            self.last_cpu_ticks = Some(ticks);
            self.last_sample_at = Some(now);
        }
    }
}

/// Shared cell holding the current child's [`LiveProcess`], or `None` when
/// no child is running.
pub(crate) type ProcessCell = Arc<Mutex<Option<LiveProcess>>>;

/// A fresh, empty [`ProcessCell`].
pub(crate) fn new_cell() -> ProcessCell {
    Arc::new(Mutex::new(None))
}

/// Read `/proc/<pid>/stat` + `/statm` and return
/// `(rss_bytes, total_cpu_ticks)`. Linux only; every other platform — and
/// any read/parse failure — yields `(None, None)`.
#[cfg(target_os = "linux")]
pub(crate) fn sample(pid: u32) -> (Option<u64>, Option<u64>) {
    let ticks = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|s| parse_stat_total_ticks(&s));
    let rss = std::fs::read_to_string(format!("/proc/{pid}/statm"))
        .ok()
        .and_then(|s| parse_statm_rss_bytes(&s));
    (rss, ticks)
}

/// Non-Linux platforms have no `/proc`; stats stay `None`.
#[cfg(not(target_os = "linux"))]
pub(crate) fn sample(_pid: u32) -> (Option<u64>, Option<u64>) {
    (None, None)
}

/// Parse `utime + stime` (fields 14 + 15, 1-indexed) from the contents of
/// `/proc/<pid>/stat`, in clock ticks.
///
/// Field 2 (`comm`) is wrapped in parentheses and may itself contain
/// spaces and `)` characters, so we split *after the last* `')'` — every
/// remaining field is then a simple whitespace-delimited token.
pub(crate) fn parse_stat_total_ticks(contents: &str) -> Option<u64> {
    let close = contents.rfind(')')?;
    // Tokens after `comm` start at field 3 (`state`). Index into this
    // tail: utime is field 14 → tail index 11, stime is field 15 → 12.
    let tail = contents.get(close + 1..)?;
    let fields: Vec<&str> = tail.split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some(utime.saturating_add(stime))
}

/// Parse the resident-set size from `/proc/<pid>/statm` and return it in
/// bytes. The second whitespace-delimited field is the resident page
/// count; multiply by [`PAGE_SIZE`].
pub(crate) fn parse_statm_rss_bytes(contents: &str) -> Option<u64> {
    let resident_pages: u64 = contents.split_whitespace().nth(1)?.parse().ok()?;
    Some(resident_pages.saturating_mul(PAGE_SIZE))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn parses_stat_with_simple_comm() {
        // Synthetic /proc/<pid>/stat: pid (comm) state ppid ... utime stime
        // Fields after `)`: state(3) ppid(4) pgrp(5) session(6) tty(7)
        // tpgid(8) flags(9) minflt(10) cminflt(11) majflt(12) cmajflt(13)
        // utime(14)=120 stime(15)=30 ...
        let line = "1234 (myproc) S 1 1234 1234 0 -1 4194560 100 0 0 0 120 30 0 0 20 0 1 0 99 0";
        assert_eq!(parse_stat_total_ticks(line), Some(150));
    }

    #[test]
    fn parses_stat_with_spaces_and_parens_in_comm() {
        // comm contains spaces and a ')' — must split after the LAST ')'.
        let line = "77 (we ird )name) S 1 77 77 0 -1 0 5 0 0 0 7 3 0 0 20 0 1 0 0 0";
        assert_eq!(parse_stat_total_ticks(line), Some(10));
    }

    #[test]
    fn stat_parse_rejects_garbage() {
        assert_eq!(parse_stat_total_ticks("no parens here"), None);
        assert_eq!(parse_stat_total_ticks("123 (c) S 1"), None);
    }

    #[test]
    fn parses_statm_rss() {
        // size resident shared text lib data dt → resident = 512 pages.
        let line = "2048 512 128 1 0 256 0";
        assert_eq!(parse_statm_rss_bytes(line), Some(512 * PAGE_SIZE));
    }

    #[test]
    fn statm_parse_rejects_garbage() {
        assert_eq!(parse_statm_rss_bytes("onlyonefield"), None);
        assert_eq!(parse_statm_rss_bytes(""), None);
    }

    #[test]
    fn cpu_pct_computed_from_second_sample() {
        let t0 = Instant::now();
        let mut lp = LiveProcess::new(10, 0, t0);
        // First sample: no prior point → cpu stays None, ticks recorded.
        lp.apply_sample(t0, Some(4096), Some(100));
        assert_eq!(lp.cpu_pct, None);
        assert_eq!(lp.rss_bytes, Some(4096));
        // Second sample 1s later, +50 ticks → 50/100/1s = 50% of a core.
        let t1 = t0 + Duration::from_secs(1);
        lp.apply_sample(t1, Some(8192), Some(150));
        assert_eq!(lp.rss_bytes, Some(8192));
        let pct = lp.cpu_pct.expect("cpu_pct after two samples");
        assert!((pct - 50.0).abs() < 0.01, "expected ~50%, got {pct}");
    }

    #[test]
    fn to_stats_reports_uptime() {
        let t0 = Instant::now();
        let lp = LiveProcess::new(42, 3, t0);
        let stats = lp.to_stats(t0 + Duration::from_secs(5));
        assert_eq!(stats.pid, 42);
        assert_eq!(stats.restarts, 3);
        assert_eq!(stats.uptime, Duration::from_secs(5));
    }
}
