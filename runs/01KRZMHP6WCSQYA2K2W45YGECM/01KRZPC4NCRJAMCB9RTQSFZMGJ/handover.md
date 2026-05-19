## Done

- New crate `crates/starter-service-slack` registered in workspace and workspace deps table.
- `SlackSocketModeService` implementing `starter_spi::service::Service`: POST `apps.connections.open` (base_url + bearer app_token), dial WSS, ack each envelope, emit `events_api` events into `ctx.sink` as `kind = "slack.<event_type>"` with raw JSON payload (R4). `ctx.shutdown` observed both during the pump (via `select!`) and during backoff sleeps.
- `RetryPolicy` (exp-backoff doubling, capped at `max_backoff`, trip after `max_attempts` consecutive failures). Clean disconnects reset the counter — Slack rotates sockets ~30 min and that must not count toward the circuit (R9, service-owned restart).
- Metrics (R7): `starter_service_slack_events_total{kind}`, `starter_service_slack_restarts_total{reason}`, `starter_service_slack_running` gauge, all registered on `ctx.metrics`.
- `SlackSocketModeConfig { app_token: SecretString, base_url: String }`. Note: the existing `starter-tool-slack::SlackConfig` has `bot_token` + `signing_secret` but socket-mode needs the distinct app-level `xapp-…` token (different Slack scope), so the config is intentionally separate.
- `SlackSocketModeError` with `Transport / HttpStatus / SlackApi / BadWssUrl / WebSocket / CircuitTripped`, maps to `starter_spi::Error::Internal`.
- 9 tests pass: 3 retry unit, 2 socket_mode unit, 3 wiremock+tungstenite integration (happy-path emit+ack, shutdown-during-backoff = smoke test 5, circuit-trip), 1 doctest. `cargo build --workspace` clean.

## Next

- Stage 7: starter-tool-telegram outbound (sibling of stage 5 but for Telegram bot API).

## What you need to know

- `tokio-tungstenite = "0.24"` chosen to match codeless workspace; newer 0.29 exists but 0.24 was sufficient and Cargo.lock now pins it for the workspace. dev-deps add `handshake` feature for the test server.
- Test uses a hand-rolled `tokio_tungstenite::accept_async` server on a TCP listener (port 0) rather than `tokio_tungstenite::tungstenite::ServerBuilder` because that gave the simplest reliable shape for "send one frame, collect acks, close."
- `kind` label uses raw `event.type` from Slack with `slack.` prefix; envelopes whose inner event has no `type` get `slack.unknown` (defensive, not seen in practice). `hello`/`disconnect` protocol envelopes are *not* emitted — they're not `events_api`.
- Sink emit errors are logged-and-continued inside the pump (D4 says back-pressure bubbling is the FanOut helper's job, not the per-sink call site). The `events_total` counter is only bumped on successful emit.

## Open questions

- (none)
