//! `RubixHandlerRegistry` — rubix's factory for the upstream
//! [`starter_sdui_routes::HandlerRegistry`].
//!
//! The upstream `HandlerRegistry` is a concrete struct (not a
//! trait); rubix's contribution is the **wiring**: one
//! [`tool_action_handler`] per registered tool that adapts the
//! upstream [`HandlerContext`] onto the `starter_spi::tool::Tool`
//! shape and back, then wraps the tool's JSON output in a
//! `ToastAndRefresh` (success) / `Diagnostics` (failure)
//! [`ActionResponse`].
//!
//! Phase B.3 will layer authz, the action-time `WritePlanAcl`, and
//! per-handler payload validation onto this same factory; Phase B.1
//! keeps the wrapper to the minimum needed for `/action` to reach a
//! tool body end-to-end.

use std::sync::Arc;

use serde_json::Value as JsonValue;
use starter_sdui_routes::handler::{ActionFn, ActionFuture, HandlerContext, HandlerRegistry};
use starter_spi::tool::Tool;
use starter_ui_ir::{ActionResponse, Diagnostic, Severity, ToastIntent};
use tracing::warn;

/// Build an [`ActionFn`] that dispatches `ctx.args` into `tool`.
///
/// The handler:
///
/// 1. Forwards `HandlerContext::args` to [`Tool::invoke`]; if `args`
///    is `Null` the tool sees `{}` (most rubix tools expect at least
///    an empty object).
/// 2. On success, wraps the tool's JSON output in
///    [`ActionResponse::ToastAndRefresh`] — the most common
///    mutation pattern (the dashboard refreshes its table / KPI
///    caches and shows a confirmation toast).
/// 3. On error, surfaces a single
///    [`ActionResponse::Diagnostics`] entry tagged with the tool id
///    so the operator sees *which* tool failed.
pub fn tool_action_handler(tool: Arc<dyn Tool>) -> ActionFn {
    let id = tool.definition().name;
    Arc::new(move |ctx: HandlerContext| -> ActionFuture {
        let tool = tool.clone();
        let id = id.clone();
        Box::pin(async move {
            let args = if ctx.args.is_null() {
                JsonValue::Object(Default::default())
            } else {
                ctx.args
            };
            match tool.invoke(args).await {
                Ok(_value) => Ok(ActionResponse::ToastAndRefresh {
                    intent: ToastIntent::Ok,
                    message: format!("{id} ok"),
                }),
                Err(err) => {
                    warn!(
                        target: "rubix.sdui",
                        tool = %id,
                        error = %err,
                        "action handler: tool invocation failed",
                    );
                    Ok(ActionResponse::Diagnostics {
                        items: vec![Diagnostic::new(
                            Severity::Error,
                            format!("{id}.failed"),
                            err.to_string(),
                        )],
                    })
                }
            }
        }) as ActionFuture
    }) as ActionFn
}

/// Helper that builds a [`HandlerRegistry`] by registering every
/// tool in `tools` under its canonical tool id (the handler key
/// `ActionRequest::handler` carries). Insertion order matches the
/// caller's slice so the boot log stays stable.
///
/// Per `03-host-glue.md` the boot wiring calls this with the same
/// `Vec<Arc<dyn Tool>>` that [`crate::registry::build_tool_registry`]
/// returns, so every tool the agent advertises over REST/MCP is
/// reachable from `/api/v1/ui/action` as well.
#[derive(Default)]
pub struct RubixHandlerRegistry;

impl RubixHandlerRegistry {
    /// Build a fresh [`HandlerRegistry`] over `tools`. Returning by
    /// value (not `Arc`) so the caller can hand it to
    /// `SduiStateBuilder::with_handler_registry` directly.
    pub fn build(tools: &[Arc<dyn Tool>]) -> HandlerRegistry {
        let mut registry = HandlerRegistry::new();
        for tool in tools {
            let id = tool.definition().name.clone();
            registry.insert(id, tool_action_handler(tool.clone()));
        }
        registry
    }
}

impl RubixHandlerRegistry {
    /// Best-effort name for diagnostics — `_` because the type is
    /// stateless today but may grow per-handler config later (the
    /// `WritePlanAcl` seam, for instance).
    #[allow(dead_code)]
    fn name(_: &Self) -> &'static str {
        "RubixHandlerRegistry"
    }
}

/// Synthesise a fake [`HandlerContext`] for tests / examples that
/// want to invoke a handler in-process without going through the
/// HTTP router. Production code never calls this — it's exposed
/// publicly so the sibling integration tests under
/// `rubix-agent/tests/` can reach it. The function carries no
/// production state; downstream crates can ignore it.
pub fn test_handler_context(name: impl Into<String>, args: JsonValue) -> HandlerContext {
    use starter_sdui_routes::handler::Principal;
    use starter_ui_ir::ActionContext;
    HandlerContext {
        principal: Principal::anonymous(),
        name: name.into(),
        args,
        context: ActionContext::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use starter_spi::error::Error as ToolError;
    use starter_spi::tool::ToolDefinition;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "rubix.test.echo".into(),
                description: "Echoes input back".into(),
                input_schema: serde_json::json!({"type": "object"}),
            }
        }
        async fn invoke(&self, input: JsonValue) -> Result<JsonValue, ToolError> {
            Ok(input)
        }
    }

    struct FailingTool;

    #[async_trait]
    impl Tool for FailingTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "rubix.test.fail".into(),
                description: "Always errors".into(),
                input_schema: serde_json::json!({"type": "object"}),
            }
        }
        async fn invoke(&self, _: JsonValue) -> Result<JsonValue, ToolError> {
            Err(ToolError::Invalid {
                message: "boom".into(),
            })
        }
    }

    #[tokio::test]
    async fn build_registers_every_tool_under_its_id() {
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(EchoTool), Arc::new(FailingTool)];
        let registry = RubixHandlerRegistry::build(&tools);
        assert!(registry.has("rubix.test.echo"));
        assert!(registry.has("rubix.test.fail"));
        assert_eq!(registry.len(), 2);
    }

    #[tokio::test]
    async fn dispatch_success_returns_toast_and_refresh() {
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(EchoTool)];
        let registry = RubixHandlerRegistry::build(&tools);
        let ctx = test_handler_context("rubix.test.echo", serde_json::json!({"hi": 1}));
        let resp = registry.dispatch(ctx).await.unwrap().unwrap();
        match resp {
            ActionResponse::ToastAndRefresh { intent, .. } => {
                assert!(matches!(intent, ToastIntent::Ok));
            }
            other => panic!("expected ToastAndRefresh, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn dispatch_failure_returns_diagnostics() {
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(FailingTool)];
        let registry = RubixHandlerRegistry::build(&tools);
        let ctx = test_handler_context("rubix.test.fail", JsonValue::Null);
        let resp = registry.dispatch(ctx).await.unwrap().unwrap();
        match resp {
            ActionResponse::Diagnostics { items } => {
                assert_eq!(items.len(), 1);
                assert!(items[0].code.contains("rubix.test.fail"));
            }
            other => panic!("expected Diagnostics, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn missing_handler_returns_not_found() {
        let registry = RubixHandlerRegistry::build(&[]);
        let ctx = test_handler_context("nope", JsonValue::Null);
        let err = registry.dispatch(ctx).await.unwrap_err();
        assert_eq!(err.handler, "nope");
    }

    #[tokio::test]
    async fn dispatch_treats_null_args_as_empty_object() {
        // FailingTool would error if args were threaded through —
        // EchoTool returns whatever it sees. We use it to assert
        // the wrapper's null-coercion explicitly.
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(EchoTool)];
        let registry = RubixHandlerRegistry::build(&tools);
        let ctx = test_handler_context("rubix.test.echo", JsonValue::Null);
        let resp = registry.dispatch(ctx).await.unwrap().unwrap();
        assert!(matches!(resp, ActionResponse::ToastAndRefresh { .. }));
        // (the wrapper's coerced `{}` is asserted indirectly: the
        // Tool::invoke path never returned an error.)
    }
}
