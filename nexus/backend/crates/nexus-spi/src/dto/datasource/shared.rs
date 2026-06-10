//! Types shared across the datasource verbs.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The kind of datasource, which selects the engine's input builder. v1 ships
/// SQL-over-Postgres as the first connector; the enum is the extension point as
/// more registry input builders land.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DatasourceKind {
    /// A SQL database queried through the `sql` input (driver: postgres).
    Postgres,
}

/// Connection details safe to return over the API: everything *except* the
/// secret. The password lives only as ciphertext in the store and is never
/// serialized here — `GET /datasources/:id` returns this redacted view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RedactedConnection {
    /// Database host.
    pub host: String,
    /// Database port.
    pub port: u16,
    /// Database name.
    pub database: String,
    /// Connecting user.
    pub user: String,
    /// Always `true` — present so the UI can show a "secret set" affordance
    /// without ever receiving the secret itself.
    pub has_secret: bool,
}
