//! [`PgRubixTenantStore`] \u{2014} Postgres-backed implementation of
//! [`rubix_spi::tenant::TenantStore`] over the `rubix_tenants`
//! table.
//!
//! Sister to [`crate::audit::PgAuditPolicyStore`]: same layering
//! pattern (trait in `rubix-spi`, in-memory fake in
//! `rubix-tools`, Pg impl here), same `match pg_pool` selection
//! at registry-build time. The bundled `"system"` tenant is
//! seeded by [`crate::RUBIX_TENANTS_MIGRATION_SOURCE`] so a
//! fresh Pg-backed boot lands with the same row the in-memory
//! `InMemoryTenantStore::seeded(...)` registry path installs.
//!
//! Contract per [`rubix_spi::tenant::store`]:
//!
//! - [`create`](Self::create) \u{2014} returns
//!   [`Error::Conflict`] on either `tenant_id` or `name`
//!   collision. The Pg schema enforces both with
//!   `PRIMARY KEY (tenant_id)` and `UNIQUE (name)`; we map the
//!   `23505 unique_violation` constraint name back to a clean
//!   `Conflict { message }` rather than leaking the SQL error.
//! - [`put`](Self::put) \u{2014} the undo path. Bypasses
//!   uniqueness via `ON CONFLICT (tenant_id) DO UPDATE`; the
//!   snapshot must land verbatim. Note we do NOT also handle
//!   conflict on `name`: an undo that would violate the name
//!   constraint is a genuine semantic conflict (some other actor
//!   wrote that name between the original write and the undo)
//!   and surfacing the raw error is the right behavior.
//! - [`delete`](Self::delete) \u{2014} returns
//!   [`Error::NotFound`] when no row matches (the trait contract;
//!   distinct from `audit_policy.delete` which is idempotent).

use async_trait::async_trait;
use rubix_spi::starter::error::{Error, Result};
use rubix_spi::tenant::{TenantRow, TenantStore};
use starter_store_postgres::pool::Pool;

/// Cheap-to-clone handle over the [`Pool`].
#[derive(Clone)]
pub struct PgRubixTenantStore {
    pool: Pool,
}

impl PgRubixTenantStore {
    /// Construct over an existing [`Pool`]. The
    /// [`crate::RUBIX_TENANTS_MIGRATION_SOURCE`] must have been
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

/// Map a `23505 unique_violation` to a clean `Conflict` and pass
/// everything else through as `Internal`. The constraint name is
/// inspected so we can surface a message that matches the
/// in-memory impl ("tenant with id X already exists" vs
/// "tenant with name Y already exists").
fn map_create_err(row: &TenantRow, e: sqlx::Error) -> Error {
    if let Some(db_err) = e.as_database_error() {
        if db_err.code().as_deref() == Some("23505") {
            // Constraint name on the primary key is
            // `rubix_tenants_pkey`; on the unique name index it
            // is `rubix_tenants_name_key` (Postgres default).
            let constraint = db_err.constraint().unwrap_or("");
            return if constraint.contains("pkey") {
                Error::Conflict {
                    message: format!("tenant with id {} already exists", row.tenant_id),
                }
            } else {
                Error::Conflict {
                    message: format!("tenant with name {} already exists", row.name),
                }
            };
        }
    }
    backend(e)
}

#[derive(sqlx::FromRow)]
struct PgTenantRow {
    tenant_id: String,
    name: String,
    locale: String,
}

impl From<PgTenantRow> for TenantRow {
    fn from(r: PgTenantRow) -> Self {
        TenantRow {
            tenant_id: r.tenant_id,
            name: r.name,
            locale: r.locale,
        }
    }
}

const SELECT_COLS: &str = "tenant_id, name, locale";

#[async_trait]
impl TenantStore for PgRubixTenantStore {
    async fn list(&self) -> Result<Vec<TenantRow>> {
        let sql = format!(
            "SELECT {SELECT_COLS}
               FROM rubix_tenants
              ORDER BY tenant_id ASC"
        );
        let rows: Vec<PgTenantRow> = sqlx::query_as(&sql)
            .fetch_all(self.pool.sqlx())
            .await
            .map_err(backend)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get(&self, tenant_id: &str) -> Result<Option<TenantRow>> {
        let sql = format!(
            "SELECT {SELECT_COLS}
               FROM rubix_tenants
              WHERE tenant_id = $1
              LIMIT 1"
        );
        let row: Option<PgTenantRow> = sqlx::query_as(&sql)
            .bind(tenant_id)
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(backend)?;
        Ok(row.map(Into::into))
    }

    async fn create(&self, row: TenantRow) -> Result<TenantRow> {
        let sql = format!(
            "INSERT INTO rubix_tenants (tenant_id, name, locale)
                  VALUES ($1, $2, $3)
              RETURNING {SELECT_COLS}"
        );
        let inserted: PgTenantRow = sqlx::query_as(&sql)
            .bind(&row.tenant_id)
            .bind(&row.name)
            .bind(&row.locale)
            .fetch_one(self.pool.sqlx())
            .await
            .map_err(|e| map_create_err(&row, e))?;
        Ok(inserted.into())
    }

    async fn put(&self, row: TenantRow) -> Result<()> {
        sqlx::query(
            "INSERT INTO rubix_tenants (tenant_id, name, locale)
                  VALUES ($1, $2, $3)
             ON CONFLICT (tenant_id) DO UPDATE
                  SET name   = EXCLUDED.name,
                      locale = EXCLUDED.locale",
        )
        .bind(&row.tenant_id)
        .bind(&row.name)
        .bind(&row.locale)
        .execute(self.pool.sqlx())
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn delete(&self, tenant_id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM rubix_tenants WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(self.pool.sqlx())
            .await
            .map_err(backend)?;
        if result.rows_affected() == 0 {
            return Err(Error::NotFound {
                what: format!("tenant:{tenant_id}"),
            });
        }
        Ok(())
    }
}
