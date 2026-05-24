//! In-memory backing store + [`Reversible`] glue for the user-admin
//! verbs.
//!
//! The four write verbs (`user.create`, `user.disable`, `team.create`,
//! `team.assign`) talk to a small [`UserAdminStore`] trait so the
//! production binary can swap a PG-backed impl in without touching
//! the verb files. Today only the [`InMemoryUserStore`] exists —
//! it is enough for unit tests, the agent loop's recorded-LLM
//! integration tests, and the smoke session that lights the verbs
//! end-to-end. The PG impl lands in a follow-up phase that wires
//! `starter-auth-users` to the same trait.
//!
//! [`UserReversible`] is the single `Reversible` impl for resource
//! kind `"user"`. See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md)
//! §"Snapshot shape" for the JSON layout in `Change::before` /
//! `Change::after`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Change, ChangeTx, Op, Reversible};
use starter_spi::error::{Error, Result};

/// Resource-kind discriminator. Matches [`ResourceRef::kind`] on
/// every recorded `Change` for a user row.
pub const USER_KIND: &str = "user";

/// One user row as persisted by the rubix user-admin verbs.
///
/// This is the snapshot shape `UserReversible` reads/writes via
/// `Change::before` / `Change::after`. Mirrors the trimmed columns
/// the four phase-B verbs need; the production PG impl is free to
/// carry more columns as long as this subset round-trips.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserRow {
    /// Stable id (assigned at create time).
    pub user_id: String,
    /// Login email.
    pub email: String,
    /// Role string (`reader` / `writer` / `admin`).
    pub role: String,
    /// `Some(epoch_ms)` when the user is disabled, `None` when
    /// enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_at_ms: Option<i64>,
}

/// Persistence surface the four write verbs target.
#[async_trait]
pub trait UserAdminStore: Send + Sync {
    /// Insert a new user. Returns the row that landed.
    async fn create(&self, row: UserRow) -> Result<UserRow>;
    /// Mark a user as disabled and return `(prior_row, new_row)`.
    /// When the row is already disabled, both halves are equal and
    /// the verb reports `was_already_disabled = true` to the caller.
    async fn disable(&self, user_id: &str, now_ms: i64) -> Result<(UserRow, UserRow)>;
    /// Fetch by user_id.
    async fn get(&self, user_id: &str) -> Result<Option<UserRow>>;
    /// Fetch by email.
    async fn find_by_email(&self, email: &str) -> Result<Option<UserRow>>;
    /// List all rows (read-only). Order is unspecified — callers
    /// sort if they need stability.
    async fn list(&self) -> Result<Vec<UserRow>>;
    /// Restore (or insert) a row to the supplied snapshot. Used by
    /// `UserReversible::apply_inverse` to walk a `Change` backwards.
    async fn put(&self, row: UserRow) -> Result<()>;
    /// Hard-delete a row by id. Used by `apply_inverse` to undo a
    /// `Op::Create`.
    async fn delete(&self, user_id: &str) -> Result<()>;
}

/// In-memory [`UserAdminStore`] for tests and the in-process smoke
/// session.
#[derive(Default, Clone)]
pub struct InMemoryUserStore {
    rows: Arc<Mutex<HashMap<String, UserRow>>>,
}

impl InMemoryUserStore {
    /// New empty store.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, UserRow>> {
        self.rows.lock().expect("UserStore mutex poisoned")
    }
}

#[async_trait]
impl UserAdminStore for InMemoryUserStore {
    async fn create(&self, row: UserRow) -> Result<UserRow> {
        let mut guard = self.lock();
        if guard.values().any(|r| r.email == row.email) {
            return Err(Error::Conflict {
                message: format!("user with email {} already exists", row.email),
            });
        }
        guard.insert(row.user_id.clone(), row.clone());
        Ok(row)
    }
    async fn disable(&self, user_id: &str, now_ms: i64) -> Result<(UserRow, UserRow)> {
        let mut guard = self.lock();
        let prior = guard.get(user_id).cloned().ok_or_else(|| Error::NotFound {
            what: format!("user:{user_id}"),
        })?;
        if prior.disabled_at_ms.is_some() {
            return Ok((prior.clone(), prior));
        }
        let mut new = prior.clone();
        new.disabled_at_ms = Some(now_ms);
        guard.insert(user_id.to_owned(), new.clone());
        Ok((prior, new))
    }
    async fn get(&self, user_id: &str) -> Result<Option<UserRow>> {
        Ok(self.lock().get(user_id).cloned())
    }
    async fn find_by_email(&self, email: &str) -> Result<Option<UserRow>> {
        Ok(self.lock().values().find(|r| r.email == email).cloned())
    }
    async fn list(&self) -> Result<Vec<UserRow>> {
        Ok(self.lock().values().cloned().collect())
    }
    async fn put(&self, row: UserRow) -> Result<()> {
        self.lock().insert(row.user_id.clone(), row);
        Ok(())
    }
    async fn delete(&self, user_id: &str) -> Result<()> {
        self.lock().remove(user_id);
        Ok(())
    }
}

/// Single [`Reversible`] impl for the `"user"` kind. Registered once
/// per server build alongside the user-admin verbs.
pub struct UserReversible {
    store: Arc<dyn UserAdminStore>,
}

impl UserReversible {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn UserAdminStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Reversible for UserReversible {
    fn kind(&self) -> &'static str {
        USER_KIND
    }

    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        let id = ch.resource.id.as_deref().ok_or_else(|| Error::Invalid {
            message: "UserReversible: Change::resource.id is None".to_owned(),
        })?;
        match ch.op {
            Op::Create => self.store.delete(id).await,
            Op::Update => {
                // Before-snapshot is a *full* UserRow per the verb
                // contract — we trust it as the canonical prior state.
                let row: UserRow = parse_row(ch.before.as_ref(), "before")?;
                self.store.put(row).await
            }
            Op::Delete => {
                let row: UserRow = parse_row(ch.before.as_ref(), "before")?;
                self.store.put(row).await
            }
            Op::Custom(ref op) => Err(Error::Invalid {
                message: format!("UserReversible: unsupported custom op {op}"),
            }),
        }
    }

    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        let id = ch.resource.id.as_deref().ok_or_else(|| Error::Invalid {
            message: "UserReversible: Change::resource.id is None".to_owned(),
        })?;
        match ch.op {
            Op::Create | Op::Update => {
                let row: UserRow = parse_row(ch.after.as_ref(), "after")?;
                self.store.put(row).await
            }
            Op::Delete => self.store.delete(id).await,
            Op::Custom(ref op) => Err(Error::Invalid {
                message: format!("UserReversible: unsupported custom op {op}"),
            }),
        }
    }

    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        // Cloning users is intentionally out of scope — duplicating
        // a user row would silently bypass email-uniqueness, which
        // is precisely the constraint operators rely on.
        Err(Error::Invalid {
            message: "user kind does not support clone".to_owned(),
        })
    }
}

fn parse_row(payload: Option<&Value>, field: &str) -> Result<UserRow> {
    let v = payload.ok_or_else(|| Error::Invalid {
        message: format!("UserReversible: Change::{field} is None"),
    })?;
    serde_json::from_value::<UserRow>(v.clone()).map_err(|e| Error::Invalid {
        message: format!("UserReversible: Change::{field} is not a UserRow: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, email: &str) -> UserRow {
        UserRow {
            user_id: id.into(),
            email: email.into(),
            role: "reader".into(),
            disabled_at_ms: None,
        }
    }

    #[tokio::test]
    async fn create_rejects_duplicate_email() {
        let store = InMemoryUserStore::new();
        store.create(row("u-1", "a@x")).await.unwrap();
        let err = store.create(row("u-2", "a@x")).await.unwrap_err();
        assert!(matches!(err, Error::Conflict { .. }));
    }

    #[tokio::test]
    async fn disable_is_idempotent_and_keeps_prior_timestamp() {
        let store = InMemoryUserStore::new();
        store.create(row("u-1", "a@x")).await.unwrap();
        let (prior, new) = store.disable("u-1", 100).await.unwrap();
        assert!(prior.disabled_at_ms.is_none());
        assert_eq!(new.disabled_at_ms, Some(100));
        let (prior2, new2) = store.disable("u-1", 200).await.unwrap();
        assert_eq!(prior2.disabled_at_ms, Some(100));
        assert_eq!(new2.disabled_at_ms, Some(100));
    }
}
