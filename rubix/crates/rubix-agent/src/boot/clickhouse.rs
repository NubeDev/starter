//! Apply rubix-owned ClickHouse migrations at boot.
//!
//! Reads the ClickHouse HTTP endpoint from the loaded `AgentConfig`
//! (which the `starter-config` layered loader assembles from
//! defaults < `rubix/dev/agent.toml` < `RUBIX_*` env). When unset,
//! the warehouse side is not configured in this deployment; the
//! binary logs a warn and continues so a developer can boot the
//! agent on a laptop without a ClickHouse running. See
//! [docs/design/warehouse/](../../../docs/design/warehouse/README.md)
//! and [docs/design/migrations/](../../../docs/design/migrations/README.md)
//! for the boot-order rules this fits into.
//!
//! The migration is registered through
//! `starter-store-clickhouse::MigrationRunner::with_extra_migration`
//! — there is no parallel runner. The warehouse-owned migrations
//! still run from the same runner; the rubix file is just an
//! extra in declaration order.
//!
//! The shared migration set includes `0005_entities_dict.sql`, a
//! `CREATE DICTIONARY` that pulls from Postgres via mustache-
//! substituted connection knobs. Rubix supplies a [`PgSource`]
//! derived from the agent's configured database URL so the runner
//! can render the file; rubix does not yet maintain the source
//! `entities` table itself, but `CREATE DICTIONARY IF NOT EXISTS`
//! is valid at DDL time and the dictionary is only consulted at
//! query time (not at boot).

use anyhow::Result;
use starter_store_clickhouse::{ChClient, ChConfig, MigrationRunner, PgSource};
use tracing::{info, warn};

/// The single rubix-owned ClickHouse migration applied at boot.
/// Embedded with `include_str!` so the binary does not need the
/// source tree at runtime.
const RUBIX_0002_HISTORY_UP: &str =
    include_str!("../../migrations/0002_history/up.sql");

/// Name of the ClickHouse database that owns every rubix-owned
/// warehouse table (today: `system_disk_history`; tomorrow:
/// L2/L3 marts). Matches `CLICKHOUSE_DB` in
/// `rubix/docker/docker-compose.dev.yaml` and the named-tenant
/// intent documented in
/// [docs/design/warehouse/](../../../docs/design/warehouse/README.md).
/// Every `ChClient` rubix-agent constructs binds to this database
/// so that unqualified DDL / INSERT / SELECT against
/// `system_disk_history` resolves here instead of `default`.
pub const RUBIX_CH_DATABASE: &str = "rubix";

/// Build a [`ChConfig`] that pins the connection to the rubix
/// database. Used by the boot wiring, the admin stdio MCP, and
/// any integration test that wants the same routing the binary
/// uses in production. Inherits the W8 `async_insert` default
/// from [`ChConfig::local`].
pub fn rubix_ch_config(url: impl Into<String>) -> ChConfig {
    ChConfig {
        database: RUBIX_CH_DATABASE.to_owned(),
        ..ChConfig::local(url)
    }
}

/// What happened during the CH migrations step.
#[derive(Debug, Clone)]
pub struct ChMigrationReport {
    /// `true` when `RUBIX_CH_URL` was unset and the step was a no-op.
    pub skipped: bool,
}

/// Apply the rubix-owned `0002_history` ClickHouse migration via
/// the shared `MigrationRunner`. Skips with a warn when
/// `clickhouse_url` is `None`.
///
/// `database_url` is the agent's Postgres DSN; it is parsed into
/// the [`PgSource`] required by the shared
/// `0005_entities_dict.sql` migration. When `None`, the runner
/// step is skipped entirely (the binary still boots) so a
/// developer can run the agent against a ClickHouse without a
/// Postgres alongside.
pub async fn apply_ch_migrations(
    clickhouse_url: Option<&str>,
    database_url: Option<&str>,
) -> Result<ChMigrationReport> {
    let Some(url) = clickhouse_url else {
        warn!(
            target: "rubix.boot",
            "ClickHouse URL unset — skipping ClickHouse migrations; agent will boot without warehouse-backed features",
        );
        return Ok(ChMigrationReport { skipped: true });
    };

    let Some(pg) = database_url.and_then(parse_pg_dsn) else {
        warn!(
            target: "rubix.boot",
            "ClickHouse migrations require a parseable Postgres DSN for the shared entities_dict; skipping CH migrations",
        );
        return Ok(ChMigrationReport { skipped: true });
    };

    // Ensure the named-tenant database exists before binding the
    // migration client to it. Issued against `default` so the
    // statement succeeds even on a fresh ClickHouse that was not
    // bootstrapped via `CLICKHOUSE_DB=rubix`. `IF NOT EXISTS`
    // keeps the step idempotent across re-runs.
    let bootstrap = ChClient::connect(ChConfig::local(url.to_owned()));
    bootstrap
        .inner()
        .query(&format!(
            "CREATE DATABASE IF NOT EXISTS {RUBIX_CH_DATABASE}"
        ))
        .execute()
        .await
        .map_err(|e| anyhow::anyhow!("create rubix CH database: {e}"))?;

    let client = ChClient::connect(rubix_ch_config(url.to_owned()));
    MigrationRunner::new(&client)
        .with_pg_source(pg)
        .with_extra_migration("rubix/0002_history/up.sql", RUBIX_0002_HISTORY_UP)
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("apply rubix CH migrations: {e}"))?;

    info!(
        target: "rubix.boot",
        "rubix ClickHouse migrations applied",
    );
    Ok(ChMigrationReport { skipped: false })
}

/// Parse a `postgres://user:password@host:port/db` DSN into the
/// [`PgSource`] the shared `0005_entities_dict.sql` migration
/// needs. Returns `None` if the URL is malformed or missing any
/// required component; callers treat that as "skip the CH step".
fn parse_pg_dsn(url: &str) -> Option<PgSource> {
    let rest = url
        .strip_prefix("postgres://")
        .or_else(|| url.strip_prefix("postgresql://"))?;
    let (creds, host_db) = rest.split_once('@')?;
    let (user, password) = creds.split_once(':')?;
    let (host_port, db) = host_db.split_once('/')?;
    let (host, port_str) = host_port.split_once(':').unwrap_or((host_port, "5432"));
    let port: u16 = port_str.parse().ok()?;
    let db = db.split('?').next().unwrap_or(db);
    Some(PgSource {
        host: host.to_owned(),
        port,
        user: user.to_owned(),
        password: password.to_owned(),
        db: db.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn skips_when_ch_url_unset() {
        let report = apply_ch_migrations(None, Some("postgres://u:p@h:5432/d"))
            .await
            .expect("skip path succeeds");
        assert!(report.skipped);
    }

    #[tokio::test]
    async fn skips_when_database_url_unset() {
        let report = apply_ch_migrations(Some("http://127.0.0.1:8124"), None)
            .await
            .expect("skip path succeeds");
        assert!(report.skipped);
    }

    #[test]
    fn parse_pg_dsn_extracts_components() {
        let pg = parse_pg_dsn("postgres://rubix:rubix-dev@127.0.0.1:5433/rubix")
            .expect("DSN parses");
        assert_eq!(pg.host, "127.0.0.1");
        assert_eq!(pg.port, 5433);
        assert_eq!(pg.user, "rubix");
        assert_eq!(pg.password, "rubix-dev");
        assert_eq!(pg.db, "rubix");
    }

    #[test]
    fn parse_pg_dsn_rejects_missing_password() {
        assert!(parse_pg_dsn("postgres://rubix@127.0.0.1:5433/rubix").is_none());
    }
}
