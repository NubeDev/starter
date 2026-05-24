//! PR 3 — MCP exposure of `com.rubix.scheduled-system-check`.
//!
//! Drives the boot-time MCP surface (`build_flow_registry` →
//! `FlowAsTool::from_registry` → starter-mcp's
//! [`ToolRegistry`]) through the in-memory transport pair from
//! `starter_mcp::testing`. The same JSON-RPC frames the HTTP and
//! stdio transports carry round-trip through real serialisation —
//! only the carrier changes.
//!
//! The test asserts the U1 locale contract: an `initialize` frame
//! carrying `params._meta.acceptLanguage` binds the BCP-47 tag on
//! a task-local for every subsequent dispatch; the bundled flow's
//! seed adapter reads it via `starter_mcp::current_locale()`; the
//! diag-render node renders the same `rubix.system.disk.warn`
//! diagnostic in two locales with the matching date format and
//! timezone shift.

use std::sync::Arc;

use serde_json::{json, Value};

use rubix_agent::boot::mcp::{build_flow_registry, SCHEDULED_SYSTEM_CHECK_FLOW};
use starter_flow_surfaces::FlowAsTool;
use starter_mcp::registry::ToolRegistry;
use starter_mcp::testing::pair;

/// Build the tool registry the test pair dispatches against, using
/// the same `FlowAsTool::from_registry` contract `boot::mcp` uses
/// in production — zero per-flow registration code.
async fn build_tool_registry() -> Arc<ToolRegistry> {
    let (registry, flow_id, revision, engine) = build_flow_registry()
        .await
        .expect("flow registry builds");

    let tool = FlowAsTool::from_registry(&registry, &flow_id, &revision, engine)
        .await
        .expect("FlowAsTool::from_registry");

    Arc::new(ToolRegistry::new().register(tool))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scheduled_system_check_renders_in_en_us() {
    let tools = build_tool_registry().await;
    let (mut client, _server) = pair(tools);

    // U1: the in-memory transport captures `_meta.acceptLanguage`
    // on `initialize` and holds it as the session locale for every
    // subsequent dispatch.
    let init = client
        .request(
            1,
            "initialize",
            json!({ "_meta": { "acceptLanguage": "en-US" } }),
        )
        .await
        .expect("initialize round-trips");
    assert!(
        init.error.is_none(),
        "initialize must succeed: {:?}",
        init.error
    );

    // tools/list must surface the bundled flow under its
    // `com.rubix.scheduled-system-check` name, with zero per-flow
    // registration code in rubix-agent (the registration was made
    // through `FlowRegistry::register` + `FlowAsTool::from_registry`
    // only — the U3 one-liner).
    let list = client
        .request(2, "tools/list", Value::Null)
        .await
        .expect("tools/list round-trips");
    let tool_names = tool_names(&list.result.expect("tools/list result"));
    assert!(
        tool_names.iter().any(|n| n == SCHEDULED_SYSTEM_CHECK_FLOW),
        "MCP tool catalogue must contain {SCHEDULED_SYSTEM_CHECK_FLOW}; saw {tool_names:?}",
    );

    // tools/call drives the flow. The seed adapter reads
    // `starter_mcp::current_locale()` (bound by the in-memory
    // transport's `with_locale` wrap) and snapshots the matching
    // ResolvedPreferences onto the input slot; the render node
    // builds a Diagnostic and renders it through the rubix bundle.
    let call = client
        .request(
            3,
            "tools/call",
            json!({
                "name": SCHEDULED_SYSTEM_CHECK_FLOW,
                "arguments": {}
            }),
        )
        .await
        .expect("tools/call round-trips");
    assert!(
        call.error.is_none(),
        "tools/call must succeed: {:?}",
        call.error
    );
    let result = call.result.expect("tools/call result");
    let rendered = result["structuredContent"]["rendered"]
        .as_str()
        .unwrap_or_else(|| panic!("expected `rendered` string in {result}"));

    // EN catalogue rendering: starts with "Disk is nearly full".
    assert!(
        rendered.starts_with("Disk is nearly full"),
        "EN rendering must use English catalogue; got {rendered:?}",
    );
    // US date format: MM/DD/YYYY (2024-01-15 in MdySlash).
    assert!(
        rendered.contains("01/15/2024"),
        "EN rendering must use US date format MM/DD/YYYY; got {rendered:?}",
    );
    // Timezone shift: 2024-01-15 12:00 UTC → 07:00 in America/New_York (UTC-5 in January).
    assert!(
        rendered.contains("07:00"),
        "EN rendering must shift into America/New_York (07:00); got {rendered:?}",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scheduled_system_check_renders_in_es_ar() {
    let tools = build_tool_registry().await;
    let (mut client, _server) = pair(tools);

    let init = client
        .request(
            1,
            "initialize",
            json!({ "_meta": { "acceptLanguage": "es-AR" } }),
        )
        .await
        .expect("initialize round-trips");
    assert!(init.error.is_none(), "initialize: {:?}", init.error);

    let list = client
        .request(2, "tools/list", Value::Null)
        .await
        .expect("tools/list round-trips");
    let tool_names = tool_names(&list.result.expect("tools/list result"));
    assert!(
        tool_names.iter().any(|n| n == SCHEDULED_SYSTEM_CHECK_FLOW),
        "MCP tool catalogue must contain {SCHEDULED_SYSTEM_CHECK_FLOW}; saw {tool_names:?}",
    );

    let call = client
        .request(
            3,
            "tools/call",
            json!({
                "name": SCHEDULED_SYSTEM_CHECK_FLOW,
                "arguments": {}
            }),
        )
        .await
        .expect("tools/call round-trips");
    assert!(call.error.is_none(), "tools/call: {:?}", call.error);
    let result = call.result.expect("tools/call result");
    let rendered = result["structuredContent"]["rendered"]
        .as_str()
        .unwrap_or_else(|| panic!("expected `rendered` string in {result}"));

    // ES catalogue: starts with "El disco está casi lleno".
    assert!(
        rendered.starts_with("El disco está casi lleno"),
        "ES rendering must use Spanish catalogue; got {rendered:?}",
    );
    // EU date format: DD/MM/YYYY.
    assert!(
        rendered.contains("15/01/2024"),
        "ES rendering must use EU date format DD/MM/YYYY; got {rendered:?}",
    );
    // Timezone shift: 2024-01-15 12:00 UTC → 09:00 in
    // America/Argentina/Buenos_Aires (UTC-3 year-round).
    assert!(
        rendered.contains("09:00"),
        "ES rendering must shift into America/Argentina/Buenos_Aires (09:00); got {rendered:?}",
    );
}

fn tool_names(list_result: &Value) -> Vec<String> {
    list_result["tools"]
        .as_array()
        .map(|tools| {
            tools
                .iter()
                .filter_map(|t| t["name"].as_str().map(|s| s.to_owned()))
                .collect()
        })
        .unwrap_or_default()
}
