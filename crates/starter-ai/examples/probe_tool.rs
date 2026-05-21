//! P0 verification probe for the live page builder
//! ([`examples/flow-agent/PAGE-BUILDER-LIVE.md`] §0).
//!
//! Drives the **CLI** Claude runner with a single `emit_ui_tree`
//! [`ToolDef`] and prints every [`Event`] yielded. Pass = at least one
//! `EventKind::ToolUse { name: "emit_ui_tree", input: Some(JSON
//! object) }` arrives. Fail = prose, stringified tool call, or CLI
//! error — in that case the live SCOPE has to swap to the REST runner.
//!
//! Run with:
//! ```text
//! cargo run -p starter-ai --features provider-claude --example probe_tool
//! ```

use serde_json::json;
use std::time::Duration;
use tokio::sync::mpsc;

use starter_ai::{Registry, TokenCancel};
use starter_spi::ai::{
    CliCfg, Event, EventKind, PermissionMode, Provider, RunnerInput, SessionId, ToolDef,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = Registry::with_defaults();
    let runner = registry
        .get(&Provider::Claude)
        .ok_or("claude provider not registered — rebuild with --features provider-claude")?;
    if !runner.ready().await {
        eprintln!(
            "WARN: claude runner reports !ready (binary missing or not authed). \
             Continuing anyway; you should see a runner error."
        );
    }

    let tool = ToolDef {
        name: "emit_ui_tree".to_string(),
        description: Some("Emit a one-node page tree.".to_string()),
        input_schema: json!({
            "type": "object",
            "required": ["root"],
            "properties": {
                "root": {
                    "type": "object",
                    "required": ["id", "type"],
                    "properties": {
                        "id":   { "type": "string" },
                        "type": { "type": "string", "enum": ["page"] }
                    }
                }
            }
        }),
    };

    // CLI runners don't take a tools list at the input level (claude-
    // wrapper drives tools via MCP / its own surface), so the most we
    // can do at the CliCfg seam is fold the schema into the system
    // prompt and ask the model to emit a JSON tool-call. The Claude
    // CLI surfaces structured tool_use blocks regardless when the
    // model decides to call a tool — see `runners/claude.rs`.
    let system_prompt = format!(
        "You are a UI builder. Call the tool `{}` exactly once with \
         JSON arguments `{{\"root\":{{\"id\":\"r\",\"type\":\"page\"}}}}`. \
         Do not reply with prose. Tool schema:\n{}",
        tool.name,
        serde_json::to_string_pretty(&tool.input_schema).unwrap_or_default()
    );

    let input = RunnerInput::Cli(CliCfg {
        prompt: "Emit a one-node page tree via the emit_ui_tree tool.".to_string(),
        system_prompt: Some(system_prompt),
        permission_mode: Some(PermissionMode::Bypass),
        ..CliCfg::default()
    });

    let (tx, mut rx) = mpsc::channel::<Event>(32);
    let cancel = TokenCancel::new();
    let session = SessionId::from("probe-tool-1".to_string());

    let probe_task = {
        let cancel = cancel.clone();
        tokio::spawn(async move {
            // Hard 60s safety net — if the CLI hangs we bail.
            tokio::time::sleep(Duration::from_secs(60)).await;
            cancel.cancel();
        })
    };

    let pump = tokio::spawn(async move {
        let mut saw = false;
        while let Some(ev) = rx.recv().await {
            match &ev.kind {
                EventKind::ToolUse { id, name, input } => {
                    println!(
                        "[ToolUse] id={:?} name={} input_kind={:?} input={}",
                        id,
                        name,
                        input.as_ref().map(serde_json::Value::is_object),
                        input
                            .as_ref()
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "<absent>".into()),
                    );
                    if name == "emit_ui_tree"
                        && input.as_ref().is_some_and(serde_json::Value::is_object)
                    {
                        saw = true;
                    }
                }
                EventKind::Text { content } => {
                    println!("[Text] {}", content.trim());
                }
                EventKind::Connected { model } => {
                    println!("[Connected] model={:?}", model);
                }
                EventKind::Done {
                    duration_ms,
                    cost_usd,
                    input_tokens,
                    output_tokens,
                } => {
                    println!(
                        "[Done] {duration_ms}ms cost=${cost_usd:.4} \
                         in={input_tokens} out={output_tokens}"
                    );
                }
                EventKind::Error { message } => {
                    println!("[Error] {message}");
                }
            }
        }
        saw
    });

    let run_res = runner.run(input, session, tx, &cancel).await;
    let saw_tool_use = pump.await.unwrap_or(false);
    probe_task.abort();

    match run_res {
        Ok(r) => println!(
            "[RunResult] text_len={} tool_uses={} error={:?}",
            r.text.len(),
            r.tool_uses.len(),
            r.error
        ),
        Err(e) => println!("[RunnerError] {e}"),
    }

    if saw_tool_use {
        println!("\nPASS: structured ToolUse(emit_ui_tree, object) observed.");
        Ok(())
    } else {
        println!(
            "\nFAIL: no structured ToolUse(emit_ui_tree, object) observed. \
             Live SCOPE should switch to the REST runner path."
        );
        std::process::exit(1);
    }
}
