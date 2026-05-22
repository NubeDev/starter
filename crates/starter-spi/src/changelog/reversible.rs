//! The consumer-side extension point.
//!
//! Implemented once per resource kind. This is the ONLY extension
//! point — new resource kind = one [`Reversible`] impl registered at
//! server build time. See SCOPE §"The seam".

use async_trait::async_trait;

use crate::authz::ResourceRef;
use crate::Result;

use super::{Change, ChangeTx};

/// Domain glue that lets a single resource kind participate in undo,
/// redo, duplicate, and paste.
///
/// # Errors
///
/// - Return [`crate::Error::NotFound`] if the target row is gone.
/// - Return [`crate::Error::Conflict`] if `ch.resource_version`
///   doesn't match the current row. The message SHOULD include the
///   observed version so the UI can render a meaningful refusal.
#[async_trait]
pub trait Reversible: Send + Sync {
    /// Stable, machine-readable kind discriminator. Matches
    /// [`ResourceRef::kind`].
    fn kind(&self) -> &'static str;

    /// Undo. Implementations MUST honor `ch.resource_version` when
    /// the resource supports versioning.
    async fn apply_inverse(&self, ch: &Change) -> Result<()>;

    /// Redo / paste.
    async fn apply_forward(&self, ch: &Change) -> Result<()>;

    /// Duplicate / paste-as-new.
    ///
    /// Returns `Vec<ResourceRef>` because a composite resource
    /// (dashboard + widgets, doc + sections) maps to N new rows.
    /// The implementation is responsible for emitting one
    /// [`ChangeTx::record`] per new row so they all share one
    /// `group_id` and undo collapses them into a single step.
    async fn clone_with(
        &self,
        tx: &dyn ChangeTx,
        src: &ResourceRef,
        overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>>;
}
