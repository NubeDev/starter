//! [`ApprovalStore`] trait + [`InMemoryApprovalStore`].
//!
//! Per R-skills-7: the approval store is **append-mostly**. The only
//! state-changing operations are:
//!
//! - [`ApprovalStore::record`] — operator approves a bundle hash,
//! - [`ApprovalStore::revoke`] — operator explicitly revokes an
//!   approval row.
//!
//! Drift on [`crate::SkillRegistry::reload`] **never** mutates the
//! store. If a bundle is approved at hash `H1` and the bytes change
//! to `H2`, the `H1` row stays in [`ApprovalStore::list`] (inert,
//! because no on-disk bundle hashes to `H1` anymore) and `H2` simply
//! has no row, so the trust matrix re-quarantines the bundle.
//! Operators can later prune inert rows manually; the loader never
//! does it implicitly.
//!
//! The trait is async so the same shape covers in-memory test stores
//! and the (Phase 5) SQLite + Postgres impls. `InMemoryApprovalStore`
//! is the only concrete impl shipped from this crate; it backs the
//! Phase 3 smoke tests and the registry's own unit tests.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use starter_flow_spi::skill::SkillId;
use thiserror::Error;

/// One approval row keyed on `(skill_id, bundle_hash)`.
///
/// R-skills-3 row 4 / R-skills-7: the registry promotes a
/// quarantined bundle to approved iff a row with the exact
/// `(skill_id, bundle_hash)` pair exists. Any other shape of "this
/// skill is approved" (a row with a different hash, no row at all)
/// keeps the bundle quarantined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRow {
    /// Skill the approval row applies to.
    pub skill_id: SkillId,
    /// Bundle content hash the approval was granted for (R-skills-2).
    pub bundle_hash: String,
    /// Stable identifier of the principal who recorded the approval
    /// (typically `Principal::subject`). Free-form: the trait only
    /// stores it for audit, it is not used for routing.
    pub approved_by: String,
    /// Unix milliseconds when the row was recorded. UTC; clock skew
    /// is the operator's problem.
    pub approved_at_unix_ms: u64,
}

impl ApprovalRow {
    /// Construct an [`ApprovalRow`] stamped with the current wall
    /// clock. Tests that need a deterministic timestamp can
    /// construct the struct literally instead.
    pub fn now(
        skill_id: SkillId,
        bundle_hash: impl Into<String>,
        approved_by: impl Into<String>,
    ) -> Self {
        let approved_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            skill_id,
            bundle_hash: bundle_hash.into(),
            approved_by: approved_by.into(),
            approved_at_unix_ms,
        }
    }
}

/// Errors a backing store may surface. In-memory has no failure
/// modes; SQL impls (Phase 5) will surface their driver errors
/// through the `#[from] Backend` arm of a per-impl wrapper.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ApprovalStoreError {
    /// Backend-specific failure (driver error, connection loss,
    /// constraint violation). Boxed so the trait stays
    /// dyn-compatible without a generic error parameter.
    #[error("approval store backend error: {0}")]
    Backend(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl ApprovalStoreError {
    /// Wrap an arbitrary backend error.
    pub fn backend<E>(err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Backend(Box::new(err))
    }
}

/// Append-mostly approval store (R-skills-7).
///
/// The trait is intentionally narrow. There is no `clear()`, no
/// `update()`, no "drift fix" hook — the only state transitions are
/// `record` (operator approves) and `revoke` (operator un-approves).
/// Drift on registry reload never calls into this trait.
#[async_trait]
pub trait ApprovalStore: Send + Sync + 'static {
    /// Persist `row` (the operator just approved a bundle hash).
    ///
    /// Recording the same `(skill_id, bundle_hash)` twice is
    /// idempotent: the second call is allowed and may either keep
    /// the first row's metadata or replace it — the registry only
    /// reads existence, not history.
    async fn record(&self, row: ApprovalRow) -> Result<(), ApprovalStoreError>;

    /// Return the approval row for `(skill_id, bundle_hash)`, or
    /// `None` if none has been recorded. The registry uses this at
    /// build / reload time to decide whether a quarantined bundle
    /// promotes to approved.
    async fn lookup(
        &self,
        skill_id: &SkillId,
        bundle_hash: &str,
    ) -> Result<Option<ApprovalRow>, ApprovalStoreError>;

    /// Return every approval row currently in the store. Operators
    /// use this to surface "which bundles have I approved?" in
    /// management UIs. Order is unspecified.
    async fn list(&self) -> Result<Vec<ApprovalRow>, ApprovalStoreError>;

    /// Remove the row for `(skill_id, bundle_hash)`. Removing a row
    /// that does not exist is a no-op (no error). This is the only
    /// way to take an approved bundle back to quarantined without
    /// changing its bytes.
    async fn revoke(&self, skill_id: &SkillId, bundle_hash: &str)
        -> Result<(), ApprovalStoreError>;
}

/// In-memory [`ApprovalStore`]. Ships in `starter-skills` so the
/// registry's own tests, the Phase 3 smoke tests, and any host that
/// does not yet need persistence can wire one up with zero deps.
///
/// Not for production: the rows live for the lifetime of the
/// process. Production deployments wire a SQLite or Postgres impl
/// (Phase 5).
#[derive(Debug, Default)]
pub struct InMemoryApprovalStore {
    // Mutex (not RwLock) is sufficient — the critical sections are
    // tiny map operations and the store is only ever a build-time /
    // approve-time bottleneck, never a per-request hot path
    // (R-skills-8: `select()` does no I/O, approvals are cached at
    // registry build time).
    rows: Mutex<HashMap<(SkillId, String), ApprovalRow>>,
}

impl InMemoryApprovalStore {
    /// Construct an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of rows currently held. Test-only convenience; the
    /// registry never reads this.
    #[doc(hidden)]
    pub fn len(&self) -> usize {
        self.rows.lock().expect("approval store poisoned").len()
    }

    /// Are there no rows? Mirrors [`Self::len`] for clippy.
    #[doc(hidden)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl ApprovalStore for InMemoryApprovalStore {
    async fn record(&self, row: ApprovalRow) -> Result<(), ApprovalStoreError> {
        let key = (row.skill_id.clone(), row.bundle_hash.clone());
        self.rows
            .lock()
            .expect("approval store poisoned")
            .insert(key, row);
        Ok(())
    }

    async fn lookup(
        &self,
        skill_id: &SkillId,
        bundle_hash: &str,
    ) -> Result<Option<ApprovalRow>, ApprovalStoreError> {
        // Clone-on-read keeps the lock window short and the trait
        // owned-only (no lifetime escape from the lock).
        Ok(self
            .rows
            .lock()
            .expect("approval store poisoned")
            .get(&(skill_id.clone(), bundle_hash.to_owned()))
            .cloned())
    }

    async fn list(&self) -> Result<Vec<ApprovalRow>, ApprovalStoreError> {
        Ok(self
            .rows
            .lock()
            .expect("approval store poisoned")
            .values()
            .cloned()
            .collect())
    }

    async fn revoke(
        &self,
        skill_id: &SkillId,
        bundle_hash: &str,
    ) -> Result<(), ApprovalStoreError> {
        self.rows
            .lock()
            .expect("approval store poisoned")
            .remove(&(skill_id.clone(), bundle_hash.to_owned()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(s: &str) -> SkillId {
        SkillId::new(s).expect("valid skill id")
    }

    #[tokio::test]
    async fn record_then_lookup_round_trips() {
        let store = InMemoryApprovalStore::new();
        let row = ApprovalRow::now(sid("starter.x.y"), "h1", "alice");
        store.record(row.clone()).await.unwrap();

        let got = store.lookup(&sid("starter.x.y"), "h1").await.unwrap();
        assert_eq!(got, Some(row));
    }

    #[tokio::test]
    async fn lookup_with_unknown_hash_returns_none() {
        let store = InMemoryApprovalStore::new();
        store
            .record(ApprovalRow::now(sid("starter.x.y"), "h1", "alice"))
            .await
            .unwrap();

        // Different hash → no row (R-skills-3 row 4: hash must
        // match exactly).
        assert!(store
            .lookup(&sid("starter.x.y"), "h2")
            .await
            .unwrap()
            .is_none());
        // Different skill id, same hash → no row.
        assert!(store
            .lookup(&sid("starter.x.z"), "h1")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn revoke_removes_only_the_targeted_row() {
        let store = InMemoryApprovalStore::new();
        store
            .record(ApprovalRow::now(sid("starter.x.y"), "h1", "alice"))
            .await
            .unwrap();
        store
            .record(ApprovalRow::now(sid("starter.x.y"), "h2", "alice"))
            .await
            .unwrap();

        store.revoke(&sid("starter.x.y"), "h1").await.unwrap();

        assert!(store
            .lookup(&sid("starter.x.y"), "h1")
            .await
            .unwrap()
            .is_none());
        assert!(store
            .lookup(&sid("starter.x.y"), "h2")
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn revoke_of_missing_row_is_a_noop() {
        let store = InMemoryApprovalStore::new();
        // Must not error.
        store.revoke(&sid("starter.x.y"), "h1").await.unwrap();
    }

    #[tokio::test]
    async fn list_returns_every_recorded_row() {
        let store = InMemoryApprovalStore::new();
        store
            .record(ApprovalRow::now(sid("starter.x.a"), "h1", "alice"))
            .await
            .unwrap();
        store
            .record(ApprovalRow::now(sid("starter.x.b"), "h2", "bob"))
            .await
            .unwrap();
        let rows = store.list().await.unwrap();
        assert_eq!(rows.len(), 2);
    }
}
