//! The single error type Tauri commands return. It folds the two
//! domain errors (agent transport, local queue) into one shape that
//! serialises to the frontend as `{ kind, message }`, so the UI can
//! branch on `kind` (e.g. show "go online to sync" for `agent`) without
//! string-matching the message.

use serde::Serialize;

use crate::agent::error::AgentError;
use crate::queue::error::QueueError;

/// What broke, coarse enough for the UI to branch on.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppErrorKind {
    /// Talking to the rubix-agent failed (network, auth, agent status).
    Agent,
    /// Local SQLite offline queue failed (disk, migration, bad payload).
    Queue,
    /// The caller's input was malformed before any IO happened.
    Input,
}

/// Serialised to the frontend as `{ kind, message }`.
#[derive(Debug, Serialize)]
pub struct AppError {
    pub kind: AppErrorKind,
    pub message: String,
}

impl AppError {
    /// Build an input-validation error (bad params from the UI).
    pub fn input(message: impl Into<String>) -> Self {
        Self {
            kind: AppErrorKind::Input,
            message: message.into(),
        }
    }
}

impl From<AgentError> for AppError {
    fn from(e: AgentError) -> Self {
        Self {
            kind: AppErrorKind::Agent,
            message: e.to_string(),
        }
    }
}

impl From<QueueError> for AppError {
    fn from(e: QueueError) -> Self {
        Self {
            kind: AppErrorKind::Queue,
            message: e.to_string(),
        }
    }
}
