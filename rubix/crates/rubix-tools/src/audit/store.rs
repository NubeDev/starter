//! In-memory backing store + [`Reversible`] glue for the
//! audit-policy verbs.
//!
//! Mirrors [`crate::tenant::store`] and [`crate::user::store`]:
//! same trait shape, same snapshot-shape Reversible. The
//! production binary swaps a Postgres impl in
//! (`rubix_store_postgres::PgAuditPolicyStore`) without touching
//! the verb files.
//!
//! Snapshot shape rationale: the policy row is tiny
//! `(resource_kind, max_age_days, updated_at)` and the lifecycle
//! covers create/update/delete \u{2014} `before` is canonical full
//! state and `before == None` marks "did not exist" (the
//! implicit-unbounded baseline).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Change, ChangeTx, Op, Reversible};
use starter_spi::error::{Error, Result};

/// Resource-kind discriminator for audit-policy rows.
pub const AUDIT_POLICY_KIND: &str = "audit_policy";

/// One row in `changelog_kind_policy`.
///
/// `max_age_days = None` pins the kind to "keep forever".
/// `Some(n)` applies a finite retention curve in days.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditPolicyRow {
    /// Resource kind the policy applies to.
    pub resource_kind: String,
    /// Retention curve in days. `None` = pinned to forever.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_age_days: Option<i32>,
    /// Epoch milliseconds (UTC) at which the row was last
    /// upserted. Stored on the row so undo restores the
    /// byte-exact prior timestamp \u{2014} not a fresh `NOW()`.
    pub updated_at_ms: i64,
}

/// Persistence surface the audit-policy verbs target.
#[async_trait]
pub trait AuditPolicyStore: Send + Sync {
    /// List every row ordered by `resource_kind` ascending.
    /// Stable order is part of the contract: callers (the
    /// `list` verb) surface rows directly to operators.
    async fn list(&self) -> Result<Vec<AuditPolicyRow>>;
    /// Fetch a single row by kind. `None` when no row exists
    /// (the kind is implicitly unbounded).
    async fn get(&self, resource_kind: &str) -> Result<Option<AuditPolicyRow>>;
    /// Upsert. Returns `(prior_row, new_row)` \u{2014} `prior_row`
    /// is `None` when the upsert is an insert. Implementations
    /// MUST stamp `updated_at_ms` on the new row at write time
    /// (the caller's `updated_at_ms` is advisory; the store is
    /// canonical). On a no-op (same kind + same `max_age_days`)
    /// implementations MUST return `(Some(prior), prior)`
    /// without touching `updated_at` \u{2014} the verb relies on
    /// this to detect idempotency.
    async fn upsert(
        &self,
        resource_kind: &str,
        max_age_days: Option<i32>,
    ) -> Result<(Option<AuditPolicyRow>, AuditPolicyRow)>;
    /// Restore a row to the supplied snapshot. Used by
    /// [`AuditPolicyReversible::apply_inverse`] to undo an
    /// upsert. Bypasses idempotency \u{2014} the snapshot must
    /// land verbatim, including its `updated_at_ms`.
    async fn put(&self, row: AuditPolicyRow) -> Result<()>;
    /// Hard-delete a row by kind. Idempotent on missing rows
    /// (undo of an insert deletes; if the row is already gone
    /// the undo still succeeds). Used by `apply_inverse` to
    /// undo `Op::Create`.
    async fn delete(&self, resource_kind: &str) -> Result<()>;
}

/// In-memory [`AuditPolicyStore`] for tests and the in-process
/// smoke session.
#[derive(Default, Clone)]
pub struct InMemoryAuditPolicyStore {
    rows: Arc<Mutex<HashMap<String, AuditPolicyRow>>>,
}

impl InMemoryAuditPolicyStore {
    /// New empty store.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, AuditPolicyRow>> {
        self.rows.lock().expect("AuditPolicyStore mutex poisoned")
    }
}

#[async_trait]
impl AuditPolicyStore for InMemoryAuditPolicyStore {
    async fn list(&self) -> Result<Vec<AuditPolicyRow>> {
        let mut rows: Vec<_> = self.lock().values().cloned().collect();
        rows.sort_by(|a, b| a.resource_kind.cmp(&b.resource_kind));
        Ok(rows)
    }
    async fn get(&self, resource_kind: &str) -> Result<Option<AuditPolicyRow>> {
        Ok(self.lock().get(resource_kind).cloned())
    }
    async fn upsert(
        &self,
        resource_kind: &str,
        max_age_days: Option<i32>,
    ) -> Result<(Option<AuditPolicyRow>, AuditPolicyRow)> {
        let mut guard = self.lock();
        let prior = guard.get(resource_kind).cloned();
        if let Some(ref p) = prior {
            if p.max_age_days == max_age_days {
                // No-op: return identical rows so the verb can
                // detect was_unchanged without touching updated_at.
                return Ok((Some(p.clone()), p.clone()));
            }
        }
        let now_ms = now_epoch_ms();
        let new = AuditPolicyRow {
            resource_kind: resource_kind.to_owned(),
            max_age_days,
            updated_at_ms: now_ms,
        };
        guard.insert(resource_kind.to_owned(), new.clone());
        Ok((prior, new))
    }
    async fn put(&self, row: AuditPolicyRow) -> Result<()> {
        self.lock().insert(row.resource_kind.clone(), row);
        Ok(())
    }
    async fn delete(&self, resource_kind: &str) -> Result<()> {
        self.lock().remove(resource_kind);
        Ok(())
    }
}

/// Single [`Reversible`] impl for the `"audit_policy"` kind.
///
/// Snapshot shape \u{2014} see module docs. `Op::Create` undoes
/// to a `delete` (no row existed before); `Op::Update` undoes
/// to a `put(before)`. `Op::Delete` isn't currently emitted by
/// any verb (the surface has no policy-delete verb) but is
/// supported for symmetry with the other Reversibles.
pub struct AuditPolicyReversible {
    store: Arc<dyn AuditPolicyStore>,
}

impl AuditPolicyReversible {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn AuditPolicyStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Reversible for AuditPolicyReversible {
    fn kind(&self) -> &'static str {
        AUDIT_POLICY_KIND
    }

    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        let id = ch.resource.id.as_deref().ok_or_else(|| Error::Invalid {
            message: "AuditPolicyReversible: Change::resource.id is None".to_owned(),
        })?;
        match ch.op {
            Op::Create => self.store.delete(id).await,
            Op::Update => {
                let row: AuditPolicyRow = parse_row(ch.before.as_ref(), "before")?;
                self.store.put(row).await
            }
            Op::Delete => {
                let row: AuditPolicyRow = parse_row(ch.before.as_ref(), "before")?;
                self.store.put(row).await
            }
            Op::Custom(ref op) => Err(Error::Invalid {
                message: format!("AuditPolicyReversible: unsupported custom op {op}"),
            }),
        }
    }

    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        let id = ch.resource.id.as_deref().ok_or_else(|| Error::Invalid {
            message: "AuditPolicyReversible: Change::resource.id is None".to_owned(),
        })?;
        match ch.op {
            Op::Create | Op::Update => {
                let row: AuditPolicyRow = parse_row(ch.after.as_ref(), "after")?;
                self.store.put(row).await
            }
            Op::Delete => self.store.delete(id).await,
            Op::Custom(ref op) => Err(Error::Invalid {
                message: format!("AuditPolicyReversible: unsupported custom op {op}"),
            }),
        }
    }

    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        // Cloning policies has no operator meaning \u{2014} the
        // (kind) is the primary key and duplicating it would
        // silently overwrite. Refuse.
        Err(Error::Invalid {
            message: "audit_policy kind does not support clone".to_owned(),
        })
    }
}

fn parse_row(payload: Option<&serde_json::Value>, field: &str) -> Result<AuditPolicyRow> {
    let v = payload.ok_or_else(|| Error::Invalid {
        message: format!("AuditPolicyReversible: Change::{field} is None"),
    })?;
    serde_json::from_value::<AuditPolicyRow>(v.clone()).map_err(|e| Error::Invalid {
        message: format!("AuditPolicyReversible: Change::{field} is not an AuditPolicyRow: {e}"),
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
    async fn upsert_then_list_is_sorted_by_kind() {
        let store = InMemoryAuditPolicyStore::new();
        store.upsert("user", None).await.unwrap();
        store.upsert("flow_def", Some(30)).await.unwrap();
        store.upsert("team", None).await.unwrap();
        let rows = store.list().await.unwrap();
        let kinds: Vec<_> = rows.iter().map(|r| r.resource_kind.as_str()).collect();
        assert_eq!(kinds, ["flow_def", "team", "user"]);
    }

    #[tokio::test]
    async fn upsert_with_same_value_is_noop_and_preserves_updated_at() {
        let store = InMemoryAuditPolicyStore::new();
        let (_, first) = store.upsert("user", Some(90)).await.unwrap();
        // Force a clock tick so a fresh write would diverge.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let (prior, again) = store.upsert("user", Some(90)).await.unwrap();
        assert_eq!(prior.as_ref().unwrap().updated_at_ms, first.updated_at_ms);
        assert_eq!(again.updated_at_ms, first.updated_at_ms);
    }

    #[tokio::test]
    async fn upsert_changing_curve_returns_prior() {
        let store = InMemoryAuditPolicyStore::new();
        let (_, first) = store.upsert("user", Some(90)).await.unwrap();
        let (prior, new) = store.upsert("user", None).await.unwrap();
        assert_eq!(prior, Some(first));
        assert_eq!(new.max_age_days, None);
    }

    #[tokio::test]
    async fn put_bypasses_uniqueness_and_restores_timestamp() {
        let store = InMemoryAuditPolicyStore::new();
        let row = AuditPolicyRow {
            resource_kind: "user".into(),
            max_age_days: Some(30),
            updated_at_ms: 100,
        };
        store.put(row.clone()).await.unwrap();
        assert_eq!(store.get("user").await.unwrap(), Some(row));
    }

    #[tokio::test]
    async fn delete_is_idempotent_on_missing() {
        let store = InMemoryAuditPolicyStore::new();
        store.delete("nope").await.unwrap();
    }
}
