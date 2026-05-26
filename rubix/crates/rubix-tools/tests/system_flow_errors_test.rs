//! Integration test for `rubix.system.flow_errors`.
//!
//! Exercises the `Tool` trait surface with an empty in-process
//! registry — same JSON shape MCP clients see.

use rubix_spi::dto::system::flow_errors::FlowErrorsResponse;
use rubix_tools::system::flow_errors::FlowErrorsTool;
use starter_spi::tool::Tool;

#[tokio::test]
async fn invoke_returns_ok_summary_on_empty_registry() {
    let tool = FlowErrorsTool::default();
    let def = tool.definition();
    assert_eq!(def.name, "rubix.system.flow_errors");

    let raw = tool
        .invoke(serde_json::json!({ "window_secs": 60 }))
        .await
        .expect("flow_errors tool succeeds");
    let resp: FlowErrorsResponse = serde_json::from_value(raw).expect("response matches DTO shape");

    assert_eq!(resp.error_count, 0);
    assert_eq!(resp.window_secs, 60);
    assert!(resp.samples.is_empty());
    assert_eq!(resp.summary.code.as_str(), "rubix.system.flow_errors.ok");
    assert!(resp.summary.params.contains_key("at"));
    assert!(resp.summary.params.contains_key("count"));
    assert!(resp.summary.params.contains_key("window"));
    assert!(resp.probed_at_ms > 0);
}
