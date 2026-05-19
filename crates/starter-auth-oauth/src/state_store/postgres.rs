//! postgres-backed [`OAuthStateStore`]. Mirrors
//! [`super::sqlite::SqliteStateStore`] column-for-column; the only
//! divergence is `TIMESTAMPTZ` storage instead of `TEXT` and `$N`
//! placeholders. Run the `starter_auth_oauth_postgres` migration
//! source before using.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use starter_store_postgres::Pool;

use super::{OAuthFlowState, OAuthStateError, OAuthStateStore, STATE_TTL};

/// postgres-backed state store.
pub struct PostgresStateStore {
    pool: Pool,
}

impl PostgresStateStore {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

fn err(e: sqlx::Error) -> OAuthStateError {
    OAuthStateError::Backend(e.to_string())
}

fn map_row(row: sqlx::postgres::PgRow) -> Result<OAuthFlowState, OAuthStateError> {
    let created_at: DateTime<Utc> = row.get(5);
    Ok(OAuthFlowState {
        state: row.get(0),
        provider: row.get(1),
        pkce_verifier: row.get(2),
        return_to: row.get(3),
        link_mode_user_id: row.get(4),
        created_at,
    })
}

#[async_trait]
impl OAuthStateStore for PostgresStateStore {
    async fn put(&self, flow: OAuthFlowState) -> Result<(), OAuthStateError> {
        sqlx::query(
            "INSERT INTO starter_auth_oauth_state \
                (state, provider, pkce_verifier, return_to, \
                 link_mode_user_id, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (state) DO UPDATE SET \
                provider = EXCLUDED.provider, \
                pkce_verifier = EXCLUDED.pkce_verifier, \
                return_to = EXCLUDED.return_to, \
                link_mode_user_id = EXCLUDED.link_mode_user_id, \
                created_at = EXCLUDED.created_at",
        )
        .bind(&flow.state)
        .bind(&flow.provider)
        .bind(&flow.pkce_verifier)
        .bind(&flow.return_to)
        .bind(&flow.link_mode_user_id)
        .bind(flow.created_at)
        .execute(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(())
    }

    async fn take(&self, state: &str) -> Result<Option<OAuthFlowState>, OAuthStateError> {
        let ttl = chrono::Duration::from_std(STATE_TTL)
            .map_err(|e| OAuthStateError::Backend(format!("ttl conversion: {e}")))?;
        let cutoff = Utc::now() - ttl;
        sqlx::query("DELETE FROM starter_auth_oauth_state WHERE created_at < $1")
            .bind(cutoff)
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;

        // Single round-trip: DELETE ... RETURNING is the canonical
        // postgres pattern for take-or-none and is atomic without an
        // explicit transaction.
        let row = sqlx::query(
            "DELETE FROM starter_auth_oauth_state WHERE state = $1 \
             RETURNING state, provider, pkce_verifier, return_to, \
                       link_mode_user_id, created_at",
        )
        .bind(state)
        .fetch_optional(self.pool.sqlx())
        .await
        .map_err(err)?;
        row.map(map_row).transpose()
    }
}
