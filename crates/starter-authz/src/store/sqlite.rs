//! Sqlite-backed [`PolicyStore`].

use async_trait::async_trait;
use sqlx::Row;
use starter_store_sqlite::Pool;

use super::{PolicyStore, PolicyStoreError, StoredAssignment, StoredRule};

/// The migrator handle the consumer registers with
/// `starter_store_sqlite::migrate(&pool).with_source(...)`.
///
/// Use it alongside any other source the binary needs:
///
/// ```ignore
/// use starter_authz::store::AUTHZ_SQLITE_MIGRATOR;
/// use starter_store_sqlite::{migrate, migrate::MigrationSource};
///
/// migrate(&pool)
///     .with_source(MigrationSource {
///         name: "starter_authz",
///         migrator: &AUTHZ_SQLITE_MIGRATOR,
///     })
///     .run().await?;
/// ```
pub static AUTHZ_SQLITE_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/starter_authz_sqlite");

/// Sqlite impl of [`PolicyStore`].
pub struct SqlitePolicyStore {
    pool: Pool,
}

impl SqlitePolicyStore {
    /// Wrap the pool. The `starter_authz` migration source must
    /// already have been applied.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

fn err(e: sqlx::Error) -> PolicyStoreError {
    PolicyStoreError::Backend(e.to_string())
}

fn map_assignment(row: sqlx::sqlite::SqliteRow) -> StoredAssignment {
    StoredAssignment {
        id: row.get(0),
        subject: row.get(1),
        role: row.get(2),
        created_by: row.get(3),
    }
}

fn map_rule(row: sqlx::sqlite::SqliteRow) -> Result<StoredRule, PolicyStoreError> {
    let actions_json: String = row.get(3);
    let actions: Vec<String> = serde_json::from_str(&actions_json)
        .map_err(|e| PolicyStoreError::Malformed(format!("rule actions json: {e}")))?;
    Ok(StoredRule {
        id: row.get(0),
        role: row.get(1),
        resource: row.get(2),
        actions,
        condition: row.get(4),
        effect: row.get(5),
        priority: row.get(6),
        created_by: row.get(7),
        tenant_id: row.get(8),
    })
}

#[async_trait]
impl PolicyStore for SqlitePolicyStore {
    async fn list_assignments(&self) -> Result<Vec<StoredAssignment>, PolicyStoreError> {
        let rows = sqlx::query(
            "SELECT id, subject, role, created_by FROM starter_authz_assignments \
             ORDER BY created_at ASC, id ASC",
        )
        .fetch_all(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(rows.into_iter().map(map_assignment).collect())
    }

    async fn list_rules(&self) -> Result<Vec<StoredRule>, PolicyStoreError> {
        let rows = sqlx::query(
            "SELECT id, role, resource, actions, condition, effect, priority, created_by, tenant_id \
             FROM starter_authz_rules \
             ORDER BY priority DESC, created_at ASC, id ASC",
        )
        .fetch_all(self.pool.sqlx())
        .await
        .map_err(err)?;
        rows.into_iter().map(map_rule).collect()
    }

    async fn insert_assignment(&self, row: &StoredAssignment) -> Result<(), PolicyStoreError> {
        let res = sqlx::query(
            "INSERT INTO starter_authz_assignments (id, subject, role, created_by) \
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(&row.id)
        .bind(&row.subject)
        .bind(&row.role)
        .bind(&row.created_by)
        .execute(self.pool.sqlx())
        .await;
        match res {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                Err(PolicyStoreError::Conflict(format!(
                    "assignment ({}, {}) already exists",
                    row.subject, row.role
                )))
            }
            Err(e) => Err(err(e)),
        }
    }

    async fn delete_assignment(&self, id: &str) -> Result<(), PolicyStoreError> {
        let res = sqlx::query("DELETE FROM starter_authz_assignments WHERE id = ?1")
            .bind(id)
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;
        if res.rows_affected() == 0 {
            return Err(PolicyStoreError::NotFound(format!("assignment {id}")));
        }
        Ok(())
    }

    async fn insert_rule(&self, row: &StoredRule) -> Result<(), PolicyStoreError> {
        let actions_json = serde_json::to_string(&row.actions)
            .map_err(|e| PolicyStoreError::Backend(format!("serialize actions: {e}")))?;
        let res = sqlx::query(
            "INSERT INTO starter_authz_rules \
                (id, role, resource, actions, condition, effect, priority, created_by, tenant_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(&row.id)
        .bind(&row.role)
        .bind(&row.resource)
        .bind(&actions_json)
        .bind(&row.condition)
        .bind(&row.effect)
        .bind(row.priority)
        .bind(&row.created_by)
        .bind(&row.tenant_id)
        .execute(self.pool.sqlx())
        .await;
        match res {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                Err(PolicyStoreError::Conflict(format!("rule {}", row.id)))
            }
            Err(e) => Err(err(e)),
        }
    }

    async fn update_rule(&self, row: &StoredRule) -> Result<(), PolicyStoreError> {
        let actions_json = serde_json::to_string(&row.actions)
            .map_err(|e| PolicyStoreError::Backend(format!("serialize actions: {e}")))?;
        let res = sqlx::query(
            "UPDATE starter_authz_rules \
             SET role = ?1, resource = ?2, actions = ?3, condition = ?4, \
                 effect = ?5, priority = ?6, tenant_id = ?7 \
             WHERE id = ?8",
        )
        .bind(&row.role)
        .bind(&row.resource)
        .bind(&actions_json)
        .bind(&row.condition)
        .bind(&row.effect)
        .bind(row.priority)
        .bind(&row.tenant_id)
        .bind(&row.id)
        .execute(self.pool.sqlx())
        .await
        .map_err(err)?;
        if res.rows_affected() == 0 {
            return Err(PolicyStoreError::NotFound(format!("rule {}", row.id)));
        }
        Ok(())
    }

    async fn delete_rule(&self, id: &str) -> Result<(), PolicyStoreError> {
        let res = sqlx::query("DELETE FROM starter_authz_rules WHERE id = ?1")
            .bind(id)
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;
        if res.rows_affected() == 0 {
            return Err(PolicyStoreError::NotFound(format!("rule {id}")));
        }
        Ok(())
    }
}
