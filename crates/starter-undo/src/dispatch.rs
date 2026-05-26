//! Reversible-aware dispatch helper.
//!
//! Tool dispatchers call [`record_if_reversible`] right after a
//! successful domain mutation. When the resource kind has a
//! [`starter_spi::changelog::Reversible`] impl registered in the
//! [`crate::ReversibleRegistry`], the draft is appended through the
//! supplied [`starter_spi::changelog::ChangeRecorder`] so a later
//! [`crate::UndoService::undo`] can replay it via
//! [`starter_spi::changelog::Reversible::apply_inverse`]. Kinds that
//! are *not* registered return [`None`] and are silently skipped —
//! by design, read-only or non-undoable verbs do not record.
//!
//! See `DOCS/backend/undo-redo/SCOPE.md` §"The seam" R2.

use std::sync::Arc;

use chrono::Utc;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Actor, Change, ChangeId, ChangeRecorder, GroupId, Op, TraceId};
use starter_spi::Result;
use tokio::sync::Mutex;

use crate::registry::ReversibleRegistry;

/// Tool-supplied draft of a [`Change`]. The recorder owns
/// `id`/`at`/`group_id` assignment, so the fields here describe the
/// mutation only.
#[derive(Debug, Clone)]
pub struct ChangeDraft {
    /// Resource that was mutated; `kind` is matched against the
    /// registry.
    pub resource: ResourceRef,
    /// What kind of mutation occurred.
    pub op: Op,
    /// Snapshot of the row *before* the mutation (drives undo).
    pub before: Option<serde_json::Value>,
    /// Snapshot of the row *after* the mutation (drives redo).
    pub after: Option<serde_json::Value>,
    /// Optimistic-concurrency token taken at read time.
    pub resource_version: Option<u64>,
    /// Optional external trace id (HTTP request id, agent run id, …).
    pub correlation: Option<TraceId>,
}

impl ChangeDraft {
    /// Convenience builder for an [`Op::Update`] draft with a
    /// before/after snapshot pair.
    pub fn update(
        resource: ResourceRef,
        before: serde_json::Value,
        after: serde_json::Value,
    ) -> Self {
        Self {
            resource,
            op: Op::Update,
            before: Some(before),
            after: Some(after),
            resource_version: None,
            correlation: None,
        }
    }
}

/// Record `draft` if the resource kind is registered as reversible.
///
/// Returns `Ok(Some(group_id))` when the draft was persisted (so the
/// caller can echo the `group_id` back to the client for a precise
/// undo target) and `Ok(None)` when the kind is not registered.
pub async fn record_if_reversible(
    registry: &ReversibleRegistry,
    recorder: &dyn ChangeRecorder,
    actor: Actor,
    draft: ChangeDraft,
) -> Result<Option<GroupId>> {
    if registry.get(&draft.resource.kind).is_none() {
        return Ok(None);
    }

    // Capture the group_id the recorder assigns so callers can
    // surface it to the client without a second query.
    let captured: Arc<Mutex<Option<GroupId>>> = Arc::new(Mutex::new(None));
    let captured_ref = captured.clone();
    // Move-into-closure prep: `transaction` consumes its closure.
    let actor_for_tx = actor;
    let draft_for_tx = draft;

    recorder
        .transaction(Box::new(move |tx| {
            let captured_ref = captured_ref.clone();
            let actor = actor_for_tx.clone();
            let draft = draft_for_tx.clone();
            Box::pin(async move {
                let group = tx.group_id().clone();
                let ch = Change {
                    // Recorder overrides id + group_id; placeholders
                    // are fine — see SqliteChangeTx::record.
                    id: ChangeId(String::new()),
                    at: Utc::now(),
                    actor,
                    resource: draft.resource,
                    resource_version: draft.resource_version,
                    op: draft.op,
                    before: draft.before,
                    after: draft.after,
                    patch: None,
                    group_id: group.clone(),
                    correlation: draft.correlation,
                };
                tx.record(ch).await?;
                *captured_ref.lock().await = Some(group);
                Ok(())
            })
        }))
        .await?;

    let group = captured.lock().await.clone();
    Ok(group)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use starter_spi::changelog::{Change, ChangeTx, Reversible};
    use std::sync::Mutex as StdMutex;

    struct FakeReversible {
        kind: &'static str,
        seen: Arc<StdMutex<Vec<Change>>>,
    }

    #[async_trait]
    impl Reversible for FakeReversible {
        fn kind(&self) -> &'static str {
            self.kind
        }
        async fn apply_inverse(&self, ch: &Change) -> Result<()> {
            self.seen.lock().unwrap().push(ch.clone());
            Ok(())
        }
        async fn apply_forward(&self, ch: &Change) -> Result<()> {
            self.seen.lock().unwrap().push(ch.clone());
            Ok(())
        }
        async fn clone_with(
            &self,
            _tx: &dyn ChangeTx,
            _src: &ResourceRef,
            _overrides: serde_json::Value,
        ) -> Result<Vec<ResourceRef>> {
            Ok(vec![])
        }
    }

    /// Minimal in-memory recorder that captures every recorded
    /// [`Change`] and assigns a deterministic group id per
    /// transaction. Lives in this unit test only — production
    /// recorders are exercised in the rubix-agent integration test.
    #[derive(Default)]
    struct CapturingRecorder {
        groups: StdMutex<u64>,
        rows: Arc<StdMutex<Vec<Change>>>,
    }

    struct CapturingTx {
        group_id: GroupId,
        rows: Arc<StdMutex<Vec<Change>>>,
    }

    #[async_trait]
    impl ChangeTx for CapturingTx {
        fn group_id(&self) -> &GroupId {
            &self.group_id
        }
        async fn record(&self, mut ch: Change) -> Result<ChangeId> {
            let id = ChangeId(format!("ch-{}", self.rows.lock().unwrap().len()));
            ch.id = id.clone();
            ch.group_id = self.group_id.clone();
            self.rows.lock().unwrap().push(ch);
            Ok(id)
        }
    }

    #[async_trait]
    impl ChangeRecorder for CapturingRecorder {
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
        ) -> Result<()> {
            let n = {
                let mut g = self.groups.lock().unwrap();
                *g += 1;
                *g
            };
            let tx = CapturingTx {
                group_id: GroupId(format!("grp-{n}")),
                rows: self.rows.clone(),
            };
            f(&tx).await
        }
        async fn forget(&self, _resource: &ResourceRef) -> Result<u64> {
            Ok(0)
        }
    }

    #[tokio::test]
    async fn registered_kind_round_trips_through_recorder_and_reversible() {
        let seen = Arc::new(StdMutex::new(Vec::new()));
        let fake = Arc::new(FakeReversible {
            kind: "widgets",
            seen: seen.clone(),
        });
        let registry = ReversibleRegistry::new().insert(fake.clone());

        let recorder = CapturingRecorder::default();
        let actor = Actor::User {
            subject: "alice".into(),
        };
        let draft = ChangeDraft::update(
            ResourceRef {
                kind: "widgets".into(),
                id: Some("w-1".into()),
                owner: None,
                tenant: None,
            },
            serde_json::json!({"name": "old"}),
            serde_json::json!({"name": "new"}),
        );

        let group = record_if_reversible(&registry, &recorder, actor.clone(), draft)
            .await
            .expect("record succeeds")
            .expect("kind is registered so a group id is returned");

        // One row recorded, carrying the right group, kind, and snapshots.
        let rows = recorder.rows.lock().unwrap().clone();
        assert_eq!(rows.len(), 1, "exactly one row is recorded");
        assert_eq!(rows[0].group_id, group);
        assert_eq!(rows[0].resource.kind, "widgets");
        assert_eq!(rows[0].before, Some(serde_json::json!({"name": "old"})));
        assert_eq!(rows[0].after, Some(serde_json::json!({"name": "new"})));

        // Round-trip: feed the recorded row back through the Reversible
        // we registered. The fake captures the call so we assert dispatch
        // reached the right impl.
        fake.apply_inverse(&rows[0]).await.expect("inverse runs");
        let captured = seen.lock().unwrap().clone();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].resource.kind, "widgets");
    }

    #[tokio::test]
    async fn unregistered_kind_is_skipped() {
        let registry = ReversibleRegistry::new();
        let recorder = CapturingRecorder::default();
        let draft = ChangeDraft::update(
            ResourceRef {
                kind: "unknown".into(),
                id: Some("x".into()),
                owner: None,
                tenant: None,
            },
            serde_json::json!({}),
            serde_json::json!({}),
        );

        let result = record_if_reversible(&registry, &recorder, Actor::System, draft)
            .await
            .expect("never errors on unregistered kinds");
        assert!(result.is_none(), "unregistered kinds return None");
        assert!(
            recorder.rows.lock().unwrap().is_empty(),
            "and never reach the recorder",
        );
    }
}
