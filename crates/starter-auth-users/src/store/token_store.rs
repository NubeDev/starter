//! `TokenStore` — manages the `starter_auth_users_tokens` table.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::scope::Scope;

/// One token row.
#[derive(Debug, Clone)]
pub struct TokenRecord {
    /// Public lookup id (the first segment of `sak_<id>.<secret>`).
    pub id: String,
    /// Owning user id.
    pub user_id: String,
    /// argon2id PHC string of the secret half.
    pub hashed_token: String,
    /// Attached scopes.
    pub scopes: Vec<Scope>,
    /// Phase 7a — tenant binding. `"*"` is the super-admin
    /// sentinel reserved for cross-tenant admin tokens. Once
    /// written, immutable (enforced by a DB-level trigger so a
    /// buggy migration cannot silently re-tenant a token).
    pub tenant_id: String,
    /// Absolute expiry (`None` = never).
    pub expires_at: Option<DateTime<Utc>>,
    /// Set when revoked. `None` = active.
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Errors specific to token persistence.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TokenStoreError {
    /// Backing store failed.
    #[error("token store error: {0}")]
    Backend(String),
    /// Scopes JSON column was malformed.
    #[error("token scopes column malformed: {0}")]
    BadScopes(String),
}

/// Persistence operations the API-token flow needs.
#[async_trait]
pub trait TokenStore: Send + Sync {
    /// Insert a token row. `tenant_id` is Phase 7a — the binding
    /// the token is minted for. Pass `"*"` for the super-admin
    /// sentinel (cross-tenant admin tokens); only callers that
    /// have verified the user holds global Admin role should do
    /// so.
    async fn create(
        &self,
        id: &str,
        user_id: &str,
        hashed_token: &str,
        scopes: &[Scope],
        tenant_id: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(), TokenStoreError>;

    /// Look up by public id; rejects expired / revoked rows.
    async fn find_active(&self, id: &str) -> Result<Option<TokenRecord>, TokenStoreError>;

    /// Update `last_used_at = now()`. Best-effort; the auth path
    /// logs but does not fail on errors here.
    async fn touch_last_used(&self, id: &str) -> Result<(), TokenStoreError>;

    /// Mark a token row revoked. Idempotent.
    async fn revoke(&self, id: &str) -> Result<(), TokenStoreError>;

    /// Phase 7a — revoke every token for `(user_id, tenant_id)`.
    /// Called from the membership-revoke transaction so a user
    /// removed from a tenant cannot continue to use a token
    /// minted while they were a member. Returns the number of
    /// rows affected. Idempotent.
    async fn revoke_for_membership(
        &self,
        user_id: &str,
        tenant_id: &str,
    ) -> Result<u64, TokenStoreError>;
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use sqlx::Row;
    use starter_store_sqlite::Pool;

    use super::{Scope, TokenRecord, TokenStore, TokenStoreError};

    /// sqlite-backed [`TokenStore`].
    pub struct SqliteTokenStore {
        pool: Pool,
    }

    impl SqliteTokenStore {
        /// Wrap the pool.
        pub fn new(pool: Pool) -> Self {
            Self { pool }
        }
    }

    fn err(e: sqlx::Error) -> TokenStoreError {
        TokenStoreError::Backend(e.to_string())
    }

    fn parse_dt(s: &str) -> Result<DateTime<Utc>, TokenStoreError> {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|d| d.with_timezone(&Utc))
            .map_err(|e| TokenStoreError::Backend(format!("bad timestamp {s:?}: {e}")))
    }

    fn map_row(row: sqlx::sqlite::SqliteRow) -> Result<TokenRecord, TokenStoreError> {
        let scopes_json: String = row.get(3);
        let raw: Vec<String> = serde_json::from_str(&scopes_json)
            .map_err(|e| TokenStoreError::BadScopes(e.to_string()))?;
        let scopes = raw.into_iter().map(Scope::new).collect();
        let expires_at_s: Option<String> = row.get(4);
        let revoked_at_s: Option<String> = row.get(5);
        let tenant_id: String = row.get(6);
        Ok(TokenRecord {
            id: row.get(0),
            user_id: row.get(1),
            hashed_token: row.get(2),
            scopes,
            tenant_id,
            expires_at: expires_at_s.as_deref().map(parse_dt).transpose()?,
            revoked_at: revoked_at_s.as_deref().map(parse_dt).transpose()?,
        })
    }

    #[async_trait]
    impl TokenStore for SqliteTokenStore {
        async fn create(
            &self,
            id: &str,
            user_id: &str,
            hashed_token: &str,
            scopes: &[Scope],
            tenant_id: &str,
            expires_at: Option<DateTime<Utc>>,
        ) -> Result<(), TokenStoreError> {
            let scope_strs: Vec<&str> = scopes.iter().map(Scope::as_str).collect();
            let scopes_json = serde_json::to_string(&scope_strs)
                .map_err(|e| TokenStoreError::BadScopes(e.to_string()))?;
            let exp = expires_at.map(|d| d.to_rfc3339());
            sqlx::query(
                "INSERT INTO starter_auth_users_tokens \
                 (id, user_id, hashed_token, scopes, tenant_id, expires_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(id)
            .bind(user_id)
            .bind(hashed_token)
            .bind(&scopes_json)
            .bind(tenant_id)
            .bind(exp.as_deref())
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;
            Ok(())
        }

        async fn find_active(&self, id: &str) -> Result<Option<TokenRecord>, TokenStoreError> {
            let row = sqlx::query(
                "SELECT id, user_id, hashed_token, scopes, expires_at, revoked_at, tenant_id \
                 FROM starter_auth_users_tokens WHERE id = ?1 LIMIT 1",
            )
            .bind(id)
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(err)?;
            let rec = match row.map(map_row).transpose()? {
                Some(r) => r,
                None => return Ok(None),
            };
            if rec.revoked_at.is_some() || rec.expires_at.is_some_and(|e| e <= Utc::now()) {
                return Ok(None);
            }
            Ok(Some(rec))
        }

        async fn touch_last_used(&self, id: &str) -> Result<(), TokenStoreError> {
            let now = Utc::now().to_rfc3339();
            sqlx::query("UPDATE starter_auth_users_tokens SET last_used_at = ?1 WHERE id = ?2")
                .bind(&now)
                .bind(id)
                .execute(self.pool.sqlx())
                .await
                .map_err(err)?;
            Ok(())
        }

        async fn revoke(&self, id: &str) -> Result<(), TokenStoreError> {
            let now = Utc::now().to_rfc3339();
            sqlx::query(
                "UPDATE starter_auth_users_tokens SET revoked_at = ?1 \
                 WHERE id = ?2 AND revoked_at IS NULL",
            )
            .bind(&now)
            .bind(id)
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;
            Ok(())
        }

        async fn revoke_for_membership(
            &self,
            user_id: &str,
            tenant_id: &str,
        ) -> Result<u64, TokenStoreError> {
            let now = Utc::now().to_rfc3339();
            let res = sqlx::query(
                "UPDATE starter_auth_users_tokens SET revoked_at = ?1 \
                 WHERE user_id = ?2 AND tenant_id = ?3 AND revoked_at IS NULL",
            )
            .bind(&now)
            .bind(user_id)
            .bind(tenant_id)
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;
            Ok(res.rows_affected())
        }
    }
}

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteTokenStore;

#[cfg(feature = "postgres")]
mod postgres {
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use sqlx::Row;
    use starter_store_postgres::Pool;

    use super::{Scope, TokenRecord, TokenStore, TokenStoreError};

    /// Postgres-backed [`TokenStore`]. Mirrors
    /// [`super::SqliteTokenStore`] row-for-row; the SQL differs
    /// only in bind placeholders (`$1` vs `?1`), in the timestamp
    /// columns being real `TIMESTAMPTZ` (vs sqlite's rfc3339-in-TEXT),
    /// and in `scopes` being `JSONB` (vs sqlite's TEXT). The Rust
    /// seam keeps `scopes` as a JSON-encoded `String` — matching the
    /// sqlite impl — by casting at the SQL boundary: `$4::jsonb` on
    /// write, `scopes::text` on read. The `JSONB` column type is
    /// asserted by the integration test against information_schema.
    pub struct PgTokenStore {
        pool: Pool,
    }

    impl PgTokenStore {
        /// Wrap the pool. Run the `starter_auth_users` Postgres
        /// migrations first — see
        /// [`crate::migration::postgres_migration_source`].
        pub fn new(pool: Pool) -> Self {
            Self { pool }
        }
    }

    fn err(e: sqlx::Error) -> TokenStoreError {
        TokenStoreError::Backend(e.to_string())
    }

    fn map_row(row: sqlx::postgres::PgRow) -> Result<TokenRecord, TokenStoreError> {
        let scopes_json: String = row.get(3);
        let raw: Vec<String> = serde_json::from_str(&scopes_json)
            .map_err(|e| TokenStoreError::BadScopes(e.to_string()))?;
        let scopes = raw.into_iter().map(Scope::new).collect();
        Ok(TokenRecord {
            id: row.get(0),
            user_id: row.get(1),
            hashed_token: row.get(2),
            scopes,
            expires_at: row.get(4),
            revoked_at: row.get(5),
            tenant_id: row.get(6),
        })
    }

    #[async_trait]
    impl TokenStore for PgTokenStore {
        async fn create(
            &self,
            id: &str,
            user_id: &str,
            hashed_token: &str,
            scopes: &[Scope],
            tenant_id: &str,
            expires_at: Option<DateTime<Utc>>,
        ) -> Result<(), TokenStoreError> {
            let scope_strs: Vec<&str> = scopes.iter().map(Scope::as_str).collect();
            let scopes_json = serde_json::to_string(&scope_strs)
                .map_err(|e| TokenStoreError::BadScopes(e.to_string()))?;
            sqlx::query(
                "INSERT INTO starter_auth_users_tokens \
                 (id, user_id, hashed_token, scopes, tenant_id, expires_at) \
                 VALUES ($1, $2, $3, $4::jsonb, $5, $6)",
            )
            .bind(id)
            .bind(user_id)
            .bind(hashed_token)
            .bind(&scopes_json)
            .bind(tenant_id)
            .bind(expires_at)
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;
            Ok(())
        }

        async fn find_active(&self, id: &str) -> Result<Option<TokenRecord>, TokenStoreError> {
            let row = sqlx::query(
                "SELECT id, user_id, hashed_token, scopes::text, expires_at, revoked_at, tenant_id \
                 FROM starter_auth_users_tokens WHERE id = $1 LIMIT 1",
            )
            .bind(id)
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(err)?;
            let rec = match row.map(map_row).transpose()? {
                Some(r) => r,
                None => return Ok(None),
            };
            if rec.revoked_at.is_some() || rec.expires_at.is_some_and(|e| e <= Utc::now()) {
                return Ok(None);
            }
            Ok(Some(rec))
        }

        async fn touch_last_used(&self, id: &str) -> Result<(), TokenStoreError> {
            sqlx::query("UPDATE starter_auth_users_tokens SET last_used_at = $1 WHERE id = $2")
                .bind(Utc::now())
                .bind(id)
                .execute(self.pool.sqlx())
                .await
                .map_err(err)?;
            Ok(())
        }

        async fn revoke(&self, id: &str) -> Result<(), TokenStoreError> {
            sqlx::query(
                "UPDATE starter_auth_users_tokens SET revoked_at = $1 \
                 WHERE id = $2 AND revoked_at IS NULL",
            )
            .bind(Utc::now())
            .bind(id)
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;
            Ok(())
        }

        async fn revoke_for_membership(
            &self,
            user_id: &str,
            tenant_id: &str,
        ) -> Result<u64, TokenStoreError> {
            let res = sqlx::query(
                "UPDATE starter_auth_users_tokens SET revoked_at = $1 \
                 WHERE user_id = $2 AND tenant_id = $3 AND revoked_at IS NULL",
            )
            .bind(Utc::now())
            .bind(user_id)
            .bind(tenant_id)
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;
            Ok(res.rows_affected())
        }
    }
}

#[cfg(feature = "postgres")]
pub use postgres::PgTokenStore;
