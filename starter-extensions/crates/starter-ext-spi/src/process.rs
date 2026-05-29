//! [`ProcessStats`] — live PID + sampled resource usage for a
//! process-flavour extension.
//!
//! The process tab of the admin surface answers "is this thing running,
//! what is its pid, how long has it been up, and how much memory / CPU is
//! it using right now?". Only **process-flavour** extensions
//! ([`RuntimeKind::Process`]) have a host-visible child process — builtin
//! extensions run inside the host (reporting the host pid would be
//! misleading) and wasm components are instantiated in-process. Those
//! flavours report `null` and the UI hides the tab; the
//! [`ProcessFlavour`] discriminator is the contract value the consumer
//! keys that decision on.
//!
//! The numbers are **sampled, not pushed** (plan rule 4): the supervisor's
//! existing health loop reads `/proc/<pid>/stat` + `/statm` on its tick
//! while the child is `Running`, so there is no extra always-on collector
//! thread. `rss_bytes` / `cpu_pct` are best-effort — `None` on platforms
//! without `/proc`, or before the first sample lands.
//!
//! [`RuntimeKind::Process`]: crate::manifest::RuntimeKind::Process

use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::manifest::RuntimeKind;

/// Which packaging flavour a record is, projected down to the single bit
/// the process surface cares about: does it have a host-visible child
/// process whose stats can be sampled?
///
/// This mirrors [`RuntimeKind`] but lives next to [`ProcessStats`] so the
/// process endpoint can decide "report stats vs report `null`" without
/// re-deriving the rule, and so the wire contract for the process tab is
/// self-contained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessFlavour {
    /// Child process spawned by `starter-ext-supervisor`; reports stats.
    Process,
    /// Statically linked into the host; no host-visible child — the host
    /// pid is never reported. Reports `null`.
    Builtin,
    /// WASI component instantiated in-process; no child. Reports `null`.
    Wasm,
}

impl ProcessFlavour {
    /// Whether this flavour has a host-visible child process whose
    /// [`ProcessStats`] can be sampled. Only [`ProcessFlavour::Process`]
    /// does; builtin / wasm report `null` and the UI hides the tab.
    pub const fn reports_process_stats(self) -> bool {
        matches!(self, Self::Process)
    }
}

impl From<RuntimeKind> for ProcessFlavour {
    fn from(kind: RuntimeKind) -> Self {
        match kind {
            RuntimeKind::Process => Self::Process,
            RuntimeKind::Builtin => Self::Builtin,
            RuntimeKind::Wasm => Self::Wasm,
        }
    }
}

/// Live process statistics for a process-flavour extension's current
/// child. Returned by `SupervisorHandle::process_stats` and served by
/// `GET /extensions/<id>/process`; `None` / `404 ext.process.not_running`
/// when the child is not `Running` (builtin, wasm, stopped, failed).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessStats {
    /// OS process id of the current child.
    pub pid: u32,
    /// Wall-clock time the current child was spawned. Resets on restart.
    pub started_at: SystemTime,
    /// How long the current child has been alive (since `started_at`).
    pub uptime: Duration,
    /// Resident set size in bytes, sampled on the last health tick.
    /// `None` on platforms without `/proc`, or before the first sample.
    pub rss_bytes: Option<u64>,
    /// CPU usage as a percentage of one core, averaged over the interval
    /// between the last two health samples. `None` until two samples have
    /// been taken, or on platforms without `/proc`.
    pub cpu_pct: Option<f32>,
    /// Number of times the supervisor has restarted this extension across
    /// the process's lifetime (carried across child respawns).
    pub restarts: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flavour_round_trips_snake_case() {
        assert_eq!(
            serde_json::to_string(&ProcessFlavour::Process).unwrap(),
            "\"process\""
        );
        let f: ProcessFlavour = serde_json::from_str("\"wasm\"").unwrap();
        assert_eq!(f, ProcessFlavour::Wasm);
    }

    #[test]
    fn only_process_reports_stats() {
        assert!(ProcessFlavour::Process.reports_process_stats());
        assert!(!ProcessFlavour::Builtin.reports_process_stats());
        assert!(!ProcessFlavour::Wasm.reports_process_stats());
    }

    #[test]
    fn flavour_from_runtime_kind() {
        assert_eq!(
            ProcessFlavour::from(RuntimeKind::Process),
            ProcessFlavour::Process
        );
        assert_eq!(
            ProcessFlavour::from(RuntimeKind::Builtin),
            ProcessFlavour::Builtin
        );
        assert_eq!(
            ProcessFlavour::from(RuntimeKind::Wasm),
            ProcessFlavour::Wasm
        );
    }

    #[test]
    fn stats_round_trip() {
        let s = ProcessStats {
            pid: 4242,
            started_at: SystemTime::UNIX_EPOCH,
            uptime: Duration::from_secs(12),
            rss_bytes: Some(1024 * 1024),
            cpu_pct: Some(3.5),
            restarts: 2,
        };
        let j = serde_json::to_value(&s).unwrap();
        assert_eq!(j["pid"], 4242);
        assert_eq!(j["rss_bytes"], 1024 * 1024);
        let back: ProcessStats = serde_json::from_value(j).unwrap();
        assert_eq!(back, s);
    }
}
