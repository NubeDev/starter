//! Bounded ring buffer per extension.
//!
//! Per SCOPE.md "What each crate / package owns — `starter-ext-supervisor`":
//!
//! > `EventRing` — bounded ring buffer (default 1000 entries) per extension
//! > capturing state transitions, crash reasons, restart counts, and the
//! > last N stderr lines. Surfaced by `starter-ext-server` at
//! > `/extensions/<id>/events`. Free diagnostics; no IO on the hot path.
//!
//! Implementation is a `VecDeque<Event>` behind a `Mutex` so multiple
//! supervisor tasks (the I/O reader, the health pinger, the exit-watcher)
//! can append concurrently. Reads (for the admin endpoint) take a snapshot
//! under the lock and release it; pagination happens above this layer.
//!
//! The ring is intentionally typed (an `EventKind` enum + payload) rather
//! than just `String` so the admin endpoint can filter — operators
//! diagnosing a crash loop want "show me only the `Crashed` events", not
//! a grep across free-form text.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use starter_ext_spi::LifecycleState;

/// Kind of event recorded in the ring. Free to extend additively within a
/// minor — admin UIs that don't know a new kind render it generically.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum EventKind {
    /// Lifecycle transition the supervisor published into the registry.
    StateTransition {
        /// New lifecycle state.
        to: LifecycleState,
    },
    /// Child process spawned successfully. Carries the OS pid for
    /// operator-level diagnostics.
    Spawned {
        /// OS pid of the spawned child.
        pid: u32,
    },
    /// Child exited cleanly (exit code 0 or otherwise normal).
    ExitedClean {
        /// Exit code if observable; `None` for signal-terminated processes
        /// on platforms that don't surface one.
        code: Option<i32>,
    },
    /// Child crashed. Reason is free-form — "non-zero exit", "killed
    /// after health timeout", "spawn refused: manifest hash mismatch".
    Crashed {
        /// Human-readable reason. Surfaced verbatim by the admin endpoint.
        reason: String,
    },
    /// Supervisor scheduled a restart with a wait window.
    RestartScheduled {
        /// Wait duration before the next spawn, in milliseconds.
        wait_ms: u64,
        /// Cumulative restart count.
        total: u64,
    },
    /// Intensity cap exceeded; the supervisor will not restart again.
    RestartCapExceeded {
        /// Number of restarts seen in the cap window.
        count: u32,
    },
    /// Missed health ping; treated as a crash by the restart tracker.
    HealthTimeout,
    /// A capability-gate refusal at the JSON-RPC wire boundary (R8).
    CapabilityViolation {
        /// Host method the child attempted to call.
        method: String,
    },
    /// Forwarded stderr line. Captured up to `MAX_STDERR_LINES_PER_EVENT`
    /// characters so a misbehaving child cannot wedge the ring.
    Stderr {
        /// The stderr line (trimmed of trailing newlines).
        line: String,
    },
}

/// Maximum characters captured for one `Stderr` event. A child that
/// streams megabytes of stack traces still fits inside the ring.
pub const MAX_STDERR_LINE_BYTES: usize = 1024;

/// One ring entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    /// Wall-clock time the event was recorded.
    pub at: SystemTime,
    /// Monotonically-increasing per-ring sequence number. Surfaced so
    /// `GET /extensions/<id>/events?after=<seq>` (and its SSE live-tail
    /// upgrade) can resume cleanly across reconnects without depending
    /// on wall-clock equality. Never re-used: the counter is monotone
    /// even when older entries fall off the front of the ring.
    pub seq: u64,
    /// Typed event payload.
    pub kind: EventKind,
}

/// Default capacity. Matches SCOPE.md ("default 1000 entries").
pub const DEFAULT_CAPACITY: usize = 1000;

/// Bounded ring buffer. Cheap to clone the snapshot; appends are O(1)
/// amortised behind a `Mutex`.
#[derive(Debug)]
pub struct EventRing {
    inner: Mutex<RingInner>,
    capacity: usize,
}

#[derive(Debug)]
struct RingInner {
    queue: VecDeque<Event>,
    /// Total number of pushes ever seen (monotone; survives ring eviction).
    next_seq: u64,
}

impl EventRing {
    /// Build a ring with the [`DEFAULT_CAPACITY`].
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Build a ring with a custom capacity. Minimum is 1 — a zero-capacity
    /// ring would silently drop every push, which is rarely what the
    /// caller meant.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: Mutex::new(RingInner {
                queue: VecDeque::with_capacity(cap.max(1)),
                next_seq: 0,
            }),
            capacity: cap.max(1),
        }
    }

    /// Append. Drops the oldest entry once the ring is full.
    pub fn push(&self, kind: EventKind) {
        let mut inner = self.inner.lock().expect("event ring mutex poisoned");
        let seq = inner.next_seq;
        inner.next_seq = inner.next_seq.wrapping_add(1);
        let event = Event {
            at: SystemTime::now(),
            seq,
            kind,
        };
        if inner.queue.len() == self.capacity {
            inner.queue.pop_front();
        }
        inner.queue.push_back(event);
    }

    /// Snapshot every entry, oldest first. Bounded by capacity; safe to
    /// hand to the admin endpoint.
    pub fn snapshot(&self) -> Vec<Event> {
        self.inner
            .lock()
            .expect("event ring mutex poisoned")
            .queue
            .iter()
            .cloned()
            .collect()
    }

    /// Snapshot every entry whose `seq` is strictly greater than `after`,
    /// oldest first. Used by the live-tail SSE upgrade in
    /// `starter-ext-server` to resume from a known cursor without
    /// re-emitting history.
    pub fn since(&self, after: u64) -> Vec<Event> {
        self.inner
            .lock()
            .expect("event ring mutex poisoned")
            .queue
            .iter()
            .filter(|e| e.seq > after)
            .cloned()
            .collect()
    }

    /// The sequence number the *next* push will receive. Equivalent to
    /// "total pushes seen so far". The admin endpoint exposes this as
    /// the `next_seq` cursor in paginated responses.
    pub fn next_seq(&self) -> u64 {
        self.inner
            .lock()
            .expect("event ring mutex poisoned")
            .next_seq
    }

    /// Number of entries currently in the ring.
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .expect("event ring mutex poisoned")
            .queue
            .len()
    }

    /// `true` when no events have been recorded.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for EventRing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_drops_oldest_at_capacity() {
        let ring = EventRing::with_capacity(3);
        for i in 0..5 {
            ring.push(EventKind::Spawned { pid: i });
        }
        let snap = ring.snapshot();
        assert_eq!(snap.len(), 3);
        assert!(matches!(snap[0].kind, EventKind::Spawned { pid: 2 }));
        assert!(matches!(snap[2].kind, EventKind::Spawned { pid: 4 }));
    }

    #[test]
    fn state_transition_round_trips_json() {
        let e = EventKind::StateTransition {
            to: LifecycleState::Running,
        };
        let j = serde_json::to_value(&e).unwrap();
        assert_eq!(j["kind"], "state_transition");
        let back: EventKind = serde_json::from_value(j).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn seq_is_monotone_and_survives_eviction() {
        let ring = EventRing::with_capacity(3);
        for i in 0..5 {
            ring.push(EventKind::Spawned { pid: i });
        }
        let snap = ring.snapshot();
        assert_eq!(snap[0].seq, 2);
        assert_eq!(snap[1].seq, 3);
        assert_eq!(snap[2].seq, 4);
        assert_eq!(ring.next_seq(), 5);
        assert_eq!(ring.since(2).len(), 2);
        assert_eq!(ring.since(4).len(), 0);
    }
}
