//! Integration test for `rubix.alert.send`.
//!
//! Exercises the `Tool` trait surface — emits to the local tracing
//! sink and asserts the response shape MCP clients see.

use rubix_spi::dto::system::alert_send::{AlertSendResponse, AlertSeverity};
use rubix_tools::system::alert_send::AlertSendTool;
use starter_spi::tool::Tool;

#[tokio::test]
async fn invoke_emits_warn_alert_and_returns_summary() {
    let tool = AlertSendTool;
    let def = tool.definition();
    assert_eq!(def.name, "rubix.alert.send");

    let raw = tool
        .invoke(serde_json::json!({
            "severity": "warn",
            "message": "Disk 89% full on /"
        }))
        .await
        .expect("alert_send tool succeeds");
    let resp: AlertSendResponse =
        serde_json::from_value(raw).expect("response matches DTO shape");

    assert_eq!(resp.severity, AlertSeverity::Warn);
    assert_eq!(resp.delivered_chars, 18);
    assert_eq!(resp.summary.code.as_str(), "rubix.alert.send.ok");
    assert!(resp.summary.params.contains_key("severity"));
    assert!(resp.summary.params.contains_key("at"));
    assert!(resp.probed_at_ms > 0);
}

#[tokio::test]
async fn invoke_rejects_empty_message() {
    let tool = AlertSendTool;
    let err = tool
        .invoke(serde_json::json!({ "severity": "info", "message": "" }))
        .await;
    assert!(err.is_err(), "empty message must error");
}
