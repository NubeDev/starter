//! sqlite-backed [`OAuthStateStore`].
//!
//! The cross-instance handoff for the OAuth redirect flow. With the
//! in-memory default a user who starts on instance A and gets 302'd
//! by the provider to instance B sees `state token not found` on the
//! callback (the entry only exists in A's process RAM). Pointing
//! `OAUTH_STATE_STORE=sqlite` at a shared file (or any shared
//! pool) makes the entry visible to every node behind the load
//! balancer.
//!
//! TTL eviction is opportunistic — every `take` also deletes every
//! row past [`crate::state_store::STATE_TTL`]. No background task.
//! The map of in-flight redirects is bounded by realistic concurrency
//! across the TTL window (a few thousand at peak), so the sweep is
//! cheap and the schema stays simple.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::Row;
use starter_store_sqlite::Pool;

use super::{OAuthFlowState, OAuthStateError, OAuthStateStore, STATE_TTL};

/// sqlite-backed state store. Run the
/// `starter_auth_oauth_sqlite` migration source before using.
pub struct SqliteStateStore {
    pool: Pool,
}

impl SqliteStateStore {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

fn err(e: sqlx::Error) -> OAuthStateError {
    OAuthStateError::Backend(e.to_string())
}

/// `created_at` round-trips through the same string shape used by
/// the identity-store ts columns so the two impls behave the same
/// way under reflection.
fn parse_ts(s: &str) -> Result<DateTime<Utc>, OAuthStateError> {
    let naive = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.fZ"))
        .map_err(|e| OAuthStateError::Backend(format!("created_at parse: {e}")))?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

fn ts_to_db(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%d %H:%M:%S%.f").to_string()
}

fn map_row(row: sqlx::sqlite::SqliteRow) -> Result<OAuthFlowState, OAuthStateError> {
    let created_at_raw: String = row.get(5);
    Ok(OAuthFlowState {
        state: row.get(0),
        provider: row.get(1),
        pkce_verifier: row.get(2),
        return_to: row.get(3),
        link_mode_user_id: row.get(4),
        created_at: parse_ts(&created_at_raw)?,
    })
}

#[async_trait]
impl OAuthStateStore for SqliteStateStore {
    async fn put(&self, flow: OAuthFlowState) -> Result<(), OAuthStateError> {
        // `INSERT OR REPLACE` matches the in-memory impl's "last
        // write wins" semantics for the same `state` key.
        sqlx::query(
            "INSERT OR REPLACE INTO starter_auth_oauth_state \
                (state, provider, pkce_verifier, return_to, \
                 link_mode_user_id, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&flow.state)
        .bind(&flow.provider)
        .bind(&flow.pkce_verifier)
        .bind(&flow.return_to)
        .bind(&flow.link_mode_user_id)
        .bind(ts_to_db(flow.created_at))
        .execute(self.pool.sqlx())
        .await
        .map_err(err)?;
        Ok(())
    }

    async fn take(&self, state: &str) -> Result<Option<OAuthFlowState>, OAuthStateError> {
        // Sweep first so the SELECT can ignore TTL. Using a wall
        // clock cutoff rather than per-row comparison so the sweep
        // is one DELETE not a join.
        let ttl = chrono::Duration::from_std(STATE_TTL)
            .map_err(|e| OAuthStateError::Backend(format!("ttl conversion: {e}")))?;
        let cutoff = ts_to_db(Utc::now() - ttl);
        sqlx::query(
            "DELETE FROM starter_auth_oauth_state WHERE created_at < ?1",
        )
        .bind(&cutoff)
        .execute(self.pool.sqlx())
        .await
        .map_err(err)?;

        // `RETURNING *` would let us collapse this into one
        // statement, but sqlite < 3.35 lacks it and the dev pool
        // can run on any vendored version. The two-statement form
        // is correct under serialised isolation (sqlite's default).
        let row = sqlx::query(
            "SELECT state, provider, pkce_verifier, return_to, \
                    link_mode_user_id, created_at \
             FROM starter_auth_oauth_state WHERE state = ?1 LIMIT 1",
        )
        .bind(state)
        .fetch_optional(self.pool.sqlx())
        .await
        .map_err(err)?;
        let Some(row) = row else { return Ok(None) };
        let flow = map_row(row)?;
        sqlx::query("DELETE FROM starter_auth_oauth_state WHERE state = ?1")
            .bind(state)
            .execute(self.pool.sqlx())
            .await
            .map_err(err)?;
        Ok(Some(flow))
    }
}
