//! Errors raised by the local SQLite offline queue. Disjoint from agent
//! transport errors: a queue failure is local-disk, an agent failure is
//! network — the UI treats them differently.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum QueueError {
    /// Could not resolve the app data dir to place the DB file.
    #[error("could not resolve app data dir: {0}")]
    DataDir(String),

    /// rusqlite open / migrate / statement failure.
    #[error("sqlite error: {0}")]
    Sqlite(String),

    /// A queued payload column held text that is not valid JSON.
    #[error("invalid queued payload json: {0}")]
    Payload(String),
}

impl From<rusqlite::Error> for QueueError {
    fn from(e: rusqlite::Error) -> Self {
        QueueError::Sqlite(e.to_string())
    }
}
