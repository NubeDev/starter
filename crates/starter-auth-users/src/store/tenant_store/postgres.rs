//! Postgres-backed [`super::TenantStore`]. Mirrors
//! [`super::SqliteTenantStore`] row-for-row; the SQL differs only in
//! bind placeholders (`$N` vs `?N`), in `created_at` being a real
//! `TIMESTAMPTZ` (vs sqlite rfc3339-in-TEXT), and in the
//! reserved-slug / immutability checks raising SQLSTATE `23514`
//! (`check_violation`) via the migration's CHECK + trigger
//! functions. See docs/design/auth/README.md for the contract.
//!
//! The Postgres migrations to apply first live in
//! `migrations_postgres/starter_auth_users/0005_tenants.sql` and
//! `0006_teams.sql`. Use [`crate::migration::postgres_migration_source`]
//! to chain them into the consumer's `migrate(&pool)` plan.

use async_trait::async_trait;
use sqlx::Row;
use starter_store_postgres::Pool;

use super::{
    is_reserved_slug, MembershipRecord, TeamRecord, TenantRecord, TenantStore, TenantStoreError,
};

/// SQLSTATE for Postgres `check_violation`, raised by both `CHECK`
/// constraints and `RAISE EXCEPTION ... USING ERRCODE =
/// 'check_violation'` in the immutability trigger functions.
const SQLSTATE_CHECK_VIOLATION: &str = "23514";

/// Postgres-backed [`TenantStore`].
pub struct PgTenantStore {
    pool: Pool,
}

impl PgTenantStore {
    /// Wrap the pool. Apply the Postgres migrations first via
    /// [`crate::migration::postgres_migration_source`].
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Borrow the pool — needed by admin route handlers that revoke
    /// tokens in the same transaction as a membership delete.
    pub fn pool(&self) -> &Pool {
        &self.pool
    }
}

fn err(e: sqlx::Error) -> TenantStoreError {
    TenantStoreError::Backend(e.to_string())
}

fn is_check_violation(e: &dyn sqlx::error::DatabaseError) -> bool {
    e.code().as_deref() == Some(SQLSTATE_CHECK_VIOLATION)
}

fn map_tenant(row: sqlx::postgres::PgRow) -> TenantRecord {
    TenantRecord {
        id: row.get(0),
        slug: row.get(1),
        display_name: row.get(2),
        audit_allow_sample: row.get(3),
    }
}

fn map_membership(row: sqlx::postgres::PgRow) -> MembershipRecord {
    MembershipRecord {
        tenant_id: row.get(0),
        user_id: row.get(1),
        role: row.get(2),
    }
}

#[async_trait]
impl TenantStore for PgTenantStore {
    async fn create_tenant(&self, row: &TenantRecord) -> Result<(), TenantStoreError> {
        if is_reserved_slug(&row.slug) {
            return Err(TenantStoreError::ReservedSlug(row.slug.clone()));
        }
        let res = sqlx::query(
            "INSERT INTO starter_auth_users_tenants \
             (id, slug, display_name, audit_allow_sample) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&row.id)
        .bind(&row.slug)
        .bind(&row.display_name)
        .bind(row.audit_allow_sample)
        .execute(self.pool.sqlx())
        .await;
        match res {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                Err(TenantStoreError::SlugConflict(row.slug.clone()))
            }
            Err(sqlx::Error::Database(e)) if is_check_violation(e.as_ref()) => {
                Err(TenantStoreError::ReservedSlug(row.slug.clone()))
            }
            Err(e) => Err(err(e)),
        }
    }

    async fn list_tenants(&self) -> Result<Vec<TenantRecord>, TenantStoreError> {
        let rows = sqlx::query(
            "SELECT id, slug, display_name, audit_allow_sample \
             FROM starter_auth_users_tenants \
             ORDER BY created_at ASC, id ASC",
        )
        .fetch_all(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(rows.into_iter().map(map_tenant).collect())
    }

    async fn get_tenant(&self, id: &str) -> Result<Option<TenantRecord>, TenantStoreError> {
        let row = sqlx::query(
            "SELECT id, slug, display_name, audit_allow_sample \
             FROM starter_auth_users_tenants WHERE id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(row.map(map_tenant))
    }

    async fn get_tenant_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<TenantRecord>, TenantStoreError> {
        let row = sqlx::query(
            "SELECT id, slug, display_name, audit_allow_sample \
             FROM starter_auth_users_tenants WHERE slug = $1 LIMIT 1",
        )
        .bind(slug)
        .fetch_optional(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(row.map(map_tenant))
    }

    async fn patch_tenant(
        &self,
        id: &str,
        display_name: Option<&str>,
        audit_allow_sample: Option<Option<i32>>,
    ) -> Result<(), TenantStoreError> {
        if display_name.is_none() && audit_allow_sample.is_none() {
            return Ok(());
        }
        let res = sqlx::query(
            "UPDATE starter_auth_users_tenants \
             SET display_name = COALESCE($1, display_name), \
                 audit_allow_sample = CASE WHEN $2 = 1 THEN $3 ELSE audit_allow_sample END \
             WHERE id = $4",
        )
        .bind(display_name)
        .bind(if audit_allow_sample.is_some() {
            1i32
        } else {
            0i32
        })
        .bind(audit_allow_sample.flatten())
        .bind(id)
        .execute(self.pool.sqlx())
        .await
        .map_err(err)?;
        if res.rows_affected() == 0 {
            return Err(TenantStoreError::NotFound(id.into()));
        }
        Ok(())
    }

    async fn add_member(&self, row: &MembershipRecord) -> Result<(), TenantStoreError> {
        let res = sqlx::query(
            "INSERT INTO starter_auth_users_memberships \
             (tenant_id, user_id, role) VALUES ($1, $2, $3)",
        )
        .bind(&row.tenant_id)
        .bind(&row.user_id)
        .bind(&row.role)
        .execute(self.pool.sqlx())
        .await;
        match res {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Err(
                TenantStoreError::SlugConflict(format!("{}:{}", row.tenant_id, row.user_id)),
            ),
            Err(e) => Err(err(e)),
        }
    }

    async fn patch_member_role(
        &self,
        tenant_id: &str,
        user_id: &str,
        role: &str,
    ) -> Result<(), TenantStoreError> {
        let res = sqlx::query(
            "UPDATE starter_auth_users_memberships SET role = $1 \
             WHERE tenant_id = $2 AND user_id = $3",
        )
        .bind(role)
        .bind(tenant_id)
        .bind(user_id)
        .execute(self.pool.sqlx())
        .await
        .map_err(err)?;
        if res.rows_affected() == 0 {
            return Err(TenantStoreError::NotFound(format!("{tenant_id}:{user_id}")));
        }
        Ok(())
    }

    async fn remove_member(&self, tenant_id: &str, user_id: &str) -> Result<(), TenantStoreError> {
        // Membership revoke cascades to token revoke in the same
        // transaction (see docs/design/auth/README.md).
        let mut tx = self.pool.sqlx().begin().await.map_err(err)?;
        let now = chrono::Utc::now();
        sqlx::query(
            "UPDATE starter_auth_users_tokens SET revoked_at = $1 \
             WHERE user_id = $2 AND tenant_id = $3 AND revoked_at IS NULL",
        )
        .bind(now)
        .bind(user_id)
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(err)?;
        let res = sqlx::query(
            "DELETE FROM starter_auth_users_memberships \
             WHERE tenant_id = $1 AND user_id = $2",
        )
        .bind(tenant_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(err)?;
        if res.rows_affected() == 0 {
            tx.rollback().await.map_err(err)?;
            return Err(TenantStoreError::NotFound(format!("{tenant_id}:{user_id}")));
        }
        tx.commit().await.map_err(err)?;
        Ok(())
    }

    async fn memberships_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<MembershipRecord>, TenantStoreError> {
        let rows = sqlx::query(
            "SELECT tenant_id, user_id, role \
             FROM starter_auth_users_memberships WHERE user_id = $1 \
             ORDER BY created_at ASC",
        )
        .bind(user_id)
        .fetch_all(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(rows.into_iter().map(map_membership).collect())
    }

    async fn members_of_tenant(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<MembershipRecord>, TenantStoreError> {
        let rows = sqlx::query(
            "SELECT tenant_id, user_id, role \
             FROM starter_auth_users_memberships WHERE tenant_id = $1 \
             ORDER BY created_at ASC",
        )
        .bind(tenant_id)
        .fetch_all(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(rows.into_iter().map(map_membership).collect())
    }

    async fn create_team(&self, row: &TeamRecord) -> Result<(), TenantStoreError> {
        let res = sqlx::query(
            "INSERT INTO starter_auth_users_teams \
             (id, tenant_id, slug, display_name) VALUES ($1, $2, $3, $4)",
        )
        .bind(&row.id)
        .bind(&row.tenant_id)
        .bind(&row.slug)
        .bind(&row.display_name)
        .execute(self.pool.sqlx())
        .await;
        match res {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Err(
                TenantStoreError::SlugConflict(format!("{}:{}", row.tenant_id, row.slug)),
            ),
            Err(e) => Err(err(e)),
        }
    }

    async fn delete_team(&self, team_id: &str) -> Result<(), TenantStoreError> {
        let res = sqlx::query("DELETE FROM starter_auth_users_teams WHERE id = $1")
            .bind(team_id)
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;
        if res.rows_affected() == 0 {
            return Err(TenantStoreError::NotFound(team_id.into()));
        }
        Ok(())
    }

    async fn get_team(&self, team_id: &str) -> Result<Option<TeamRecord>, TenantStoreError> {
        let row = sqlx::query(
            "SELECT id, tenant_id, slug, display_name \
             FROM starter_auth_users_teams WHERE id = $1 LIMIT 1",
        )
        .bind(team_id)
        .fetch_optional(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(row.map(|r| TeamRecord {
            id: r.get(0),
            tenant_id: r.get(1),
            slug: r.get(2),
            display_name: r.get(3),
        }))
    }

    async fn list_teams(&self, tenant_id: &str) -> Result<Vec<TeamRecord>, TenantStoreError> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, slug, display_name \
             FROM starter_auth_users_teams WHERE tenant_id = $1 \
             ORDER BY created_at ASC, slug ASC",
        )
        .bind(tenant_id)
        .fetch_all(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(rows
            .into_iter()
            .map(|r| TeamRecord {
                id: r.get(0),
                tenant_id: r.get(1),
                slug: r.get(2),
                display_name: r.get(3),
            })
            .collect())
    }

    async fn add_team_member(&self, team_id: &str, user_id: &str) -> Result<(), TenantStoreError> {
        let res = sqlx::query(
            "INSERT INTO starter_auth_users_team_members \
             (team_id, user_id) VALUES ($1, $2)",
        )
        .bind(team_id)
        .bind(user_id)
        .execute(self.pool.sqlx())
        .await;
        match res {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                // Idempotent: already a member is fine.
                Ok(())
            }
            Err(e) => Err(err(e)),
        }
    }

    async fn remove_team_member(
        &self,
        team_id: &str,
        user_id: &str,
    ) -> Result<(), TenantStoreError> {
        let res = sqlx::query(
            "DELETE FROM starter_auth_users_team_members \
             WHERE team_id = $1 AND user_id = $2",
        )
        .bind(team_id)
        .bind(user_id)
        .execute(self.pool.sqlx())
        .await
        .map_err(err)?;
        if res.rows_affected() == 0 {
            return Err(TenantStoreError::NotFound(format!("{team_id}:{user_id}")));
        }
        Ok(())
    }

    async fn team_slugs_for_user(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<Vec<String>, TenantStoreError> {
        // Join team_members → teams; filter by tenant so a row that
        // leaks past the FK does not surface a slug from another
        // tenant (rules are tenant-scoped — see docs/design/auth/).
        let rows = sqlx::query(
            "SELECT t.slug FROM starter_auth_users_teams t \
             INNER JOIN starter_auth_users_team_members m ON m.team_id = t.id \
             WHERE t.tenant_id = $1 AND m.user_id = $2 \
             ORDER BY t.slug ASC",
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_all(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(rows.into_iter().map(|r| r.get::<String, _>(0)).collect())
    }
}
