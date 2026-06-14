//! Create a datasource for a tenant, sealing its secret on the way in.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{DatasourceRecord, NewDatasource};
use super::secret::Envelope;
use crate::tenant_tx;

/// Insert a new datasource owned by `tenant_id`. When `new.secret` is set the
/// plaintext is sealed with `envelope` and only its ciphertext is written; a
/// secret-less file kind leaves the four secret columns NULL and `key_version`
/// at `0`. Runs inside a tenant-bound transaction so the RLS policy applies.
pub async fn insert(
    pool: &PgPool,
    envelope: &Envelope,
    tenant_id: &str,
    new: &NewDatasource,
) -> Result<DatasourceRecord, Error> {
    // Seal only when the kind carries a secret; file kinds bind NULLs instead so
    // the secret columns stay empty rather than holding a fabricated cipher.
    let sealed = match new.secret.as_deref() {
        Some(s) => Some(envelope.seal(s.as_bytes()).map_err(seal_err)?),
        None => None,
    };
    let (cipher, nonce, wrapped, dk_nonce) = match &sealed {
        Some(s) => (
            Some(&s.secret_cipher),
            Some(&s.secret_nonce),
            Some(&s.wrapped_data_key),
            Some(&s.data_key_nonce),
        ),
        None => (None, None, None, None),
    };
    // A row with no sealed secret reports key_version 0 — distinguishable from the
    // `>= 1` of a real envelope, so callers never mistake it for a rotatable key.
    let key_version = sealed.as_ref().map(|s| s.key_version);

    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_datasources \
         (tenant_id, name, kind, host, port, database, db_user, \
          secret_cipher, secret_nonce, wrapped_data_key, data_key_nonce, key_version, config) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,COALESCE($12,0),$13) \
         RETURNING id, key_version",
    )
    .bind(tenant_id)
    .bind(&new.name)
    .bind(&new.kind)
    .bind(&new.host)
    .bind(new.port)
    .bind(&new.database)
    .bind(&new.db_user)
    .bind(cipher)
    .bind(nonce)
    .bind(wrapped)
    .bind(dk_nonce)
    .bind(key_version)
    .bind(&new.config)
    .fetch_one(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(DatasourceRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: tenant_id.to_string(),
        name: new.name.clone(),
        kind: new.kind.clone(),
        host: new.host.clone(),
        port: new.port,
        database: new.database.clone(),
        db_user: new.db_user.clone(),
        key_version: row.get::<i32, _>("key_version"),
        config: new.config.clone(),
    })
}

fn seal_err(e: super::secret::SecretError) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
