//! The datasource domain record and the input to create one.
//!
//! These are store-layer types, distinct from the wire DTOs in `nexus-spi`: the
//! record carries the sealed secret and tenant, which never cross the API
//! boundary. The route layer maps between them.

use uuid::Uuid;

/// A stored datasource. The connection secret is held only in sealed form; this
/// record is never serialized to a client.
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
}

/// Everything needed to create a datasource, including the plaintext secret to
/// seal. The plaintext lives only for the duration of the insert call.
#[derive(Debug, Clone)]
pub struct NewDatasource {
    pub name: String,
    pub kind: String,
    pub host: String,
    pub port: i32,
    pub database: String,
    pub db_user: String,
    pub secret: String,
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
