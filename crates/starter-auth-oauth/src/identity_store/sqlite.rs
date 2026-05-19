//! sqlite-backed [`IdentityStore`]. Behind `feature = "sqlite"`.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::Row;
use starter_store_sqlite::Pool;

use super::{IdentityStore, IdentityStoreError, OAuthIdentity};

/// sqlite implementation of [`IdentityStore`]. Run the
/// `starter_auth_oauth_sqlite` migration source before using.
pub struct SqliteIdentityStore {
    pool: Pool,
}

impl SqliteIdentityStore {
    /// Wrap an already-pooled connection.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

fn err(e: sqlx::Error) -> IdentityStoreError {
    IdentityStoreError::Backend(e.to_string())
}

/// `linked_at` is stored as a `TEXT` column (`CURRENT_TIMESTAMP`,
/// `YYYY-MM-DD HH:MM:SS` UTC). Parse leniently — both with and
/// without fractional seconds — so we don't fail just because
/// sqlite formatted the default differently from what we wrote.
fn parse_ts(s: &str) -> Result<DateTime<Utc>, IdentityStoreError> {
    let naive = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.fZ"))
        .map_err(|e| IdentityStoreError::Backend(format!("linked_at parse: {e}")))?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

fn map_row(row: sqlx::sqlite::SqliteRow) -> Result<OAuthIdentity, IdentityStoreError> {
    let linked_at_raw: String = row.get(5);
    Ok(OAuthIdentity {
        provider: row.get(0),
        provider_sub: row.get(1),
        user_id: row.get(2),
        email: row.get(3),
        display_name: row.get(4),
        linked_at: parse_ts(&linked_at_raw)?,
    })
}

#[async_trait]
impl IdentityStore for SqliteIdentityStore {
    async fn find(
        &self,
        provider: &str,
        provider_sub: &str,
    ) -> Result<Option<OAuthIdentity>, IdentityStoreError> {
        let row = sqlx::query(
            "SELECT provider, provider_sub, user_id, email, display_name, linked_at \
             FROM starter_auth_oauth_identities \
             WHERE provider = ?1 AND provider_sub = ?2 LIMIT 1",
        )
        .bind(provider)
        .bind(provider_sub)
        .fetch_optional(self.pool.sqlx())
        .await
        .map_err(err)?;
        row.map(map_row).transpose()
    }

    async fn insert(&self, identity: &OAuthIdentity) -> Result<(), IdentityStoreError> {
        let res = sqlx::query(
            "INSERT INTO starter_auth_oauth_identities \
                (provider, provider_sub, user_id, email, display_name, linked_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&identity.provider)
        .bind(&identity.provider_sub)
        .bind(&identity.user_id)
        .bind(&identity.email)
        .bind(&identity.display_name)
        // Round-trip as the same `TEXT` shape sqlite's default
        // `CURRENT_TIMESTAMP` uses; keeps `parse_ts` honest.
        .bind(identity.linked_at.format("%Y-%m-%d %H:%M:%S").to_string())
        .execute(self.pool.sqlx())
        .await;
        match res {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                Err(IdentityStoreError::Conflict)
            }
            Err(e) => Err(err(e)),
        }
    }

    async fn delete(&self, provider: &str, provider_sub: &str) -> Result<(), IdentityStoreError> {
        sqlx::query(
            "DELETE FROM starter_auth_oauth_identities \
             WHERE provider = ?1 AND provider_sub = ?2",
        )
        .bind(provider)
        .bind(provider_sub)
        .execute(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(())
    }

    async fn list_for_user(&self, user_id: &str) -> Result<Vec<OAuthIdentity>, IdentityStoreError> {
        let rows = sqlx::query(
            "SELECT provider, provider_sub, user_id, email, display_name, linked_at \
             FROM starter_auth_oauth_identities \
             WHERE user_id = ?1 ORDER BY linked_at ASC, provider ASC",
        )
        .bind(user_id)
        .fetch_all(self.pool.sqlx())
        .await
        .map_err(err)?;
        rows.into_iter().map(map_row).collect()
    }
}
