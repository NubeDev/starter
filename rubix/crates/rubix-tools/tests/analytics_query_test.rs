//! Integration test for `rubix.analytics.query`.
//!
//! Boots an ephemeral ClickHouse container via
//! `starter_store_clickhouse::testing::with_clickhouse`, seeds the
//! six tables the bundled templates read from with a handful of
//! synthetic rows, then asserts every named template returns at
//! least one row through the tool's `invoke` path. Live LLM calls
//! are not involved — the test exercises `Tool::invoke` directly,
//! the same seam the MCP transport drives.
//!
//! `#[ignore]` keeps Docker off the unit-test path. Run with:
//!
//! ```text
//! cargo test -p rubix-tools --test analytics_query_test -- --ignored
//! ```

use std::sync::Arc;

use rubix_tools::analytics::query::AnalyticsQueryTool;
use serde_json::json;
use starter_spi::tool::Tool;
use starter_store_clickhouse::testing::with_clickhouse;
use starter_store_clickhouse::ChClient;

/// Create the six tables the templates read from and seed each
/// with rows inside the last 7 days. Schemas are the minimum the
/// bundled SQL needs to parse and produce a row — production
/// migrations live elsewhere (rubix-agent + starter-changelog) and
/// will supersede these once the real warehouse ships.
async fn seed(client: &ChClient) {
    let now_ms = chrono::Utc::now().timestamp_millis();

    // system_disk_history — mirrors rubix-agent migration 0002.
    exec(
        client,
        "CREATE TABLE IF NOT EXISTS system_disk_history (\
            tenant_id UUID, host String, percent_used UInt8, \
            free_bytes UInt64, epoch_ms Int64\
         ) ENGINE = MergeTree ORDER BY epoch_ms",
    )
    .await;
    exec(
        client,
        &format!(
            "INSERT INTO system_disk_history VALUES \
             (toUUID('00000000-0000-0000-0000-000000000000'),'h',42,1000,{now_ms}), \
             (toUUID('00000000-0000-0000-0000-000000000000'),'h',55,800,{now_ms})"
        ),
    )
    .await;

    // changelog mirror — minimal projection of starter_spi::changelog::Change
    // the four changelog-reading templates need (verb / severity /
    // actor_id / epoch_ms).
    exec(
        client,
        "CREATE TABLE IF NOT EXISTS changelog (\
            verb String, severity String, actor_id String, epoch_ms Int64\
         ) ENGINE = MergeTree ORDER BY epoch_ms",
    )
    .await;
    exec(
        client,
        &format!(
            "INSERT INTO changelog VALUES \
             ('rubix.alert.send','error','u1',{now_ms}), \
             ('rubix.alert.send','warn','u2',{now_ms}), \
             ('rubix.clickhouse.rule.write','info','u1',{now_ms}), \
             ('rubix.clickhouse.mart.create','info','u1',{now_ms}), \
             ('rubix.undo.last','info','u2',{now_ms})"
        ),
    )
    .await;

    // flow_run_history — single status column is enough for the
    // weekly summary template.
    exec(
        client,
        "CREATE TABLE IF NOT EXISTS flow_run_history (\
            status String, epoch_ms Int64\
         ) ENGINE = MergeTree ORDER BY epoch_ms",
    )
    .await;
    exec(
        client,
        &format!(
            "INSERT INTO flow_run_history VALUES \
             ('ok',{now_ms}),('ok',{now_ms}),('error',{now_ms})"
        ),
    )
    .await;

    // rubix.meter_readings_15m — L3 mart the stage 05 dashboard
    // templates read from. Mirrors rubix-agent migration 0005.
    exec(client, "CREATE DATABASE IF NOT EXISTS rubix").await;
    exec(
        client,
        "CREATE TABLE IF NOT EXISTS rubix.meter_readings_15m (\
            tenant_id String, meter_id String, \
            kind LowCardinality(String), unit LowCardinality(String), \
            bucket_start DateTime, \
            value_avg Nullable(Float64), value_min Nullable(Float64), \
            value_max Nullable(Float64), \
            quality_mix Map(LowCardinality(String), UInt32)\
         ) ENGINE = ReplacingMergeTree \
         ORDER BY (tenant_id, meter_id, bucket_start)",
    )
    .await;
    exec(
        client,
        "INSERT INTO rubix.meter_readings_15m \
           (tenant_id, meter_id, kind, unit, bucket_start, \
            value_avg, value_min, value_max, quality_mix) VALUES \
           ('system','elec.main','electricity','kWh',now() - INTERVAL 1 HOUR, 12.5, 10.0, 15.0, map()), \
           ('system','elec.hvac','electricity','kWh',now() - INTERVAL 1 HOUR,  8.0,  7.0,  9.0, map()), \
           ('system','water.main','water','L',      now() - INTERVAL 1 HOUR, 42.0, 40.0, 45.0, map())",
    )
    .await;
}

async fn exec(client: &ChClient, sql: &str) {
    client
        .inner()
        .query(sql)
        .execute()
        .await
        .unwrap_or_else(|e| panic!("setup SQL failed: {e}\nSQL: {sql}"));
}

#[tokio::test]
#[ignore = "requires docker"]
async fn every_template_runs_against_seeded_clickhouse() {
    let (client, _g) = with_clickhouse().await;
    seed(&client).await;

    let tool = AnalyticsQueryTool::new(Arc::new(client));

    for name in AnalyticsQueryTool::known_templates() {
        let params = match name {
            "meter_kwh_last_24h" | "meter_litres_last_24h" => {
                json!({ "tenant_id": "system" })
            }
            _ => json!({}),
        };
        let out = tool
            .invoke(json!({ "name": name, "params": params }))
            .await
            .unwrap_or_else(|e| panic!("template {name} failed: {e:?}"));

        let summary_code = out
            .get("summary")
            .and_then(|s| s.get("code"))
            .and_then(|c| c.as_str())
            .unwrap_or("");
        assert_eq!(
            summary_code, "rubix.analytics.query.ran",
            "template {name} must report ran; got summary={:?}",
            out.get("summary"),
        );

        let row_count = out.get("row_count").and_then(|n| n.as_u64()).unwrap_or(0);
        assert!(
            row_count >= 1,
            "template {name} must return at least one row against synthetic data; got {row_count}"
        );
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn unknown_template_yields_messagekey_invalid() {
    let (client, _g) = with_clickhouse().await;
    let tool = AnalyticsQueryTool::new(Arc::new(client));

    let err = tool
        .invoke(json!({ "name": "does_not_exist", "params": {} }))
        .await
        .unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("rubix.analytics.query.unknown_template"),
        "msg: {msg}"
    );
}
