//! # starter-clipboard-sqlite
//!
//! SQLite backend for [`starter_clipboard::ClipboardStore`]. Owns
//! the `starter_clipboard` table and HMAC-signs every entry with a
//! key fetched from [`starter_spi::SecretStore`] under
//! `starter.clipboard.hmac` (SCOPE §"Storage shape").
//!
//! Wire-up:
//!
//! ```ignore
//! use starter_store_sqlite::migrate;
//! use starter_clipboard_sqlite::{migration_source, SqliteClipboard, HMAC_SECRET_NAME};
//!
//! migrate(&pool).with_source(migration_source()).run().await?;
//! let secret = secrets.get(HMAC_SECRET_NAME)?.expect("clipboard hmac key");
//! let store = SqliteClipboard::new(pool.clone(), secret.expose().as_bytes())?;
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
use starter_store_sqlite::{MigrationSource, Pool};
use subtle::ConstantTimeEq;

/// Well-known secret name. Consumers put their HMAC key here in the
/// `SecretStore` at boot; rotating the key invalidates outstanding
/// clipboard entries (signatures stop verifying).
pub const HMAC_SECRET_NAME: &str = "starter.clipboard.hmac";

/// SQLite migrator for the `starter_clipboard` table.
pub static CLIPBOARD_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Migration source identifier. Lives under its own
/// `_sqlx_migrations_clipboard` table.
pub fn migration_source() -> MigrationSource {
    MigrationSource {
        name: "clipboard",
        migrator: &CLIPBOARD_MIGRATOR,
    }
}

type HmacSha256 = Hmac<Sha256>;

/// Field separator used inside the signed message. `\x1e` is the
/// ASCII "record separator" — it cannot appear in a UTF-8 principal
/// id, kind, or JSON serialisation, so concatenation is injective.
const SEP: u8 = 0x1e;

/// SQLite-backed [`ClipboardStore`]. Stores the canonical-ish JSON
/// payload as TEXT and signs every entry with HMAC-SHA256.
pub struct SqliteClipboard {
    pool: Pool,
    key: Arc<Vec<u8>>,
}

impl SqliteClipboard {
    /// Wrap a pool with the HMAC key. The key SHOULD have at least
    /// 256 bits of entropy; HMAC-SHA256 tolerates longer keys by
    /// pre-hashing.
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
        // ConstantTimeEq guards against timing oracles.
        expected.ct_eq(signature).into()
    }
}

#[async_trait]
impl ClipboardStore for SqliteClipboard {
    async fn put(&self, entry: ClipboardEntry) -> Result<()> {
        let payload_text = serde_json::to_string(&entry.payload).map_err(invalid_payload)?;
        let signature = self.sign(&entry.principal_id, &entry.resource_kind, &payload_text);
        let created_at = entry.created_at.to_rfc3339();
        let expires_at = entry.expires_at.to_rfc3339();

        sqlx::query(
            r#"INSERT INTO starter_clipboard (
                    id, principal_id, resource_kind, payload, signature,
                    created_at, expires_at
               ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
               ON CONFLICT(id) DO UPDATE SET
                    principal_id  = excluded.principal_id,
                    resource_kind = excluded.resource_kind,
                    payload       = excluded.payload,
                    signature     = excluded.signature,
                    created_at    = excluded.created_at,
                    expires_at    = excluded.expires_at"#,
        )
        .bind(&entry.id)
        .bind(&entry.principal_id)
        .bind(&entry.resource_kind)
        .bind(&payload_text)
        .bind(signature.as_slice())
        .bind(&created_at)
        .bind(&expires_at)
        .execute(self.pool.sqlx())
        .await
        .map_err(internal)?;
        Ok(())
    }

    async fn get(&self, principal_id: &str, id: &str) -> Result<Option<ClipboardEntry>> {
        let row: Option<(String, String, String, Vec<u8>, String, String)> = sqlx::query_as(
            r#"SELECT principal_id, resource_kind, payload, signature,
                      created_at, expires_at
                 FROM starter_clipboard
                WHERE id = ?1"#,
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
            // Signature mismatch → key rotated, row tampered with,
            // or some other corruption. Fail closed and surface it
            // to the operator via the structured error.
            return Err(Error::Invalid {
                message: format!("clipboard entry {id}: signature verification failed"),
            });
        }

        let expires_at = DateTime::parse_from_rfc3339(&expires_at)
            .map_err(|e| Error::Internal {
                source: format!("invalid expires_at on clipboard row {id}: {e}").into(),
            })?
            .with_timezone(&Utc);
        if expires_at <= Utc::now() {
            return Ok(None);
        }
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| Error::Internal {
                source: format!("invalid created_at on clipboard row {id}: {e}").into(),
            })?
            .with_timezone(&Utc);

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
