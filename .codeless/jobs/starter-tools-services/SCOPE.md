# Scope — starter-tools-services

> Source of truth: [`DOCS/tools/scope/SCOPE.md`](../../../DOCS/tools/scope/SCOPE.md)
> in the starter repo. This file is the per-job brief the runner
> reads before every stage; it is intentionally short. When this
> file disagrees with the source-of-truth SCOPE, that doc wins —
> open an issue and update this file.

## Goal

Ship the `starter-tool-*` / `starter-service-*` family: a set of
**sibling crates**, each wrapping one third-party integration as
either a `Tool` (outbound, one-shot, MCP-callable) or a `Service`
(inbound, long-running listener with a lifecycle). On the starter
side, add **only** the new `Service` / `ServiceRegistry` /
`ServiceContext` / `ServiceHandle` / `EventSink` types and a
`SecretString` re-export to `starter-spi`. Everything else — every
provider crate, every config struct, every credential source — is
sibling code the consumer composes in `main.rs`.

v0.1 ships **three integrations** mined from the codeless
workspace:

- `starter-tool-slack` + `starter-service-slack` (socket-mode).
- `starter-tool-telegram` + `starter-service-telegram` (long-poll).
- `starter-tool-gmail` (send only; inbound deferred).

## In scope

- **`starter-spi` additions** per the source SCOPE §"What lands in
  starter-spi": `Service` trait, `EventSink` trait, `ServiceContext`
  (`#[non_exhaustive]`), `ServiceHandle`, `ServiceRegistry` (which
  owns the single `tokio::sync::watch::Sender<bool>` and fans out
  receivers), plus `SecretString` re-exported from `secrecy`.
- **Blanket `EventSink` impl** on
  `tokio::sync::broadcast::Sender<Event>` and a `Vec<Arc<dyn
  EventSink>>` fan-out helper. The fan-out helper logs per-sink
  errors and bubbles a typed `Saturated` variant when the broadcast
  channel is full.
- **`DOCS/tools/scope/starter-spi-deps.baseline.txt`** — a snapshot
  of `cargo tree -p starter-spi --edges normal` committed alongside
  this scope. The dep-leakage smoke test compares against it; the
  baseline is updated only when `starter-spi` itself changes.
- **Five provider crates**: `starter-tool-slack`,
  `starter-service-slack`, `starter-tool-telegram`,
  `starter-service-telegram`, `starter-tool-gmail`.
- Each provider crate ships: a `Config` struct with `SecretString`
  fields, one or more `Tool` / `Service` impls, a `register(...)`
  helper, a README mirroring the `examples/notes` "how it's
  extended" table, and at least one integration test against a
  mock HTTP server (`wiremock` for HTTP, `tokio-tungstenite` test
  server for socket-mode).
- The five **design smoke tests** from the source SCOPE §"Smoke
  test for the design" — every one passing in CI before merge,
  with the dep-baseline check as a CI gate not an after-the-fact
  audit.

## Out of scope

- **A dynamic plugin loader.** No `dlopen`, no WASM host, no
  manifest scanner. Cargo-dependency selection is the contract.
- **A shared "integration" type** subsuming `Tool` + `Service`.
  R2 forbids it.
- **An outbox / retry queue / scheduler.** Provider crates may
  retry their own HTTP calls; durable scheduling is a separate
  crate when a real consumer needs it.
- **A supervisor / auto-restart policy in the registry.** R9 —
  service failure is observability-only by default; restart
  policy lives in the service implementation, not the registry.
- **A `starter-tools` mega-crate** with cargo features per
  provider. R1 — cargo feature unification would leak provider
  deps across every workspace consumer.
- **A cross-crate SemVer lockstep.** Each provider crate is
  versioned independently with a pinned `starter-spi` major.
- Inbound Gmail (`users.watch` + Pub/Sub or `history.list`
  long-poll) — explicitly deferred per the source SCOPE's
  "Gmail (send only at first; Gmail watch lands later)".
- Provider-side admin UIs, settings pages, or operator dashboards.
  Provider crates hand back `Router<S>` fragments and metric
  exposers; consumers compose them.

## Hard rules (load-bearing)

- **R1** — One integration per crate. No mega-crate; no cargo
  features per provider. Per-crate gives the consumer full control
  over what is compiled, audited, and linked.
- **R2** — `Tool` and `Service` are different traits. `Tool` is
  stateless from starter's point of view and has a caller;
  `Service` has no caller, runs on its own, and publishes into an
  `EventSink`. Collapsing them forces both sides to leak.
- **R3** — Registries are open and consumer-built. The consumer's
  `main.rs` decides what gets registered. Runtime on/off is an
  `if` around `.register(...)`; no plugin manifest, no dynamic
  loading.
- **R4** — Provider crates do not own domain logic. A
  `starter-service-slack` deserializes events and emits via
  `EventSink`; it does not pattern-match on payloads beyond what
  is needed to deserialize. The consumer's domain layer decides
  what to do.
- **R5** — Credentials come from a `SecretString` in the
  `Config`, never read by the provider crate. The consumer
  resolves through `starter-secrets-*` (or hardcodes in dev).
  Switching backends does not touch any provider crate.
- **R6** — The same `Authenticator` gates anything user-facing.
  Provider HTTP routes either carry their own provider-defined
  signature middleware (Slack signing secret, Telegram secret
  token) or are wrapped by the consumer in `with_principal(...)`.
  No third auth model.
- **R7** — Observability is required, not optional. Every
  `Tool::invoke` emits structured tracing with a stable
  `tool.name` and registers a latency histogram + error counter.
  Every `Service::start` registers an event-emitted counter, a
  restart counter, and an "is running" gauge.
- **R8** — No transitive vendor SDKs in `starter-spi`. The new
  types are the only additions; nothing Slack/Gmail/Telegram-
  specific leaks in. The dep-leakage smoke test against the
  committed baseline enforces this.
- **R9** — Service failure is observability-only by default. The
  registry records the error on the span, increments the restart
  counter, and **does not auto-restart**. Restart policy
  (backoff, jitter, max attempts) lives in the service's own
  loop.

## Constraints

- Each provider crate depends only on `starter-spi` +
  `starter-observability` from the starter set, plus whatever
  vendor SDK / HTTP / websocket it actually needs. The
  dep-leakage smoke test enforces this; nothing else from the
  starter set is mandatory.
- `SecretString` re-export is at the `starter-spi` crate root so
  provider crates write `use starter_spi::SecretString` and never
  `use secrecy::SecretString`.
- `ServiceContext` is `#[non_exhaustive]`; adding fields is
  additive across SemVer. Removing or retyping is breaking.
- `ServiceRegistry` exposes `SHUTDOWN_DEADLINE_DEFAULT` as a
  constant (5s) plus `shutdown` and `shutdown_with_deadline`
  methods. The smoke test asserts against the constant, not a
  magic number.
- Provider services that need to listen for events implement
  cooperative shutdown by `.changed().await`-ing on
  `ctx.shutdown` inside their main loop. Services that miss
  this fail the shutdown-actually-shuts-down smoke test.
- The blanket `EventSink` impl on
  `tokio::sync::broadcast::Sender<Event>` is gated behind a
  default-on `broadcast` feature so a minimal SPI consumer
  without `tokio::sync::broadcast` still compiles cleanly.

## Phasing

- **Phase 1** — `starter-spi` additions (the five types + the
  `SecretString` re-export + the blanket impl + the fan-out
  helper). The dep-leakage baseline file lands here.
- **Phase 2** — Slack: `starter-tool-slack` (outbound,
  `chat.postMessage`) and `starter-service-slack` (inbound,
  socket-mode).
- **Phase 3** — Telegram: `starter-tool-telegram` (outbound,
  `sendMessage`) and `starter-service-telegram` (inbound,
  long-poll `getUpdates`).
- **Phase 4** — Gmail send-only: `starter-tool-gmail`
  (`users.messages.send`). Inbound deferred.
- **Smoke tests** — the five design checks from the source SCOPE
  pass in CI for every provider crate.

## Deliverables

- `starter-spi` SemVer-minor bump landing the five new types +
  `SecretString` re-export + blanket `EventSink` impl + fan-out
  helper.
- Five new sibling crates per §"In scope".
- `DOCS/tools/scope/starter-spi-deps.baseline.txt` committed and
  enforced.
- Five design smoke tests passing in CI for each provider crate.
- Each provider crate carries a README mirroring the
  `examples/notes` "how it's extended" table.

## Open questions (resolve in stage 1)

The source SCOPE does not enumerate explicit open questions, but
landing the SPI additions cleanly requires the runner to pin four
small design points before code starts:

1. **`starter-spi-deps.baseline.txt` location.** Bias:
   `DOCS/tools/scope/starter-spi-deps.baseline.txt`; updated only
   when `starter-spi` itself changes (a separate commit, reviewed
   independently from any provider-crate work).
2. **Blanket `EventSink` impl placement.** Bias: in `starter-spi`
   behind a default-on `broadcast` feature so consumers without
   `tokio::sync::broadcast` still compile a minimal SPI.
3. **5-second shutdown deadline.** Bias: `ServiceRegistry`
   exposes `SHUTDOWN_DEADLINE_DEFAULT` as a public constant; the
   smoke test references it. Consumers wanting a different value
   call `shutdown_with_deadline(d)`.
4. **`EventSink::emit` error semantics.** Bias: `SpiResult<()>`
   with a typed `EmitError` enum (`Saturated` when broadcast full,
   `Closed`, `Other`). The `Vec<Arc<dyn EventSink>>` fan-out
   helper logs and continues on per-sink `Other` errors but
   bubbles `Saturated` so a wedged consumer is visible, not
   silently dropping events.

Record decisions in this file under "Decisions" before stage 3
(the first code stage) begins.

## Decisions

(populated in stage 1)

## Cross-cutting checks the runner must keep honest

- **Dep-leakage baseline** — `cargo tree -p starter-spi --edges
  normal` must match
  `DOCS/tools/scope/starter-spi-deps.baseline.txt`. CI gate.
- **No special-case wiring** — every provider's
  `Tool` / `Service` registers via the same `.register(...)` call
  the notes demo uses. No provider-specific helper modules under
  `starter-server`.
- **Config-guarded construction** — a single env var flips an
  integration off via the `if`-around-`.register(...)` pattern
  in `main.rs`. Disabling does not require a recompile.
- **Secrets-backend-swappable** — switching from
  `starter-secrets-file` to `starter-secrets-keyring` requires
  zero changes to any provider crate's `Config` or tests.
- **Shutdown-actually-shuts-down** — flipping the registry's
  watch sender resolves every `Service`'s `JoinHandle` within
  `SHUTDOWN_DEADLINE_DEFAULT = 5s`. Provider services that miss
  cooperative shutdown fail the test.
- **No `secrecy` import outside `starter-spi`** — provider crates
  must `use starter_spi::SecretString`; a CI grep enforces.
- **No raw `prometheus` registrations** — providers register
  metrics through the helpers (when stabilised) or against the
  registry from `ServiceContext.metrics`; a CI grep rejects raw
  `Registry::register_counter` etc. inside provider crates once
  the helper API exists. Until then, review-time.
