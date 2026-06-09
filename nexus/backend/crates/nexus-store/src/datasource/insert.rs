//! Create a datasource for a tenant, sealing its secret on the way in.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{DatasourceRecord, NewDatasource};
use super::secret::Envelope;
use crate::tenant_tx;

/// Insert a new datasource owned by `tenant_id`. The plaintext secret is sealed
/// with `envelope` and only its ciphertext is written. Runs inside a
/// tenant-bound transaction so the RLS policy applies.
pub async fn insert(
    pool: &PgPool,
    envelope: &Envelope,
    tenant_id: &str,
    new: &NewDatasource,
) -> Result<DatasourceRecord, Error> {
    let sealed = envelope.seal(new.secret.as_bytes()).map_err(seal_err)?;

    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_datasources \
         (tenant_id, name, kind, host, port, database, db_user, \
          secret_cipher, secret_nonce, wrapped_data_key, data_key_nonce, key_version) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) \
         RETURNING id, key_version",
    )
    .bind(tenant_id)
    .bind(&new.name)
    .bind(&new.kind)
    .bind(&new.host)
    .bind(new.port)
    .bind(&new.database)
    .bind(&new.db_user)
    .bind(&sealed.secret_cipher)
    .bind(&sealed.secret_nonce)
    .bind(&sealed.wrapped_data_key)
    .bind(&sealed.data_key_nonce)
    .bind(sealed.key_version)
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
