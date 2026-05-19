# starter-tool-slack — outbound Slack messages as a `Tool`

A sibling provider crate per
[`DOCS/tools/scope/SCOPE.md`](../../DOCS/tools/scope/SCOPE.md): one
integration per crate, selected by Cargo dependency and constructed in
the consumer's `main.rs`. Depending on this crate compiles a single
`Tool` impl wrapping Slack's
[`chat.postMessage`](https://api.slack.com/methods/chat.postMessage)
Web API method. Nothing in `starter-*` reaches for this crate; the
consumer wires it.

## What it extends

| Surface  | How it's extended                                                                | Where to look |
|----------|----------------------------------------------------------------------------------|---------------|
| MCP      | `SlackPostTool` registered into the consumer's `ToolRegistry` next to starter tools | [src/post.rs](src/post.rs) |
| Config   | `SlackConfig` carries already-resolved `SecretString`s — no env reads here (R5)    | [src/config.rs](src/config.rs) |
| Metrics  | `starter_tool_slack_post_{duration_seconds,errors_total}` on the consumer's `prometheus::Registry` (R7) | [src/metrics.rs](src/metrics.rs) |
| Errors   | `SlackError` distinguishes `Transport` / `RateLimited` / `HttpStatus` / `SlackApi` / `MissingTs`; maps into `starter_spi::Error` | [src/error.rs](src/error.rs) |
| Tests    | One integration test against `wiremock` covering success + 429 + 5xx + auth failure | [tests/post.rs](tests/post.rs) |

## Wire it into `main.rs`

```rust
use std::sync::Arc;
use prometheus::Registry;
use starter_spi::SecretString;
use starter_tool_slack::{SlackConfig, SlackPostTool};

# fn ex(registry: Arc<Registry>, tool_registry: &mut starter_mcp::ToolRegistry) -> Result<(), Box<dyn std::error::Error>> {
let cfg = SlackConfig {
    bot_token:      SecretString::from(std::env::var("SLACK_BOT_TOKEN")?),
    signing_secret: SecretString::from(std::env::var("SLACK_SIGNING_SECRET")?),
    base_url:       SlackConfig::default_base_url(),
};
let tool = SlackPostTool::new(cfg, &registry)?;
tool_registry.register(tool);
# Ok(())
# }
```

The consumer flips the integration off by *not* calling `register(...)`
— the R3 `if cfg.slack.enabled { … }` pattern, no special config knob
inside this crate.

## What's NOT here

- **No env / file reads.** Per R5 the consumer resolves secrets via
  `starter-secrets-keyring` / `starter-secrets-file` (or a literal)
  and hands them in via `SlackConfig`. This crate never names a
  filesystem path.
- **No retry / backoff.** A 429 surfaces as `SlackError::RateLimited`
  with the `Retry-After` seconds attached; the consumer wraps the tool
  in their own retry layer if they want one. SCOPE explicitly rules
  out a built-in scheduler / outbox.
- **No Socket Mode / inbound events.** That's `starter-service-slack`
  (a later stage). The inbound side reuses the same `SlackConfig`'s
  `signing_secret` for HMAC verification — both fields ship in the
  same struct so a consumer's `main.rs` doesn't carry two parallel
  Slack configs once both sides are wired.
- **No `secrecy` direct dep.** The crate imports `SecretString` /
  `ExposeSecret` from `starter_spi::*` — R5.
- **No bot/dispatch architecture.** The implementation is mined from
  `codeless/crates/codeless-slack::web_api` but the architecture
  (Tool/Registry, `prometheus::Registry` passed in, error variants
  mapped into `starter_spi::Error`) is starter's. We lifted the
  request shape; we did not lift `codeless-bot-core`.

## Test plan

```bash
cargo test -p starter-tool-slack
```

The integration suite covers, against a `wiremock` server:

1. `200 ok=true` — happy path, returns `(channel, ts)`.
2. `429 Retry-After: 7` — surfaces `SlackError::RateLimited { retry_after_secs: Some(7) }`, bumps `kind="rate_limited"`.
3. `503` — surfaces `SlackError::HttpStatus { status: 503 }`, bumps `kind="http_status"`.
4. `200 ok=false error=invalid_auth` — maps to `starter_spi::Error::Unauthenticated`, bumps `kind="slack_api"`.
5. Latency histogram observes exactly one sample on the happy path (R7 dashboard regression guard).
