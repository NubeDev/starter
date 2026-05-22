//! Traits for recording [`super::Change`] rows.
//!
//! There is no top-level `record()`. All writes go through
//! [`ChangeRecorder::transaction`] so `group_id` is assigned once and
//! correct grouping is the easy default. See SCOPE §"Causation
//! grouping API".

use async_trait::async_trait;

use crate::Result;

use super::{Change, ChangeId, GroupId};

/// The ONLY way to record changes.
///
/// Implementations assign a fresh [`GroupId`] *before* the closure
/// runs and share it with every [`super::Change`] emitted through
/// the [`ChangeTx`] handle. A failed closure rolls the group back.
#[async_trait]
pub trait ChangeRecorder: Send + Sync {
    /// Run `f` inside a recorder-managed transaction. Every
    /// [`ChangeTx::record`] call inside the closure shares one
    /// [`GroupId`]. If `f` returns `Err`, no rows are persisted.
    async fn transaction<'a>(
        &'a self,
        f: Box<
            dyn for<'tx> FnOnce(
                    &'tx (dyn ChangeTx + 'tx),
                ) -> std::pin::Pin<
                    Box<dyn std::future::Future<Output = Result<()>> + Send + 'tx>,
                > + Send
                + 'a,
        >,
    ) -> Result<()>;
}

/// Transaction handle passed to the closure given to
/// [`ChangeRecorder::transaction`]. Holds the [`GroupId`] assigned to
/// this transaction and accepts [`super::Change`] rows.
#[async_trait]
pub trait ChangeTx: Send + Sync {
    /// The group id shared by every row recorded through this handle.
    fn group_id(&self) -> &GroupId;

    /// Append a single change to the log. Returns the assigned
    /// [`ChangeId`]. Implementations MUST set `ch.group_id` to
    /// `self.group_id()` before persisting.
    async fn record(&self, ch: Change) -> Result<ChangeId>;
}
