//! In-memory backing store + [`Reversible`] glue for the team verbs.
//!
//! The trait + row + patch types live in [`rubix_spi::team`] so
//! this crate and `rubix-store-postgres` share the same contract
//! without depending on each other (SCOPE R5: tools and
//! store-postgres are siblings, both rooted in `rubix-spi`). The
//! production binary swaps in
//! `rubix_store_postgres::PgTeamAdminStore` without touching the
//! verb files. See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md)
//! \u{00A7}"Snapshot shape" for the JSON layout in
//! `Change::before` / `Change::after`.
//!
//! [`TeamReversible`] is the single `Reversible` impl for
//! resource kind `"team"`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
// Re-export the contract from `rubix-spi::team` so existing verb
// code (`use crate::team::store::{TeamAdminStore, TeamRow, TEAM_KIND}`)
// keeps compiling after the trait/row moved out of this crate.
pub use rubix_spi::team::{TeamAdminStore, TeamPatch, TeamRow, TEAM_KIND};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Change, ChangeTx, Op, Reversible};
use starter_spi::error::{Error, Result};

/// In-memory [`TeamAdminStore`].
#[derive(Default, Clone)]
pub struct InMemoryTeamStore {
    rows: Arc<Mutex<HashMap<String, TeamRow>>>,
}

impl InMemoryTeamStore {
    /// New empty store.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, TeamRow>> {
        self.rows.lock().expect("TeamStore mutex poisoned")
    }
}

#[async_trait]
impl TeamAdminStore for InMemoryTeamStore {
    async fn create(&self, row: TeamRow) -> Result<TeamRow> {
        let mut guard = self.lock();
        if guard.values().any(|r| r.name == row.name) {
            return Err(Error::Conflict {
                message: format!("team with name {} already exists", row.name),
            });
        }
        guard.insert(row.team_id.clone(), row.clone());
        Ok(row)
    }
    async fn assign(
        &self,
        team_id: &str,
        user_id: &str,
        now_ms: i64,
    ) -> Result<(TeamRow, TeamRow)> {
        let mut guard = self.lock();
        let prior = guard.get(team_id).cloned().ok_or_else(|| Error::NotFound {
            what: format!("team:{team_id}"),
        })?;
        if prior.members.contains_key(user_id) {
            return Ok((prior.clone(), prior));
        }
        let mut new = prior.clone();
        new.members.insert(user_id.to_owned(), now_ms);
        guard.insert(team_id.to_owned(), new.clone());
        Ok((prior, new))
    }
    async fn unassign(&self, team_id: &str, user_id: &str) -> Result<(TeamRow, TeamRow)> {
        let mut guard = self.lock();
        let prior = guard.get(team_id).cloned().ok_or_else(|| Error::NotFound {
            what: format!("team:{team_id}"),
        })?;
        if !prior.members.contains_key(user_id) {
            return Ok((prior.clone(), prior));
        }
        let mut new = prior.clone();
        new.members.remove(user_id);
        guard.insert(team_id.to_owned(), new.clone());
        Ok((prior, new))
    }
    async fn get(&self, team_id: &str) -> Result<Option<TeamRow>> {
        Ok(self.lock().get(team_id).cloned())
    }
    async fn list(&self) -> Result<Vec<TeamRow>> {
        Ok(self.lock().values().cloned().collect())
    }
    async fn put(&self, row: TeamRow) -> Result<()> {
        self.lock().insert(row.team_id.clone(), row);
        Ok(())
    }
    async fn delete(&self, team_id: &str) -> Result<()> {
        let mut guard = self.lock();
        if guard.remove(team_id).is_none() {
            return Err(Error::NotFound {
                what: format!("team:{team_id}"),
            });
        }
        Ok(())
    }
}

/// Single [`Reversible`] impl for the `"team"` kind.
///
/// Payload shape: **patch** (see
/// [`starter_spi::changelog::Reversible`] choice matrix).
/// Membership edits are naturally diff-shaped \u{2014} `before`
/// carries the touched fields only and merges against the
/// current row in `apply_inverse`, which is why two concurrent
/// edits to unrelated fields do not clobber each other.
pub struct TeamReversible {
    store: Arc<dyn TeamAdminStore>,
}

impl TeamReversible {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn TeamAdminStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Reversible for TeamReversible {
    fn kind(&self) -> &'static str {
        TEAM_KIND
    }

    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        let id = ch.resource.id.as_deref().ok_or_else(|| Error::Invalid {
            message: "TeamReversible: Change::resource.id is None".to_owned(),
        })?;
        match ch.op {
            Op::Create => self.store.delete(id).await,
            Op::Update => {
                // Update snapshots carry only the fields the verb
                // touched (e.g. `members`). We merge the patch into
                // the *current* row to avoid clobbering concurrent
                // edits to unrelated fields.
                let patch = parse_patch(ch.before.as_ref(), "before")?;
                let current = self.store.get(id).await?.ok_or_else(|| Error::NotFound {
                    what: format!("team:{id}"),
                })?;
                self.store.put(merge_patch(current, patch)).await
            }
            Op::Delete => {
                let row: TeamRow = parse_row(ch.before.as_ref(), "before")?;
                self.store.put(row).await
            }
            Op::Custom(ref op) => Err(Error::Invalid {
                message: format!("TeamReversible: unsupported custom op {op}"),
            }),
        }
    }

    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        let id = ch.resource.id.as_deref().ok_or_else(|| Error::Invalid {
            message: "TeamReversible: Change::resource.id is None".to_owned(),
        })?;
        match ch.op {
            Op::Create => {
                let row: TeamRow = parse_row(ch.after.as_ref(), "after")?;
                self.store.put(row).await
            }
            Op::Update => {
                let patch = parse_patch(ch.after.as_ref(), "after")?;
                let current = self.store.get(id).await?.ok_or_else(|| Error::NotFound {
                    what: format!("team:{id}"),
                })?;
                self.store.put(merge_patch(current, patch)).await
            }
            Op::Delete => self.store.delete(id).await,
            Op::Custom(ref op) => Err(Error::Invalid {
                message: format!("TeamReversible: unsupported custom op {op}"),
            }),
        }
    }

    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        Err(Error::Invalid {
            message: "team kind does not support clone".to_owned(),
        })
    }
}

fn parse_patch(payload: Option<&Value>, field: &str) -> Result<TeamPatch> {
    let v = payload.ok_or_else(|| Error::Invalid {
        message: format!("TeamReversible: Change::{field} is None"),
    })?;
    serde_json::from_value::<TeamPatch>(v.clone()).map_err(|e| Error::Invalid {
        message: format!("TeamReversible: Change::{field} is not a TeamPatch: {e}"),
    })
}

fn merge_patch(mut row: TeamRow, patch: TeamPatch) -> TeamRow {
    if let Some(members) = patch.members {
        row.members = members;
    }
    if let Some(name) = patch.name {
        row.name = name;
    }
    if let Some(desc) = patch.description {
        row.description = desc;
    }
    row
}

fn parse_row(payload: Option<&Value>, field: &str) -> Result<TeamRow> {
    let v = payload.ok_or_else(|| Error::Invalid {
        message: format!("TeamReversible: Change::{field} is None"),
    })?;
    serde_json::from_value::<TeamRow>(v.clone()).map_err(|e| Error::Invalid {
        message: format!("TeamReversible: Change::{field} is not a TeamRow: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn row(id: &str, name: &str) -> TeamRow {
        TeamRow {
            team_id: id.into(),
            name: name.into(),
            description: None,
            members: BTreeMap::new(),
        }
    }

    #[tokio::test]
    async fn assign_is_idempotent_and_keeps_prior_timestamp() {
        let store = InMemoryTeamStore::new();
        store.create(row("t-1", "Ops")).await.unwrap();
        let (_, new) = store.assign("t-1", "u-1", 100).await.unwrap();
        assert_eq!(new.members.get("u-1"), Some(&100));
        let (_, new2) = store.assign("t-1", "u-1", 200).await.unwrap();
        assert_eq!(
            new2.members.get("u-1"),
            Some(&100),
            "re-assign keeps the original assigned_at",
        );
    }
}
