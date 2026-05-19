//! [`LifecycleState`] — the single state enum used by both the host registry
//! and the process supervisor.
//!
//! Per SCOPE.md "What each crate / package owns": `starter-ext-host`'s
//! `ExtensionRegistry::state(id)` and `starter-ext-supervisor`'s per-process
//! state machine return values of this enum so dashboards, the
//! `GET /extensions/<id>` route, and the event ring agree on one vocabulary.

use serde::{Deserialize, Serialize};

/// The state of an extension instance.
///
/// The variants form a small state machine, not an arbitrary set:
///
/// - `Discovered` → `Validated` (manifest parsed and validated).
/// - `Validated` → `Starting` (host / supervisor is bringing it up).
/// - `Starting`  → `Running` (init handshake succeeded) **or** `Crashed`
///   (failure during init).
/// - `Running`   → `Stopping` (graceful) → `Stopped`,
///   **or** `Crashed` (abnormal exit / missed health ping),
///   **or** `Failed` (intensity cap exceeded; no further restart).
/// - `Stopped`   → `Starting` (operator re-enabled it).
///
/// `Failed` is terminal until an operator action (re-enable / redeploy)
/// resets the supervisor's restart counter. SCOPE R9: every extension is
/// its own restart unit; there are no supervisor groups in v0.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    /// Manifest seen on disk; not yet validated.
    Discovered,
    /// Manifest validated against the schema, namespace, and capability
    /// rules. Ready to start.
    Validated,
    /// Host / supervisor is bringing the extension up (spawn + init
    /// handshake in progress).
    Starting,
    /// Init handshake succeeded; extension is serving requests.
    Running,
    /// Graceful shutdown in progress (`on_shutdown` running inside the
    /// supervisor's grace window).
    Stopping,
    /// Cleanly stopped. Operator can re-enable.
    Stopped,
    /// Abnormal exit, missed health ping, or panic; supervisor will restart
    /// per policy unless the intensity cap is exceeded.
    Crashed,
    /// Terminal failure (intensity cap exceeded, manifest validation error,
    /// or singleton-mismatch for UI extensions). No automatic restart.
    Failed,
}

impl LifecycleState {
    /// `true` if the supervisor considers this state terminal — no further
    /// transitions happen without an explicit operator action.
    #[inline]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Failed)
    }

    /// `true` if the extension is actively able to serve a request.
    #[inline]
    pub fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_only_failed() {
        for s in [
            LifecycleState::Discovered,
            LifecycleState::Validated,
            LifecycleState::Starting,
            LifecycleState::Running,
            LifecycleState::Stopping,
            LifecycleState::Stopped,
            LifecycleState::Crashed,
        ] {
            assert!(!s.is_terminal(), "{:?} should not be terminal", s);
        }
        assert!(LifecycleState::Failed.is_terminal());
    }

    #[test]
    fn snake_case_wire_form() {
        let j = serde_json::to_string(&LifecycleState::Running).unwrap();
        assert_eq!(j, "\"running\"");
        let back: LifecycleState = serde_json::from_str("\"crashed\"").unwrap();
        assert_eq!(back, LifecycleState::Crashed);
    }
}
