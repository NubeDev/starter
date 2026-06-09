//! Apply a partial update to a datasource, re-sealing the secret if rotated.

use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use super::record::DatasourcePatch;
use super::secret::Envelope;
use crate::tenant_tx;

/// Apply `patch` to datasource `id` within `tenant_id`. Only the supplied fields
/// change; a supplied secret is re-sealed under a fresh data key. Returns
/// whether a row was updated (RLS makes a cross-tenant update a no-op).
pub async fn update(
    pool: &PgPool,
    envelope: &Envelope,
    tenant_id: &str,
    id: Uuid,
    patch: &DatasourcePatch,
) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;

    // COALESCE keeps the existing value where the patch field is NULL, so one
    // statement handles any subset of fields without dynamic SQL.
    let sealed = match patch.secret.as_ref() {
        Some(s) => Some(envelope.seal(s.as_bytes()).map_err(|e| Error::Internal {
            source: Box::new(e),
        })?),
        None => None,
    };
    let (cipher, nonce, wrapped, dk_nonce, version) = match &sealed {
        Some(s) => (
            Some(&s.secret_cipher),
            Some(&s.secret_nonce),
            Some(&s.wrapped_data_key),
            Some(&s.data_key_nonce),
            Some(s.key_version),
        ),
        None => (None, None, None, None, None),
    };

    let done = sqlx::query(
        "UPDATE nexus_datasources SET \
           name        = COALESCE($2, name), \
           host        = COALESCE($3, host), \
           port        = COALESCE($4, port), \
           database    = COALESCE($5, database), \
           db_user     = COALESCE($6, db_user), \
           secret_cipher    = COALESCE($7, secret_cipher), \
           secret_nonce     = COALESCE($8, secret_nonce), \
           wrapped_data_key = COALESCE($9, wrapped_data_key), \
           data_key_nonce   = COALESCE($10, data_key_nonce), \
           key_version      = COALESCE($11, key_version) \
         WHERE id = $1",
    )
    .bind(id)
    .bind(&patch.name)
    .bind(&patch.host)
    .bind(patch.port)
    .bind(&patch.database)
    .bind(&patch.db_user)
    .bind(cipher)
    .bind(nonce)
    .bind(wrapped)
    .bind(dk_nonce)
    .bind(version)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::Internal {
        source: Box::new(e),
    })?;
    tx.commit().await.map_err(|e| Error::Internal {
        source: Box::new(e),
    })?;
    Ok(done.rows_affected() > 0)
}
