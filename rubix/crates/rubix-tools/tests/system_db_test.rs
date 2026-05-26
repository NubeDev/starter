//! Integration test for `rubix.system.db`.
//!
//! Round-trips the verb through the `Tool` trait — the same surface
//! the MCP server exposes — proving the JSON in / JSON out shape is
//! stable.

use rubix_spi::dto::system::db::DbHealthResponse;
use rubix_tools::system::db::DbTool;
use starter_spi::tool::Tool;

#[tokio::test]
async fn invoke_returns_well_formed_response_for_default_request() {
    let tool = DbTool;
    let def = tool.definition();
    assert_eq!(def.name, "rubix.system.db");

    let raw = tool
        .invoke(serde_json::json!({}))
        .await
        .expect("db tool succeeds on the test host");

    let resp: DbHealthResponse = serde_json::from_value(raw).expect("response matches DTO shape");

    assert!(resp.reachable);
    assert_eq!(resp.dsn, "sqlite::memory:");
    assert_eq!(resp.summary.code.as_str(), "rubix.system.db.ok");
    assert!(resp.summary.params.contains_key("at"));
    assert!(resp.probed_at_ms > 0);
}

#[tokio::test]
async fn invoke_honours_caller_supplied_dsn() {
    let tool = DbTool;
    let raw = tool
        .invoke(serde_json::json!({ "dsn": "postgres://x/y" }))
        .await
        .expect("db tool succeeds with overridden dsn");
    let resp: DbHealthResponse = serde_json::from_value(raw).expect("dto shape");
    assert_eq!(resp.dsn, "postgres://x/y");
}
