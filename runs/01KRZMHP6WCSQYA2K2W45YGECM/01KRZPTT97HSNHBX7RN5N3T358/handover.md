## Done

- Created `crates/starter-tool-telegram` (Cargo.toml, src/{lib,config,error,metrics,send}.rs, tests/send.rs, README) wrapping Bot API `sendMessage` as a `Tool`. `TelegramConfig { bot_token: SecretString, base_url }`. Latency histogram + error counter register on the consumer's `prometheus::Registry` (R7). Wiremock integration test covers happy / 429-with-`parameters.retry_after` / 5xx / 200+`ok=false`+Unauthorized / bad-input / latency-observation paths.
- Created `crates/starter-service-telegram` (Cargo.toml, src/{lib,config,error,metrics,offset,retry,long_poll,service}.rs, tests/long_poll.rs, README) implementing `TelegramBotService::start` as a `getUpdates` long-poll. Each update emitted via `ctx.sink` as `kind="telegram.<update_type>"`. `OffsetStore` trait + `InMemoryOffsetStore` v0.1 impl persist the `getUpdates` cookie in memory; the trait is the at-rest seam reserved for later (sqlite/redis backend slots in without breaking change). Restart policy in the impl per R9 — exponential backoff (1s→60s) with 6-attempt circuit + immediate trip on non-transient 401 / 404. Respects `ctx.shutdown` at every await point (poll race + backoff race).
- Both crates added to root `Cargo.toml` `members` + `[workspace.dependencies]`.
- `cargo test -p starter-tool-telegram -p starter-service-telegram`: 6 + 4 integration tests + 6 unit tests + 2 doctests all pass.
- Committed as `4cd4739` with subject starting "Phase 3 — starter-tool-telegram + starter-service-telegram".

## Next

- Stage 8 of 9 (per the SCOPE / job model): `starter-tool-gmail` (send-only Phase 3 deliverable). Will mine `codeless/crates/codeless-tools` for the Gmail send path; uses OAuth2 credentials, so consult the existing `starter-auth-oauth` work for the secret-resolution shape. Telegram inbound (webhook) and Gmail watch / push are explicitly out of scope per SCOPE's "First crates to ship" note.

## What you need to know

- `starter-spi` already exposes `service::{Service, ServiceContext, ServiceHandle, ServiceRegistry, EventSink, SinkResult, ServiceShutdownOutcome}` and `SecretString` / `ExposeSecret` re-exports — no additions were needed in this stage.
- `ServiceContext` is `#[non_exhaustive]`; construct via `ServiceRegistry::context_with_sink` or via the existing `start_all(metrics, sink)` helper. Tests use the registry path.
- The Telegram update-kind dispatch picks `kind` by checking which top-level field is present on the JSON update (`message`, `edited_message`, `callback_query`, …). The mapping list is in `long_poll.rs::update_kind` — extend it there if Telegram adds a new update type. Unknown shapes surface as `telegram.unknown` so an emit always happens (R4: forward verbatim).
- Bot API URL shape: `<base_url>/bot<bot_token>/<method>`. Token goes in the path, *not* in an `Authorization` header. The crates pre-build the `<base_url>/bot<bot_token>` prefix at construction so the token never reappears in a per-call `format!` an operator might log.
- The service crate's retry layer treats 401 / 404 as fatal (`is_fatal()`) and trips the circuit immediately, bypassing the backoff schedule. Other failures (transport, 5xx, `ok=false`) go through `RetryPolicy::next_step`.

## Open questions

- (none)
