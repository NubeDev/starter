//! `TenantStore` — manages the Phase 7a tenants + memberships
//! tables. The reserved-slug list is enforced both at the DB
//! level (CHECK constraint in the migration) and here in the
//! application before INSERT — the DB is the last line of
//! defence; the application gives a friendly error.
//!
//! See `DOCS/auth/authz/SCOPE-EXT.md` R11/R12.

use async_trait::async_trait;

/// Tenant row.
#[derive(Debug, Clone)]
pub struct TenantRecord {
    /// Stable id (UUID).
    pub id: String,
    /// URL-facing identifier.
    pub slug: String,
    /// Display name shown in UIs.
    pub display_name: String,
    /// Per-tenant override of the audit-log allow-sample rate.
    pub audit_allow_sample: Option<i32>,
}

/// Membership row joining a user to a tenant with a role.
#[derive(Debug, Clone)]
pub struct MembershipRecord {
    /// Tenant the user belongs to.
    pub tenant_id: String,
    /// User id.
    pub user_id: String,
    /// Role within the tenant. One of `reader | writer | admin`.
    pub role: String,
}

/// Tenant-store failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TenantStoreError {
    /// Backing store failed.
    #[error("tenant store error: {0}")]
    Backend(String),
    /// Slug collided with another tenant or with the reserved list.
    #[error("tenant slug conflict: {0}")]
    SlugConflict(String),
    /// Slug is on the reserved list (rejected before the INSERT).
    #[error("tenant slug reserved: {0}")]
    ReservedSlug(String),
    /// Lookup found no row.
    #[error("tenant not found: {0}")]
    NotFound(String),
}

/// Reserved slugs — rejected by both the application and the
/// DB-level CHECK constraint. Adding a new entry here is one
/// migration + this list bump.
pub const RESERVED_SLUGS: &[&str] = &[
    "admin", "api", "auth", "v1", "v2", "static", "health", "metrics", "openapi", "extensions",
    "mcp", "tools", "default", "system",
];

/// Returns true if `slug` is reserved (in the static list or
/// all-digits).
pub fn is_reserved_slug(slug: &str) -> bool {
    if RESERVED_SLUGS.iter().any(|r| *r == slug) {
        return true;
    }
    !slug.is_empty() && slug.bytes().all(|b| b.is_ascii_digit())
}

/// CRUD over tenants + memberships. The Phase 7a admin REST
/// routes (`/v1/tenants/*`) call into this trait.
#[async_trait]
pub trait TenantStore: Send + Sync {
    /// Insert a new tenant. Refuses reserved slugs with
    /// `ReservedSlug`; collides with `SlugConflict`.
    async fn create_tenant(&self, row: &TenantRecord) -> Result<(), TenantStoreError>;

    /// List every tenant (used by super-admin views).
    async fn list_tenants(&self) -> Result<Vec<TenantRecord>, TenantStoreError>;

    /// Look up a tenant by id.
    async fn get_tenant(&self, id: &str) -> Result<Option<TenantRecord>, TenantStoreError>;

    /// Look up a tenant by slug.
    async fn get_tenant_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<TenantRecord>, TenantStoreError>;

    /// Patch display_name / audit_allow_sample. Slug is immutable
    /// per SCOPE-EXT.md (admins must re-create the tenant to
    /// rename the slug).
    async fn patch_tenant(
        &self,
        id: &str,
        display_name: Option<&str>,
        audit_allow_sample: Option<Option<i32>>,
    ) -> Result<(), TenantStoreError>;

    /// Add a membership.
    async fn add_member(&self, row: &MembershipRecord) -> Result<(), TenantStoreError>;

    /// Patch a membership's role.
    async fn patch_member_role(
        &self,
        tenant_id: &str,
        user_id: &str,
        role: &str,
    ) -> Result<(), TenantStoreError>;

    /// Delete a membership.
    async fn remove_member(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<(), TenantStoreError>;

    /// List a user's memberships (used by login / OAuth callback
    /// to choose a tenant).
    async fn memberships_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<MembershipRecord>, TenantStoreError>;

    /// List the members of a tenant (used by admin UIs).
    async fn members_of_tenant(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<MembershipRecord>, TenantStoreError>;
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use async_trait::async_trait;
    use sqlx::Row;
    use starter_store_sqlite::Pool;

    use super::{
        is_reserved_slug, MembershipRecord, TenantRecord, TenantStore, TenantStoreError,
    };

    /// sqlite-backed [`TenantStore`].
    pub struct SqliteTenantStore {
        pool: Pool,
    }

    impl SqliteTenantStore {
        /// Wrap the pool. The `starter_auth_users` migrations
        /// (including `0005_tenants.sql`) must have been applied.
        pub fn new(pool: Pool) -> Self {
            Self { pool }
        }

        /// Borrow the pool — needed by the admin route handlers
        /// that revoke tokens in the same transaction as a
        /// membership delete (SCOPE-EXT.md R12 cascade).
        pub fn pool(&self) -> &Pool {
            &self.pool
        }
    }

    fn err(e: sqlx::Error) -> TenantStoreError {
        TenantStoreError::Backend(e.to_string())
    }

    fn map_tenant(row: sqlx::sqlite::SqliteRow) -> TenantRecord {
        TenantRecord {
            id: row.get(0),
            slug: row.get(1),
            display_name: row.get(2),
            audit_allow_sample: row.get(3),
        }
    }

    fn map_membership(row: sqlx::sqlite::SqliteRow) -> MembershipRecord {
        MembershipRecord {
            tenant_id: row.get(0),
            user_id: row.get(1),
            role: row.get(2),
        }
    }

    #[async_trait]
    impl TenantStore for SqliteTenantStore {
        async fn create_tenant(&self, row: &TenantRecord) -> Result<(), TenantStoreError> {
            if is_reserved_slug(&row.slug) {
                return Err(TenantStoreError::ReservedSlug(row.slug.clone()));
            }
            let res = sqlx::query(
                "INSERT INTO starter_auth_users_tenants \
                 (id, slug, display_name, audit_allow_sample) \
                 VALUES (?1, ?2, ?3, ?4)",
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
                Err(sqlx::Error::Database(e))
                    if e.message().contains("CHECK constraint failed") =>
                {
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
                 FROM starter_auth_users_tenants WHERE id = ?1 LIMIT 1",
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
                 FROM starter_auth_users_tenants WHERE slug = ?1 LIMIT 1",
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
            // Build the UPDATE dynamically; coalesce-style updates
            // are clearer than per-field queries for two fields.
            if display_name.is_none() && audit_allow_sample.is_none() {
                return Ok(());
            }
            let res = sqlx::query(
                "UPDATE starter_auth_users_tenants \
                 SET display_name = COALESCE(?1, display_name), \
                     audit_allow_sample = CASE WHEN ?2 = 1 THEN ?3 ELSE audit_allow_sample END \
                 WHERE id = ?4",
            )
            .bind(display_name)
            .bind(if audit_allow_sample.is_some() { 1 } else { 0 })
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
                 (tenant_id, user_id, role) VALUES (?1, ?2, ?3)",
            )
            .bind(&row.tenant_id)
            .bind(&row.user_id)
            .bind(&row.role)
            .execute(self.pool.sqlx())
            .await;
            match res {
                Ok(_) => Ok(()),
                Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                    Err(TenantStoreError::SlugConflict(format!(
                        "{}:{}",
                        row.tenant_id, row.user_id
                    )))
                }
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
                "UPDATE starter_auth_users_memberships SET role = ?1 \
                 WHERE tenant_id = ?2 AND user_id = ?3",
            )
            .bind(role)
            .bind(tenant_id)
            .bind(user_id)
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;
            if res.rows_affected() == 0 {
                return Err(TenantStoreError::NotFound(format!(
                    "{tenant_id}:{user_id}"
                )));
            }
            Ok(())
        }

        async fn remove_member(
            &self,
            tenant_id: &str,
            user_id: &str,
        ) -> Result<(), TenantStoreError> {
            // SCOPE-EXT.md R12 — membership revoke cascades to
            // token_store.revoke_for_membership IN THE SAME TXN.
            // We open one transaction, delete the membership and
            // revoke every token bound to (user_id, tenant_id).
            let mut tx = self.pool.sqlx().begin().await.map_err(err)?;
            let now = chrono::Utc::now().to_rfc3339();
            sqlx::query(
                "UPDATE starter_auth_users_tokens SET revoked_at = ?1 \
                 WHERE user_id = ?2 AND tenant_id = ?3 AND revoked_at IS NULL",
            )
            .bind(&now)
            .bind(user_id)
            .bind(tenant_id)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
            let res = sqlx::query(
                "DELETE FROM starter_auth_users_memberships \
                 WHERE tenant_id = ?1 AND user_id = ?2",
            )
            .bind(tenant_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
            if res.rows_affected() == 0 {
                tx.rollback().await.map_err(err)?;
                return Err(TenantStoreError::NotFound(format!(
                    "{tenant_id}:{user_id}"
                )));
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
                 FROM starter_auth_users_memberships WHERE user_id = ?1 \
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
                 FROM starter_auth_users_memberships WHERE tenant_id = ?1 \
                 ORDER BY created_at ASC",
            )
            .bind(tenant_id)
            .fetch_all(self.pool.sqlx())
            .await
            .map_err(err)?;
            Ok(rows.into_iter().map(map_membership).collect())
        }
    }
}

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteTenantStore;
