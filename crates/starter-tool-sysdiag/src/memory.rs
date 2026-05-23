//! Memory-usage probe. Reads total / used / available RAM via
//! `sysinfo`, returning a [`MemoryUsage`] result.
//!
//! Severity decisions (warn / full thresholds, MessageKey selection)
//! live in the consumer, not here.

use serde::{Deserialize, Serialize};
use sysinfo::System;

/// Result of a successful memory probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// Total system RAM in bytes.
    pub total_bytes: u64,
    /// Bytes currently in use (`total - available`).
    pub used_bytes: u64,
    /// Bytes available for new allocations (sysinfo's `available_memory`).
    pub available_bytes: u64,
    /// Percent used (0–100, rounded to nearest integer).
    pub percent_used: u8,
}

/// Errors raised while probing.
#[derive(Debug, thiserror::Error)]
pub enum MemoryProbeError {
    /// The kernel reported zero total memory — impossible on a real
    /// host, but possible inside a misconfigured container or test
    /// double.
    #[error("kernel reports zero total memory")]
    ZeroTotal,
}

/// Probe the host's RAM. A single `System` is built per call and
/// refreshed for memory only — cheap on every supported OS.
pub fn memory_usage() -> Result<MemoryUsage, MemoryProbeError> {
    let mut sys = System::new();
    sys.refresh_memory();

    let total = sys.total_memory();
    if total == 0 {
        return Err(MemoryProbeError::ZeroTotal);
    }
    let available = sys.available_memory();
    let used = total.saturating_sub(available);
    let percent = ((used as f64 / total as f64) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u8;

    Ok(MemoryUsage {
        total_bytes: total,
        used_bytes: used,
        available_bytes: available,
        percent_used: percent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_usage_returns_sane_values() {
        let usage = memory_usage().expect("local memory probe succeeds");
        assert!(usage.total_bytes > 0, "real hosts have RAM");
        assert!(usage.used_bytes <= usage.total_bytes);
        assert!(usage.available_bytes <= usage.total_bytes);
        assert!(usage.percent_used <= 100);
    }
}
