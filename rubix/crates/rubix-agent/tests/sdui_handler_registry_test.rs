//! Sibling integration coverage for `RubixHandlerRegistry` —
//! verifies that the factory wires every tool under its canonical
//! id and that `/action` dispatch surfaces success as
//! `ToastAndRefresh` and failure as `Diagnostics`.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_agent::sdui::handler_registry::{test_handler_context, RubixHandlerRegistry};
use serde_json::{json, Value as JsonValue};
use starter_spi::error::{Error as ToolError, Result as ToolResult};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_ui_ir::{ActionResponse, ToastIntent};

struct OkTool;

#[async_trait]
impl Tool for OkTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.test.ok".into(),
            description: "ok".into(),
            input_schema: json!({"type": "object"}),
        }
    }
    async fn invoke(&self, _: JsonValue) -> ToolResult<JsonValue> {
        Ok(json!({"ok": true}))
    }
}

struct ErrTool;

#[async_trait]
impl Tool for ErrTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.test.err".into(),
            description: "err".into(),
            input_schema: json!({"type": "object"}),
        }
    }
    async fn invoke(&self, _: JsonValue) -> ToolResult<JsonValue> {
        Err(ToolError::Invalid {
            message: "bad input".into(),
        })
    }
}

#[tokio::test]
async fn build_registers_each_tool_under_its_name() {
    let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(OkTool), Arc::new(ErrTool)];
    let registry = RubixHandlerRegistry::build(&tools);
    assert_eq!(registry.len(), 2);
    assert!(registry.has("rubix.test.ok"));
    assert!(registry.has("rubix.test.err"));
}

#[tokio::test]
async fn dispatch_ok_returns_toast_and_refresh() {
    let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(OkTool)];
    let registry = RubixHandlerRegistry::build(&tools);
    let ctx = test_handler_context("rubix.test.ok", json!({}));
    let resp = registry.dispatch(ctx).await.unwrap().unwrap();
    assert!(matches!(
        resp,
        ActionResponse::ToastAndRefresh {
            intent: ToastIntent::Ok,
            ..
        }
    ));
}

#[tokio::test]
async fn dispatch_failure_surfaces_diagnostics_tagged_with_tool_id() {
    let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(ErrTool)];
    let registry = RubixHandlerRegistry::build(&tools);
    let ctx = test_handler_context("rubix.test.err", json!({}));
    let resp = registry.dispatch(ctx).await.unwrap().unwrap();
    match resp {
        ActionResponse::Diagnostics { items } => {
            assert_eq!(items.len(), 1);
            assert!(items[0].code.starts_with("rubix.test.err"));
        }
        other => panic!("expected Diagnostics, got {other:?}"),
    }
}

#[tokio::test]
async fn unknown_handler_returns_not_found() {
    let registry = RubixHandlerRegistry::build(&[]);
    let ctx = test_handler_context("nope", json!({}));
    let err = registry.dispatch(ctx).await.unwrap_err();
    assert_eq!(err.handler, "nope");
}
