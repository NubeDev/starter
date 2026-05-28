//! [`PgTeamAdminStore`] \u{2014} Postgres-backed implementation
//! of [`rubix_spi::team::TeamAdminStore`] over the `rubix_teams`
//! table.
//!
//! Sister to [`crate::users::PgUserAdminStore`],
//! [`crate::tenants::PgRubixTenantStore`], and
//! [`crate::audit::PgAuditPolicyStore`]. Same layering pattern
//! (trait in `rubix-spi`, in-memory fake in `rubix-tools`, Pg
//! impl here), same `match pg_pool` selection at registry-build
//! time, same `Error::Conflict` mapping for `23505`
//! unique_violation.
//!
//! Contract per [`rubix_spi::team::store`]:
//!
//! - All mutating methods (`assign` / `unassign`) open a
//!   transaction, take `FOR UPDATE` on the prior row, detect
//!   no-ops (member already present / already absent), and
//!   commit the empty transaction without touching the row.
//!   The serialisation is required so the `(prior, new)` pair
//!   the verb echoes under \u{00A7}3.1 cannot race a peer write
//!   between the SELECT and the UPDATE \u{2014} the failure
//!   mode is two concurrent assigns racing one another, the
//!   last write silently losing a member.
//! - `create` returns [`Error::Conflict`] on a duplicate `name`
//!   (PRIMARY KEY collisions on `team_id` reach the same
//!   mapper because the caller's id-generation path is the only
//!   producer; the error message names the offending name to
//!   match the in-memory fake byte-exact).
//! - `put` bypasses uniqueness via `ON CONFLICT (team_id) DO
//!   UPDATE`. Used by `TeamReversible::apply_inverse` to
//!   restore a snapshot verbatim including the `members` map.
//! - `delete` returns `Error::NotFound` when the row does not
//!   resolve \u{2014} the verb relies on this signal to
//!   distinguish a missing-target call from a successful no-op.

use std::collections::BTreeMap;

use async_trait::async_trait;
use rubix_spi::starter::error::{Error, Result};
use rubix_spi::team::{TeamAdminStore, TeamRow};
use serde_json::Value;
use starter_store_postgres::pool::Pool;

/// Cheap-to-clone handle over the [`Pool`].
#[derive(Clone)]
pub struct PgTeamAdminStore {
    pool: Pool,
}

impl PgTeamAdminStore {
    /// Construct over an existing [`Pool`]. The
    /// [`crate::RUBIX_TEAMS_MIGRATION_SOURCE`] must have been
    /// applied first.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

fn backend<E: std::error::Error + Send + Sync + 'static>(e: E) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}

/// Map `23505 unique_violation` on `rubix_teams_name_key` (or
/// the team_id PRIMARY KEY) to a clean `Conflict` matching the
/// in-memory wording. Other DB errors pass through as `Internal`.
fn map_create_err(row: &TeamRow, e: sqlx::Error) -> Error {
    if let Some(db_err) = e.as_database_error() {
        if db_err.code().as_deref() == Some("23505") {
            // The in-memory fake only checks the name (id
            // collisions are an upstream id-generation bug, not
            // an operator-visible conflict), so we report the
            // same way regardless of which constraint fired.
            return Error::Conflict {
                message: format!("team with name {} already exists", row.name),
            };
        }
    }
    backend(e)
}

#[derive(sqlx::FromRow)]
struct PgTeamRow {
    team_id: String,
    name: String,
    description: Option<String>,
    members: Value,
}

impl TryFrom<PgTeamRow> for TeamRow {
    type Error = Error;
    fn try_from(r: PgTeamRow) -> Result<Self> {
        let members: BTreeMap<String, i64> = serde_json::from_value(r.members)
            .map_err(|e| Error::Invalid {
                message: format!(
                    "rubix_teams.members row {} is not a string->i64 map: {e}",
                    r.team_id
                ),
            })?;
        Ok(TeamRow {
            team_id: r.team_id,
            name: r.name,
            description: r.description,
            members,
        })
    }
}

const SELECT_COLS: &str = "team_id, name, description, members";

fn members_to_json(members: &BTreeMap<String, i64>) -> Value {
    // BTreeMap iteration is deterministic, so the JSON we hand
    // sqlx is byte-stable across runs. Useful for the
    // `(prior, new)` comparison on no-op assigns where we want
    // serde_json::Value equality to mean "same snapshot".
    serde_json::to_value(members)
        .expect("BTreeMap<String, i64> -> serde_json::Value is infallible")
}

#[async_trait]
impl TeamAdminStore for PgTeamAdminStore {
    async fn create(&self, row: TeamRow) -> Result<TeamRow> {
        let members_json = members_to_json(&row.members);
        let sql = format!(
            "INSERT INTO rubix_teams (team_id, name, description, members)
              VALUES ($1, $2, $3, $4)
             RETURNING {SELECT_COLS}"
        );
        let inserted: PgTeamRow = sqlx::query_as(&sql)
            .bind(&row.team_id)
            .bind(&row.name)
            .bind(&row.description)
            .bind(&members_json)
            .fetch_one(self.pool.sqlx())
            .await
            .map_err(|e| map_create_err(&row, e))?;
        inserted.try_into()
    }

    async fn assign(
        &self,
        team_id: &str,
        user_id: &str,
        now_ms: i64,
    ) -> Result<(TeamRow, TeamRow)> {
        let mut tx = self.pool.sqlx().begin().await.map_err(backend)?;
        let select_sql = format!(
            "SELECT {SELECT_COLS} FROM rubix_teams WHERE team_id = $1 FOR UPDATE"
        );
        let prior_pg: PgTeamRow = sqlx::query_as(&select_sql)
            .bind(team_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(backend)?
            .ok_or_else(|| Error::NotFound {
                what: format!("team:{team_id}"),
            })?;
        let prior: TeamRow = prior_pg.try_into()?;
        if prior.members.contains_key(user_id) {
            tx.commit().await.map_err(backend)?;
            return Ok((prior.clone(), prior));
        }
        let mut new = prior.clone();
        new.members.insert(user_id.to_owned(), now_ms);
        let new_members = members_to_json(&new.members);
        sqlx::query("UPDATE rubix_teams SET members = $2 WHERE team_id = $1")
            .bind(team_id)
            .bind(&new_members)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        Ok((prior, new))
    }

    async fn unassign(
        &self,
        team_id: &str,
        user_id: &str,
    ) -> Result<(TeamRow, TeamRow)> {
        let mut tx = self.pool.sqlx().begin().await.map_err(backend)?;
        let select_sql = format!(
            "SELECT {SELECT_COLS} FROM rubix_teams WHERE team_id = $1 FOR UPDATE"
        );
        let prior_pg: PgTeamRow = sqlx::query_as(&select_sql)
            .bind(team_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(backend)?
            .ok_or_else(|| Error::NotFound {
                what: format!("team:{team_id}"),
            })?;
        let prior: TeamRow = prior_pg.try_into()?;
        if !prior.members.contains_key(user_id) {
            tx.commit().await.map_err(backend)?;
            return Ok((prior.clone(), prior));
        }
        let mut new = prior.clone();
        new.members.remove(user_id);
        let new_members = members_to_json(&new.members);
        sqlx::query("UPDATE rubix_teams SET members = $2 WHERE team_id = $1")
            .bind(team_id)
            .bind(&new_members)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        Ok((prior, new))
    }

    async fn get(&self, team_id: &str) -> Result<Option<TeamRow>> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM rubix_teams WHERE team_id = $1 LIMIT 1"
        );
        let row: Option<PgTeamRow> = sqlx::query_as(&sql)
            .bind(team_id)
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(backend)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list(&self) -> Result<Vec<TeamRow>> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM rubix_teams ORDER BY team_id ASC"
        );
        let rows: Vec<PgTeamRow> = sqlx::query_as(&sql)
            .fetch_all(self.pool.sqlx())
            .await
            .map_err(backend)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn put(&self, row: TeamRow) -> Result<()> {
        let members_json = members_to_json(&row.members);
        // Snapshot restore (undo path). Bypass uniqueness via
        // `ON CONFLICT DO UPDATE`. Note: if another team has
        // grabbed the name between the snapshot and the restore,
        // the UPDATE branch still succeeds because the conflict
        // target is `team_id`, not `name` \u{2014} the name
        // collision would only surface on `INSERT` and only if
        // the team had been deleted in between. That is the
        // intended semantic: restoring a snapshot is meant to
        // recover the row as it was; a downstream uniqueness
        // violation will surface on the next operator-visible
        // operation.
        sqlx::query(
            "INSERT INTO rubix_teams (team_id, name, description, members)
              VALUES ($1, $2, $3, $4)
             ON CONFLICT (team_id) DO UPDATE
                SET name        = EXCLUDED.name,
                    description = EXCLUDED.description,
                    members     = EXCLUDED.members",
        )
        .bind(&row.team_id)
        .bind(&row.name)
        .bind(&row.description)
        .bind(&members_json)
        .execute(self.pool.sqlx())
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn delete(&self, team_id: &str) -> Result<()> {
        let res = sqlx::query("DELETE FROM rubix_teams WHERE team_id = $1")
            .bind(team_id)
            .execute(self.pool.sqlx())
            .await
            .map_err(backend)?;
        if res.rows_affected() == 0 {
            return Err(Error::NotFound {
                what: format!("team:{team_id}"),
            });
        }
        Ok(())
    }
}
