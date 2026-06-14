//! Resolve a datasource id into the engine's `datasource` sink config.
//!
//! A flow's output names a datasource by id (`{datasource, table, ...}`); the
//! pipeline engine, which never depends on this crate, needs the *resolved*
//! connection material instead. This is the single place that translation
//! happens for the write path: it fetches the tenant's record, recovers the
//! secret through the audited [`super::open_secret`] boundary (the same decrypt
//! audit the query side fires), and emits the JSON config the engine's
//! `datasource` sink builder consumes. The plaintext secret lives only inside the
//! returned config `Value`, which the caller hands straight to the pipeline
//! builder and drops — it is never persisted or logged here.

use serde_json::{json, Value};
use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use super::secret::Envelope;

/// Build the engine `datasource` sink config for datasource `id` within
/// `tenant_id`, writing into `table`. `actor` is recorded on the decrypt audit.
/// `batch_rows`/`batch_ms` are passed through to the engine's batching policy when
/// present. Returns `NotFound` when the datasource is not visible to the tenant
/// and `Invalid` for a kind that has no sink writer.
pub async fn resolve_sink_config(
    pool: &PgPool,
    envelope: &Envelope,
    tenant_id: &str,
    actor: &str,
    id: Uuid,
    table: &str,
    batch_rows: Option<u64>,
    batch_ms: Option<u64>,
) -> Result<Value, Error> {
    let record = super::get(pool, tenant_id, id)
        .await?
        .ok_or_else(|| Error::NotFound {
            what: format!("datasource {id}"),
        })?;

    let mut cfg = match record.kind.as_str() {
        "postgres" => {
            let secret = super::open_secret(pool, envelope, tenant_id, actor, id).await?;
            let port = u16::try_from(record.port).map_err(|_| Error::Invalid {
                message: format!("datasource port {} out of range", record.port),
            })?;
            json!({
                "type": "datasource",
                "kind": "postgres",
                "table": table,
                "conn": {
                    "host": record.host,
                    "port": port,
                    "database": record.database,
                    "user": record.db_user,
                    "password": secret,
                },
            })
        }
        other => {
            return Err(Error::Invalid {
                message: format!("datasource kind '{other}' has no sink writer"),
            })
        }
    };

    // Thread the optional batching overrides through to the engine config.
    if let Some(rows) = batch_rows {
        cfg["batch_rows"] = json!(rows);
    }
    if let Some(ms) = batch_ms {
        cfg["batch_ms"] = json!(ms);
    }
    Ok(cfg)
}
