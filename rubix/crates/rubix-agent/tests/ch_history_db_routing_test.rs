//! B9 — `system_disk_history` lands in the `rubix` ClickHouse
//! database, not `default`.
//!
//! The smoke test on PR #30 surfaced an inconsistency: the
//! `0002_history` migration was applied through a connection bound
//! to `default`, so the rubix-named tenant database created by the
//! compose bootstrap was empty. The fix (`boot::clickhouse::
//! rubix_ch_config`) pins every `ChClient` the agent builds to
//! `RUBIX_CH_DATABASE` = `"rubix"`; `apply_ch_migrations` issues
//! `CREATE DATABASE IF NOT EXISTS rubix` before binding the
//! migration client so the named-tenant database exists even when
//! the operator did not bootstrap with `CLICKHOUSE_DB=rubix`.
//!
//! This test exercises the contract end-to-end against an
//! ephemeral ClickHouse container:
//!
//! 1. Run `apply_ch_migrations` against a vanilla container (no
//!    `CLICKHOUSE_DB` set — the default-database is `default`).
//! 2. Connect with the rubix-bound config and insert one history
//!    row via the same unqualified DDL the disk tool uses at
//!    runtime.
//! 3. Assert the row is visible via the fully qualified
//!    `rubix.system_disk_history` path AND that `default.
//!    system_disk_history` does not exist — i.e. the row landed
//!    in the named-tenant database, not the default one.
//!
//! `#[ignore]` keeps Docker off the unit-test path; run with
//! `cargo test -p rubix-agent --test ch_history_db_routing_test
//!  -- --ignored`. See
//! [docs/design/warehouse/README.md](../../../docs/design/warehouse/README.md)
//! for the routing contract.

use rubix_agent::boot::{apply_ch_migrations, rubix_ch_config, RUBIX_CH_DATABASE};
use starter_store_clickhouse::{testing::with_clickhouse, ChClient};

#[tokio::test]
#[ignore = "requires Docker (ClickHouse testcontainer)"]
async fn history_row_lands_in_rubix_database() {
    let (default_client, _guard) = with_clickhouse().await;
    let url = default_client.config().url.clone();

    // Apply rubix-owned CH migrations. The bootstrap connection
    // creates the `rubix` database; the runner then binds to it.
    // No Postgres source is configured here, so the shared
    // `0005_entities_dict` migration is the only one that would
    // need one — and on this code path it is rendered without
    // placeholders thanks to `parse_pg_dsn` returning `Some`
    // below.
    let pg_dsn = "postgres://rubix:rubix-dev@127.0.0.1:5433/rubix";
    let report = apply_ch_migrations(Some(&url), Some(pg_dsn), None)
        .await
        .expect("apply_ch_migrations succeeds against a fresh container");
    assert!(!report.skipped, "migration step should run, not skip");

    // Insert one row using the same unqualified-table-name shape
    // the disk tool uses in production. With `rubix_ch_config`
    // the connection binds to `rubix`, so `system_disk_history`
    // resolves to `rubix.system_disk_history`.
    let rubix_client = ChClient::connect(rubix_ch_config(url.clone()));
    rubix_client
        .inner()
        .query(
            "INSERT INTO system_disk_history \
             (tenant_id, host, percent_used, free_bytes, epoch_ms) \
             VALUES (toUUID('00000000-0000-0000-0000-000000000000'), \
                     'test-host', 50, 1024, 1700000000000)",
        )
        .execute()
        .await
        .expect("insert one history row");

    // The row must be visible under the fully qualified
    // `rubix.system_disk_history` path. We query the count
    // through a default-bound client so the `rubix.` qualifier
    // does the routing work — proving the row lives in the named
    // database rather than `default`.
    let count: u64 = default_client
        .inner()
        .query("SELECT count() FROM rubix.system_disk_history")
        .fetch_one()
        .await
        .expect("count rubix.system_disk_history");
    assert_eq!(count, 1, "exactly one row should be in rubix DB");

    // And the parallel `default.system_disk_history` table must
    // not exist — a defence against the regression we are fixing.
    let exists_in_default: u8 = default_client
        .inner()
        .query(
            "SELECT count() FROM system.tables \
             WHERE database = 'default' AND name = 'system_disk_history'",
        )
        .fetch_one()
        .await
        .expect("query system.tables");
    assert_eq!(
        exists_in_default, 0,
        "no system_disk_history must exist in `default` DB"
    );

    // Sanity: the rubix-bound client also sees the row through
    // the unqualified name, matching the production query path.
    let count_unqualified: u64 = rubix_client
        .inner()
        .query("SELECT count() FROM system_disk_history")
        .fetch_one()
        .await
        .expect("count via unqualified name");
    assert_eq!(count_unqualified, 1);

    // Belt-and-braces: confirm the database name baked into the
    // helper matches what we asserted against above. If someone
    // ever renames the constant, this test will fail loudly
    // rather than silently passing against the wrong DB.
    assert_eq!(RUBIX_CH_DATABASE, "rubix");
}
