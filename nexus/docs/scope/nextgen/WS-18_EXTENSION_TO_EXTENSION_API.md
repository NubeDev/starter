# WS-18 — Extension-to-Extension Access: the event bus + a synchronous peer-call surface

> Status: **IMPLEMENTED** (Wave B + Wave A publish; Wave A subscribe stream-back
> transport is the one documented follow-up). See §10 for what shipped.
> (Original design below kept for context.)
> Extends [WS-14 §4.3 Capability host-methods](WS-14_EXTENSIONS_RUNTIME.md) and
> sits beside [WS-17 Extension Data Access](WS-17_EXTENSION_DATA_ACCESS.md)
> (host/DB/datasource access). WS-17 made an extension a first-class **data**
> citizen; WS-18 makes it a first-class **peer** citizen.
> Relates to [WS-10 Kinds](WS-10_KINDS_EXTENSIBILITY.md).

---

## 1. The idea in one paragraph

Today a nexus extension can talk to the **host** (warehouse, datasources, authz,
dashboards, ingest — WS-17) but it **cannot talk to another extension**. There is
no synchronous "call extension B's tool from extension A's node body," and the one
async channel that was *designed* for cross-extension coordination — the
`event_bus` capability — exists only as SPI wire types and is **not routed in
nexus**. This workstream closes both halves: (1) **finish the event bus** (wire
the `event_bus.*` host methods + the subscribe handle) as the sanctioned *async,
fire-and-forget* peer channel, and (2) add a **gated synchronous `extension.call`
host method** so extension A can invoke a tool/node another extension explicitly
**`provides`** — under a default-deny grant, tenant-scoped, never broader than the
caller's own grants.

---

## 2. What already exists — evidence

**Host → extension data access is solved (WS-17).** The host-method router
(`nexus-api/src/extensions/host_methods.rs:133-158`) routes `authz.check`,
`dashboard.read`, `warehouse.query/.write/.update/.delete`, `ingest.write`,
`datasource.query/.execute`. All tenant-scoped, all capability-gated. **None of
them reaches another extension.**

**The event bus is designed but not wired:**
- `Capability::EventBus { publish, subscribe }` exists
  (`starter-ext-spi/src/capability.rs:113-124`) — two reverse-DNS topic
  allowlists; supervisor enforces publish-side namespace ownership "the same way
  it enforces tool ids."
- Wire types exist (`starter-ext-spi/src/event_bus.rs`): `EventBusMessage`,
  `EventBusPublishRequest/Response`, `EventBusSubscribeRequest`. The doc-comment
  there is explicit: *"the v1 handle only exposes `publish`"* and the subscribe
  handle is a **follow-up** that returns a `BoxStream<EventBusMessage>` over the
  existing `stream.event` / `stream.end` / `stream.cancel` notifications.
- **Gap:** `host_methods.rs` routes no `event_bus.publish` and no
  `event_bus.subscribe`. The capability can be granted in a manifest but a body
  calling `ctx.event_bus().publish(...)` has nowhere to land in nexus.

**`requires:` does NOT name a peer extension.** `Manifest::requires`
(`starter-ext-spi/src/manifest.rs:60-61,95-125`) is a list of **host interface**
dependencies (`starter.spi.tool`, JS singletons like
`@nube/starter-ui-core/preferences`). There is **no `depends_on: [extension_id]`**
and no `provides:` surface another extension can target.

**Indirect template sharing is the only thing that smells cross-extension, and
it isn't a peer call.** `warehouse.query` (`host_methods.rs:251-291`) resolves a
contributed query-kind from `state.extension_kinds` regardless of which extension
contributed it — but the *caller* talks to the **host**, read-only, tenant-clamped;
it never invokes the contributing extension's process.

**Net:** extensions are isolated peers. No sync call path, no shared common-API
surface, and the async channel is half-built.

---

## 3. The actual gaps — evidence

| Capability | Designed in SPI? | Wired in nexus? | Gap |
|---|---|---|---|
| Async publish (`event_bus.publish`) | ✅ types only | ❌ | no host method; supervisor topic-ownership gate not enforced at the host |
| Async subscribe (`event_bus.subscribe`) | ⚠️ request type only, handle deferred | ❌ | no host method; no `BoxStream` subscribe handle in the SDK |
| Sync peer call (`extension.call`) | ❌ | ❌ | no host method, no `provides:` manifest surface, no `requires.extensions` allowlist, tool registry not exposed to bodies |
| Peer dependency declaration | ❌ | ❌ | `requires:` names host interfaces only |
| Peer discovery (list peers + their provided surface) | ❌ | ❌ | `ExtensionRegistry` is host-side; no read-only projection for a body |

---

## 4. Proposed design

Two channels, deliberately different shapes. Pick by coupling: **bus for
fan-out/decoupled coordination, call for a contracted request/response.**

### 4.1 Wave A — finish the event bus (async, fire-and-forget)

The bus is the lower-risk, already-designed half. It is **broadcast**, payload is
**opaque** (the host never inspects it), and there is **no reply** — so it can
never become an ambient-authority backdoor.

**4.1.1 `event_bus.publish` host method.** Route it in `host_methods.rs` beside
the WS-17 methods, gated by the `event_bus` capability:
- Params: `EventBusPublishRequest { topic, payload }`.
- The supervisor gate enforces **publish-side namespace ownership** — the topic
  must be in the caller's grant `publish: […]` allowlist **and** owned by the
  caller's extension id (reverse-DNS prefix match, same rule as tool ids). A topic
  not owned → `Error::Capability`, even if the allowlist lists it.
- Host stamps `ts_unix_ms` so every subscriber sees one value regardless of
  fan-out path.
- Fan-out to all current subscribers of `topic`; publish never blocks on slow
  subscribers (bounded per-subscriber queue; drop-oldest + a dropped-count metric
  rather than back-pressuring the publisher).

**4.1.2 `event_bus.subscribe` host method + SDK handle.** The deferred v2:
- Params: `EventBusSubscribeRequest { topic }` (supports a trailing `*` wildcard
  segment, per the SPI test `com.acme.charts.*`).
- Gated by the grant `subscribe: […]` allowlist. Subscription is **open across
  namespaces** (the whole point of the bus) but bounded by the allowlist.
- Returns a stream over the existing `stream.event` / `stream.end` /
  `stream.cancel` JSON-RPC notifications — `EventBusHandle::subscribe(topic) ->
  BoxStream<EventBusMessage>` (the SPI doc-comment already specifies this shape).
- Lifecycle: the subscription dies with the calling node/tool invocation (or the
  child); the host drops the fan-out entry on `stream.cancel` or child exit.

**4.1.3 Tenant scoping.** A subscriber receives only messages published **within
its own tenant**. The host tags each fan-out entry with the subscriber's
`tenant_id` and only delivers same-tenant messages — cross-tenant leakage through
the bus is a hard no. (Topics are reverse-DNS *namespaces*, not tenants; tenant
isolation is enforced by the host independently of topic.)

### 4.2 Wave B — synchronous peer call (`extension.call`)

The higher-risk half: a real request/response into another extension's process.
Gated three ways — **provides** (callee opts in), **requires** (caller declares
the dependency), and **operator grant** (capability).

**4.2.1 The callee opts in — `contributes.provides[]`.** A new manifest surface:
an extension explicitly publishes the tools/nodes other extensions may call.
```yaml
contributes:
  provides:                              # the callee's public peer-API
    - id: com.acme.geocode.lookup        # a tool/node id this ext already contributes
      input_schema: schemas/lookup_in.json
      output_schema: schemas/lookup_out.json
```
Only ids listed here are reachable via `extension.call`. A contributed tool not in
`provides[]` is private — the callee's normal surface (MCP/REST), not its peer
surface. (No implicit "all my tools are callable.")

**4.2.2 The caller declares the dependency — `requires.extensions[]`.** Extend the
`requires` surface (or add a sibling) so the operator sees the peer edge at
install time without parsing bodies:
```yaml
requires:
  extensions:
    - id: com.acme.geocode
      provides: [com.acme.geocode.lookup]   # the specific provided ids it will call
      version: "^1"
```
Load fails fast (clear error) if `com.acme.geocode` is absent/disabled or does not
`provide` `lookup` — a missing peer is a manifest error, not a runtime surprise.

**4.2.3 The operator grants it — `Capability::Extension`.** A new typed capability
(mirrors WS-17's `Capability::Datasource` shape):
```yaml
capabilities:
  - kind: extension
    targets:                              # allowlist of "<ext_id>:<provided_id>"
      - com.acme.geocode:com.acme.geocode.lookup
```
Empty `targets: []` is the legal neutralised form (ext loads, every peer call
denied). **The WS-14 §4.3 invariant holds: the call runs with the *caller's*
identity and is never broader than the caller's own grants** — the callee cannot
launder authority on the caller's behalf.

**4.2.4 `extension.call` host method.** Route in `host_methods.rs`:
- Params: `{ extension_id, provided_id, input }`.
- Host validates: target is in the caller's `extension` capability `targets`
  allowlist **and** in the caller's `requires.extensions[…].provides` **and** the
  callee `provides` it. Triple-check; any miss → `Error::Capability`.
- Resolve the callee via `AppState.extensions` (the `Arc<ExtensionRegistry>`
  WS-17 already added to `AppState`), dispatch into the callee's process as a
  tool/node invocation, propagating the **caller's** `tenant_id` / `team_ids` (the
  callee runs under the *caller's* identity, not its own).
- Validate `input`/`output` against the `provides[].*_schema` if declared.
- Guard rails: a **timeout** (callee can't hang the caller); **no re-entrancy
  loops** — detect and reject a call cycle (`A→B→A`) with a depth cap +
  cycle-detection on the call stack; the callee **cannot** itself `extension.call`
  back into the caller within the same chain unless its own grants independently
  allow it (and the cycle guard still applies).

### 4.3 Capability model (both waves)

Extend the `Capability` enum (`starter-ext-spi/src/capability.rs`):
- `EventBus { publish, subscribe }` (**exists**) — now actually enforced at a
  routed `event_bus.publish` / `event_bus.subscribe` host method.
- `Extension { targets: Vec<String> }` (**new**) — `<ext_id>:<provided_id>`
  allowlist for `extension.call`. Empty = neutralised.

And the manifest (`starter-ext-spi/src/manifest.rs`):
- `contributes.provides[]` (**new**) — the callee's public peer-API.
- `requires.extensions[]` (**new**) — the caller's declared peer dependencies,
  surfaced to the operator at install.

---

## 5. What an extension looks like (the deliverable shape)

**Callee** (`com.acme.geocode/block.yaml`) — publishes one provided tool:
```yaml
contributes:
  tools:
    - id: com.acme.geocode.lookup
  provides:
    - { id: com.acme.geocode.lookup, input_schema: schemas/in.json, output_schema: schemas/out.json }
```

**Caller** (`com.acme.sites/block.yaml`) — declares + is granted the peer edge:
```yaml
requires:
  extensions:
    - { id: com.acme.geocode, provides: [com.acme.geocode.lookup], version: "^1" }
capabilities:
  - kind: extension
    targets: [com.acme.geocode:com.acme.geocode.lookup]
  - kind: event_bus
    publish:   [com.acme.sites.selection]      # owns this namespace
    subscribe: [com.acme.geocode.refreshed]    # listens to a peer's topic
```

**Caller node body (SDK):**
```rust
// sync request/response into a peer (runs under THIS caller's tenant/identity)
let geo = ctx.extension("com.acme.geocode")
    .call("com.acme.geocode.lookup", json!({ "addr": addr }))?;

// async fan-out — fire and forget, no reply
ctx.event_bus().publish("com.acme.sites.selection", json!({ "site_id": id }))?;

// async subscribe — react to a peer's stream
let mut sub = ctx.event_bus().subscribe("com.acme.geocode.refreshed")?;
while let Some(msg) = sub.next().await { /* re-render */ }
```

---

## 6. Scope (this workstream)

**Wave A — event bus (async):**
1. `event_bus.publish` host method — supervisor topic-ownership gate, host
   `ts_unix_ms` stamp, non-blocking same-tenant fan-out (§4.1.1).
2. `event_bus.subscribe` host method + `EventBusHandle::subscribe -> BoxStream`
   SDK handle over `stream.event/.end/.cancel` (§4.1.2).
3. Tenant-scoped delivery; per-subscriber bounded queue + dropped-count metric
   (§4.1.3).

**Wave B — synchronous peer call:**
4. `contributes.provides[]` manifest surface + loader validation (§4.2.1).
5. `requires.extensions[]` manifest surface + fail-fast resolution at load
   (§4.2.2).
6. `Capability::Extension { targets }` + supervisor gate (§4.2.3).
7. `extension.call` host method — triple-gate, caller-identity propagation,
   schema validation, timeout + cycle guard (§4.2.4).
8. SDK `ctx.extension(id).call(...)` + `ctx.event_bus().publish/subscribe(...)`
   handles wired through every adapter's stub.

**Demo (proves both):**
9. `com.acme.geocode` `provides` a `lookup`; `com.acme.sites` `requires` + is
   granted it, **calls** it synchronously from a node, and **subscribes** to
   `com.acme.geocode.refreshed` to re-render — with a denied-call and a
   wrong-tenant-bus-message both proven to be refused.

## 7. Acceptance criteria

- [ ] `event_bus.publish` on an **owned** topic fans out to same-tenant
      subscribers; publishing on an **unowned** topic (even if allowlisted) is
      refused with a capability error.
- [ ] `event_bus.subscribe` delivers only same-tenant messages; a wildcard
      (`com.acme.charts.*`) matches child topics; cancel/child-exit drops the
      subscription.
- [ ] A slow subscriber never blocks the publisher; drops are counted in a
      metric, not silently lost.
- [ ] `extension.call` succeeds only when the target is in **all three** of the
      callee `provides`, caller `requires.extensions`, and the `extension`
      capability `targets` — a miss on any one is a hard deny.
- [ ] The callee runs under the **caller's** tenant/identity; it cannot reach data
      the caller couldn't (WS-14 §4.3 invariant proven with a scoped query).
- [ ] A call cycle (`A→B→A`) is rejected by the cycle guard; a hung callee trips
      the timeout without hanging the caller.
- [ ] A bundle declaring `requires.extensions[]` against a missing/disabled peer
      fails to load with a clear operator-visible error.
- [ ] Existing nexus tests stay green; `event_bus`-granted bundles that never call
      the bus still load (neutralised-grant parity).

## 8. Out of scope (defer / hand off)

- **Cross-tenant bus or cross-tenant peer call** — tenant isolation is absolute;
  any cross-tenant coordination is a separate, deliberately-designed workstream.
- **Versioned peer-API negotiation beyond semver `requires`** — start with a
  semver pin + `provides` id match; richer contract evolution (deprecation
  windows, multi-version `provides`) is a follow-up.
- **A network/RPC transport for *remote* extensions** — WS-18 is **in-process**
  peers in one nexus, mirroring the in-process event bus. Distributed extensions
  are not in scope.
- **Implicit "all tools callable"** — peer reachability is opt-in via
  `provides[]`; there is no blanket peer-call surface.
- **Frontend↔extension bus** — the SPI mentions `extension↔frontend` coordination
  for the bus; this WS lands the **extension↔extension** half. The frontend leg
  (a federation-side subscribe) is a sibling FE workstream.

## 9. Open questions to settle first

- **Q1 — bus delivery semantics.** At-most-once, drop-oldest on a full
  per-subscriber queue (recommended — matches a live-update/cross-filter
  transport where the latest value wins). Confirm vs. any need for replay/durable
  topics (which would push this toward a real broker — out of scope here).
- **Q2 — `requires.extensions[]` placement.** A sub-field of the existing
  `requires` (which currently holds **host interfaces** and JS singletons), or a
  sibling top-level `requires_extensions`? Leaning: a distinct shape so the
  operator UI can render "peer extensions" separately from "host interfaces."
- **Q3 — caller-identity vs. callee-identity for `extension.call`.** Recommended:
  the callee runs under the **caller's** identity (no privilege laundering, WS-14
  §4.3). The alternative (callee runs as itself, like a microservice) is more
  powerful but breaks the "never broader than the caller's grants" invariant —
  reject unless a concrete need appears.
- **Q4 — `provides` id == contributed tool/node id, or a separate namespace?**
  Lean: reuse the contributed tool/node id (already namespace-owned by the
  extension), with `provides[]` simply marking which are public. A separate
  peer-id namespace adds indirection for no clear gain.
- **Q5 — cycle/timeout policy.** A fixed call-depth cap + per-call timeout, or
  operator-tunable via `supervision:`? Lean: fixed sane defaults first, tunable
  later only if a real workload needs it.

---

## 10. What shipped (implementation notes)

**Decisions taken** (the §9 open questions, resolved as recommended):
- **Q1** — bus delivery is **at-most-once, drop-oldest**: each `(tenant, topic)`
  is a `tokio::sync::broadcast` channel (capacity 256); a lagging subscriber
  loses the oldest messages rather than back-pressuring the publisher.
- **Q2** — `requires_extensions[]` is a **distinct top-level manifest field**
  (not folded into `requires`), so the operator surface renders peer edges
  separately from host interfaces.
- **Q3** — the callee runs under the **caller's** identity: `extension.call`
  forwards the caller's `CallerIdentity` (`call_as`), never the callee's.
- **Q4** — a `provides[]` id **is** a contributed tool id or node kind; the
  loader rejects a provided id that references neither.
- **Q5** — a fixed per-call **timeout** (30 s) bounds a hung callee and, since
  each hop consumes its own window, bounds runaway chains. A depth-based cycle
  guard threaded through `_meta` is the remaining follow-up (see below).

**SPI** (`starter-ext-spi`):
- `Capability::Extension { targets }`; `contributes.provides[]`
  (`ContributeProvides`); `Manifest.requires_extensions[]` (`RequireExtension`);
  wire DTOs `extension::ExtensionCall{Request,Response}` and the already-present
  `event_bus` DTOs (publish/subscribe + `EventBusMessage`).
- `starter-ext-host::validate`: `provides[]` ids must be owned + reference a real
  tool/node; `requires_extensions[].provides[]` must be owned by the named peer.

**Supervisor** (`starter-ext-supervisor`): the `extension` capability category
(gates `extension.call`) and `event_bus` (gates `event_bus.publish/.subscribe`).

**SDK** (`starter-ext-sdk`): `ctx.extension_call().call(...)`
(`RealExtensionCallBackend`) and `ctx.event_bus().publish(...)` are live on the
process flavour; `requires!{extension}` / `requires!{event_bus}` expose them.
`ctx.event_bus().subscribe(...)` returns an `mpsc::Receiver<EventBusMessage>`;
its backend default-errors until the stream-back transport lands. All adapter
stubs (mcp/grpc/cli/workers/rest/wasm) carry the new backend.

**Nexus host** (`nexus-api/src/extensions/`):
- `event_bus.rs` — `ExtensionEventBus`: tenant+topic-keyed `broadcast`, wildcard
  match, lazy prune, on `AppState`.
- `peer.rs` — `PeerSupervisors` write-once registry (filled by `boot` after
  supervisors spawn, shared with `AppState` via `Arc`) + the `extension.call`
  host method: the **triple-gate** (`check_gates`: caller grant + caller
  declaration + callee opt-in) then `call_as` into the callee's child carrying
  the caller's identity.
- `host_methods.rs` — `event_bus.publish` (topic-ownership + publish-allowlist
  gates, tenant-scoped fan-out) and `event_bus.subscribe` (clear capability error
  pending the stream-back transport) and the `extension.call` arm.

**Demo** (`nexus/extensions/`): `com.acme.geocode` **provides** `lookup` (callee);
`com.acme.sites` declares `requires_extensions` + the `extension`/`event_bus`
grants, **calls** the geocode peer via `ctx.extension_call()`, then **publishes**
the registered site on `com.acme.sites.registered`.

**Tests**: SPI capability/DTO round-trips; host `validate` provides/requires
ownership; supervisor gate for both categories; nexus `event_bus` (exact +
wildcard delivery, tenant isolation, prune) and `extension.call` triple-gate;
an integration test asserting both demo manifests parse + validate.

**Out / follow-up**: the `event_bus.subscribe` **stream-back transport** (push
`stream.event` to the subscribing child over the invocation lifetime) — the bus
and host method support subscriptions; the SDK handle and host method return a
clear capability error until that transport lands. A depth-based **cycle guard**
for `extension.call` (beyond the timeout bound) is the other follow-up.
