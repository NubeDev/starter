//! The resolved inputs a federated query joins over: one entry per alias.
//!
//! Federation never reads the control-plane database or decrypts a secret — like
//! the RW-04 `datasource` sink, the caller resolves each referenced datasource
//! into connection material (or a file path) through the audited envelope path
//! and hands the engine an already-resolved [`FederatedSource`] per alias. The
//! alias is the SQL-visible schema name (`ds_<alias>.<table>`); the engine maps
//! it to the provider that fetches its rows.

use serde::Deserialize;

/// One resolved input table, keyed in [`super::FederatedRequest::sources`] by its
/// SQL alias. The kind selects the provider: a live SQL datasource fetched over
/// sqlx, or a local file DataFusion reads natively.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FederatedSource {
    /// A Postgres/Timescale datasource. `conn` is the decrypted connection
    /// material (resolved upstream); `table` is the remote table to pull.
    Postgres {
        /// Decrypted connection components. The password never appears in logs.
        conn: PostgresConn,
        /// The remote table to read into the federation engine.
        table: String,
    },
    /// A local Parquet file. DataFusion reads it natively (no creds).
    Parquet {
        /// Absolute path to the `.parquet` file.
        path: String,
    },
    /// A local CSV file. DataFusion reads and infers it natively.
    Csv {
        /// Absolute path to the `.csv` file.
        path: String,
        /// Whether the first row is a header (default true).
        #[serde(default = "default_true")]
        has_header: bool,
    },
}

/// Resolved Postgres connection components. Discrete fields (not a URI) so a
/// password with URL metacharacters needs no percent-encoding. Mirrors the sink
/// side's `PostgresConn` so both resolve paths emit the same shape.
#[derive(Debug, Clone, Deserialize)]
pub struct PostgresConn {
    /// Database host.
    pub host: String,
    /// Database port.
    pub port: u16,
    /// Database name.
    pub database: String,
    /// Login user.
    pub user: String,
    /// Plaintext password, decrypted upstream. Never logged.
    pub password: String,
}

/// CSV headers default to present — the common case for an exported table.
fn default_true() -> bool {
    true
}
