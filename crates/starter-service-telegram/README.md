# starter-service-telegram — inbound Telegram updates as a `Service`

A sibling provider crate per
[`DOCS/tools/scope/SCOPE.md`](../../DOCS/tools/scope/SCOPE.md): one
integration per crate, selected by Cargo dependency and constructed in
the consumer's `main.rs`. Depending on this crate compiles a single
`Service` impl driving the Telegram Bot API
[`getUpdates`](https://core.telegram.org/bots/api#getupdates)
long-poll. Inbound updates surface as `EventSink` emits with
`kind = "telegram.<update_type>"`; the domain interpretation is the
consumer's (SCOPE R4).

## What it extends

| Surface  | How it's extended                                                                | Where to look |
|----------|----------------------------------------------------------------------------------|---------------|
| Lifecycle | `TelegramBotService` registered into the consumer's `ServiceRegistry` next to other services | [src/service.rs](src/service.rs) |
| Events    | Every `getUpdates` element forwarded verbatim as `telegram.<update_type>` via `EventSink::emit` | [src/long_poll.rs](src/long_poll.rs) |
| Offset    | `OffsetStore` trait keeps the `getUpdates` cookie; `InMemoryOffsetStore` is the v0.1 impl, at-rest seam reserved | [src/offset.rs](src/offset.rs) |
| Config    | `TelegramBotConfig` carries an already-resolved `SecretString` — no env reads here (R5) | [src/config.rs](src/config.rs) |
| Metrics   | `starter_service_telegram_{events_total,restarts_total,running}` on the consumer's `prometheus::Registry` (R7) | [src/metrics.rs](src/metrics.rs) |
| Retry     | Exponential backoff + circuit breaker lives **in the impl**, not the registry (R9) | [src/retry.rs](src/retry.rs) |
| Errors    | `TelegramBotError` covers `Transport` / `HttpStatus` / `BotApi` / `CircuitTripped`; maps into `starter_spi::Error::Internal` | [src/error.rs](src/error.rs) |
| Tests     | Integration tests against `wiremock` covering emit, offset persistence, shutdown-during-backoff, 401 circuit-trip | [tests/long_poll.rs](tests/long_poll.rs) |

## Wire it into `main.rs`

```rust
use std::sync::Arc;
use prometheus::Registry;
use starter_spi::SecretString;
use starter_spi::service::ServiceRegistry;
use starter_service_telegram::{TelegramBotConfig, TelegramBotService};

# async fn ex(registry: Arc<Registry>, sink: Arc<dyn starter_spi::service::EventSink>) -> Result<(), Box<dyn std::error::Error>> {
let cfg = TelegramBotConfig {
    bot_token: SecretString::from(std::env::var("TELEGRAM_BOT_TOKEN")?),
    base_url:  TelegramBotConfig::default_base_url(),
};
let svc = TelegramBotService::new(cfg);
let mut services = ServiceRegistry::new().register(svc);
services.start_all(registry, sink).await?;
# Ok(())
# }
```

The consumer flips the integration off by *not* calling `register(...)`
— the R3 `if cfg.telegram.enabled { … }` pattern, no special config
knob inside this crate.

## What's NOT here

- **No webhook receiver.** v0.1 ships long-poll only. A webhook
  variant would be a separate `Service` constructor inside this crate
  later, not a separate sibling.
- **No persistent offset store.** v0.1 keeps the `getUpdates` cookie
  in memory; restarting the consumer re-delivers everything Telegram
  still has in its 24h retention window. The [`OffsetStore`] trait is
  the seam an at-rest backend will slot into.
- **No domain interpretation.** R4 forbids pattern-matching past the
  envelope shape. The service picks `kind = "telegram.message"` etc.
  from *which* top-level field is present (`message`, `edited_message`,
  `callback_query`, …) and forwards the rest of the JSON verbatim.
- **No auto-restart from the registry.** R9: failed services exit
  with an `Internal` error; the registry observes that and stops.
  The retry layer inside this crate handles transient failures (DNS
  blip, 502) within a single run.
- **No `secrecy` direct dep.** The crate imports `SecretString` /
  `ExposeSecret` from `starter_spi::*` — R5.
- **No `codeless-bot-core` framing.** The codeless reference owns a
  `Dispatcher` + `BotTransport` + `ThreadMap` stack tuned to the
  codeless control plane; this crate lifts the *HTTP shape* and the
  *long-poll loop* and stops there.

## Test plan

```bash
cargo test -p starter-service-telegram
```

The integration suite, against a `wiremock` Bot API:

1. One `getUpdates` returning a `message` update → sink sees
   `("telegram.message", payload)`, events counter bumps.
2. Two consecutive polls — the second MUST carry `offset = max(update_id) + 1`.
3. Service started against a closed port enters backoff; `shutdown`
   from the registry returns `ServiceShutdownOutcome::Clean` (not
   `Aborted`) well under the backoff sleep — the loop races shutdown
   against the sleep.
4. A 401 from `getUpdates` trips the circuit immediately (non-transient).
