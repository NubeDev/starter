//! [`ServiceHandle`] — newtype around the `JoinHandle` a started
//! service hands back.

use tokio::task::JoinHandle;

use crate::error::Result;

/// Opaque handle to a running [`Service`](super::Service).
///
/// Wraps the spawned `JoinHandle` so the registry can await it on
/// shutdown without callers reaching for raw `tokio::task` types.
pub struct ServiceHandle {
    /// The spawned task's join handle. Resolves to `Ok(())` on clean
    /// exit, `Err(_)` if the service's loop returned an error.
    pub join: JoinHandle<Result<()>>,
}

impl ServiceHandle {
    /// Convenience constructor.
    pub fn new(join: JoinHandle<Result<()>>) -> Self {
        Self { join }
    }
}
