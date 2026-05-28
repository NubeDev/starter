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
/// # Choosing snapshot vs patch
///
/// Each [`Reversible`] picks one of two payload shapes per resource
/// kind and stays with it; mixing within a kind is forbidden because
/// undo dispatch becomes per-row guesswork. The choice is mandatory
/// before merging any new impl. Use this matrix:
///
/// | Use **snapshot** (full `before` + `after`) when…             | Use **patch** (RFC 6902 in `patch`) when…                       |
/// |--------------------------------------------------------------|------------------------------------------------------------------|
/// | Resource is small (< ~10 KB serialized)                      | Resource is large and most edits touch a tiny slice              |
/// | Resource has no useful intermediate state                    | Edits are naturally diff-shaped (rename, field flip, cell)       |
/// | Lifecycle includes creation/deletion (`before` may be `{}`)  | Edits never create or destroy the resource                       |
/// | Round-trip cost is dominated by network, not storage         | Storage cost of full snapshots × revision count would dominate   |
///
/// Worked references: `UserReversible` is snapshot (create/disable
/// flip the row in/out of existence); `TeamReversible` is patch
/// (membership flips are diff-shaped, the row itself never
/// vanishes); `FlowDefReversible` stays snapshot today because the
/// unit of change is the whole YAML on deploy — it is the candidate
/// to flip to patch once node-level granularity lands.
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
