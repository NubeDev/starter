//! `SessionId` — caller-supplied identifier grouping every event and
//! final result for one AI run.
//!
//! A newtype rather than a bare `String` so the agent-domain id (which
//! the orchestrator chooses and uses to scope locks, traces, and
//! approvals) cannot be silently mixed up with the upstream-CLI session
//! id that some providers return for resume support — that one stays a
//! plain `String` on [`super::RunResult::session_id`].

use serde::{Deserialize, Serialize};

/// Caller-supplied identifier grouping all events emitted during a
/// single run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(String);

impl SessionId {
    /// Borrow the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the wrapper and return the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
