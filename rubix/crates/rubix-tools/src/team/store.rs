//! In-memory backing store + [`Reversible`] glue for the team verbs.
//!
//! Companion to [`crate::user::store`]; same trait shape, same
//! intent: the production binary swaps a PG-backed impl in without
//! touching the verb files. See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md)
//! §"Snapshot shape".

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Change, ChangeTx, Op, Reversible};
use starter_spi::error::{Error, Result};

/// Resource-kind discriminator for team rows / memberships.
pub const TEAM_KIND: &str = "team";

/// One team row + its current membership map. The membership map
/// is part of the team's snapshot so a single [`Change`] envelope
/// can undo either a `create` (Op::Create, snapshot in `after`) or
/// an `assign` (Op::Update, snapshots in `before` / `after`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamRow {
    /// Stable id.
    pub team_id: String,
    /// Human-facing name.
    pub name: String,
    /// Optional description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// `user_id -> assigned_at_ms`. BTreeMap so the snapshot JSON is
    /// deterministic (matching the choice in `Diagnostic::params`).
    #[serde(default)]
    pub members: BTreeMap<String, i64>,
}

/// Persistence surface the team verbs target.
#[async_trait]
pub trait TeamAdminStore: Send + Sync {
    /// Insert a new team. Returns the row that landed.
    async fn create(&self, row: TeamRow) -> Result<TeamRow>;
    /// Add `user_id` to `team_id`. Returns `(prior_row, new_row)`;
    /// on a no-op re-assignment both halves are equal.
    async fn assign(
        &self,
        team_id: &str,
        user_id: &str,
        now_ms: i64,
    ) -> Result<(TeamRow, TeamRow)>;
    /// Fetch by team_id.
    async fn get(&self, team_id: &str) -> Result<Option<TeamRow>>;
    /// Restore (or insert) a row to the supplied snapshot.
    async fn put(&self, row: TeamRow) -> Result<()>;
    /// Hard-delete a row by id.
    async fn delete(&self, team_id: &str) -> Result<()>;
}

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
    async fn get(&self, team_id: &str) -> Result<Option<TeamRow>> {
        Ok(self.lock().get(team_id).cloned())
    }
    async fn put(&self, row: TeamRow) -> Result<()> {
        self.lock().insert(row.team_id.clone(), row);
        Ok(())
    }
    async fn delete(&self, team_id: &str) -> Result<()> {
        self.lock().remove(team_id);
        Ok(())
    }
}

/// Single [`Reversible`] impl for the `"team"` kind.
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

/// Sparse update payload — only the fields the verb actually
/// touched are populated; the rest stay as the current row had
/// them. See [docs/design/user-admin/](../../../../docs/design/user-admin/README.md)
/// §"Snapshot shape".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamPatch {
    /// Replace the `members` map verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<BTreeMap<String, i64>>,
    /// Replace the team name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Replace the team description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
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
