# `starter-tool-*` / `starter-service-*` — Scope

## One-line summary

A family of small, sibling Rust crates that each wrap one third-party
integration (Gmail, Slack, Telegram, …) as either a **tool** (one-shot
request/response, MCP-callable) or a **service** (long-running
listener/poller with a lifecycle). Each integration is its **own
crate**, selected by Cargo dependency and constructed in `main.rs`.
There is no mega-crate, no cargo-feature matrix, and no built-in plugin
loader.

The starter side ships **two abstractions** in `starter-spi`: `Tool`
(already exists) and a new `Service` (start/shutdown lifecycle), plus
the matching `ToolRegistry` (exists), a new `ServiceRegistry`, and a
typed `EventSink` for services to publish into. Everything else —
every provider crate, every config struct, every credential source —
is sibling code.

## Why this exists

Real products built on starter quickly want to do three things:

1. **Send things outward**: post a Slack message, send a Gmail, push a
   Telegram notification, file a Linear issue. These are stateless
   from starter's point of view: input in, side effect out, result
   back. They fit the existing `Tool` trait perfectly and naturally
   become MCP tools (LLM-callable) and REST handler primitives
   simultaneously.
2. **Listen for events**: Slack socket-mode events, Telegram bot
   updates, Gmail watch push, generic webhooks. These need a
   lifecycle starter does not currently model: `start(ctx) ->
   ServiceHandle`, cooperative shutdown, plus a way to dispatch
   incoming events back into the consumer's domain.
3. **Add more integrations over time** without touching `starter-*`
   crates and without forcing every consumer to compile every
   integration's dependencies (Slack drags in websockets; Gmail drags
   in OAuth2 + a fat Google SDK; Telegram drags in its own client).

Today none of this is modelled. A consumer who wants Slack writes
ad-hoc code in their own crate; the next consumer reinvents the same
thing. We need a shared shape so integrations are interchangeable —
not a shared codebase that owns them all.

## Relationship to existing crates

```
starter-spi   (Tool, ToolRegistry, Authenticator, Principal
                — and NEW: Service, ServiceRegistry,
                  ServiceContext, ServiceHandle, EventSink)
   ▲
   │ depends on
   │
starter-mcp                     (uses Tool, unchanged)
starter-server                  (consumer composes routes, unchanged)
starter-observability           (metric/tracing helpers; required dep
                                 of every provider crate — see R7)
starter-tool-gmail              (outbound Tool impls)
starter-tool-slack              (outbound Tool impls)
starter-tool-telegram           (outbound Tool impls)
starter-service-slack           (inbound Service impl, socket-mode)
starter-service-telegram        (inbound Service impl, long-poll)
```

All arrows point at `starter-spi`. `starter-spi` does not know any
provider exists. No provider crate depends on another provider crate.
A consumer enables Slack by adding `starter-tool-slack` (and/or
`starter-service-slack`) to their `Cargo.toml` and constructing it in
`main.rs`. A consumer who only wants Gmail pays zero compile cost for
Slack.

This mirrors the pattern `starter-store-sqlite` / `starter-store-postgres`
and `starter-auth-token` / `starter-auth-users` already establish: one
concern, one crate, pick what you need.

## Hard rules (load-bearing)

### R1 — One integration per crate

Each provider is its own crate. There is **no** `starter-tools`
mega-crate with cargo features per provider.

Reason: cargo's feature unification means any workspace member that
pulls a mega-crate with `slack` enabled drags Slack's deps into
**every** binary that transitively depends on the mega-crate.
Per-crate gives the consumer full control over what's compiled,
audited, and linked. The starter set already commits to this pattern
(stores, auth, secrets); tools follow it.

### R2 — Tools and services are different traits

`Tool` is `async fn invoke(input: Value) -> Result<Value>`. It is
**stateless** between calls from starter's point of view; any caching
or rate limiting lives inside the implementation. A `Tool` has a
**caller** — the result flows back, errors flow back, latency is the
caller's to observe.

`Service` is a new trait in `starter-spi`. A `Service` has **no
caller** — it runs on its own and publishes into an `EventSink`. Its
error path is observability-only (logs + metrics + restart policy),
not a return value to anyone. Collapsing the two into one trait would
force every `Tool` to handle a no-op event sink and every `Service`
to fabricate an input-shape it never receives — both sides leak.

The concrete v1 contract:

```rust
#[async_trait]
pub trait Service: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    async fn start(&self, ctx: ServiceContext) -> SpiResult<ServiceHandle>;
}

#[non_exhaustive]
pub struct ServiceContext {
    pub metrics: Arc<prometheus::Registry>,
    pub shutdown: tokio::sync::watch::Receiver<bool>,
    pub sink:     EventSink,
}

pub struct ServiceHandle {
    pub join: tokio::task::JoinHandle<SpiResult<()>>,
}
```

`ServiceContext` is `#[non_exhaustive]` so new fields are additive,
not breaking. `start` returns only a `JoinHandle` — the **registry
owns the single `watch::Sender<bool>`** and fans out receivers via
`ServiceContext.shutdown` to every service it spawns. To stop a
service, the registry flips the watch to `true`; the service's loop
observes it and exits cooperatively. No service holds a sender.

### R3 — Registries are open and consumer-built

```rust
let tools = ToolRegistry::new()
    .register(GmailSendTool::new(gmail_cfg))
    .register(SlackPostTool::new(slack_cfg));

let mut services = ServiceRegistry::new();
if cfg.slack.enabled {
    services = services.register(SlackSocketModeService::new(slack_cfg.clone()));
}
if cfg.telegram.enabled {
    services = services.register(TelegramBotService::new(tg_cfg));
}
```

The consumer's `main.rs` decides what gets registered. Runtime
on/off is **the consumer's `if` statement around `.register(...)`**.
`starter-config` provides the env/file read; the decision to
construct still lives in `main.rs`. There is no plugin manifest, no
dynamic loading, no `if cfg!(feature = …)` inside starter crates.

### R4 — Provider crates do not own domain logic

A `starter-tool-slack` knows how to **post a Slack message**. It does
not know what the consumer wants to say, when, or in response to
what. The consumer's domain layer calls the tool; the tool does not
call back into the domain.

A `starter-service-slack` knows how to **receive events from Slack**.
It publishes every event into the `EventSink` it was handed via
`ServiceContext`. It does not pattern-match on event payloads
beyond what's necessary to deserialize them.

The `EventSink` itself lives in `starter-spi`:

```rust
#[async_trait]
pub trait EventSink: Send + Sync + 'static {
    /// `kind` is a stable, service-supplied string (e.g. "slack.message").
    /// `payload` is the deserialized provider event, as JSON.
    async fn emit(&self, kind: &str, payload: serde_json::Value) -> SpiResult<()>;
}
```

The sink is the **only** way a service hands events to the consumer.
A blanket impl on `tokio::sync::broadcast::Sender<Event>` (for some
consumer-defined `Event`) and a `Vec<Arc<dyn EventSink>>` fan-out
helper ship from `starter-spi`. Provider crates do not invent their
own dispatch shape — that's what makes services interchangeable.

### R5 — Credentials come from a SecretString, not the tool crate

No provider crate reads env vars or files directly for tokens. Each
crate exposes a `Config` struct that takes already-resolved secrets:

```rust
pub struct SlackConfig {
    pub bot_token:      SecretString,   // re-exported from starter-spi,
    pub signing_secret: SecretString,   // which re-exports secrecy::SecretString
}
```

`SecretString` is `secrecy::SecretString`, re-exported from
`starter-spi` so provider crates don't pick up `secrecy` directly.
The consumer resolves secrets via `starter-secrets-keyring` or
`starter-secrets-file` (or hardcodes them in dev) and hands the
config to the tool. Reason: a product migrating from
file-secrets-in-dev to keyring-secrets-in-prod must not have to
touch every provider crate.

### R6 — Same `Authenticator` gates anything user-facing

When a provider needs to receive HTTP (a webhook receiver,
typically), the crate exposes an `axum::Router<S>` the consumer
merges into `ServerBuilder` exactly like the notes demo's
`notes_router` — same pattern, no new mounting mechanism. That
router either:

- carries its own provider-defined signature verification (Slack
  signing secret, Telegram secret token) as a `tower` middleware
  layer on the provider's routes, **and** is mounted on a public
  path the consumer chooses; or
- is wrapped by the consumer in `with_principal(...)` using the
  same `Authenticator` the rest of the app uses.

Provider crates **never invent a third auth model** for their own
HTTP surface, and never call into `ServerBuilder` themselves —
they hand back a `Router<S>` and the consumer composes.

### R7 — Observability is required, not optional

Every `Tool::invoke` and every `Service::start` emits structured
tracing events with a stable `tool.name` / `service.name` field, and
registers metrics on the prometheus `Registry` the consumer hands in
(via `McpHttpOptions` for tools, via `ServiceContext.metrics` for
services):

- For tools: a latency histogram and an error counter.
- For services: an event-emitted counter, a restart counter, and an
  "is running" gauge.

**Every provider crate depends on `starter-spi` and
`starter-observability`. Nothing else from the starter set is
mandatory.** A provider crate that bypasses the metric helpers and
registers raw prometheus metrics fails review. (Enforcement today is
review-time; a clippy lint may follow once the helper API stabilises.)

### R8 — No transitive vendor SDKs in `starter-spi`

`starter-spi` stays zero-deps-on-providers. It only learns the new
`Service` / `ServiceRegistry` / `ServiceContext` / `ServiceHandle` /
`EventSink` types (and the `secrecy::SecretString` re-export from
R5). Anything specific to Slack/Gmail/Telegram lives in the provider
crate. This preserves the property that `starter-spi` is cheap to
depend on for any crate in the workspace.

### R9 — Service failure is observability-only by default

If a `Service`'s `JoinHandle` resolves to `Err`, the registry:

1. records the error on the service's tracing span,
2. increments the restart counter, and
3. **does not automatically restart the service.**

Auto-restart is a policy decision (backoff, jitter, max attempts,
circuit-breaker) the service implementation owns — typically by
wrapping its own loop in a retry layer. The registry is a lifetime
manager, not a supervisor. A consumer who wants supervised restart
wraps the service themselves or builds a thin
`RestartingService<S: Service>` adapter; the v1 SPI does not bake
this in.

## What lands in `starter-spi`

Exactly this, and no more:

```rust
pub mod service {
    pub trait Service { /* see R2 */ }
    pub trait EventSink { /* see R4 */ }

    #[non_exhaustive]
    pub struct ServiceContext {
        pub metrics:  Arc<prometheus::Registry>,
        pub shutdown: tokio::sync::watch::Receiver<bool>,
        pub sink:     Arc<dyn EventSink>,
    }

    pub struct ServiceHandle {
        pub join: tokio::task::JoinHandle<SpiResult<()>>,
    }

    pub struct ServiceRegistry { /* owns the watch::Sender<bool>;
                                    mirrors ToolRegistry’s shape */ }
}

pub use secrecy::SecretString;  // R5
```

`ServiceContext` is `#[non_exhaustive]`; adding fields is additive.
Removing or retyping a field is a breaking change subject to the
same SemVer discipline as the rest of `starter-spi`.

## What does NOT land in starter

These are explicit non-goals — name them so future contributors
don't paper-mache them on:

- **A dynamic plugin loader.** No `dlopen`, no WASM host, no
  manifest scanner. Cargo-dependency selection is the contract.
- **A shared "integration" type that subsumes Tool + Service.** R2
  forbids this; tools and services have different shapes.
- **An outbox / retry queue / scheduler.** Provider crates may
  retry their own HTTP calls; durable scheduling is a separate
  concern that gets its own crate when a real consumer needs it.
- **A supervisor / auto-restart policy.** See R9.
- **A bundled "all the integrations" crate.** R1 forbids this.
- **A starter-side opinion on which integrations exist.** Provider
  crates can live in this workspace or in a consumer's own
  workspace — `starter-spi` does not care.
- **A cross-crate SemVer lockstep.** Each `starter-tool-*` and
  `starter-service-*` is versioned independently. They pin a
  compatible `starter-spi` major (initially `0.x` like the rest of
  the workspace). When `starter-spi` cuts a new major, provider
  crates migrate on their own cadence; we do not maintain a
  workspace-wide "every provider must be at the same minor."

## First crates to ship (and reference prior art)

> Local reference, NubeDev only. The path below is on the author's
> machine and not portable. External readers: skip to "Suggested
> first three" below.

Mine the codeless workspace
(`/home/user/code/rust/codeless-workspace/codeless/crates/codeless-{slack,telegram,bot-core,tools}`)
for working integration code and bot-core abstractions. **Reuse the
implementations, not the architecture** — codeless predates this
scope doc and bundles concerns differently. Each integration we
lift becomes:

1. `starter-tool-<provider>` — the outbound side (one or more `Tool`
   impls).
2. `starter-service-<provider>` — the inbound side (one `Service`
   impl), only if the provider supports listening.

Suggested first three:

- `starter-tool-slack` + `starter-service-slack` (socket-mode).
- `starter-tool-telegram` + `starter-service-telegram` (long-poll).
- `starter-tool-gmail` (send only at first; Gmail watch lands later).

Each ships with: a README mirroring `examples/notes/README.md`'s
"how it's extended" table; a config struct with `SecretString`
fields; one `register(...)` helper; one integration test against a
mock HTTP server.

## Smoke test for the design

Before merging any provider crate, verify:

1. **No dep leakage via `starter-spi`.** Check `cargo tree -p
   starter-spi --edges normal` against a baseline snapshot committed
   alongside this scope doc
   (`DOCS/tools/scope/starter-spi-deps.baseline.txt`, updated only
   when `starter-spi` itself changes). The provider crate's
   dependencies must not appear in the snapshot.
2. **No special-case wiring.** The provider crate's `Tool` or
   `Service` registers via the same `.register(...)` call that the
   notes demo uses — no new `main.rs` shape, no provider-specific
   helper module under `starter-server`.
3. **Config-guarded construction.** The consumer can flip the
   integration off by changing one config value (env var or config
   file), with the `if`-around-`.register(...)` pattern from R3.
   Disabling does not require a recompile of the consumer binary.
4. **Secrets backend is swappable.** Switching the secrets backend
   (file → keyring) requires zero changes to the provider crate; the
   `Config` struct still compiles, all tests still pass.
5. **Shutdown actually shuts down.** Stopping the `ServiceRegistry`
   causes every `Service`'s `JoinHandle` to resolve within a bounded
   deadline (default 5s). Provider services that don't observe
   `ServiceContext.shutdown` fail this test.

If any of these fails, the design is wrong, not the implementation.

## Decisions

Stage 1 resolved the design points the rules above left implicit. Each
decision names its **revisit trigger** — the observable condition that
should make a future contributor reopen the choice rather than working
around it.

### D1 — `starter-spi` dep baseline is pinned, not regenerated

`DOCS/tools/scope/starter-spi-deps.baseline.txt` is the canonical
transitive-dep list for `starter-spi`. Smoke test 1 (R8) compares the
live `cargo tree -p starter-spi --edges normal` output against this
file. The baseline is **only** updated in the same commit that changes
`starter-spi`'s own direct dependencies — never to silence a
provider-crate leak, and never as a "sync the snapshot" janitorial PR.

Header at the top of the file documents the exact generating command
so the comparison is reproducible across machines.

**Revisit trigger:** the baseline drifts on more than two unrelated PRs
in a release cycle (signals the comparison is too brittle and needs a
canonical normalization step, e.g. dropping `(proc-macro)` markers or
patch versions), **or** a provider crate's deps slip into the baseline
without anyone noticing (signals the smoke test isn't actually wired
into CI).

### D2 — Blanket `EventSink for broadcast::Sender<Event>` ships in `starter-spi` behind a default-on `broadcast` feature

The blanket impl lives in `starter-spi::service::broadcast` and is
gated on a `broadcast` Cargo feature that is **on by default**. Rationale:

- Default-on means the happy-path consumer (`cargo add starter-spi`)
  gets the `broadcast::Sender<Event>` ergonomics with zero ceremony, so
  provider crates can document `let (tx, _rx) = broadcast::channel(…);
  registry.context_with_sink(Arc::new(tx))` as the canonical pattern.
- Opt-out exists because R8 keeps `starter-spi` cheap to depend on. A
  minimal embedded consumer that doesn't want `tokio::sync::broadcast`
  in its tree disables `default-features` and still gets the `Service`
  / `EventSink` trait machinery — they just bring their own sink impl
  (e.g. an `mpsc::Sender`-backed one).
- The impl lives next to the `EventSink` trait it implements, not in
  a fan-out helper crate or each provider, so there is exactly one
  implementation to audit for back-pressure semantics (see D4).

The `EventSink` trait itself, the `Vec<Arc<dyn EventSink>>` fan-out
helper, and the `Saturated` error variant (D4) live in the **default**
module, not behind the feature — only the broadcast blanket impl is
gated.

**Revisit trigger:** more than one provider crate ends up needing to
turn `default-features = false` on `starter-spi` (signals the feature
is mis-defaulted), **or** a consumer reports the blanket impl picking
up the wrong type inference in their tree (signals the impl should
move out into an explicit newtype).

### D3 — `ServiceRegistry` owns the shutdown deadline contract

`starter-spi::service::ServiceRegistry` exposes:

```rust
pub const SHUTDOWN_DEADLINE_DEFAULT: Duration = Duration::from_secs(5);

impl ServiceRegistry {
    pub async fn shutdown(&self) -> ShutdownReport {
        self.shutdown_with_deadline(SHUTDOWN_DEADLINE_DEFAULT).await
    }
    pub async fn shutdown_with_deadline(&self, deadline: Duration) -> ShutdownReport { … }
}
```

The constant is the **contract**: smoke test 5 (R9 / SCOPE Smoke
test 5) asserts against `ServiceRegistry::SHUTDOWN_DEADLINE_DEFAULT`
rather than hard-coding `5s` in the test, so changing the default is a
one-line edit visible in SemVer review. `shutdown()` is the
zero-argument convenience; consumers who need a different deadline
(e.g. a long-running batch service that flushes on exit) call
`shutdown_with_deadline(...)` explicitly.

`ShutdownReport` records, per service: whether it exited cleanly, the
join error if any, and whether it was force-aborted at the deadline.
Services that ignore `ServiceContext.shutdown` and get aborted show up
in the report — smoke test 5 fails the build for any such service.

**Revisit trigger:** a real provider service legitimately needs more
than 5s to drain (signals the default is too short and we should
either lengthen it or split into `drain_deadline` +
`abort_deadline`), **or** a consumer reports `ShutdownReport` is
unergonomic to log/inspect (signals the shape needs to grow a
`Display` or be replaced with a typed event stream).

### D4 — `EventSink::emit` returns `SpiResult<()>` with a typed `SinkError`; fan-out logs-and-continues but `Saturated` bubbles

```rust
#[derive(Debug, thiserror::Error)]
pub enum SinkError {
    /// The sink's buffer / channel is full. Drop policy is the caller's;
    /// the broadcast blanket impl raises this rather than silently
    /// dropping the event.
    #[error("sink saturated: {kind}")]
    Saturated { kind: &'static str },

    /// The sink is permanently closed (receiver dropped, task gone).
    #[error("sink closed: {kind}")]
    Closed { kind: &'static str },

    /// Anything else — serde failure, downstream I/O, etc.
    #[error("sink failed: {0}")]
    Other(#[source] Box<dyn std::error::Error + Send + Sync>),
}

#[async_trait]
pub trait EventSink: Send + Sync + 'static {
    async fn emit(&self, kind: &str, payload: serde_json::Value)
        -> SpiResult<(), SinkError>;
}
```

`SpiResult<T, E>` is the existing `starter-spi` result alias; if it is
single-parameter today, this decision extends it (or adds a sibling
`SpiSinkResult<()>`)—Stage 3 makes the call when the trait actually
lands. Either way, the public surface is a `Result<(), SinkError>`.

The `Vec<Arc<dyn EventSink>>` **fan-out helper**:

- iterates sinks, calls `emit` on each, and **logs and continues** on
  any individual sink's `Err` so one broken downstream does not gag
  the rest;
- **but** if any sink returns `SinkError::Saturated`, the helper
  bubbles a `Saturated` back to the service caller after the fan-out
  completes. Rationale: a slow sink silently dropping events is the
  exact failure mode R7's "event-emitted counter" cannot catch — the
  service needs to know it is losing data so it can apply
  back-pressure on its own upstream (e.g. stop ack'ing Slack events,
  pause the long-poll loop) rather than reading from the network
  faster than the consumer can absorb.
- `SinkError::Closed` is logged-and-skipped, **not** bubbled — a
  closed sink is a normal shutdown race, not a back-pressure signal.

The broadcast blanket impl from D2 maps:

- `broadcast::Sender::send` returning `Err(SendError(_))` →
  `SinkError::Saturated { kind }` when the channel is full
  (`tokio::sync::broadcast` overwrites the oldest message when slow
  receivers lag; the blanket impl treats lag-overwrite as saturation
  for the same reason).
- All-receivers-dropped → `SinkError::Closed { kind }`.

**Revisit trigger:** a real consumer reports `Saturated` is too noisy
(every Slack burst trips it) and they'd rather drop, signalling the
default policy should flip to log-and-drop with an opt-in
"strict" mode; **or** sinks grow a second method (`emit_batch`)
because per-event `async` overhead dominates, signalling the trait
needs a wider v2.

## Decisions log

- 2026-05-19 — D1–D4 recorded (Stage 1 of the
  starter-tool-* / starter-service-* rollout). No code in
  `starter-spi` yet; decisions are the input to Stage 2.

