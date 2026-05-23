//! CPU-usage probe. Reads logical-core count and global CPU
//! utilisation via `sysinfo`, returning a [`CpuUsage`] result.
//!
//! sysinfo needs two refreshes ~`MINIMUM_CPU_UPDATE_INTERVAL` apart
//! to compute non-zero usage; this probe handles the sleep itself
//! so callers get a meaningful number on the first call. Severity
//! decisions live in the consumer, not here.

use std::thread::sleep;

use serde::{Deserialize, Serialize};
use sysinfo::{System, MINIMUM_CPU_UPDATE_INTERVAL};

/// Result of a successful CPU probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuUsage {
    /// Logical CPU cores visible to the process.
    pub logical_cores: u16,
    /// Global CPU utilisation percent across all cores
    /// (0–100, rounded to nearest integer).
    pub percent_used: u8,
}

/// Errors raised while probing.
#[derive(Debug, thiserror::Error)]
pub enum CpuProbeError {
    /// sysinfo reported zero CPUs — impossible on a real host, but
    /// possible inside a misconfigured container or test double.
    #[error("kernel reports zero logical cpus")]
    ZeroCpus,
}

/// Probe global CPU usage. Blocks for `MINIMUM_CPU_UPDATE_INTERVAL`
/// (currently ~200ms) between the two required refreshes.
pub fn cpu_usage() -> Result<CpuUsage, CpuProbeError> {
    let mut sys = System::new();
    sys.refresh_cpu_usage();
    sleep(MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_cpu_usage();

    let cores = sys.cpus().len();
    if cores == 0 {
        return Err(CpuProbeError::ZeroCpus);
    }
    let logical_cores = u16::try_from(cores).unwrap_or(u16::MAX);
    let global = sys.global_cpu_usage();
    let percent = global.round().clamp(0.0, 100.0) as u8;

    Ok(CpuUsage {
        logical_cores,
        percent_used: percent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_usage_returns_sane_values() {
        let usage = cpu_usage().expect("local cpu probe succeeds");
        assert!(usage.logical_cores >= 1, "real hosts have at least one core");
        assert!(usage.percent_used <= 100);
    }
}
