//! Apply starter-changelog migrations at boot.
//!
//! Reads the Postgres DSN from the loaded `AgentConfig` (which the
//! `starter-config` layered loader assembles from defaults <
//! `rubix/dev/agent.toml` < `RUBIX_*` env). If the DSN is unset, the
//! binary logs a warn and continues without any DB I/O so a developer
//! can still boot the agent on a laptop without Postgres running.
//! See [docs/design/migrations/](../../../docs/design/migrations/README.md)
//! for the full boot-order plan.

use anyhow::Result;
use rubix_store_postgres::{
    DASHBOARDS_DEFINITIONS_MIGRATION_SOURCE, FLOWS_DEFINITIONS_MIGRATION_SOURCE,
    UNDO_SNAPSHOTS_MIGRATION_SOURCE,
};
use starter_auth_users::migration::postgres_migration_source;
use starter_changelog_postgres::migration_source;
use starter_store_postgres::{migrate, pool::connect, SCHEDULED_FLOWS_MIGRATION_SOURCE};
use tracing::{info, warn};

/// What happened during the migrations step. Returned so the boot
/// log can announce the source count + skip reason in one place.
#[derive(Debug, Clone)]
pub struct MigrationReport {
    /// Number of distinct migration sources applied (0 when the
    /// step was skipped).
    pub sources_applied: usize,
    /// True when the DSN was unset and the step was a no-op.
    pub skipped: bool,
}

/// Apply every rubix-owned migration source against the supplied DSN.
/// `None` is the laptop / no-Postgres path — log a warn and return a
/// `skipped` report so the boot can continue without DB-backed
/// features.
pub async fn apply_migrations(dsn: Option<&str>) -> Result<MigrationReport> {
    let Some(dsn) = dsn else {
        warn!(
            target: "rubix.boot",
            "Postgres DSN unset — skipping migrations; agent will boot without DB-backed features",
        );
        return Ok(MigrationReport {
            sources_applied: 0,
            skipped: true,
        });
    };

    let pool = connect(dsn)
        .await
        .map_err(|e| anyhow::anyhow!("connect to RUBIX_DSN: {e}"))?;

    // Chain every rubix-owned source through one runner so a failure
    // in any of them aborts the whole boot atomically — half-applied
    // schemas are the worst failure mode for the auth tables. See
    // docs/design/migrations/README.md.
    let sources = [
        migration_source(),
        postgres_migration_source(),
        UNDO_SNAPSHOTS_MIGRATION_SOURCE,
        FLOWS_DEFINITIONS_MIGRATION_SOURCE,
        DASHBOARDS_DEFINITIONS_MIGRATION_SOURCE,
        SCHEDULED_FLOWS_MIGRATION_SOURCE,
    ];
    let sources_applied = sources.len();
    let mut plan = migrate(&pool);
    for source in sources {
        plan = plan.with_source(source);
    }
    plan.run()
        .await
        .map_err(|e| anyhow::anyhow!("apply migrations: {e}"))?;

    let report = MigrationReport {
        sources_applied,
        skipped: false,
    };
    info!(
        target: "rubix.boot",
        sources = report.sources_applied,
        "rubix migrations applied",
    );
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn apply_migrations_skips_when_dsn_unset() {
        let report = apply_migrations(None).await.expect("skip path succeeds");
        assert!(report.skipped);
        assert_eq!(report.sources_applied, 0);
    }

    #[tokio::test]
    #[ignore = "requires a reachable Postgres; pass the DSN via RUBIX_DSN and run from the integration job"]
    async fn apply_migrations_runs_against_live_postgres() {
        let dsn = std::env::var("RUBIX_DSN").expect("test requires RUBIX_DSN");
        let report = apply_migrations(Some(&dsn))
            .await
            .expect("live migrations succeed");
        assert!(!report.skipped);
        // Both the changelog source and the auth-users source must
        // be reported — see docs/design/migrations/README.md. A
        // half-applied plan would fail the runner before getting
        // here, so the count assertion is the right shape.
        assert!(report.sources_applied >= 2);
    }
}
