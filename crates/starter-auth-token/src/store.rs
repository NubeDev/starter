//! `ClaimStore` — the persistence seam under the claim flow.
//!
//! Two rows total live in the database, in their own tables:
//!
//! - `starter_auth_token_pending`: zero or one row. Holds the
//!   plaintext claim token until the operator consumes it.
//! - `starter_auth_token_claimed`: zero or one row. Holds the
//!   SHA-256 digest of the issued owner token plus the claim id.
//!
//! Backend impls live behind `feature = "sqlite"` and
//! `feature = "postgres"`; the trait is what every consumer programs
//! against.

use async_trait::async_trait;

use crate::claim::{ClaimError, PendingToken};

/// Persistence operations the claim flow needs. Implementations
/// must enforce: at most one pending row at any time; promoting to
/// claimed deletes the pending row in the same transaction;
/// resetting clears both.
#[async_trait]
pub trait ClaimStore: Send + Sync {
    /// Read the current pending token, if any. `Ok(None)` means the
    /// server has not been seeded (call
    /// [`crate::regenerate_claim_pending`]).
    async fn fetch_pending(&self) -> Result<Option<PendingToken>, ClaimError>;

    /// `true` once `promote_to_claimed` has been called and not
    /// reset since.
    async fn is_claimed(&self) -> Result<bool, ClaimError>;

    /// Atomically: delete the pending row at `id`, insert the
    /// claimed row carrying `digest`.
    async fn promote_to_claimed(&self, id: &str, digest: &[u8; 32]) -> Result<(), ClaimError>;

    /// Look up the claimed row's digest for bearer verification.
    /// `Ok(None)` means the server is not claimed.
    async fn fetch_claimed_digest(&self) -> Result<Option<ClaimedDigest>, ClaimError>;

    /// Wipe pending + claimed and insert a fresh pending row with
    /// `plaintext`. Returns the new pending row's id.
    async fn reset_with_new_pending(&self, plaintext: &str) -> Result<String, ClaimError>;
}

/// The persisted half of the owner token: claim id + SHA-256 digest.
#[derive(Debug, Clone)]
pub struct ClaimedDigest {
    /// Row id — becomes `Principal.subject`.
    pub claim_id: String,
    /// SHA-256 of the plaintext owner token.
    pub digest: [u8; 32],
}

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteClaimStore;

#[cfg(feature = "sqlite")]
mod sqlite {
    //! sqlx-backed `ClaimStore` over a `starter_store_sqlite::Pool`.

    use async_trait::async_trait;
    use sqlx::Row;
    use starter_store_sqlite::Pool;
    use uuid::Uuid;

    use super::{ClaimError, ClaimStore, ClaimedDigest, PendingToken};

    /// sqlite implementation of [`ClaimStore`].
    pub struct SqliteClaimStore {
        pool: Pool,
    }

    impl SqliteClaimStore {
        /// Wrap the pool. The required migrations live under
        /// `migrations/starter_auth_token/` in this crate; the
        /// consumer registers them via the namespaced runner.
        pub fn new(pool: Pool) -> Self {
            Self { pool }
        }
    }

    fn err(e: sqlx::Error) -> ClaimError {
        ClaimError::Store(e.to_string())
    }

    #[async_trait]
    impl ClaimStore for SqliteClaimStore {
        async fn fetch_pending(&self) -> Result<Option<PendingToken>, ClaimError> {
            let row = sqlx::query("SELECT id, plaintext FROM starter_auth_token_pending LIMIT 1")
                .fetch_optional(self.pool.sqlx())
                .await
                .map_err(err)?;
            Ok(row.map(|r| PendingToken {
                id: r.get::<String, _>(0),
                plaintext: r.get::<String, _>(1),
            }))
        }

        async fn is_claimed(&self) -> Result<bool, ClaimError> {
            let row = sqlx::query("SELECT 1 FROM starter_auth_token_claimed LIMIT 1")
                .fetch_optional(self.pool.sqlx())
                .await
                .map_err(err)?;
            Ok(row.is_some())
        }

        async fn promote_to_claimed(&self, id: &str, digest: &[u8; 32]) -> Result<(), ClaimError> {
            let mut tx = self.pool.sqlx().begin().await.map_err(err)?;
            let deleted = sqlx::query("DELETE FROM starter_auth_token_pending WHERE id = ?1")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(err)?;
            if deleted.rows_affected() == 0 {
                return Err(ClaimError::NoPending);
            }
            sqlx::query(
                "INSERT INTO starter_auth_token_claimed (claim_id, digest) VALUES (?1, ?2)",
            )
            .bind(id)
            .bind(&digest[..])
            .execute(&mut *tx)
            .await
            .map_err(err)?;
            tx.commit().await.map_err(err)?;
            Ok(())
        }

        async fn fetch_claimed_digest(&self) -> Result<Option<ClaimedDigest>, ClaimError> {
            let row =
                sqlx::query("SELECT claim_id, digest FROM starter_auth_token_claimed LIMIT 1")
                    .fetch_optional(self.pool.sqlx())
                    .await
                    .map_err(err)?;
            row.map(|r| {
                let bytes: Vec<u8> = r.get(1);
                let digest: [u8; 32] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| ClaimError::Store("digest column is not 32 bytes".into()))?;
                Ok(ClaimedDigest {
                    claim_id: r.get::<String, _>(0),
                    digest,
                })
            })
            .transpose()
        }

        async fn reset_with_new_pending(&self, plaintext: &str) -> Result<String, ClaimError> {
            let id = Uuid::new_v4().to_string();
            let mut tx = self.pool.sqlx().begin().await.map_err(err)?;
            sqlx::query("DELETE FROM starter_auth_token_claimed")
                .execute(&mut *tx)
                .await
                .map_err(err)?;
            sqlx::query("DELETE FROM starter_auth_token_pending")
                .execute(&mut *tx)
                .await
                .map_err(err)?;
            sqlx::query("INSERT INTO starter_auth_token_pending (id, plaintext) VALUES (?1, ?2)")
                .bind(&id)
                .bind(plaintext)
                .execute(&mut *tx)
                .await
                .map_err(err)?;
            // Bump the auth epoch so any in-memory cached bearer is
            // invalidated. Single-row table; INSERT-or-update is
            // fine.
            sqlx::query(
                "INSERT INTO starter_auth_token_epoch (id, epoch) VALUES (1, COALESCE(\
                    (SELECT epoch FROM starter_auth_token_epoch WHERE id = 1), 0) + 1)\
                 ON CONFLICT (id) DO UPDATE SET epoch = excluded.epoch",
            )
            .execute(&mut *tx)
            .await
            .map_err(err)?;
            tx.commit().await.map_err(err)?;
            Ok(id)
        }
    }
}
