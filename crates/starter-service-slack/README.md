# starter-service-slack — inbound Slack events as a `Service`

A sibling provider crate per
[`DOCS/tools/scope/SCOPE.md`](../../DOCS/tools/scope/SCOPE.md): one
integration per crate, selected by Cargo dependency and constructed in
the consumer's `main.rs`. Depending on this crate compiles a single
`Service` impl wrapping Slack's
[Socket Mode](https://api.slack.com/apis/socket-mode) transport: it
opens the WSS connection via `apps.connections.open`, acks each
envelope, and emits every `events_api` event into the consumer's
`EventSink` as `kind = "slack.<event_type>"`. Nothing in `starter-*`
reaches for this crate; the consumer wires it.

## What it extends

| Surface  | How it's extended                                                                                                                  | Where to look                          |
|----------|------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------|
| Services | `SlackSocketModeService` registered into the consumer's `ServiceRegistry` next to other services                                   | [src/service.rs](src/service.rs)       |
| Transport| Socket Mode connect+pump loop; lifted from `codeless-slack::socket_mode`                                                            | [src/socket_mode.rs](src/socket_mode.rs)|
| Retry    | Exponential-backoff `RetryPolicy` with a max-attempts circuit (R9 — service owns its own restart policy, registry does not auto-restart) | [src/retry.rs](src/retry.rs)            |
| Config   | `SlackSocketModeConfig` carries an already-resolved `SecretString` app token — no env reads here (R5)                              | [src/config.rs](src/config.rs)         |
| Metrics  | `starter_service_slack_events_total{kind}`, `starter_service_slack_restarts_total{reason}`, `starter_service_slack_running` on the consumer's `prometheus::Registry` (R7) | [src/metrics.rs](src/metrics.rs)       |
| Errors   | `SlackSocketModeError` distinguishes `Transport` / `HttpStatus` / `SlackApi` / `BadWssUrl` / `WebSocket` / `CircuitTripped`; maps into `starter_spi::Error::Internal` | [src/error.rs](src/error.rs)           |
| Tests    | Integration tests stub `apps.connections.open` with `wiremock` and the WebSocket with a hand-rolled `tokio-tungstenite` server     | [tests/socket_mode.rs](tests/socket_mode.rs)|

## Wire it into `main.rs`

```rust
use std::sync::Arc;
use prometheus::Registry;
use starter_spi::SecretString;
use starter_spi::service::{EventSink, ServiceRegistry};
use starter_service_slack::{SlackSocketModeConfig, SlackSocketModeService};

# async fn ex(
#     registry: Arc<Registry>,
#     sink: Arc<dyn EventSink>,
# ) -> Result<(), Box<dyn std::error::Error>> {
let cfg = SlackSocketModeConfig {
    app_token: SecretString::from(std::env::var("SLACK_APP_TOKEN")?),
    base_url:  SlackSocketModeConfig::default_base_url(),
};
let svc = SlackSocketModeService::new(cfg);

let mut services = ServiceRegistry::new().register(svc);
services.start_all(registry, sink).await?;
// … hand `services` to the shutdown driver; the consumer calls
// `services.shutdown().await` on Ctrl-C.
# Ok(())
# }
```

The consumer flips the service off by *not* calling `register(...)` —
the R3 `if cfg.slack.enabled { … }` pattern, no special config knob
inside this crate.

## What's NOT here

- **No domain interpretation.** Per R4 the service deserializes each
  envelope only enough to know the event-type label, then forwards the
  raw JSON payload into the sink. Whether `slack.app_mention` triggers
  a reply, a workflow, or nothing is the consumer's call.
- **No env / file reads.** Per R5 the consumer resolves secrets via
  `starter-secrets-keyring` / `starter-secrets-file` (or a literal) and
  hands them in via `SlackSocketModeConfig`.
- **No registry-side auto-restart.** Per R9 the registry is a lifetime
  manager, not a supervisor. The retry layer lives inside this crate:
  exponential backoff between connect attempts, capped at
  `max_backoff`, with a `max_attempts` circuit that surfaces
  `SlackSocketModeError::CircuitTripped` and lets the JoinHandle
  resolve to `Err` rather than pinning a permanently-broken Slack app
  in a hot loop.
- **No outbound `chat.postMessage`.** That's `starter-tool-slack` — a
  consumer who wants both sides depends on both crates.
- **No `secrecy` direct dep.** The crate imports `SecretString` /
  `ExposeSecret` from `starter_spi::*` — R5.
- **No `codeless-bot-core` dispatcher.** The implementation is mined
  from `codeless/crates/codeless-slack::socket_mode` but the
  architecture (Service/ServiceRegistry, EventSink, prometheus passed
  in via `ServiceContext`) is starter's. We lifted the connect+ack
  loop; we did not lift `codeless-bot-core`.

## Test plan

```bash
cargo test -p starter-service-slack
```

The integration suite covers:

1. **Happy path** — `apps.connections.open` returns `ok=true`, the
   WebSocket server emits one `events_api` envelope, the service
   acks it, emits `slack.app_mention` into the sink, and bumps the
   `events_total` counter.
2. **Shutdown during backoff** — `apps.connections.open` is
   unreachable; the service sits in the backoff sleep. Shutdown via
   `ServiceRegistry::shutdown` short-circuits the sleep and produces
   `ServiceShutdownOutcome::Clean` (Smoke test 5).
3. **Circuit trip** — `apps.connections.open` returns `ok=false`
   forever; after `max_attempts` consecutive failures the service
   exits via `SlackSocketModeError::CircuitTripped` rather than
   retrying indefinitely.
