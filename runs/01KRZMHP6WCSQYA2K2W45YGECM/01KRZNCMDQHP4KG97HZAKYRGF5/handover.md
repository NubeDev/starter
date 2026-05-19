## Done

- Added `crates/starter-spi/src/service/` with: `Service` trait, `ServiceContext` (`#[non_exhaustive]`; `metrics`/`shutdown`/`sink`), `ServiceHandle`, `ServiceRegistry` (owns the single `watch::Sender<bool>`; `SHUTDOWN_DEADLINE_DEFAULT = 5s`; `shutdown()` / `shutdown_with_deadline(...)`; `ShutdownReport` with `Clean`/`Error`/`JoinError`/`Aborted`), `EventSink` + `SinkError` (`Saturated`/`Closed`/`Other`) + `SinkResult` + `Event`, `FanOut` helper (logs-and-continues on Closed/Other, bubbles Saturated), and a `broadcast`-feature-gated blanket `EventSink for tokio::sync::broadcast::Sender<E>` impl (D2; default-on).
- Re-exported `secrecy::SecretString` at `starter_spi::SecretString` (R5).
- Updated `crates/starter-spi/Cargo.toml`: new direct deps `prometheus`, `secrecy`, `tracing`; tokio features extended with `time`/`rt`/`macros`; `[features] default = ["broadcast"]` added.
- Added workspace `secrecy = "0.10"` dep in root `Cargo.toml`.
- Refreshed `DOCS/tools/scope/starter-spi-deps.baseline.txt` to reflect the new direct deps (R8/D1 — updated in the same commit that changed `starter-spi`'s deps).
- Added `crates/starter-spi/tests/service.rs` (6 tests; all pass).
- Committed as `Phase 1 — starter-spi additions per the source SCOPE` on `codeless/starter-tools-services`.

## Next

- Stage 4: the next stage in the plan picks up from here — likely starter-observability helper review and/or kicking off the first provider crate (per SCOPE "Suggested first three", `starter-tool-slack` or `starter-tool-gmail`). A fresh session will pick it up.

## What you need to know

- D4's note that `broadcast::Sender::send` Err maps to `Saturated` is semantically incorrect for `tokio::sync::broadcast` (Sender::send only errors on no-receivers; lag-overwrite isn't observable to the sender). I mapped no-receivers to `SinkError::Closed`, documented the rationale in `service/broadcast.rs`, and kept the `Saturated` variant for sinks that *can* detect overflow synchronously (e.g. `mpsc::try_send` returning `Full`). Worth flagging if D4 wording gets revisited.
- D4 wording said `EventSink::emit -> SpiResult<()>`; I returned `Result<(), SinkError>` via a new `SinkResult` alias (per D4's "Stage 3 makes the call when the trait actually lands") rather than folding sink failures into `starter_spi::Error`. Sink errors are a back-pressure signal, not an HTTP-mappable domain failure — keeping them on their own axis matches D4 intent.
- `ServiceRegistry` owns started handles internally; `start_all(metrics, sink)` + `shutdown()` is the canonical flow. `context_with_sink(metrics, sink)` is exposed for consumers who want to drive `Service::start` manually.
- `FanOut::emit` iterates sinks **sequentially** (because the trait method is `async`). Doc-comment says callers wanting concurrent dispatch should wrap each sink in a spawned task. v1 trade-off; revisit if it bites.
- Baseline file: workspace-local path on the `starter-spi` line is stripped (matches the pre-existing convention).

## Open questions

- D4 mapping of broadcast `SendError` — confirm `Closed` (semantically accurate) vs. the literal `Saturated` wording in SCOPE. Flagged in `service/broadcast.rs` doc-comment.
- Whether D4's intent was `SpiResult<()>` over the existing `Error` enum or a separate `SinkError` — I went with the latter; revisit if a consumer wants sink failures to flow through the same `?` chain as domain errors.
