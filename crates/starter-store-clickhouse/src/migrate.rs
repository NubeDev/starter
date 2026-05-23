//! In-crate migration runner.
//!
//! ClickHouse has no `sqlx::migrate` equivalent in the Rust
//! ecosystem, so we ship a tiny one. The contract:
//!
//! - The `.sql` files in [`MIGRATION_SOURCE`] are applied in the
//!   order listed by [`crate::MIGRATION_FILES`].
//! - Each file is a single DDL statement (CH DDL is
//!   non-transactional, so multi-statement files would not roll
//!   back on partial failure — one statement per file makes a
//!   crash mid-apply observable).
//! - Every file uses `IF NOT EXISTS`, so the runner is idempotent
//!   without a versions table — re-running is a no-op.
//! - A small `_starter_ch_migrations` table is still written for
//!   observability (filename + applied_at) but the apply path
//!   does not gate on it. This is deliberate: if an operator
//!   truncates the table the next run will re-apply DDL safely,
//!   and the table can be inspected to confirm the apply
//!   timestamp.
//! - File 0005 contains mustache `{{name}}` placeholders for the
//!   Postgres dictionary source. Callers must call
//!   [`MigrationRunner::with_pg_source`] before [`run`] or the
//!   runner refuses to apply 0005.

use std::collections::HashMap;

use crate::client::{ChClient, ChClientError};

/// Marker for the migration source. The actual file contents are
/// `include_str!`'d in [`MIGRATION_BLOBS`] so callers do not need
/// the source tree at runtime.
pub const MIGRATION_SOURCE: &str = "starter-store-clickhouse/migrations";

const MIGRATION_BLOBS: &[(&str, &str)] = &[
    (
        "0001_raw_events.sql",
        include_str!("../migrations/0001_raw_events.sql"),
    ),
    (
        "0002_samples.sql",
        include_str!("../migrations/0002_samples.sql"),
    ),
    (
        "0003_events.sql",
        include_str!("../migrations/0003_events.sql"),
    ),
    (
        "0004_documents.sql",
        include_str!("../migrations/0004_documents.sql"),
    ),
    (
        "0005_entities_dict.sql",
        include_str!("../migrations/0005_entities_dict.sql"),
    ),
];

/// Postgres dictionary-source config. Required for migration 0005.
#[derive(Clone, Debug)]
pub struct PgSource {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub db: String,
}

/// Migration error envelope.
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("missing PgSource for migration 0005_entities_dict.sql")]
    MissingPgSource,
    #[error("unresolved placeholder {{{0}}} in {1}")]
    UnresolvedPlaceholder(String, &'static str),
    #[error("clickhouse: {0}")]
    Client(#[from] ChClientError),
    #[error("clickhouse: {0}")]
    Clickhouse(#[from] clickhouse::error::Error),
}

/// In-crate migration runner. Mirrors the shape of
/// `sqlx::migrate::Migrator` to keep the call site familiar.
pub struct MigrationRunner<'c> {
    client: &'c ChClient,
    pg_source: Option<PgSource>,
}

impl<'c> MigrationRunner<'c> {
    pub fn new(client: &'c ChClient) -> Self {
        Self {
            client,
            pg_source: None,
        }
    }

    /// Provide the Postgres connection coordinates substituted into
    /// migration 0005. Required if 0005 is on the apply set.
    pub fn with_pg_source(mut self, src: PgSource) -> Self {
        self.pg_source = Some(src);
        self
    }

    /// Apply every migration in declaration order. Idempotent: each
    /// statement is `IF NOT EXISTS`, so re-running is safe.
    pub async fn run(self) -> Result<(), MigrationError> {
        // Ensure the audit table exists. Best-effort — if this
        // fails we still try to apply DDL (the audit table is for
        // observability, not control flow).
        let _ = self
            .client
            .inner()
            .query(
                "CREATE TABLE IF NOT EXISTS _starter_ch_migrations (\
                    filename String,\
                    applied_at DateTime DEFAULT now()\
                 ) ENGINE = MergeTree ORDER BY applied_at",
            )
            .execute()
            .await;

        for (name, blob) in MIGRATION_BLOBS {
            let sql = render(name, blob, self.pg_source.as_ref())?;
            self.client
                .inner()
                .query(&sql)
                .execute()
                .await
                .map_err(MigrationError::Clickhouse)?;
            let _ = self
                .client
                .inner()
                .query("INSERT INTO _starter_ch_migrations(filename) VALUES (?)")
                .bind(*name)
                .execute()
                .await;
        }
        Ok(())
    }
}

fn render(
    name: &'static str,
    blob: &str,
    pg: Option<&PgSource>,
) -> Result<String, MigrationError> {
    if !blob.contains("{{") {
        return Ok(blob.to_string());
    }
    let pg = pg.ok_or(MigrationError::MissingPgSource)?;
    let mut vars: HashMap<&str, String> = HashMap::new();
    vars.insert("pg_host", pg.host.clone());
    vars.insert("pg_port", pg.port.to_string());
    vars.insert("pg_user", pg.user.clone());
    vars.insert("pg_password", pg.password.clone());
    vars.insert("pg_db", pg.db.clone());

    let mut out = blob.to_string();
    for (k, v) in &vars {
        out = out.replace(&format!("{{{{{k}}}}}"), v);
    }
    if let Some(start) = out.find("{{") {
        let end = out[start..].find("}}").map(|e| e + start + 2).unwrap_or(out.len());
        let placeholder = &out[start + 2..end - 2];
        return Err(MigrationError::UnresolvedPlaceholder(
            placeholder.to_string(),
            name,
        ));
    }
    Ok(out)
}
