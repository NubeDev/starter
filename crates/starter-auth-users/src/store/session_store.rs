//! `SessionStore` — manages the `starter_auth_users_sessions` table.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// One session row.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    /// Opaque session id (this is what lives in the cookie).
    pub id: String,
    /// Owning user.
    pub user_id: String,
    /// CSRF double-submit token paired with this session.
    pub csrf_token: String,
    /// Phase 7a — tenant the session is bound to (set at login
    /// from the user's membership row). `None` for pre-Phase-7
    /// sessions; tenant-scoped routes deny those with
    /// `no_tenant_binding`. Switching tenants is a re-login per
    /// SCOPE-EXT.md "Multi-tenant session model".
    pub tenant_id: Option<String>,
    /// Absolute expiry. Sessions past this are treated as not found.
    pub expires_at: DateTime<Utc>,
    /// Set when the user logs out. Treated as not found by lookups.
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Errors specific to session persistence.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SessionStoreError {
    /// Backing store failed.
    #[error("session store error: {0}")]
    Backend(String),
}

/// Persistence operations the cookie-session flow needs.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Insert a fresh session row. Returns the new record.
    /// Phase 7a — `tenant_id` is the binding the session is
    /// minted for. `None` keeps pre-Phase-7 behaviour (session
    /// without tenant); tenant-scoped routes deny those sessions.
    async fn create(
        &self,
        id: &str,
        user_id: &str,
        csrf_token: &str,
        tenant_id: Option<&str>,
        expires_at: DateTime<Utc>,
    ) -> Result<SessionRecord, SessionStoreError>;

    /// Look up a session by id. Skips rows that are expired or
    /// revoked.
    async fn find_active(&self, id: &str) -> Result<Option<SessionRecord>, SessionStoreError>;

    /// Mark a session revoked. Idempotent — revoking a missing or
    /// already-revoked session is not an error.
    async fn revoke(&self, id: &str) -> Result<(), SessionStoreError>;
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use sqlx::Row;
    use starter_store_sqlite::Pool;

    use super::{SessionRecord, SessionStore, SessionStoreError};

    /// sqlite-backed [`SessionStore`].
    pub struct SqliteSessionStore {
        pool: Pool,
    }

    impl SqliteSessionStore {
        /// Wrap the pool.
        pub fn new(pool: Pool) -> Self {
            Self { pool }
        }
    }

    fn err(e: sqlx::Error) -> SessionStoreError {
        SessionStoreError::Backend(e.to_string())
    }

    fn parse_dt(s: &str) -> Result<DateTime<Utc>, SessionStoreError> {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|d| d.with_timezone(&Utc))
            .map_err(|e| SessionStoreError::Backend(format!("bad timestamp {s:?}: {e}")))
    }

    fn map_row(row: sqlx::sqlite::SqliteRow) -> Result<SessionRecord, SessionStoreError> {
        let expires_at_s: String = row.get(3);
        let revoked_at_s: Option<String> = row.get(4);
        let tenant_id: Option<String> = row.get(5);
        Ok(SessionRecord {
            id: row.get(0),
            user_id: row.get(1),
            csrf_token: row.get(2),
            tenant_id,
            expires_at: parse_dt(&expires_at_s)?,
            revoked_at: revoked_at_s.as_deref().map(parse_dt).transpose()?,
        })
    }

    #[async_trait]
    impl SessionStore for SqliteSessionStore {
        async fn create(
            &self,
            id: &str,
            user_id: &str,
            csrf_token: &str,
            tenant_id: Option<&str>,
            expires_at: DateTime<Utc>,
        ) -> Result<SessionRecord, SessionStoreError> {
            let exp = expires_at.to_rfc3339();
            sqlx::query(
                "INSERT INTO starter_auth_users_sessions \
                 (id, user_id, csrf_token, tenant_id, expires_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(id)
            .bind(user_id)
            .bind(csrf_token)
            .bind(tenant_id)
            .bind(&exp)
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;
            Ok(SessionRecord {
                id: id.into(),
                user_id: user_id.into(),
                csrf_token: csrf_token.into(),
                tenant_id: tenant_id.map(str::to_owned),
                expires_at,
                revoked_at: None,
            })
        }

        async fn find_active(&self, id: &str) -> Result<Option<SessionRecord>, SessionStoreError> {
            let row = sqlx::query(
                "SELECT id, user_id, csrf_token, expires_at, revoked_at, tenant_id \
                 FROM starter_auth_users_sessions \
                 WHERE id = ?1 LIMIT 1",
            )
            .bind(id)
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(err)?;
            let rec = match row.map(map_row).transpose()? {
                Some(r) => r,
                None => return Ok(None),
            };
            if rec.revoked_at.is_some() || rec.expires_at <= Utc::now() {
                return Ok(None);
            }
            Ok(Some(rec))
        }

        async fn revoke(&self, id: &str) -> Result<(), SessionStoreError> {
            let now = Utc::now().to_rfc3339();
            sqlx::query(
                "UPDATE starter_auth_users_sessions SET revoked_at = ?1 \
                 WHERE id = ?2 AND revoked_at IS NULL",
            )
            .bind(&now)
            .bind(id)
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;
            Ok(())
        }
    }
}

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteSessionStore;

#[cfg(feature = "postgres")]
mod postgres {
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use sqlx::Row;
    use starter_store_postgres::Pool;

    use super::{SessionRecord, SessionStore, SessionStoreError};

    /// Postgres-backed [`SessionStore`]. Mirrors
    /// [`super::SqliteSessionStore`] row-for-row; the SQL differs
    /// only in bind placeholders (`$1` vs `?1`) and in the timestamp
    /// columns being real `TIMESTAMPTZ` (vs sqlite's rfc3339-in-TEXT).
    /// `chrono::DateTime<Utc>` binds and decodes against both column
    /// types via the `sqlx/chrono` feature; no rfc3339 parsing here.
    pub struct PgSessionStore {
        pool: Pool,
    }

    impl PgSessionStore {
        /// Wrap the pool. Run the `starter_auth_users` Postgres
        /// migrations first — see
        /// [`crate::migration::POSTGRES_MIGRATOR`].
        pub fn new(pool: Pool) -> Self {
            Self { pool }
        }
    }

    fn err(e: sqlx::Error) -> SessionStoreError {
        SessionStoreError::Backend(e.to_string())
    }

    fn map_row(row: sqlx::postgres::PgRow) -> Result<SessionRecord, SessionStoreError> {
        Ok(SessionRecord {
            id: row.get(0),
            user_id: row.get(1),
            csrf_token: row.get(2),
            expires_at: row.get(3),
            revoked_at: row.get(4),
            tenant_id: row.get(5),
        })
    }

    #[async_trait]
    impl SessionStore for PgSessionStore {
        async fn create(
            &self,
            id: &str,
            user_id: &str,
            csrf_token: &str,
            tenant_id: Option<&str>,
            expires_at: DateTime<Utc>,
        ) -> Result<SessionRecord, SessionStoreError> {
            sqlx::query(
                "INSERT INTO starter_auth_users_sessions \
                 (id, user_id, csrf_token, tenant_id, expires_at) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(id)
            .bind(user_id)
            .bind(csrf_token)
            .bind(tenant_id)
            .bind(expires_at)
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;
            Ok(SessionRecord {
                id: id.into(),
                user_id: user_id.into(),
                csrf_token: csrf_token.into(),
                tenant_id: tenant_id.map(str::to_owned),
                expires_at,
                revoked_at: None,
            })
        }

        async fn find_active(&self, id: &str) -> Result<Option<SessionRecord>, SessionStoreError> {
            let row = sqlx::query(
                "SELECT id, user_id, csrf_token, expires_at, revoked_at, tenant_id \
                 FROM starter_auth_users_sessions \
                 WHERE id = $1 LIMIT 1",
            )
            .bind(id)
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(err)?;
            let rec = match row.map(map_row).transpose()? {
                Some(r) => r,
                None => return Ok(None),
            };
            if rec.revoked_at.is_some() || rec.expires_at <= Utc::now() {
                return Ok(None);
            }
            Ok(Some(rec))
        }

        async fn revoke(&self, id: &str) -> Result<(), SessionStoreError> {
            sqlx::query(
                "UPDATE starter_auth_users_sessions SET revoked_at = $1 \
                 WHERE id = $2 AND revoked_at IS NULL",
            )
            .bind(Utc::now())
            .bind(id)
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;
            Ok(())
        }
    }
}

#[cfg(feature = "postgres")]
pub use postgres::PgSessionStore;
