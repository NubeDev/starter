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
    MAX_TENANT_DEPTH,
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
        parent_id: row.get(4),
    }
}

/// Column list shared by every tenant SELECT so `map_tenant`'s
/// positional indices stay in lock-step.
const TENANT_COLS: &str = "id, slug, display_name, audit_allow_sample, parent_id";

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

        // Tenant insert + closure maintenance share one transaction
        // (ADR-tenant-hierarchy).
        let mut tx = self.pool.sqlx().begin().await.map_err(err)?;

        if let Some(parent) = &row.parent_id {
            let parent_max: Option<i32> = sqlx::query_scalar(
                "SELECT MAX(depth) FROM starter_auth_users_tenant_closure \
                 WHERE descendant_id = $1",
            )
            .bind(parent)
            .fetch_one(&mut *tx)
            .await
            .map_err(err)?;
            match parent_max {
                None => {
                    tx.rollback().await.map_err(err)?;
                    return Err(TenantStoreError::ParentNotFound(parent.clone()));
                }
                Some(d) if d + 1 >= MAX_TENANT_DEPTH => {
                    tx.rollback().await.map_err(err)?;
                    return Err(TenantStoreError::MaxDepthExceeded(row.id.clone()));
                }
                Some(_) => {}
            }
        }

        let res = sqlx::query(
            "INSERT INTO starter_auth_users_tenants \
             (id, slug, display_name, audit_allow_sample, parent_id) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&row.id)
        .bind(&row.slug)
        .bind(&row.display_name)
        .bind(row.audit_allow_sample)
        .bind(&row.parent_id)
        .execute(&mut *tx)
        .await;
        if let Err(e) = res {
            tx.rollback().await.ok();
            return match e {
                sqlx::Error::Database(e) if e.is_unique_violation() => {
                    Err(TenantStoreError::SlugConflict(row.slug.clone()))
                }
                sqlx::Error::Database(e) if is_check_violation(e.as_ref()) => {
                    Err(TenantStoreError::ReservedSlug(row.slug.clone()))
                }
                e => Err(err(e)),
            };
        }

        sqlx::query(
            "INSERT INTO starter_auth_users_tenant_closure \
             (ancestor_id, descendant_id, depth) VALUES ($1, $1, 0)",
        )
        .bind(&row.id)
        .execute(&mut *tx)
        .await
        .map_err(err)?;

        if let Some(parent) = &row.parent_id {
            sqlx::query(
                "INSERT INTO starter_auth_users_tenant_closure \
                 (ancestor_id, descendant_id, depth) \
                 SELECT ancestor_id, $1, depth + 1 \
                 FROM starter_auth_users_tenant_closure \
                 WHERE descendant_id = $2",
            )
            .bind(&row.id)
            .bind(parent)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        }

        tx.commit().await.map_err(err)?;
        Ok(())
    }

    async fn list_tenants(&self) -> Result<Vec<TenantRecord>, TenantStoreError> {
        let rows = sqlx::query(&format!(
            "SELECT {TENANT_COLS} FROM starter_auth_users_tenants \
             ORDER BY created_at ASC, id ASC"
        ))
        .fetch_all(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(rows.into_iter().map(map_tenant).collect())
    }

    async fn get_tenant(&self, id: &str) -> Result<Option<TenantRecord>, TenantStoreError> {
        let row = sqlx::query(&format!(
            "SELECT {TENANT_COLS} FROM starter_auth_users_tenants WHERE id = $1 LIMIT 1"
        ))
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
        let row = sqlx::query(&format!(
            "SELECT {TENANT_COLS} FROM starter_auth_users_tenants WHERE slug = $1 LIMIT 1"
        ))
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

    async fn subtree_ids(&self, tenant_id: &str) -> Result<Vec<String>, TenantStoreError> {
        let rows = sqlx::query(
            "SELECT descendant_id FROM starter_auth_users_tenant_closure \
             WHERE ancestor_id = $1 ORDER BY depth ASC, descendant_id ASC",
        )
        .bind(tenant_id)
        .fetch_all(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(rows.into_iter().map(|r| r.get::<String, _>(0)).collect())
    }

    async fn is_ancestor(
        &self,
        ancestor: &str,
        descendant: &str,
    ) -> Result<bool, TenantStoreError> {
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM starter_auth_users_tenant_closure \
             WHERE ancestor_id = $1 AND descendant_id = $2",
        )
        .bind(ancestor)
        .bind(descendant)
        .fetch_one(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(n > 0)
    }

    async fn list_children(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<TenantRecord>, TenantStoreError> {
        let rows = sqlx::query(&format!(
            "SELECT {TENANT_COLS} FROM starter_auth_users_tenants \
             WHERE parent_id = $1 ORDER BY created_at ASC, id ASC"
        ))
        .bind(tenant_id)
        .fetch_all(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(rows.into_iter().map(map_tenant).collect())
    }

    async fn list_subtree(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<TenantRecord>, TenantStoreError> {
        let rows = sqlx::query(&format!(
            "SELECT {} FROM starter_auth_users_tenants t \
             INNER JOIN starter_auth_users_tenant_closure c ON c.descendant_id = t.id \
             WHERE c.ancestor_id = $1 ORDER BY c.depth ASC, t.created_at ASC, t.id ASC",
            TENANT_COLS
                .split(", ")
                .map(|c| format!("t.{c}"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
        .bind(tenant_id)
        .fetch_all(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(rows.into_iter().map(map_tenant).collect())
    }
}
