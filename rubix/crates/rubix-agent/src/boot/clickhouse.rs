//! Apply rubix-owned ClickHouse migrations at boot.
//!
//! `RUBIX_CH_URL` selects the ClickHouse HTTP endpoint. Unset means
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

use anyhow::Result;
use starter_store_clickhouse::{ChClient, ChConfig, MigrationRunner};
use tracing::{info, warn};

/// The single rubix-owned ClickHouse migration applied at boot.
/// Embedded with `include_str!` so the binary does not need the
/// source tree at runtime.
const RUBIX_0002_HISTORY_UP: &str =
    include_str!("../../migrations/0002_history/up.sql");

/// What happened during the CH migrations step.
#[derive(Debug, Clone)]
pub struct ChMigrationReport {
    /// `true` when `RUBIX_CH_URL` was unset and the step was a no-op.
    pub skipped: bool,
}

/// Apply the rubix-owned `0002_history` ClickHouse migration via
/// the shared `MigrationRunner`. Skips with a warn when
/// `RUBIX_CH_URL` is absent.
pub async fn apply_ch_migrations() -> Result<ChMigrationReport> {
    let Ok(url) = std::env::var("RUBIX_CH_URL") else {
        warn!(
            target: "rubix.boot",
            "RUBIX_CH_URL unset — skipping ClickHouse migrations; agent will boot without warehouse-backed features",
        );
        return Ok(ChMigrationReport { skipped: true });
    };

    let client = ChClient::connect(ChConfig::local(url));
    MigrationRunner::new(&client)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn skips_when_ch_url_unset() {
        let prior = std::env::var("RUBIX_CH_URL").ok();
        std::env::remove_var("RUBIX_CH_URL");
        let report = apply_ch_migrations().await.expect("skip path succeeds");
        if let Some(v) = prior {
            std::env::set_var("RUBIX_CH_URL", v);
        }
        assert!(report.skipped);
    }
}
