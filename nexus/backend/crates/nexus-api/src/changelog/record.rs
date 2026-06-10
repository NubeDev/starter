//! The C6 recording helper every mutating handler calls.
//!
//! `record(...)` wraps [`starter_undo::record_if_reversible`] with a tenant-pinned
//! [`nexus_store::changelog::NexusRecorder`], so the row lands in `nexus_changes`
//! under the caller's tenant (RLS-checked). Unregistered kinds are a silent no-op
//! by design, so a handler can call this unconditionally. The call goes right
//! after the successful domain mutation; the handler already holds the `before`
//! (read for authz) and `after` (what it wrote).

use sqlx::PgPool;
use starter_spi::auth::Principal;
use starter_spi::changelog::{Actor, GroupId};
use starter_spi::Result;
use starter_undo::{record_if_reversible, ChangeDraft, ReversibleRegistry};

use nexus_store::changelog::NexusRecorder;

/// Map an authenticated [`Principal`] to a changelog [`Actor`]. AI-agent runs
/// record as `Actor::Agent` (the agent log is the same ledger); everything a
/// human does is `Actor::User { subject }`.
pub fn actor_from(principal: &Principal) -> Actor {
    Actor::User {
        subject: principal.subject.clone(),
    }
}

/// Record `draft` against the caller's tenant if its kind is reversible.
///
/// Returns the assigned `group_id` (for the handler to echo back so the client
/// can target a precise undo) or `None` when the kind is not registered. The
/// recorder opens its own tenant-bound transaction, so this is called *after* the
/// mutation's own transaction commits — a recording failure must not roll back a
/// committed mutation, and is logged rather than surfaced to the client.
pub async fn record(
    registry: &ReversibleRegistry,
    metadata: PgPool,
    tenant_id: &str,
    actor: Actor,
    draft: ChangeDraft,
) -> Result<Option<GroupId>> {
    let recorder = NexusRecorder::new(metadata, tenant_id);
    record_if_reversible(registry, &recorder, actor, draft).await
}
