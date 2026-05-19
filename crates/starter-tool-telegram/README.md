# starter-tool-telegram — outbound Telegram messages as a `Tool`

A sibling provider crate per
[`DOCS/tools/scope/SCOPE.md`](../../DOCS/tools/scope/SCOPE.md): one
integration per crate, selected by Cargo dependency and constructed in
the consumer's `main.rs`. Depending on this crate compiles a single
`Tool` impl wrapping the Telegram Bot API
[`sendMessage`](https://core.telegram.org/bots/api#sendmessage) method.
Nothing in `starter-*` reaches for this crate; the consumer wires it.

## What it extends

| Surface  | How it's extended                                                                | Where to look |
|----------|----------------------------------------------------------------------------------|---------------|
| MCP      | `TelegramSendMessageTool` registered into the consumer's `ToolRegistry` next to starter tools | [src/send.rs](src/send.rs) |
| Config   | `TelegramConfig` carries an already-resolved `SecretString` — no env reads here (R5) | [src/config.rs](src/config.rs) |
| Metrics  | `starter_tool_telegram_send_message_{duration_seconds,errors_total}` on the consumer's `prometheus::Registry` (R7) | [src/metrics.rs](src/metrics.rs) |
| Errors   | `TelegramError` distinguishes `Transport` / `RateLimited` / `HttpStatus` / `BotApi` / `MissingResult`; maps into `starter_spi::Error` | [src/error.rs](src/error.rs) |
| Tests    | One integration test against `wiremock` covering success + 429 + 5xx + auth failure | [tests/send.rs](tests/send.rs) |

## Wire it into `main.rs`

```rust
use std::sync::Arc;
use prometheus::Registry;
use starter_spi::SecretString;
use starter_tool_telegram::{TelegramConfig, TelegramSendMessageTool};

# fn ex(registry: Arc<Registry>, tool_registry: &mut starter_mcp::ToolRegistry) -> Result<(), Box<dyn std::error::Error>> {
let cfg = TelegramConfig {
    bot_token: SecretString::from(std::env::var("TELEGRAM_BOT_TOKEN")?),
    base_url:  TelegramConfig::default_base_url(),
};
let tool = TelegramSendMessageTool::new(cfg, &registry)?;
tool_registry.register(tool);
# Ok(())
# }
```

The consumer flips the integration off by *not* calling `register(...)`
— the R3 `if cfg.telegram.enabled { … }` pattern, no special config
knob inside this crate.

## What's NOT here

- **No env / file reads.** Per R5 the consumer resolves secrets via
  `starter-secrets-keyring` / `starter-secrets-file` (or a literal)
  and hands them in via `TelegramConfig`.
- **No retry / backoff.** A 429 surfaces as
  `TelegramError::RateLimited` with the `retry_after` seconds attached
  (read from the JSON `parameters.retry_after` or the `Retry-After`
  header, whichever the Bot API provided); the consumer wraps the
  tool in their own retry layer if they want one.
- **No long-poll / inbound updates.** That's `starter-service-telegram`
  — a separate sibling crate. Both reuse the same `bot_token` shape so
  a consumer's `main.rs` doesn't carry two parallel Telegram configs
  once both sides are wired.
- **No `secrecy` direct dep.** The crate imports `SecretString` /
  `ExposeSecret` from `starter_spi::*` — R5.
- **No MarkdownV2 escaping / pre-block wrapper.** Lifted from
  `codeless-telegram::web_api`, not `outbound.rs`: the wrapping
  decision is the consumer's. `parse_mode` is forwarded verbatim.

## Test plan

```bash
cargo test -p starter-tool-telegram
```

The integration suite covers, against a `wiremock` server:

1. `200 ok=true` — happy path, returns `(message_id, chat_id)`.
2. `429` with `parameters.retry_after: 7` — surfaces
   `TelegramError::RateLimited { retry_after_secs: Some(7) }`, bumps
   `kind="rate_limited"`.
3. `503` — surfaces `TelegramError::HttpStatus { status: 503 }`, bumps
   `kind="http_status"`.
4. `200 ok=false description="Unauthorized"` — maps to
   `starter_spi::Error::Unauthenticated`, bumps `kind="bot_api"`.
5. Missing required `text` field — `starter_spi::Error::Invalid`,
   bumps `kind="bad_input"`.
6. Latency histogram observes exactly one sample on the happy path
   (R7 dashboard regression guard).
