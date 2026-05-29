//! Per-method dispatch. Parses a frame, routes to the right handler,
//! returns a `Response` (or `None` for a notification).
//!
//! Implemented methods:
//!
//! - `initialize` — capability handshake. Returns a static
//!   `serverInfo` block plus the `tools` capability marker, and
//!   `prompts` when at least one prompt is registered.
//! - `tools/list` — enumerate registered tools by definition.
//! - `tools/call` — invoke a tool by name with JSON arguments.
//! - `prompts/list` — enumerate registered prompts.
//! - `prompts/get` — render a prompt by name with JSON arguments.
//! - `ping` — echoes an empty object; clients use this as a
//!   liveness probe.
//!
//! Unknown methods fall through to JSON-RPC `-32601 method not found`.
//!
//! Authentication seam: stdio is single-process, so there is no
//! per-request credential to check at this layer. The HTTP transport
//! ([`super::http`], `feature = "http"`) wraps `dispatch` with an
//! optional bearer-token check via the spi `Authenticator` trait.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::protocol::{Request, Response, RpcError};
use crate::registry::ToolRegistry;

/// Dispatch one frame. Returns `None` for valid notifications;
/// otherwise a response (success or error).
pub async fn dispatch(registry: &Arc<ToolRegistry>, raw: &str) -> Option<Response> {
    let request: Request = match serde_json::from_str(raw) {
        Ok(req) => req,
        Err(e) => {
            return Some(Response::err(
                Value::Null,
                RpcError::invalid_params(e.to_string()),
            ));
        }
    };

    if request.jsonrpc != "2.0" {
        return Some(Response::err(
            request.id.clone().unwrap_or(Value::Null),
            RpcError::invalid_params("jsonrpc must be \"2.0\""),
        ));
    }

    let id = match request.id.clone() {
        Some(id) => id,
        None => return None, // notification — no response
    };

    let result = match request.method.as_str() {
        "initialize" => Ok(initialize(registry)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools_list(registry)),
        "tools/call" => tools_call(registry, &request.params).await,
        "prompts/list" => Ok(prompts_list(registry)),
        "prompts/get" => prompts_get(registry, &request.params).await,
        other => {
            return Some(Response::err(id, RpcError::method_not_found(other)));
        }
    };

    Some(match result {
        Ok(value) => Response::ok(id, value),
        Err(err) => Response::err(id, err),
    })
}

fn initialize(registry: &ToolRegistry) -> Value {
    let mut capabilities = json!({
        "tools": { "listChanged": false }
    });
    // Only advertise `prompts` when the consumer has actually
    // registered some — otherwise hosts will issue a `prompts/list`
    // and get an empty array, which Claude Code logs as a noise
    // event. Existing tools-only consumers see no behavior change.
    if !registry.prompts().is_empty() {
        capabilities["prompts"] = json!({ "listChanged": false });
    }
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": capabilities,
        "serverInfo": {
            "name": "starter-mcp",
            "version": env!("CARGO_PKG_VERSION"),
        }
    })
}

fn tools_list(registry: &ToolRegistry) -> Value {
    let tools: Vec<Value> = registry
        .list()
        .into_iter()
        .map(|def| {
            json!({
                "name": def.name,
                "description": def.description,
                "inputSchema": def.input_schema,
            })
        })
        .collect();
    json!({ "tools": tools })
}

async fn tools_call(registry: &ToolRegistry, params: &Value) -> Result<Value, RpcError> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("missing `name`"))?;
    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

    let tool = registry
        .get(name)
        .ok_or_else(|| RpcError::invalid_params(format!("unknown tool: {name}")))?;

    match tool.invoke(arguments).await {
        Ok(output) => Ok(json!({
            "content": [{
                "type": "text",
                "text": output.to_string(),
            }],
            "structuredContent": output,
        })),
        Err(e) => {
            tracing::warn!(
                tool = name,
                error = %e,
                "tool dispatch failed"
            );
            Err(RpcError::from_spi(&e))
        }
    }
}

fn prompts_list(registry: &ToolRegistry) -> Value {
    let prompts: Vec<Value> = registry
        .prompts()
        .list()
        .into_iter()
        .map(|def| {
            let args: Vec<Value> = def
                .arguments
                .into_iter()
                .map(|arg| {
                    let mut obj = json!({ "name": arg.name, "required": arg.required });
                    if let Some(desc) = arg.description {
                        obj["description"] = Value::String(desc);
                    }
                    obj
                })
                .collect();
            json!({
                "name": def.name,
                "description": def.description,
                "arguments": args,
            })
        })
        .collect();
    json!({ "prompts": prompts })
}

async fn prompts_get(registry: &ToolRegistry, params: &Value) -> Result<Value, RpcError> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("missing `name`"))?;
    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

    let prompt = registry
        .prompts()
        .get(name)
        .ok_or_else(|| RpcError::invalid_params(format!("unknown prompt: {name}")))?;

    match prompt.render(arguments).await {
        Ok(rendered) => {
            let messages: Vec<Value> = rendered
                .messages
                .into_iter()
                .map(|msg| {
                    json!({
                        "role": msg.role.as_str(),
                        "content": { "type": "text", "text": msg.text },
                    })
                })
                .collect();
            let mut out = json!({ "messages": messages });
            if let Some(desc) = rendered.description {
                out["description"] = Value::String(desc);
            }
            Ok(out)
        }
        Err(e) => {
            tracing::warn!(
                prompt = name,
                error = %e,
                "prompt render failed"
            );
            Err(RpcError::from_spi(&e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use starter_spi::tool::{Tool, ToolDefinition};

    use crate::registry::{Prompt, PromptDefinition, PromptMessage, PromptResponse, PromptRole};

    struct EchoTool;
    #[async_trait]
    impl Tool for EchoTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "echo".into(),
                description: "Return the input unchanged.".into(),
                input_schema: json!({ "type": "object" }),
            }
        }
        async fn invoke(&self, input: Value) -> starter_spi::Result<Value> {
            Ok(input)
        }
    }

    struct GreetPrompt;
    #[async_trait]
    impl Prompt for GreetPrompt {
        fn definition(&self) -> PromptDefinition {
            PromptDefinition {
                name: "greet".into(),
                description: "Say hello.".into(),
                arguments: Vec::new(),
            }
        }
        async fn render(&self, _args: Value) -> starter_spi::Result<PromptResponse> {
            Ok(PromptResponse {
                description: Some("Greeting body".into()),
                messages: vec![PromptMessage {
                    role: PromptRole::User,
                    text: "hello world".into(),
                }],
            })
        }
    }

    fn registry_with_echo() -> Arc<ToolRegistry> {
        Arc::new(ToolRegistry::new().register(EchoTool))
    }

    fn registry_with_echo_and_greet() -> Arc<ToolRegistry> {
        Arc::new(
            ToolRegistry::new()
                .register(EchoTool)
                .register_prompt(GreetPrompt),
        )
    }

    #[tokio::test]
    async fn initialize_returns_server_info() {
        let r = registry_with_echo();
        let resp = dispatch(&r, r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#)
            .await
            .unwrap();
        let val = resp.result.unwrap();
        assert_eq!(val["serverInfo"]["name"], "starter-mcp");
        assert!(val["capabilities"]["tools"].is_object());
        // No prompts registered → capability not advertised.
        assert!(val["capabilities"].get("prompts").is_none());
    }

    #[tokio::test]
    async fn initialize_advertises_prompts_when_registered() {
        let r = registry_with_echo_and_greet();
        let resp = dispatch(&r, r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#)
            .await
            .unwrap();
        let val = resp.result.unwrap();
        assert!(val["capabilities"]["prompts"].is_object());
    }

    #[tokio::test]
    async fn tools_list_returns_registered() {
        let r = registry_with_echo();
        let resp = dispatch(&r, r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#)
            .await
            .unwrap();
        let tools = &resp.result.unwrap()["tools"];
        assert_eq!(tools[0]["name"], "echo");
    }

    #[tokio::test]
    async fn tools_call_invokes_named_tool() {
        let r = registry_with_echo();
        let frame = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"echo","arguments":{"a":1}}}"#;
        let resp = dispatch(&r, frame).await.unwrap();
        let val = resp.result.unwrap();
        assert_eq!(val["structuredContent"]["a"], 1);
    }

    #[tokio::test]
    async fn tools_call_unknown_tool_invalid_params() {
        let r = registry_with_echo();
        let frame = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"nope"}}"#;
        let resp = dispatch(&r, frame).await.unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32602);
    }

    #[tokio::test]
    async fn unknown_method_method_not_found() {
        let r = registry_with_echo();
        let resp = dispatch(&r, r#"{"jsonrpc":"2.0","id":5,"method":"nope"}"#)
            .await
            .unwrap();
        assert_eq!(resp.error.unwrap().code, -32601);
    }

    #[tokio::test]
    async fn notification_returns_none() {
        let r = registry_with_echo();
        let resp = dispatch(&r, r#"{"jsonrpc":"2.0","method":"tools/list"}"#).await;
        assert!(resp.is_none());
    }

    #[tokio::test]
    async fn malformed_json_invalid_params_with_null_id() {
        let r = registry_with_echo();
        let resp = dispatch(&r, "not json").await.unwrap();
        assert_eq!(resp.error.unwrap().code, -32602);
        assert_eq!(resp.id, Value::Null);
    }

    #[tokio::test]
    async fn prompts_list_returns_registered() {
        let r = registry_with_echo_and_greet();
        let resp = dispatch(&r, r#"{"jsonrpc":"2.0","id":6,"method":"prompts/list"}"#)
            .await
            .unwrap();
        let prompts = &resp.result.unwrap()["prompts"];
        assert_eq!(prompts[0]["name"], "greet");
        assert_eq!(prompts[0]["description"], "Say hello.");
        assert!(prompts[0]["arguments"].is_array());
    }

    #[tokio::test]
    async fn prompts_list_empty_when_none_registered() {
        let r = registry_with_echo();
        let resp = dispatch(&r, r#"{"jsonrpc":"2.0","id":7,"method":"prompts/list"}"#)
            .await
            .unwrap();
        let prompts = &resp.result.unwrap()["prompts"];
        assert!(prompts.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn prompts_get_renders_named_prompt() {
        let r = registry_with_echo_and_greet();
        let frame = r#"{"jsonrpc":"2.0","id":8,"method":"prompts/get","params":{"name":"greet"}}"#;
        let resp = dispatch(&r, frame).await.unwrap();
        let val = resp.result.unwrap();
        assert_eq!(val["description"], "Greeting body");
        let msg = &val["messages"][0];
        assert_eq!(msg["role"], "user");
        assert_eq!(msg["content"]["type"], "text");
        assert_eq!(msg["content"]["text"], "hello world");
    }

    #[tokio::test]
    async fn prompts_get_unknown_invalid_params() {
        let r = registry_with_echo_and_greet();
        let frame = r#"{"jsonrpc":"2.0","id":9,"method":"prompts/get","params":{"name":"nope"}}"#;
        let resp = dispatch(&r, frame).await.unwrap();
        assert_eq!(resp.error.unwrap().code, -32602);
    }
}
