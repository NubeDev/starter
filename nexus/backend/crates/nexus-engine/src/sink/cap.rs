//! Bounds on how much a sink may accumulate before it aborts the stream.
//!
//! The collector buffers `RecordBatch`es in process memory, so an unbounded
//! `SELECT *` would exhaust the server. Every one-shot run carries a [`Caps`];
//! when a batch would push the running totals past a limit the sink stops the
//! stream and the result is flagged truncated rather than silently partial.

use std::time::Duration;

/// Hard limits applied to a single bounded run. A `None` field means "no limit
/// on this axis"; [`Caps::unbounded`] disables all three (test/admin use only —
/// production query paths always set at least a row and byte cap).
#[derive(Debug, Clone, Copy)]
pub struct Caps {
    /// Maximum number of rows to collect before truncating.
    pub max_rows: Option<u64>,
    /// Maximum serialized byte size to collect before truncating.
    pub max_bytes: Option<u64>,
    /// Wall-clock budget for the whole run.
    pub max_duration: Option<Duration>,
}

impl Caps {
    /// No bound on any axis. For tests and trusted internal runs only.
    pub fn unbounded() -> Self {
        Self {
            max_rows: None,
            max_bytes: None,
            max_duration: None,
        }
    }

    /// Cap on row count alone.
    pub fn rows(max_rows: u64) -> Self {
        Self {
            max_rows: Some(max_rows),
            max_bytes: None,
            max_duration: None,
        }
    }

    /// The production default: bound rows, bytes, and wall-clock together.
    pub fn new(max_rows: u64, max_bytes: u64, max_duration: Duration) -> Self {
        Self {
            max_rows: Some(max_rows),
            max_bytes: Some(max_bytes),
            max_duration: Some(max_duration),
        }
    }
}

/// Running totals checked against [`Caps`] as batches arrive. Lives behind the
/// sink's lock; `admit` decides whether the next batch fits and records the
/// breach so the runner can report `truncated`.
#[derive(Debug, Default)]
pub struct CapState {
    pub rows: u64,
    pub bytes: u64,
    pub truncated: bool,
}

impl CapState {
    /// Account for a batch of `rows`/`bytes`. Returns `true` if it fits within
    /// the caps and was admitted; `false` if it would breach a limit — in which
    /// case `truncated` is set and the batch must be dropped and the stream
    /// stopped. The row/byte axes are all-or-nothing per batch: a batch that
    /// would cross a limit is rejected whole, so totals never exceed the cap.
    pub fn admit(&mut self, rows: u64, bytes: u64, caps: &Caps) -> bool {
        if let Some(max) = caps.max_rows {
            if self.rows + rows > max {
                self.truncated = true;
                return false;
            }
        }
        if let Some(max) = caps.max_bytes {
            if self.bytes + bytes > max {
                self.truncated = true;
                return false;
            }
        }
        self.rows += rows;
        self.bytes += bytes;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admits_until_row_cap_then_truncates() {
        let caps = Caps::rows(10);
        let mut state = CapState::default();
        assert!(state.admit(6, 100, &caps));
        // 6 + 6 = 12 > 10 → rejected whole, totals stay at 6.
        assert!(!state.admit(6, 100, &caps));
        assert!(state.truncated);
        assert_eq!(state.rows, 6, "a rejected batch does not partially count");
    }

    #[test]
    fn byte_cap_is_independent_of_row_cap() {
        let caps = Caps {
            max_rows: None,
            max_bytes: Some(50),
            max_duration: None,
        };
        let mut state = CapState::default();
        assert!(state.admit(1, 40, &caps));
        assert!(!state.admit(1, 20, &caps));
        assert!(state.truncated);
    }
}
