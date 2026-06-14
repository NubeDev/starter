//! The datasource domain record and the input to create one.
//!
//! These are store-layer types, distinct from the wire DTOs in `nexus-spi`: the
//! record carries the sealed secret and tenant, which never cross the API
//! boundary. The route layer maps between them.

use serde_json::Value;
use uuid::Uuid;

/// A stored datasource. The connection secret is held only in sealed form; this
/// record is never serialized to a client.
///
/// The connection columns (`host`/`port`/`database`/`db_user`) and the secret are
/// populated for credentialed SQL kinds (postgres); secret-less file kinds
/// (parquet/csv) leave them at their defaults and instead carry their shape in
/// [`config`](Self::config) (`{path, has_header}`). `key_version` is `0` for a
/// row with no sealed secret.
#[derive(Debug, Clone)]
pub struct DatasourceRecord {
    pub id: Uuid,
    pub tenant_id: String,
    pub name: String,
    pub kind: String,
    pub host: String,
    pub port: i32,
    pub database: String,
    pub db_user: String,
    pub key_version: i32,
    /// Generic per-kind config for non-SQL connectors (`{path, has_header}` for
    /// file kinds). `None` for the Postgres-shaped rows whose config lives in the
    /// dedicated connection columns.
    pub config: Option<Value>,
}

/// Everything needed to create a datasource. `secret` carries the plaintext to
/// seal for credentialed kinds and is `None` for secret-less file kinds; the
/// plaintext lives only for the duration of the insert call.
#[derive(Debug, Clone)]
pub struct NewDatasource {
    pub name: String,
    pub kind: String,
    pub host: String,
    pub port: i32,
    pub database: String,
    pub db_user: String,
    /// Plaintext secret to seal, or `None` for a secret-less kind (parquet/csv).
    pub secret: Option<String>,
    /// Generic per-kind config (e.g. `{path, has_header}` for file kinds).
    pub config: Option<Value>,
}

/// A partial update. `None` fields are left unchanged; `secret = Some` rotates
/// the sealed secret.
#[derive(Debug, Clone, Default)]
pub struct DatasourcePatch {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub database: Option<String>,
    pub db_user: Option<String>,
    pub secret: Option<String>,
}
