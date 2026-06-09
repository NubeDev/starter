//! Changelog recording for the folder kind (C6 / WS-12).
//!
//! Keeps the create/update/delete handlers thin: each calls one of these after
//! its successful store mutation. A recording failure is logged, never surfaced —
//! a committed mutation must not be rolled back because the audit write tripped.

use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::auth::Principal;
use starter_spi::changelog::Op;
use starter_undo::ChangeDraft;

use crate::authz::KIND_FOLDER;
use crate::changelog::{actor_from, record};
use crate::reversible::folder_snapshot_json;
use crate::state::AppState;
use nexus_store::folder::FolderRecord;

/// JSON snapshot of a folder, matching what the folder `Reversible` decodes.
/// Delegates to the reversible's encoder so the producing and consuming sides
/// never drift apart.
pub fn snapshot(rec: &FolderRecord) -> Value {
    folder_snapshot_json(rec)
}

/// Record a folder mutation under the caller's tenant. `before`/`after` follow the
/// op convention: create has only `after`, delete only `before`, update both.
pub async fn record_folder(
    state: &AppState,
    principal: &Principal,
    tenant: &str,
    op: Op,
    id: &str,
    before: Option<Value>,
    after: Option<Value>,
) {
    let resource = ResourceRef::row(KIND_FOLDER, id).with_tenant(tenant);
    let draft = ChangeDraft {
        resource,
        op,
        before,
        after,
        resource_version: None,
        correlation: None,
    };
    if let Err(e) = record(
        &state.changelog.registry,
        state.metadata.clone(),
        tenant,
        actor_from(principal),
        draft,
    )
    .await
    {
        tracing::warn!(error = %e, "failed to record folder change");
    }
}
