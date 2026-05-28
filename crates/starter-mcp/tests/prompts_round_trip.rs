//! Round-trip the `prompts/*` surface through the same in-memory
//! transport `in_memory_round_trip.rs` uses for tools. The MCP
//! ndjson and HTTP transports share `dispatch` byte-for-byte, so
//! covering one transport is enough to assert wire shape; the
//! ndjson stdio path is additionally exercised end-to-end by the
//! `favai serve` Python harness in the favai workspace.

#![cfg(feature = "testing")]

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use starter_mcp::registry::{
    Prompt, PromptDefinition, PromptMessage, PromptResponse, PromptRole,
};
use starter_mcp::testing::pair;
use starter_mcp::ToolRegistry;
use starter_spi::Result as SpiResult;

struct SkillStub;

#[async_trait]
impl Prompt for SkillStub {
    fn definition(&self) -> PromptDefinition {
        PromptDefinition {
            name: "com.demo.skills.hello".into(),
            description: "Demo skill body.".into(),
            arguments: Vec::new(),
        }
    }
    async fn render(&self, _args: Value) -> SpiResult<PromptResponse> {
        Ok(PromptResponse {
            description: Some("Demo skill body.".into()),
            messages: vec![PromptMessage {
                role: PromptRole::User,
                text: "# Hello\n\nThis is the skill body.".into(),
            }],
        })
    }
}

fn registry() -> Arc<ToolRegistry> {
    Arc::new(ToolRegistry::new().register_prompt(SkillStub))
}

#[tokio::test]
async fn initialize_advertises_prompts_capability() {
    let (mut client, _server) = pair(registry());
    let resp = client
        .request(1, "initialize", Value::Null)
        .await
        .expect("initialize round-trip");
    let init = resp.result.expect("initialize returns a result");
    assert!(init["capabilities"]["prompts"].is_object());
}

#[tokio::test]
async fn prompts_list_and_get_round_trip() {
    let (mut client, _server) = pair(registry());

    let _ = client
        .request(1, "initialize", Value::Null)
        .await
        .expect("initialize round-trip");

    let list_resp = client
        .request(2, "prompts/list", Value::Null)
        .await
        .expect("prompts/list round-trip");
    let prompts = list_resp.result.expect("prompts/list result")["prompts"].clone();
    let names: Vec<String> = prompts
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_string())
        .collect();
    assert!(
        names.contains(&"com.demo.skills.hello".to_string()),
        "expected demo prompt in {:?}",
        names
    );

    let get_resp = client
        .request(
            3,
            "prompts/get",
            json!({ "name": "com.demo.skills.hello" }),
        )
        .await
        .expect("prompts/get round-trip");
    let val = get_resp.result.expect("prompts/get result");
    let msg = &val["messages"][0];
    assert_eq!(msg["role"], "user");
    assert_eq!(msg["content"]["type"], "text");
    let text = msg["content"]["text"].as_str().unwrap();
    assert!(text.contains("Hello"), "unexpected body: {text}");
}
