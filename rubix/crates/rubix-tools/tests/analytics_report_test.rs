//! Integration test for `rubix.analytics.report`.
//!
//! Spins up a testcontainers ClickHouse, seeds the
//! `system_disk_history` table the `disk_history_weekly` template
//! reads from, points the verb at a tempdir-backed `FsBlobStore`,
//! and asserts the rendered HTML report:
//!
//! 1. Returns `rubix.analytics.report.rendered` (data present).
//! 2. Mints a blob whose persisted bytes contain a `<table>` and
//!    the expected per-day rows from the disk-history template.
//! 3. Carries a non-empty presigned URL.
//!
//! Live LLM calls are not involved — `Tool::invoke` is the same
//! seam the MCP transport drives. `#[ignore]` keeps Docker off the
//! unit-test path; run with:
//!
//! ```text
//! cargo test -p rubix-tools --test analytics_report_test -- --ignored
//! ```

use std::sync::Arc;

use bytes::Bytes;
use futures::TryStreamExt;
use rubix_spi::dto::analytics::report::{AnalyticsReportResponse, ReportFormat};
use rubix_tools::analytics::report::AnalyticsReportTool;
use serde_json::json;
use starter_blob_fs::{FsBlobStore, PresignKey};
use starter_spi::blob::{BlobRef, BlobRefInternal, BlobStore, Etag};
use starter_spi::tool::Tool;
use starter_store_warehouse::testing::with_clickhouse;
use starter_store_warehouse::ChClient;

async fn exec(client: &ChClient, sql: &str) {
    client
        .inner()
        .query(sql)
        .execute()
        .await
        .unwrap_or_else(|e| panic!("setup SQL failed: {e}\nSQL: {sql}"));
}

/// Seed `system_disk_history` with two rows inside the last 7 days
/// — enough for `disk_history_weekly` to return one grouped row
/// with a recognisable peak.
async fn seed(client: &ChClient) {
    exec(
        client,
        "CREATE TABLE IF NOT EXISTS system_disk_history (\
            tenant_id UUID, host String, percent_used UInt8, \
            free_bytes UInt64, epoch_ms Int64\
         ) ENGINE = MergeTree ORDER BY epoch_ms",
    )
    .await;
    let now_ms = chrono::Utc::now().timestamp_millis();
    exec(
        client,
        &format!(
            "INSERT INTO system_disk_history VALUES \
             (toUUID('00000000-0000-0000-0000-000000000000'),'h',42,1000,{now_ms}), \
             (toUUID('00000000-0000-0000-0000-000000000000'),'h',73,800,{now_ms})"
        ),
    )
    .await;
}

async fn drain(store: &FsBlobStore, blob_id: &str) -> Vec<u8> {
    // Reconstruct an opaque BlobRef from the locator the tool
    // returned. The FsBlobStore reads `opaque_locator` only, so the
    // placeholder etag/size are fine for the read path.
    let r = BlobRef::mint(
        store.backend_id().clone(),
        blob_id.to_owned(),
        Etag::new(""),
        0,
    );
    let chunks: Vec<Bytes> = store
        .get(&r, None)
        .await
        .unwrap()
        .try_collect()
        .await
        .unwrap();
    chunks.iter().flat_map(|b| b.iter().copied()).collect()
}

#[tokio::test]
#[ignore = "requires docker"]
async fn html_report_contains_table_with_disk_history_rows() {
    let (client, _g) = with_clickhouse().await;
    seed(&client).await;

    let tempdir = tempfile::tempdir().unwrap();
    let store = FsBlobStore::open(tempdir.path(), PresignKey::ephemeral()).unwrap();
    let store_arc: Arc<dyn BlobStore> = Arc::new(store.clone());

    let tool = AnalyticsReportTool::new(Arc::new(client), store_arc);

    let out = tool
        .invoke(json!({
            "template": "weekly-ops",
            "queries":  ["disk_history_weekly"],
            "format":   "html"
        }))
        .await
        .expect("tool invoke");

    let resp: AnalyticsReportResponse = serde_json::from_value(out).unwrap();
    assert_eq!(
        resp.summary.code.as_str(),
        "rubix.analytics.report.rendered"
    );
    assert_eq!(resp.format, ReportFormat::Html);
    assert!(resp.byte_count > 0, "byte_count must be positive");
    assert!(!resp.url.is_empty(), "presigned url must be non-empty");
    assert!(
        resp.blob_id.starts_with("reports/weekly-ops/"),
        "blob_id: {}",
        resp.blob_id
    );

    let bytes = drain(&store, &resp.blob_id).await;
    let html = String::from_utf8(bytes).unwrap();
    assert!(
        html.contains("<!doctype html>"),
        "html must wrap output: {html}"
    );
    assert!(
        html.contains("<h2>disk_history_weekly</h2>"),
        "html must title the query section: {html}"
    );
    assert!(
        html.contains("<table>"),
        "html must contain the rendered table: {html}"
    );
    // disk_history_weekly groups by day, returning columns
    // `day` / `avg_percent` / `peak_percent`. The seeded rows peak
    // at 73 — assert that value appears in the rendered table so we
    // know real data made the round-trip.
    assert!(
        html.contains("<th>day</th>") && html.contains("<th>peak_percent</th>"),
        "html must include the disk_history headers: {html}"
    );
    assert!(
        html.contains(">73<") || html.contains(">73.0<"),
        "html must include the seeded peak value: {html}"
    );
}

#[tokio::test]
#[ignore = "requires docker"]
async fn pdf_format_yields_format_unsupported_messagekey() {
    let (client, _g) = with_clickhouse().await;
    let tempdir = tempfile::tempdir().unwrap();
    let store = FsBlobStore::open(tempdir.path(), PresignKey::ephemeral()).unwrap();
    let tool = AnalyticsReportTool::new(Arc::new(client), Arc::new(store) as Arc<dyn BlobStore>);

    let err = tool
        .invoke(json!({
            "template": "weekly-ops",
            "queries":  [],
            "format":   "pdf"
        }))
        .await
        .unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("rubix.analytics.report.format_unsupported"),
        "msg: {msg}"
    );
}
