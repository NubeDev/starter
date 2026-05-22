//! # starter-clipboard-postgres
//!
//! Postgres backend for [`starter_clipboard::ClipboardStore`]. Owns
//! the `starter_clipboard` table and HMAC-signs every entry with a
//! key fetched from [`starter_spi::SecretStore`] under
//! `starter.clipboard.hmac` (SCOPE §"Storage shape").
//!
//! Wire-up:
//!
//! ```ignore
//! use starter_store_postgres::migrate;
//! use starter_clipboard_postgres::{migration_source, PgClipboard, HMAC_SECRET_NAME};
//!
//! migrate(&pool).with_source(migration_source()).run().await?;
//! let secret = secrets.get(HMAC_SECRET_NAME)?.expect("clipboard hmac key");
//! let store = PgClipboard::new(pool.clone(), secret.expose().as_bytes())?;
//! let service = ClipboardService::new(Arc::new(store));
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use starter_clipboard::{ClipboardEntry, ClipboardStore};
use starter_spi::{Error, Result};
use starter_store_postgres::{MigrationSource, Pool};
use subtle::ConstantTimeEq;

/// Well-known secret name. Same constant as the SQLite backend so
/// operators rotate one key regardless of which storage they use.
pub const HMAC_SECRET_NAME: &str = "starter.clipboard.hmac";

/// Postgres migrator for the `starter_clipboard` table.
pub static CLIPBOARD_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Migration source identifier. Lives under its own
/// `_sqlx_migrations_clipboard` table so it cannot collide with
/// other starter migrators against the same database.
pub fn migration_source() -> MigrationSource {
    MigrationSource {
        name: "clipboard",
        migrator: &CLIPBOARD_MIGRATOR,
    }
}

type HmacSha256 = Hmac<Sha256>;

/// Field separator inside the signed message. `\x1e` (ASCII record
/// separator) cannot appear in a UTF-8 principal id, kind, or JSON
/// serialisation, so concatenation is injective.
const SEP: u8 = 0x1e;

/// Postgres-backed [`ClipboardStore`]. Stores the JSON payload as
/// TEXT (not JSONB) so the bytes signed at write time are the bytes
/// verified at read time.
pub struct PgClipboard {
    pool: Pool,
    key: Arc<Vec<u8>>,
}

impl PgClipboard {
    /// Wrap a pool with the HMAC key. The key SHOULD have at least
    /// 256 bits of entropy.
    pub fn new(pool: Pool, key: &[u8]) -> Result<Self> {
        if key.is_empty() {
            return Err(Error::Invalid {
                message: "starter.clipboard.hmac key is empty".into(),
            });
        }
        Ok(Self {
            pool,
            key: Arc::new(key.to_vec()),
        })
    }

    fn sign(&self, principal_id: &str, resource_kind: &str, payload: &str) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("hmac accepts any key length");
        mac.update(principal_id.as_bytes());
        mac.update(&[SEP]);
        mac.update(resource_kind.as_bytes());
        mac.update(&[SEP]);
        mac.update(payload.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }

    fn verify(
        &self,
        principal_id: &str,
        resource_kind: &str,
        payload: &str,
        signature: &[u8],
    ) -> bool {
        let expected = self.sign(principal_id, resource_kind, payload);
        expected.ct_eq(signature).into()
    }
}

#[async_trait]
impl ClipboardStore for PgClipboard {
    async fn put(&self, entry: ClipboardEntry) -> Result<()> {
        let payload_text = serde_json::to_string(&entry.payload).map_err(invalid_payload)?;
        let signature = self.sign(&entry.principal_id, &entry.resource_kind, &payload_text);

        sqlx::query(
            r#"INSERT INTO starter_clipboard (
                    id, principal_id, resource_kind, payload, signature,
                    created_at, expires_at
               ) VALUES ($1, $2, $3, $4, $5, $6, $7)
               ON CONFLICT (id) DO UPDATE SET
                    principal_id  = EXCLUDED.principal_id,
                    resource_kind = EXCLUDED.resource_kind,
                    payload       = EXCLUDED.payload,
                    signature     = EXCLUDED.signature,
                    created_at    = EXCLUDED.created_at,
                    expires_at    = EXCLUDED.expires_at"#,
        )
        .bind(&entry.id)
        .bind(&entry.principal_id)
        .bind(&entry.resource_kind)
        .bind(&payload_text)
        .bind(signature.as_slice())
        .bind(entry.created_at)
        .bind(entry.expires_at)
        .execute(self.pool.sqlx())
        .await
        .map_err(internal)?;
        Ok(())
    }

    async fn get(&self, principal_id: &str, id: &str) -> Result<Option<ClipboardEntry>> {
        let row: Option<(String, String, String, Vec<u8>, DateTime<Utc>, DateTime<Utc>)> =
            sqlx::query_as(
                r#"SELECT principal_id, resource_kind, payload, signature,
                          created_at, expires_at
                     FROM starter_clipboard
                    WHERE id = $1"#,
            )
            .bind(id)
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(internal)?;

        let Some((db_principal, resource_kind, payload, signature, created_at, expires_at)) = row
        else {
            return Ok(None);
        };

        // Cross-principal lookups MUST NOT reveal entry existence.
        if db_principal != principal_id {
            return Ok(None);
        }

        if !self.verify(&db_principal, &resource_kind, &payload, &signature) {
            return Err(Error::Invalid {
                message: format!("clipboard entry {id}: signature verification failed"),
            });
        }

        if expires_at <= Utc::now() {
            return Ok(None);
        }

        let payload_value: serde_json::Value =
            serde_json::from_str(&payload).map_err(invalid_payload)?;

        Ok(Some(ClipboardEntry {
            id: id.to_string(),
            principal_id: db_principal,
            resource_kind,
            payload: payload_value,
            created_at,
            expires_at,
        }))
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}

fn invalid_payload(e: serde_json::Error) -> Error {
    Error::Invalid {
        message: format!("clipboard payload JSON: {e}"),
    }
}
