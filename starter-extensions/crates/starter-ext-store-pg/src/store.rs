//! `PgEnablementStore` — PostgreSQL-backed [`EnablementStore`] impl.
//!
//! Schema is owned by `migrations/0001_extensions_enablement.sql`:
//!
//! ```sql
//! CREATE TABLE extensions_enablement (
//!     extension_id TEXT PRIMARY KEY,
//!     state        TEXT NOT NULL CHECK (state IN ('enabled','disabled')),
//!     updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
//!     updated_by   TEXT NOT NULL
//! );
//! ```
//!
//! `set` writes go through an idempotent UPSERT keyed on `extension_id`.
//! The trait-required `set` records `system` as the actor; operator-driven
//! writes should call [`PgEnablementStore::set_as`] so the `updated_by`
//! column carries the real principal id for audit (see SCOPE Phase A).

use std::str::FromStr;

use async_trait::async_trait;
use sqlx::PgPool;
use starter_ext_server::{EnablementState, EnablementStore, StoreError};
use starter_ext_spi::ExtensionId;

/// PostgreSQL-backed [`EnablementStore`].
///
/// Clones share the underlying `PgPool`; cheap to pass around.
#[derive(Debug, Clone)]
pub struct PgEnablementStore {
    pool: PgPool,
}

impl PgEnablementStore {
    /// Wrap a `PgPool`. The pool must already point at a database where
    /// the `extensions_enablement` table exists — run this crate's
    /// migration (`migrations/0001_extensions_enablement.sql`) before
    /// the first call.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Borrow the underlying pool — for callers that want to run the
    /// migration through their own `Migrator`.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Set the state for `id`, recording `actor` in `updated_by`. The
    /// trait's plain `set` records `system` (autostart-on-boot etc.);
    /// operator-driven writes call this so the audit row carries the
    /// real principal id.
    pub async fn set_as(
        &self,
        actor: &str,
        id: &ExtensionId,
        state: EnablementState,
    ) -> Result<(), StoreError> {
        let state_str = state_to_str(state);
        sqlx::query(
            r#"
            INSERT INTO extensions_enablement (extension_id, state, updated_by)
            VALUES ($1, $2, $3)
            ON CONFLICT (extension_id) DO UPDATE
              SET state      = EXCLUDED.state,
                  updated_at = NOW(),
                  updated_by = EXCLUDED.updated_by
            "#,
        )
        .bind(id.as_str())
        .bind(state_str)
        .bind(actor)
        .execute(&self.pool)
        .await
        .map_err(|e| StoreError::new(format!("set_as: {e}")))?;
        Ok(())
    }

    /// List every persisted row, in `extension_id` order. Rows where the
    /// `state` column does not parse as `enabled`/`disabled` are skipped
    /// — the CHECK constraint should make that impossible, but we don't
    /// want a bad row to take the whole boot down.
    pub async fn list_all(&self) -> Result<Vec<(ExtensionId, EnablementState)>, StoreError> {
        let rows = sqlx::query_as::<_, (String, String)>(
            r#"SELECT extension_id, state FROM extensions_enablement ORDER BY extension_id"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StoreError::new(format!("list_all: {e}")))?;

        let mut out = Vec::with_capacity(rows.len());
        for (raw_id, raw_state) in rows {
            let Ok(id) = ExtensionId::from_str(&raw_id) else {
                continue;
            };
            let Some(state) = state_from_str(&raw_state) else {
                continue;
            };
            out.push((id, state));
        }
        Ok(out)
    }
}

#[async_trait]
impl EnablementStore for PgEnablementStore {
    async fn get(&self, id: &ExtensionId) -> Result<Option<EnablementState>, StoreError> {
        let row = sqlx::query_as::<_, (String,)>(
            r#"SELECT state FROM extensions_enablement WHERE extension_id = $1"#,
        )
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StoreError::new(format!("get: {e}")))?;
        Ok(row.and_then(|(s,)| state_from_str(&s)))
    }

    async fn set(&self, id: &ExtensionId, state: EnablementState) -> Result<(), StoreError> {
        self.set_as("system", id, state).await
    }
}

fn state_to_str(state: EnablementState) -> &'static str {
    match state {
        EnablementState::Enabled => "enabled",
        EnablementState::Disabled => "disabled",
    }
}

fn state_from_str(s: &str) -> Option<EnablementState> {
    match s {
        "enabled" => Some(EnablementState::Enabled),
        "disabled" => Some(EnablementState::Disabled),
        _ => None,
    }
}
