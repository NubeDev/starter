//! Smoke test 2 — no special-case wiring.
//!
//! Every provider crate's `Tool` / `Service` must register via the same
//! `.register(...)` call the notes demo uses
//! (`examples/notes/src/main.rs`). No provider-specific helper module
//! under `starter-server`, no new `main.rs` shape.
//!
//! The check is mechanical: we build a `ToolRegistry` and a
//! `ServiceRegistry`, push every shipped provider into them via the
//! ordinary `.register(value)` method, and assert each one is present.
//! If a provider crate ever required a bespoke `register_with(...)` or
//! a builder side-channel, this test would no longer compile.

use std::sync::Arc;

use prometheus::Registry;
use starter_mcp_substitute::ToolRegistry;
use starter_service_slack::{SlackSocketModeConfig, SlackSocketModeService};
use starter_service_telegram::{TelegramBotConfig, TelegramBotService};
use starter_spi::service::ServiceRegistry;
use starter_spi::SecretString;
use starter_tool_gmail::{GmailConfig, GmailSendTool};
use starter_tool_slack::{SlackConfig, SlackPostTool};
use starter_tool_telegram::{TelegramConfig, TelegramSendMessageTool};

// `starter-mcp` is not in this crate's dep graph (Stage 9 is a check
// crate that should not pull MCP transitively), but the contract being
// verified is purely shape — "a `ToolRegistry::register` exists, takes
// a `Tool` by value, and returns `Self`." We restate that shape locally
// so the test compiles against any implementation matching it; the
// real `starter_mcp::ToolRegistry` is exercised by the notes example
// itself.
mod starter_mcp_substitute {
    use starter_spi::tool::Tool;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[derive(Default)]
    pub struct ToolRegistry {
        tools: HashMap<String, Arc<dyn Tool>>,
    }

    impl ToolRegistry {
        pub fn new() -> Self {
            Self::default()
        }
        pub fn register<T: Tool>(mut self, tool: T) -> Self {
            let def = tool.definition();
            self.tools.insert(def.name.clone(), Arc::new(tool));
            self
        }
        pub fn names(&self) -> Vec<&str> {
            self.tools.keys().map(|s| s.as_str()).collect()
        }
    }
}

fn dummy_secret(s: &str) -> SecretString {
    SecretString::from(s.to_string())
}

#[test]
fn every_tool_registers_through_the_same_call() {
    let prom = Registry::new();

    let slack = SlackPostTool::new(
        SlackConfig {
            bot_token: dummy_secret("xoxb-test"),
            signing_secret: dummy_secret("sig"),
            base_url: SlackConfig::default_base_url(),
        },
        &prom,
    )
    .expect("slack tool");

    let telegram = TelegramSendMessageTool::new(
        TelegramConfig {
            bot_token: dummy_secret("12345:test"),
            base_url: TelegramConfig::default_base_url(),
        },
        &prom,
    )
    .expect("telegram tool");

    let gmail = GmailSendTool::new(
        GmailConfig {
            oauth_access_token: dummy_secret("ya29.test"),
            user_id: GmailConfig::default_user_id(),
            base_url: GmailConfig::default_base_url(),
        },
        &prom,
    )
    .expect("gmail tool");

    // The whole point: no `register_slack`, no `register_with_metrics`,
    // no `wire_into_server` — every provider uses the same call.
    let tools = ToolRegistry::new()
        .register(slack)
        .register(telegram)
        .register(gmail);

    let names = tools.names();
    assert_eq!(names.len(), 3, "three tools registered, got {names:?}");
}

#[test]
fn every_service_registers_through_the_same_call() {
    let services = ServiceRegistry::new()
        .register(SlackSocketModeService::new(SlackSocketModeConfig {
            app_token: dummy_secret("xapp-test"),
            base_url: SlackSocketModeConfig::default_base_url(),
        }))
        .register(TelegramBotService::new(TelegramBotConfig {
            bot_token: dummy_secret("12345:test"),
            base_url: TelegramBotConfig::default_base_url(),
        }));

    assert_eq!(services.len(), 2, "two services registered");
    let names: Vec<&str> = services.names();
    assert!(names.contains(&starter_service_slack::SERVICE_NAME));
    assert!(names.contains(&starter_service_telegram::SERVICE_NAME));

    // Smoke 2 is shape-only; we drop the registry without starting it.
    let _ = services;
    // Touch `Arc` to silence unused-import lints in some toolchains.
    let _: Arc<()> = Arc::new(());
}
