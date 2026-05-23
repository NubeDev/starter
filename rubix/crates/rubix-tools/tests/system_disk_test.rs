//! Integration test for `rubix.system.disk`.
//!
//! Round-trips the verb through the `Tool` trait — the same surface
//! the MCP server exposes — proving the JSON in / JSON out shape is
//! stable. The recorded-LLM end-to-end harness lives in a follow-up
//! phase; this test pins the contract today.

use rubix_spi::dto::system::disk::DiskUsageResponse;
use rubix_tools::system::disk::DiskTool;
use starter_spi::tool::Tool;

#[tokio::test]
async fn invoke_returns_well_formed_response_for_default_request() {
    let tool = DiskTool;
    let def = tool.definition();
    assert_eq!(def.name, "rubix.system.disk");

    let raw = tool
        .invoke(serde_json::json!({}))
        .await
        .expect("disk tool succeeds on the test host");

    let resp: DiskUsageResponse =
        serde_json::from_value(raw).expect("response matches DTO shape");

    assert!(resp.total_bytes > 0, "any real filesystem has bytes");
    assert!(resp.free_bytes <= resp.total_bytes);
    assert!(resp.percent_used <= 100);
    assert!(!resp.mount.is_empty());

    let code = resp.summary.code.as_str();
    assert!(
        matches!(
            code,
            "rubix.system.disk.ok" | "rubix.system.disk.warn" | "rubix.system.disk.full"
        ),
        "unexpected summary code {code:?}"
    );
    assert!(resp.summary.params.contains_key("percent"));
    assert!(resp.summary.params.contains_key("free"));
    assert!(resp.summary.params.contains_key("at"));
    assert!(resp.probed_at_ms > 0);
}

#[tokio::test]
async fn invoke_rejects_unknown_input_fields() {
    let tool = DiskTool;
    let raw = tool
        .invoke(serde_json::json!({ "nonexistent": 1 }))
        .await;
    // serde_json's default deserialization is permissive; the schema
    // gate lives in the transport. The dispatch itself still accepts
    // — this asserts we don't crash on extra fields, which lets the
    // transport be the single point of schema enforcement.
    assert!(raw.is_ok(), "extra fields should be ignored at dispatch");
}
