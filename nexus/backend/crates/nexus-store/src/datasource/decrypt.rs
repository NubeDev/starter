//! The single place a datasource secret is decrypted: at connection-build time,
//! tenant-scoped, and audited.
//!
//! No read endpoint returns the secret; only this function recovers it, and it
//! logs who/when/which on every call so a decrypt is always accountable. The
//! plaintext is returned in a `String` the caller is expected to drop promptly
//! (e.g. immediately after building the connection).

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::secret::{Envelope, SealedSecret};
use crate::tenant_tx;

/// Recover the plaintext connection secret for datasource `id` within `tenant_id`.
/// `actor` identifies who triggered the decrypt, for the audit record. Returns
/// `NotFound` when the datasource is not visible to the tenant.
pub async fn open_secret(
    pool: &PgPool,
    envelope: &Envelope,
    tenant_id: &str,
    actor: &str,
    id: Uuid,
) -> Result<String, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "SELECT secret_cipher, secret_nonce, wrapped_data_key, data_key_nonce, key_version \
         FROM nexus_datasources WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;

    let row = row.ok_or_else(|| Error::NotFound {
        what: format!("datasource {id}"),
    })?;
    let sealed = SealedSecret {
        secret_cipher: row.get("secret_cipher"),
        secret_nonce: row.get("secret_nonce"),
        wrapped_data_key: row.get("wrapped_data_key"),
        data_key_nonce: row.get("data_key_nonce"),
        key_version: row.get("key_version"),
    };
    let plaintext = envelope.open(&sealed).map_err(|e| Error::Internal {
        source: Box::new(e),
    })?;

    // Every decrypt is accountable. The value is never logged — only the fact.
    tracing::info!(
        target: "nexus.audit.datasource_decrypt",
        actor,
        tenant_id,
        datasource_id = %id,
        "datasource secret decrypted"
    );

    String::from_utf8(plaintext).map_err(|e| Error::Internal {
        source: Box::new(e),
    })
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
