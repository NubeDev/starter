//! [`PgUserAdminStore`] \u{2014} Postgres-backed implementation of
//! [`rubix_spi::user::UserAdminStore`] over the `rubix_users`
//! table.
//!
//! Sister to [`crate::tenants::PgRubixTenantStore`] and
//! [`crate::audit::PgAuditPolicyStore`]: same layering pattern
//! (trait in `rubix-spi`, in-memory fake in `rubix-tools`, Pg
//! impl here), same `match pg_pool` selection at registry-build
//! time, same `Error::Conflict` mapping for `23505`
//! unique_violation.
//!
//! Tenant FK: `rubix_users.tenant_id REFERENCES
//! rubix_tenants(tenant_id) ON DELETE RESTRICT`. The
//! `rubix.tenant.delete` verb already refuses delete while users
//! are assigned at the verb layer (single `find_by_email` /
//! `list` scan); the FK catches any path that bypasses the verb
//! (raw SQL, undo replay across a stale snapshot, a future
//! cross-actor race). When the FK fires we surface a clean
//! [`Error::Conflict`] so the verb's existing error mapping
//! keeps working.
//!
//! Contract per [`rubix_spi::user::store`]:
//!
//! - All mutating methods (`disable` / `enable` / `set_role` /
//!   `set_prefs` / `set_tenant`) open a transaction, take
//!   `FOR UPDATE` on the prior row, detect no-ops, and commit
//!   the empty transaction without touching the row. The
//!   serialisation is required so the `(prior, new)` pair the
//!   verb echoes under \u{00A7}3.1 cannot race a peer write
//!   between the SELECT and the UPDATE.
//! - `create` returns [`Error::Conflict`] on email collision
//!   (PRIMARY KEY on `user_id` is enforced by the caller's id
//!   generation strategy; the UNIQUE on `email` is the
//!   operator-visible collision).
//! - `put` bypasses idempotency via `ON CONFLICT (user_id) DO
//!   UPDATE`. Snapshot lands verbatim including
//!   `disabled_at_ms`, `prefs_json`, `tenant_id`.
//! - `delete` is idempotent on missing rows (the trait
//!   contract; undo of a create deletes, and a second undo or
//!   a peer-deleted row must still succeed).

use async_trait::async_trait;
use rubix_spi::starter::error::{Error, Result};
use rubix_spi::user::{UserAdminStore, UserRow};
use serde_json::Value;
use starter_store_postgres::pool::Pool;

/// Cheap-to-clone handle over the [`Pool`].
#[derive(Clone)]
pub struct PgUserAdminStore {
    pool: Pool,
}

impl PgUserAdminStore {
    /// Construct over an existing [`Pool`]. The
    /// [`crate::RUBIX_USERS_MIGRATION_SOURCE`] AND
    /// [`crate::RUBIX_TENANTS_MIGRATION_SOURCE`] must have been
    /// applied first (the users table FKs into `rubix_tenants`).
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

fn backend<E: std::error::Error + Send + Sync + 'static>(e: E) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}

/// Map the `23505 unique_violation` on `rubix_users_email_key` to
/// a clean `Conflict` matching the in-memory message. Other DB
/// errors pass through as `Internal`.
fn map_create_err(row: &UserRow, e: sqlx::Error) -> Error {
    if let Some(db_err) = e.as_database_error() {
        if db_err.code().as_deref() == Some("23505") {
            return Error::Conflict {
                message: format!("user with email {} already exists", row.email),
            };
        }
    }
    backend(e)
}

/// Map `23503` foreign_key_violation on the tenant FK to
/// `Error::Conflict`. Surfaced by `set_tenant` and `put` (both
/// can land a row whose `tenant_id` no longer resolves \u{2014}
/// the latter is the undo path).
fn map_fk_err(e: sqlx::Error) -> Error {
    if let Some(db_err) = e.as_database_error() {
        if db_err.code().as_deref() == Some("23503") {
            let constraint = db_err.constraint().unwrap_or("");
            return Error::Conflict {
                message: format!(
                    "tenant FK violation on rubix_users (constraint: {constraint})"
                ),
            };
        }
    }
    backend(e)
}

#[derive(sqlx::FromRow)]
struct PgUserRow {
    user_id: String,
    email: String,
    role: String,
    disabled_at_ms: Option<i64>,
    prefs_json: Option<Value>,
    tenant_id: Option<String>,
}

impl From<PgUserRow> for UserRow {
    fn from(r: PgUserRow) -> Self {
        UserRow {
            user_id: r.user_id,
            email: r.email,
            role: r.role,
            disabled_at_ms: r.disabled_at_ms,
            prefs_json: r.prefs_json,
            tenant_id: r.tenant_id,
        }
    }
}

const SELECT_COLS: &str =
    "user_id, email, role, disabled_at_ms, prefs_json, tenant_id";

#[async_trait]
impl UserAdminStore for PgUserAdminStore {
    async fn create(&self, row: UserRow) -> Result<UserRow> {
        let sql = format!(
            "INSERT INTO rubix_users
                (user_id, email, role, disabled_at_ms, prefs_json, tenant_id)
              VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING {SELECT_COLS}"
        );
        let inserted: PgUserRow = sqlx::query_as(&sql)
            .bind(&row.user_id)
            .bind(&row.email)
            .bind(&row.role)
            .bind(row.disabled_at_ms)
            .bind(&row.prefs_json)
            .bind(&row.tenant_id)
            .fetch_one(self.pool.sqlx())
            .await
            .map_err(|e| {
                // Tenant FK violation possible if the caller
                // passes a `tenant_id` that does not resolve; the
                // verb pre-check should prevent this, but the
                // store stays defensive.
                if let Some(db) = e.as_database_error() {
                    if db.code().as_deref() == Some("23503") {
                        return map_fk_err(e);
                    }
                }
                map_create_err(&row, e)
            })?;
        Ok(inserted.into())
    }

    async fn disable(&self, user_id: &str, now_ms: i64) -> Result<(UserRow, UserRow)> {
        let mut tx = self.pool.sqlx().begin().await.map_err(backend)?;
        let select_sql = format!(
            "SELECT {SELECT_COLS} FROM rubix_users WHERE user_id = $1 FOR UPDATE"
        );
        let prior: PgUserRow = sqlx::query_as(&select_sql)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(backend)?
            .ok_or_else(|| Error::NotFound {
                what: format!("user:{user_id}"),
            })?;
        if prior.disabled_at_ms.is_some() {
            let row: UserRow = into_user_row(&prior);
            tx.commit().await.map_err(backend)?;
            return Ok((row.clone(), row));
        }
        let update_sql = format!(
            "UPDATE rubix_users SET disabled_at_ms = $2
              WHERE user_id = $1
             RETURNING {SELECT_COLS}"
        );
        let new_row: PgUserRow = sqlx::query_as(&update_sql)
            .bind(user_id)
            .bind(now_ms)
            .fetch_one(&mut *tx)
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        let prior_row: UserRow = into_user_row(&prior);
        Ok((prior_row, new_row.into()))
    }

    async fn enable(&self, user_id: &str) -> Result<(UserRow, UserRow)> {
        let mut tx = self.pool.sqlx().begin().await.map_err(backend)?;
        let select_sql = format!(
            "SELECT {SELECT_COLS} FROM rubix_users WHERE user_id = $1 FOR UPDATE"
        );
        let prior: PgUserRow = sqlx::query_as(&select_sql)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(backend)?
            .ok_or_else(|| Error::NotFound {
                what: format!("user:{user_id}"),
            })?;
        if prior.disabled_at_ms.is_none() {
            let row: UserRow = into_user_row(&prior);
            tx.commit().await.map_err(backend)?;
            return Ok((row.clone(), row));
        }
        let update_sql = format!(
            "UPDATE rubix_users SET disabled_at_ms = NULL
              WHERE user_id = $1
             RETURNING {SELECT_COLS}"
        );
        let new_row: PgUserRow = sqlx::query_as(&update_sql)
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        let prior_row: UserRow = into_user_row(&prior);
        Ok((prior_row, new_row.into()))
    }

    async fn set_role(&self, user_id: &str, role: &str) -> Result<(UserRow, UserRow)> {
        let mut tx = self.pool.sqlx().begin().await.map_err(backend)?;
        let select_sql = format!(
            "SELECT {SELECT_COLS} FROM rubix_users WHERE user_id = $1 FOR UPDATE"
        );
        let prior: PgUserRow = sqlx::query_as(&select_sql)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(backend)?
            .ok_or_else(|| Error::NotFound {
                what: format!("user:{user_id}"),
            })?;
        if prior.role == role {
            let row: UserRow = into_user_row(&prior);
            tx.commit().await.map_err(backend)?;
            return Ok((row.clone(), row));
        }
        let update_sql = format!(
            "UPDATE rubix_users SET role = $2
              WHERE user_id = $1
             RETURNING {SELECT_COLS}"
        );
        let new_row: PgUserRow = sqlx::query_as(&update_sql)
            .bind(user_id)
            .bind(role)
            .fetch_one(&mut *tx)
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        let prior_row: UserRow = into_user_row(&prior);
        Ok((prior_row, new_row.into()))
    }

    async fn set_prefs(&self, user_id: &str, prefs: Value) -> Result<(UserRow, UserRow)> {
        let mut tx = self.pool.sqlx().begin().await.map_err(backend)?;
        let select_sql = format!(
            "SELECT {SELECT_COLS} FROM rubix_users WHERE user_id = $1 FOR UPDATE"
        );
        let prior: PgUserRow = sqlx::query_as(&select_sql)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(backend)?
            .ok_or_else(|| Error::NotFound {
                what: format!("user:{user_id}"),
            })?;
        if prior.prefs_json.as_ref() == Some(&prefs) {
            let row: UserRow = into_user_row(&prior);
            tx.commit().await.map_err(backend)?;
            return Ok((row.clone(), row));
        }
        let update_sql = format!(
            "UPDATE rubix_users SET prefs_json = $2
              WHERE user_id = $1
             RETURNING {SELECT_COLS}"
        );
        let new_row: PgUserRow = sqlx::query_as(&update_sql)
            .bind(user_id)
            .bind(&prefs)
            .fetch_one(&mut *tx)
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        let prior_row: UserRow = into_user_row(&prior);
        Ok((prior_row, new_row.into()))
    }

    async fn set_tenant(
        &self,
        user_id: &str,
        tenant_id: Option<String>,
    ) -> Result<(UserRow, UserRow)> {
        let mut tx = self.pool.sqlx().begin().await.map_err(backend)?;
        let select_sql = format!(
            "SELECT {SELECT_COLS} FROM rubix_users WHERE user_id = $1 FOR UPDATE"
        );
        let prior: PgUserRow = sqlx::query_as(&select_sql)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(backend)?
            .ok_or_else(|| Error::NotFound {
                what: format!("user:{user_id}"),
            })?;
        if prior.tenant_id == tenant_id {
            let row: UserRow = into_user_row(&prior);
            tx.commit().await.map_err(backend)?;
            return Ok((row.clone(), row));
        }
        let update_sql = format!(
            "UPDATE rubix_users SET tenant_id = $2
              WHERE user_id = $1
             RETURNING {SELECT_COLS}"
        );
        let new_row: PgUserRow = sqlx::query_as(&update_sql)
            .bind(user_id)
            .bind(tenant_id.as_deref())
            .fetch_one(&mut *tx)
            .await
            .map_err(map_fk_err)?;
        tx.commit().await.map_err(backend)?;
        let prior_row: UserRow = into_user_row(&prior);
        Ok((prior_row, new_row.into()))
    }

    async fn get(&self, user_id: &str) -> Result<Option<UserRow>> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM rubix_users WHERE user_id = $1 LIMIT 1"
        );
        let row: Option<PgUserRow> = sqlx::query_as(&sql)
            .bind(user_id)
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(backend)?;
        Ok(row.map(Into::into))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<UserRow>> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM rubix_users WHERE email = $1 LIMIT 1"
        );
        let row: Option<PgUserRow> = sqlx::query_as(&sql)
            .bind(email)
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(backend)?;
        Ok(row.map(Into::into))
    }

    async fn list(&self) -> Result<Vec<UserRow>> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM rubix_users ORDER BY user_id ASC"
        );
        let rows: Vec<PgUserRow> = sqlx::query_as(&sql)
            .fetch_all(self.pool.sqlx())
            .await
            .map_err(backend)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn put(&self, row: UserRow) -> Result<()> {
        // Snapshot restore (undo path). Bypass idempotency via
        // `ON CONFLICT DO UPDATE`. FK violations on the tenant
        // column become `Error::Conflict` so the undo dispatcher
        // can surface a meaningful operator message rather than
        // leaking the raw sqlx error.
        sqlx::query(
            "INSERT INTO rubix_users
                (user_id, email, role, disabled_at_ms, prefs_json, tenant_id)
              VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (user_id) DO UPDATE
                SET email          = EXCLUDED.email,
                    role           = EXCLUDED.role,
                    disabled_at_ms = EXCLUDED.disabled_at_ms,
                    prefs_json     = EXCLUDED.prefs_json,
                    tenant_id      = EXCLUDED.tenant_id",
        )
        .bind(&row.user_id)
        .bind(&row.email)
        .bind(&row.role)
        .bind(row.disabled_at_ms)
        .bind(&row.prefs_json)
        .bind(&row.tenant_id)
        .execute(self.pool.sqlx())
        .await
        .map_err(map_fk_err)?;
        Ok(())
    }

    async fn delete(&self, user_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM rubix_users WHERE user_id = $1")
            .bind(user_id)
            .execute(self.pool.sqlx())
            .await
            .map_err(backend)?;
        Ok(())
    }
}

/// Copy without owning so the same prior row can be returned in
/// both halves of the `(prior, prior)` no-op tuple without
/// re-querying.
fn into_user_row(r: &PgUserRow) -> UserRow {
    UserRow {
        user_id: r.user_id.clone(),
        email: r.email.clone(),
        role: r.role.clone(),
        disabled_at_ms: r.disabled_at_ms,
        prefs_json: r.prefs_json.clone(),
        tenant_id: r.tenant_id.clone(),
    }
}
