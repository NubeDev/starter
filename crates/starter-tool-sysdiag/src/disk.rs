//! Disk-usage probe. Reads bytes-total / bytes-free for a given
//! mount point via `sysinfo`, returning a [`DiskUsage`] result.
//!
//! Severity decisions (warn / full thresholds, MessageKey selection)
//! live in the consumer, not here. A consumer wraps this probe in its
//! own Tool impl; see `rubix-tools::system::disk` for one example.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sysinfo::{Disk, Disks};

/// Result of a successful disk probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    /// Mount point selected by the probe (matches `target` when an
    /// explicit path was passed; otherwise the disk hosting the
    /// agent's CWD).
    pub mount: String,
    /// Total bytes on the filesystem.
    pub total_bytes: u64,
    /// Free bytes available to the calling user.
    pub free_bytes: u64,
    /// Percent used (0–100, rounded to nearest integer).
    pub percent_used: u8,
}

/// Errors raised while probing.
#[derive(Debug, thiserror::Error)]
pub enum DiskProbeError {
    /// The agent's current working directory could not be read (used
    /// as the default target when none is supplied).
    #[error("cannot read current working directory: {source}")]
    CwdUnavailable {
        /// The underlying io error.
        #[source]
        source: std::io::Error,
    },

    /// No mounted filesystem contains the requested target path.
    #[error("no filesystem found containing {target}")]
    NoMountForTarget {
        /// The path the probe was asked to inspect.
        target: PathBuf,
    },
}

/// Probe the filesystem hosting `target`. When `target` is `None`,
/// the probe inspects the disk containing the current working
/// directory.
pub fn disk_usage(target: Option<&Path>) -> Result<DiskUsage, DiskProbeError> {
    let target: PathBuf = match target {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir()
            .map_err(|source| DiskProbeError::CwdUnavailable { source })?,
    };

    let disks = Disks::new_with_refreshed_list();
    let disk = pick_disk(&disks, &target)
        .ok_or_else(|| DiskProbeError::NoMountForTarget { target: target.clone() })?;

    let total = disk.total_space();
    let free = disk.available_space();
    let used = total.saturating_sub(free);
    let percent = if total == 0 {
        0
    } else {
        ((used as f64 / total as f64) * 100.0).round().clamp(0.0, 100.0) as u8
    };

    Ok(DiskUsage {
        mount: disk.mount_point().display().to_string(),
        total_bytes: total,
        free_bytes: free,
        percent_used: percent,
    })
}

fn pick_disk<'a>(disks: &'a Disks, target: &Path) -> Option<&'a Disk> {
    let mut best: Option<(&Disk, usize)> = None;
    for d in disks.list() {
        let mp = d.mount_point();
        if target.starts_with(mp) {
            let len = mp.as_os_str().len();
            if best.as_ref().is_none_or(|(_, b)| len > *b) {
                best = Some((d, len));
            }
        }
    }
    best.map(|(d, _)| d).or_else(|| disks.list().first())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_usage_default_target_returns_a_mount() {
        let usage = disk_usage(None).expect("local disk probe succeeds");
        assert!(usage.total_bytes > 0, "any real filesystem reports bytes");
        assert!(usage.free_bytes <= usage.total_bytes);
        assert!(usage.percent_used <= 100);
        assert!(!usage.mount.is_empty());
    }
}
