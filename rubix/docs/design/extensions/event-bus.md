# Event bus capability

> **Tier:** design (canonical). Lifetime: durable. Describes the
> shipped row-4 capability of the extensions north-star scope.
> Plan / "we will" framing lives in
> [docs/scope/extensions-north-star/](../../scope/extensions-north-star/README.md).

## What

The `event_bus` capability gives extensions an in-process
publish/subscribe surface for cross-extension and
extension↔frontend coordination. Per row 4 of the critical
path: this is the **cross-filter / live-update** transport
that replaces N-per-click HTTP loopback round-trips in a
Power-BI-style dashboard surface.

V1 ships **publish** only on the SDK handle. The
subscribe-side SPI wire type (`EventBusSubscribeRequest`)
ships in the same release so its shape is locked in;
`EventBusHandle::subscribe(topic) -> BoxStream<EventBusMessage>`
lands as a follow-up over the existing `stream.event` /
`stream.end` / `stream.cancel` notifications. Splitting the
direction was the only way to keep the row scoped to one
session — the stream-side handle is a meaningful design
surface in its own right (backpressure, late-subscriber
replay, drop semantics).

## Capability grant

```yaml
# block.yaml
capabilities:
  event_bus:
    publish:   [com.acme.charts.filter]
    subscribe: [com.acme.charts.*]
```

Both lists are independent. An empty list on either side is
the legal neutralised form for that direction (matches every
other allowlist capability). A topic in `publish: []`
**cannot** be `.publish()`ed even though the extension
declared the capability; same for `subscribe`.

The wire-method namespace is `event_bus.*`. The supervisor's
capability gate accepts `event_bus.publish` and
`event_bus.subscribe` when the grant is present and refuses
both otherwise.

### Topic namespacing

Topic strings are reverse-DNS. The supervisor enforces
**publish-side** namespace ownership the same way it enforces
tool ids: an extension may publish only on topics it owns
(equal to or a dotted descendant of its extension id).
Subscription is open across namespaces — that's the whole
point of the bus — but mediated by the grant's
`subscribe: [...]` allowlist.

The host-reserved prefixes (`starter.`, `sys.`) are forbidden
on the publish side. A future host-published-topic surface
(e.g. `starter.health.degraded`) ships subscriptions to
those prefixes once a real producer lands; row 4 reserves
the namespace by rule, not by validation code.

## SDK surface

`requires!(event_bus)` brings the `EventBusHandle` accessor
onto the per-extension Ctx:

```rust
starter_ext_sdk::requires! {
    name = ChartsCtx,
    capabilities = [event_bus, warehouse_read, tracing],
}

// inside a handler:
ctx.event_bus().publish(
    "com.acme.charts.filter",
    json!({ "site_id": site_id }),
)?;
```

The v1 method is:

- `publish(topic, payload) -> Result<()>` — broadcast to all
  current subscribers. The host stamps `ts_unix_ms` before
  fan-out so every subscriber sees the same timestamp.

`publish` is sync (`Result<()>`). The host's fan-out is
fire-and-forget from the publisher's perspective — once the
gate accepts the call, the publisher's invocation returns
without waiting for any subscriber to drain.

### Deferred to v2

- **`subscribe(topic) -> BoxStream<EventBusMessage>`.** The SPI
  wire type (`EventBusSubscribeRequest`) ships in v1 so the
  shape locks early. The handle method lands when the
  buffer/replay/backpressure decisions for the host-side
  fan-out are made — those are not trivial and warrant their
  own design session.
- **Topic glob subscription.** Today: exact-match strings on
  both publish and subscribe. Glob matching
  (`com.acme.charts.*`) is a v2 concern; until then the
  example above is illustrative for the eventual API and is
  rejected as an unknown topic by today's supervisor.

## Wire types

```rust
// starter-ext-spi::event_bus
pub struct EventBusMessage {
    pub topic: String,
    pub payload: serde_json::Value,
    pub ts_unix_ms: u64,    // host-stamped on publish
}

pub struct EventBusPublishRequest {
    pub topic: String,
    pub payload: serde_json::Value,
}

pub struct EventBusSubscribeRequest {
    pub topic: String,
}
```

The bus does not validate `payload`. Schema is documented
per-topic by the publishing extension; the supervisor does
not enforce it.

## What is not in row 4

- **Host-side bus implementation.** Row 4 ships the
  capability, the SPI wire types, the supervisor gate, and
  the SDK handle. The real `EventBusBackend` impl
  (`tokio::sync::broadcast` per topic, garbage collection of
  empty topics, subscriber backpressure handling) lands with
  row 5's host-dispatch wiring — the same crate that wires
  `WarehouseReadBackend`. Process-flavour calls today return
  `Error::Capability("event_bus not wired …")`.
- **Subscribe handle.** See "deferred to v2" above.
- **Cross-process bus.** The "in-process" qualifier is load-
  bearing. A multi-host bus is a different design (distributed
  delivery, message ordering, durable storage) and out of
  scope for the north-star.
- **At-most-once vs. at-least-once.** Today the host fan-out
  is best-effort — a subscriber whose channel is full **drops**
  the message rather than blocking the publisher. The choice
  is encoded in the deferred subscribe handle; row 4 doesn't
  pin it down.

## Tests

- `starter-ext-spi` (`event_bus.rs`): `EventBusMessage`,
  `EventBusPublishRequest`, `EventBusSubscribeRequest`
  round-trip.
- `starter-ext-spi` (`capability.rs`): `EventBus` variant
  round-trip; empty-lists-are-legal.
- `starter-ext-supervisor` (`capability.rs`):
  `event_bus.publish` and `event_bus.subscribe` gated under
  the `event_bus` grant; refused without it.
- Every adapter's stub backend returns
  `Error::Capability("event_bus not wired …")` until the
  real backend lands.
