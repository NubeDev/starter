//! In-memory [`FlowDefStore`] backing + [`Reversible`] glue for
//! the flow-programmer verbs.
//!
//! The trait + the wire types ([`FlowDefStore`],
//! [`FlowRevisionRow`], [`FlowDefChange`], [`FLOW_DEFINITION_KIND`])
//! live in [`rubix_spi::flow_def`] so the production PG impl in
//! `rubix-store-postgres::flows` can target them without taking
//! a dependency on this verb crate. The names are re-exported
//! from this module so existing `use` sites keep working.
//!
//! [`FlowDefReversible`] is the single `Reversible` impl for
//! resource kind `"flow_definition"`. The snapshot shape is a
//! [`FlowDefChange`] embedded in `Change::after`: it carries the
//! revision_id that was inserted by the verb plus the
//! `prior_revision_id` that was superseded (if any). The inverse
//! op marks the new revision superseded and clears the prior's
//! `superseded_at`, restoring the pre-deploy live row.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use starter_spi::authz::ResourceRef;
#[cfg(test)]
use starter_spi::changelog::{Actor, ChangeId, GroupId};
use starter_spi::changelog::{Change, ChangeTx, Op, Reversible};
use starter_spi::error::{Error, Result};

pub use rubix_spi::flow_def::{FlowDefChange, FlowDefStore, FlowRevisionRow, FLOW_DEFINITION_KIND};

/// In-memory [`FlowDefStore`] for tests and the in-process smoke
/// session.
#[derive(Default, Clone)]
pub struct InMemoryFlowDefStore {
    rows: Arc<Mutex<HashMap<String, FlowRevisionRow>>>,
}

impl InMemoryFlowDefStore {
    /// New empty store.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, FlowRevisionRow>> {
        self.rows.lock().expect("FlowDefStore mutex poisoned")
    }

    /// Test helper: number of rows total (including superseded).
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// Test helper: true when no rows are stored.
    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    /// Test helper: fetch a row by revision_id.
    pub fn get(&self, revision_id: &str) -> Option<FlowRevisionRow> {
        self.lock()
            .values()
            .find(|r| r.revision_id == revision_id)
            .cloned()
    }
}

#[async_trait]
impl FlowDefStore for InMemoryFlowDefStore {
    async fn insert_revision(
        &self,
        flow_id: &str,
        body_yaml: &str,
        now_ms: i64,
    ) -> Result<(FlowRevisionRow, Option<String>)> {
        let mut guard = self.lock();
        let prior = guard
            .values_mut()
            .find(|r| r.flow_id == flow_id && r.superseded_at_ms.is_none());
        let prior_revision_id = match prior {
            Some(p) => {
                p.superseded_at_ms = Some(now_ms);
                Some(p.revision_id.clone())
            }
            None => None,
        };
        let row = FlowRevisionRow {
            id: format!("fdr-{}", uuid::Uuid::new_v4().simple()),
            flow_id: flow_id.to_owned(),
            revision_id: uuid::Uuid::new_v4().to_string(),
            body_yaml: body_yaml.to_owned(),
            created_at_ms: now_ms,
            superseded_at_ms: None,
        };
        guard.insert(row.id.clone(), row.clone());
        Ok((row, prior_revision_id))
    }

    async fn fetch_latest_live(&self, flow_id: &str) -> Result<Option<FlowRevisionRow>> {
        Ok(self
            .lock()
            .values()
            .find(|r| r.flow_id == flow_id && r.superseded_at_ms.is_none())
            .cloned())
    }

    async fn list_live(&self) -> Result<Vec<FlowRevisionRow>> {
        Ok(self
            .lock()
            .values()
            .filter(|r| r.superseded_at_ms.is_none())
            .cloned()
            .collect())
    }

    async fn mark_superseded(&self, revision_id: &str, now_ms: i64) -> Result<()> {
        let mut guard = self.lock();
        let row = guard
            .values_mut()
            .find(|r| r.revision_id == revision_id)
            .ok_or_else(|| Error::NotFound {
                what: format!("flow_definition revision:{revision_id}"),
            })?;
        row.superseded_at_ms = Some(now_ms);
        Ok(())
    }

    async fn clear_superseded(&self, revision_id: &str) -> Result<()> {
        let mut guard = self.lock();
        let row = guard
            .values_mut()
            .find(|r| r.revision_id == revision_id)
            .ok_or_else(|| Error::NotFound {
                what: format!("flow_definition revision:{revision_id}"),
            })?;
        row.superseded_at_ms = None;
        Ok(())
    }
}

/// Single [`Reversible`] impl for the `"flow_definition"` kind.
pub struct FlowDefReversible {
    store: Arc<dyn FlowDefStore>,
}

impl FlowDefReversible {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn FlowDefStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Reversible for FlowDefReversible {
    fn kind(&self) -> &'static str {
        FLOW_DEFINITION_KIND
    }

    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        // Op::Create — undo a deploy/duplicate by retiring the new
        // revision and restoring the prior head (if any).
        if !matches!(ch.op, Op::Create) {
            return Err(Error::Invalid {
                message: format!(
                    "FlowDefReversible: unsupported op {:?} (expected Create)",
                    ch.op
                ),
            });
        }
        let snap: FlowDefChange = parse(ch.after.as_ref(), "after")?;
        let now_ms = now_epoch_ms();
        self.store
            .mark_superseded(&snap.revision_id, now_ms)
            .await?;
        if let Some(prior) = snap.prior_revision_id {
            self.store.clear_superseded(&prior).await?;
        }
        Ok(())
    }

    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        if !matches!(ch.op, Op::Create) {
            return Err(Error::Invalid {
                message: format!(
                    "FlowDefReversible: unsupported op {:?} (expected Create)",
                    ch.op
                ),
            });
        }
        let snap: FlowDefChange = parse(ch.after.as_ref(), "after")?;
        let now_ms = now_epoch_ms();
        // Re-supersede the prior head (if it had been restored by
        // an undo) and mark the new revision live again.
        if let Some(prior) = snap.prior_revision_id {
            self.store.mark_superseded(&prior, now_ms).await?;
        }
        self.store.clear_superseded(&snap.revision_id).await?;
        Ok(())
    }

    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        // The `rubix.flow_ops.duplicate` verb owns the clone path;
        // the changelog-level clone is intentionally unwired.
        Err(Error::Invalid {
            message: "flow_definition kind: use rubix.flow_ops.duplicate for clones".to_owned(),
        })
    }
}

fn parse<T: for<'de> Deserialize<'de>>(payload: Option<&Value>, field: &str) -> Result<T> {
    let v = payload.ok_or_else(|| Error::Invalid {
        message: format!("FlowDefReversible: Change::{field} is None"),
    })?;
    serde_json::from_value::<T>(v.clone()).map_err(|e| Error::Invalid {
        message: format!("FlowDefReversible: Change::{field} parse: {e}"),
    })
}

fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn first_insert_records_no_prior() {
        let s = InMemoryFlowDefStore::new();
        let (row, prior) = s
            .insert_revision("com.x.a", "id: com.x.a", 10)
            .await
            .unwrap();
        assert!(prior.is_none());
        assert!(row.superseded_at_ms.is_none());
    }

    #[tokio::test]
    async fn second_insert_supersedes_first() {
        let s = InMemoryFlowDefStore::new();
        let (a, _) = s.insert_revision("com.x.a", "v1", 10).await.unwrap();
        let (b, prior) = s.insert_revision("com.x.a", "v2", 20).await.unwrap();
        assert_eq!(prior.as_deref(), Some(a.revision_id.as_str()));
        let live = s.fetch_latest_live("com.x.a").await.unwrap().unwrap();
        assert_eq!(live.revision_id, b.revision_id);
        let prior_row = s.get(&a.revision_id).unwrap();
        assert_eq!(prior_row.superseded_at_ms, Some(20));
    }

    #[tokio::test]
    async fn reversible_inverse_restores_prior_head() {
        let s: Arc<dyn FlowDefStore> = Arc::new(InMemoryFlowDefStore::new());
        let (a, _) = s.insert_revision("com.x.a", "v1", 10).await.unwrap();
        let (b, prior) = s.insert_revision("com.x.a", "v2", 20).await.unwrap();
        let rev = FlowDefReversible::new(Arc::clone(&s));
        let ch = Change {
            resource: ResourceRef {
                kind: FLOW_DEFINITION_KIND.into(),
                id: Some(b.revision_id.clone()),
                owner: None,
                tenant: None,
            },
            op: Op::Create,
            before: None,
            after: Some(
                serde_json::to_value(FlowDefChange {
                    flow_id: "com.x.a".into(),
                    revision_id: b.revision_id.clone(),
                    prior_revision_id: prior.clone(),
                })
                .unwrap(),
            ),
            resource_version: None,
            correlation: None,
            id: ChangeId("c-test".into()),
            at: chrono::Utc::now(),
            actor: Actor::System,
            group_id: GroupId("g-test".into()),
            patch: None,
        };
        rev.apply_inverse(&ch).await.unwrap();
        // a is live again, b is superseded.
        let live = s.fetch_latest_live("com.x.a").await.unwrap().unwrap();
        assert_eq!(live.revision_id, a.revision_id);
    }
}
