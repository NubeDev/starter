//! Process-group teardown + cross-restart orphan reaping.
//!
//! ## Why this module exists
//!
//! The v0.1 supervisor relied solely on tokio's `kill_on_drop(true)` to
//! terminate a child. That only fires on a *graceful* tokio `Drop`. Two
//! real-world paths skip it and leak processes:
//!
//! 1. **The agent is `SIGKILL`ed** (e.g. `make reload`, OOM, a crash). The
//!    tokio runtime never unwinds, `Drop` never runs, and the live child —
//!    plus any grandchildren it spawned — is reparented to init and wedges
//!    forever on its stdio pipes. Over days this accumulates hundreds of
//!    orphans (the bug this module fixes).
//! 2. **A crash path that `start_kill()`s without `wait()`** leaves a
//!    zombie the supervisor never reaps before respawning.
//!
//! `kill_on_drop` also only ever targets the *direct* child — never a
//! grandchild. A child that itself forks a worker leaks the worker even on
//! the happy Drop path.
//!
//! ## The fix, in two halves
//!
//! - **Process groups.** Every child is spawned into its own process group
//!   (`Command::process_group(0)` — a *safe*, stable std API that calls
//!   `setpgid(0, 0)` in the child before `exec`, so we keep the crate's
//!   `#![forbid(unsafe_code)]`). The group id equals the child pid. Teardown
//!   signals the whole group via [`signal_group`] (`killpg`), so a wedged
//!   grandchild dies with its parent.
//! - **Pidfiles + a boot reaper.** On spawn we record the group's pgid to a
//!   per-extension pidfile under a host-owned directory. On the *next* agent
//!   boot, [`reap_stale_groups`] reads those files and `killpg`s any group
//!   still alive from a prior, now-dead agent instance — the deterministic
//!   cleanup that survives a `SIGKILL`ed parent. A pidfile is the mechanism
//!   (not a fragile "scan for processes named `*-extension` parented to
//!   init", which would also nuke a co-running second agent's children).
//!
//! Everything here is `#[cfg(unix)]`. On non-unix the public surface still
//! exists but degrades to tokio's `start_kill` (handled by the caller) and
//! the pidfile/reaper become no-ops — process groups are a POSIX concept.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One reaped group, surfaced by [`reap_stale_groups`] so the host can log
/// and the metrics surface can count what a prior instance leaked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReapedGroup {
    /// Extension id the stale pidfile was recorded for.
    pub extension_id: String,
    /// Process-group id (== the original child pid) that was signalled.
    pub pgid: i32,
    /// Whether the group was still alive (true → we sent `SIGKILL`; false →
    /// already gone, we only removed the stale pidfile).
    pub was_alive: bool,
}

/// Outcome of a boot reap pass. `groups` is every pidfile we acted on;
/// `killed` is the subset that was still alive. Both are surfaced to the
/// caller for logging and the `orphans_reaped_total` metric.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReapReport {
    /// Every stale pidfile processed this pass.
    pub groups: Vec<ReapedGroup>,
}

impl ReapReport {
    /// Number of groups that were still alive and got `SIGKILL`ed — the
    /// "we actually cleaned up a leak" count.
    pub fn killed(&self) -> usize {
        self.groups.iter().filter(|g| g.was_alive).count()
    }

    /// Total stale pidfiles processed (alive + already-dead).
    pub fn total(&self) -> usize {
        self.groups.len()
    }
}

/// The on-disk pidfile body. Versioned so a future field (start time, a
/// liveness token) is additive. `pgid` is the negative-of-this we pass to
/// `killpg`; we store the positive group id.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PidfileBody {
    /// Schema version. 1 for this layout.
    v: u8,
    /// Extension id (mirrors the filename; carried in-body so a moved file
    /// is still self-describing).
    extension_id: String,
    /// Process-group id == the original child pid.
    pgid: i32,
}

/// Filesystem location of an extension's pidfile inside `dir`.
fn pidfile_path(dir: &Path, extension_id: &str) -> PathBuf {
    // Sanitise the id into a filesystem-safe stem. Extension ids are
    // reverse-DNS (`com.nubeio.rubixos`) so only `.` and alnum appear in
    // practice, but we defend against a stray separator regardless.
    let stem: String = extension_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    dir.join(format!("{stem}.pid"))
}

/// Record a spawned child's process group to a pidfile under `dir`.
///
/// Best-effort: a write failure is logged by the caller and never blocks a
/// spawn — the pidfile is a cleanup *optimisation* for the next boot, not a
/// correctness invariant for this run (this run has the live handle + group
/// signalling). Creates `dir` if absent.
pub fn write_pidfile(dir: &Path, extension_id: &str, pgid: i32) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let body = PidfileBody {
        v: 1,
        extension_id: extension_id.to_string(),
        pgid,
    };
    let json = serde_json::to_vec(&body)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let path = pidfile_path(dir, extension_id);
    // Write-then-rename so a concurrent reader never sees a torn file.
    let tmp = path.with_extension("pid.tmp");
    std::fs::write(&tmp, &json)?;
    std::fs::rename(&tmp, &path)
}

/// Remove an extension's pidfile (called when a group is torn down cleanly
/// this run, so the next boot has nothing stale to reap). Best-effort.
pub fn remove_pidfile(dir: &Path, extension_id: &str) {
    let _ = std::fs::remove_file(pidfile_path(dir, extension_id));
}

// ---------------------------------------------------------------------------
// Unix: real process-group signalling + boot reaper.
// ---------------------------------------------------------------------------

/// Signal an entire process group. `pgid` is the positive group id (the
/// original child pid); we negate it for `killpg`. Returns `true` if the
/// group existed and was signalled, `false` if it was already gone
/// (`ESRCH`). Other errors (`EPERM`) are treated as "not ours / gone".
#[cfg(unix)]
pub fn signal_group(pgid: i32, sig: nix::sys::signal::Signal) -> bool {
    use nix::errno::Errno;
    use nix::unistd::Pid;
    match nix::sys::signal::killpg(Pid::from_raw(pgid), sig) {
        Ok(()) => true,
        Err(Errno::ESRCH) => false,
        Err(_) => false,
    }
}

/// `true` if the process group is still alive (probe with signal 0).
#[cfg(unix)]
pub fn group_alive(pgid: i32) -> bool {
    use nix::errno::Errno;
    use nix::unistd::Pid;
    // `kill(-pgid, 0)` probes the group without delivering a signal.
    matches!(
        nix::sys::signal::kill(Pid::from_raw(-pgid), None),
        Ok(()) | Err(Errno::EPERM)
    )
}

/// Boot-time reaper: read every pidfile under `dir`, and for each group that
/// is still alive from a prior agent instance, send `SIGKILL` to the whole
/// group. Removes the pidfile afterwards regardless. Returns a [`ReapReport`]
/// the caller logs + counts.
///
/// This is the cross-`SIGKILL` safety net: if the previous agent was
/// `SIGKILL`ed (so its `Drop`/group teardown never ran), its children are
/// still alive and reparented to init — but their pgids are on disk, so we
/// can deterministically kill exactly those groups without touching an
/// unrelated co-running agent's children.
#[cfg(unix)]
pub fn reap_stale_groups(dir: &Path) -> ReapReport {
    use nix::sys::signal::Signal;

    let mut report = ReapReport::default();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        // No dir → nothing was ever spawned by a prior instance.
        Err(_) => return report,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("pid") {
            continue;
        }
        let body: PidfileBody = match std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
        {
            Some(b) => b,
            None => {
                // Unreadable / torn pidfile — drop it and move on.
                let _ = std::fs::remove_file(&path);
                continue;
            }
        };
        let was_alive = group_alive(body.pgid);
        if was_alive {
            signal_group(body.pgid, Signal::SIGKILL);
        }
        report.groups.push(ReapedGroup {
            extension_id: body.extension_id,
            pgid: body.pgid,
            was_alive,
        });
        let _ = std::fs::remove_file(&path);
    }
    report
}

#[cfg(not(unix))]
pub fn reap_stale_groups(_dir: &Path) -> ReapReport {
    ReapReport::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pidfile_round_trips_and_reaps_dead_group() {
        let dir = tempfile::tempdir().unwrap();
        // A pgid that is almost certainly dead (i32::MAX as a group).
        let dead_pgid = i32::MAX;
        write_pidfile(dir.path(), "com.acme.ext", dead_pgid).unwrap();
        assert!(pidfile_path(dir.path(), "com.acme.ext").exists());

        let report = reap_stale_groups(dir.path());
        assert_eq!(report.total(), 1);
        assert_eq!(report.groups[0].extension_id, "com.acme.ext");
        assert_eq!(report.groups[0].pgid, dead_pgid);
        // Dead group → not alive → not counted as killed, pidfile removed.
        assert!(!report.groups[0].was_alive);
        assert_eq!(report.killed(), 0);
        assert!(!pidfile_path(dir.path(), "com.acme.ext").exists());
    }

    #[test]
    fn reap_on_missing_dir_is_empty() {
        let report = reap_stale_groups(Path::new("/nonexistent/reaper/dir/xyz"));
        assert_eq!(report.total(), 0);
        assert_eq!(report.killed(), 0);
    }

    #[test]
    fn remove_pidfile_clears_it() {
        let dir = tempfile::tempdir().unwrap();
        write_pidfile(dir.path(), "com.acme.b", 12345).unwrap();
        assert!(pidfile_path(dir.path(), "com.acme.b").exists());
        remove_pidfile(dir.path(), "com.acme.b");
        assert!(!pidfile_path(dir.path(), "com.acme.b").exists());
    }

    #[test]
    fn id_with_separators_is_sanitised() {
        let dir = tempfile::tempdir().unwrap();
        let p = pidfile_path(dir.path(), "a/b\\c:d");
        assert_eq!(p.file_name().unwrap().to_str().unwrap(), "a_b_c_d.pid");
    }

    #[cfg(unix)]
    #[test]
    fn torn_pidfile_is_removed_not_reaped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("garbage.pid"), b"not json").unwrap();
        let report = reap_stale_groups(dir.path());
        assert_eq!(report.total(), 0);
        assert!(!dir.path().join("garbage.pid").exists());
    }
}
