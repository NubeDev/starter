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
/// # Payload contract
///
/// A [`Change`] carries three optional payload columns —
/// [`Change::before`], [`Change::after`], and [`Change::patch`].
/// Implementations MUST be prepared to handle any of these shapes:
///
/// - **Snapshot-only** (`before` + `after` set, `patch` is `None`).
///   The default shape today. `apply_inverse` writes `before`;
///   `apply_forward` writes `after`.
/// - **Patch-only** (`patch` set, `before` / `after` are `None`).
///   Reserved for a future size optimization once a real consumer
///   asks for it (see SCOPE §"Open questions" #1). When this lands,
///   `apply_inverse` reverses the patch and `apply_forward` reapplies
///   it; reconstructing the absolute state may require walking back
///   to the previous snapshot row in the changelog.
/// - **Both** (snapshot **and** patch set). Permitted so a recorder
///   can opportunistically include a patch alongside the snapshot
///   without breaking older consumers. Prefer the snapshot if both
///   are present — it's order-independent.
///
/// Today only the first shape is produced by the bundled recorders.
/// Pinning the contract here means a future
/// `PatchingChangeRecorder` can land without a trait-level breaking
/// change.
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
