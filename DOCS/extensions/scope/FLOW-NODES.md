# Hot-loadable flow node kinds via extensions

Companion to [`SCOPE.md`](./SCOPE.md) (extensions framework) and
[`../../flow/scope/SCOPE.md`](../../flow/scope/SCOPE.md) (flow engine).

This doc covers **one** new contribution kind on the extensions
manifest — `contributes.nodes` — and the work needed across the
extensions kernel, the flow SPI, the supervisor, the flow engine, and
`examples/flow-agent` to make it land. Concrete driver: a user drops
an `mqtt` extension bundle into `EXTENSIONS_DIR/` and an `mqtt-publish`
node appears in the flow editor palette without rebuilding the host.

## One-line summary

An extension declares one or more **flow node kinds** in its
`block.yaml`. The kernel (manifest + loader + supervisor) routes
`NodeBehavior::invoke` calls to the extension's child process over the
**same stdio JSON-RPC channel** the kernel already runs for
`stream.event` / `health` / `init`. The host's flow engine sees a
`Arc<dyn NodeBehavior>` proxy that forwards every invocation across
the wire. The flow editor's palette is driven by the host's
`NodeKindRegistry`, so new kinds appear automatically when the
operator triggers a reload (`POST /admin/extensions/reload`; CLI
wrapper TBD).

One trait (`NodeBehavior`), one wire format (existing JSON-RPC), one
manifest field (`contributes.nodes`), one adapter
(`starter-ext-flow`), one new SPI method (`flow.node.invoke`).

## Why this exists

The parent SCOPE.md (extensions) commits the framework to surfacing
one trait across every transport via adapter crates (R13). The flow
engine commits to "everything is a node" with extension-contributed
kinds being a first-class case (flow SCOPE R11, agent-SCOPE
supersession of R7). Today the wire exists for *tools*, *cli*, *rest*,
*workers*, and *skills*; **`contributes.nodes` is the missing column
in R13's table**. Writing it down here so the implementation lands as
one cohesive slice rather than three half-wired ones.

The trigger for landing this now is the user demand for an MQTT
extension hot-loaded into [`examples/flow-agent`](../../../examples/flow-agent)
without recompiling `flow-agent` for each new integration. Builtin
flavour cannot deliver that property by construction (statically
linked); WASI-p2 cannot deliver it for a long-lived MQTT client
without leaving the WASI-p2 capability surface. Process flavour over
stdio JSON-RPC delivers it with the kernel that already exists.

## Hard rules (load-bearing)

These rules complete the picture the parent SCOPEs draw. Numbered
`R-flow-node-N` so they cite cleanly from code and review comments
without colliding with the parent `R1..R13`.

### R-flow-node-1 — One contribution kind, one adapter, one new wire method

`contributes.nodes:` adds **one** new field on
[`Contributes`](../../../starter-extensions/crates/starter-ext-spi/src/manifest.rs)
and **one** new adapter walker in
[`starter-ext-flow`](../../../starter-extensions/crates/starter-ext-flow/src/lib.rs)
(alongside the existing `contributed_skills(...)`). The adapter
returns descriptors + proxy behaviours the host registers into its
[`NodeKindRegistry`](../../../crates/starter-flow-spi/src/node.rs).

The wire vocabulary grows by **exactly one** request method,
`flow.node.invoke`, plus reuse of the existing four
`stream.*` notifications for streaming nodes (no new streaming shape).
Cancellation reuses `stream.cancel`. Lifecycle uses the existing
`init` / `health` / shutdown — nodes are *behaviours*, not separate
processes, so they live inside the extension's existing child.

A new transport adapter (websocket, GraphQL, future) does **not**
need to know about `contributes.nodes` — the kernel already serves
node dispatch through `flow.node.invoke`; new transports surface
flows-as-tools / flows-as-services through the existing
`starter-flow-surfaces` crate (flow SCOPE R8).

### R-flow-node-2 — Descriptors are runtime-owned, not `&'static`

[`StaticNodeKindRegistry`](../../../crates/starter-flow-nodes/src/node_registry.rs)
holds `&'static NodeDescriptor` because built-in kinds are
compile-time constants. Extension-contributed descriptors are read
from YAML at load time, so `&'static` is wrong for them.

**Resolution:** widen `NodeDescriptor`'s string fields from
`&'static str` to `Cow<'static, str>` and add `DynamicNodeKindRegistry`
in `starter-flow-spi` that owns owned descriptors plus a factory
`Arc<dyn NodeBehavior>` per kind. A `CompositeNodeKindRegistry`
defers to `Static` for built-ins, then `Dynamic` for extensions.

`Cow<'static, str>` lets the existing `const`-constructible built-in
descriptors keep their zero-allocation form (`Cow::Borrowed("...")`)
while extension descriptors own their strings (`Cow::Owned(String)`).
Dropping the registry drops the owned strings — no `Box::leak`, no
self-referential dance, no `'static` lie.

All existing `NodeDescriptor::new(...)` call sites in
`starter-flow-nodes` change in one mechanical pass (`Cow::Borrowed`
around each `&'static str`); the public API stays
source-compatible via a `const fn new(...)` that takes
`&'static str` and wraps internally.

### R-flow-node-3 — Reverse-DNS namespace ownership composes across frameworks

A `contributes.nodes[].kind` id is a [`KindId`](../../../crates/starter-flow-spi/src/node.rs)
and **must** be the extension id or a dotted descendant
(extensions SCOPE R4 applied to flow ids). The loader rejects an
extension that contributes `starter.flow.mqtt` (reserved
`starter.*` prefix) or `com.other-vendor.mqtt`. This is enforced
once in `starter-ext-host::validate` against the same reverse-DNS
walker the existing R4 check uses, with a small per-`contributes.*`
table mapping field → expected id type so future additions reuse one
walker.

This rule is what keeps the catalog flat. Two extensions can both
ship `mqtt.publish` only by way of each declaring it under its own
extension id (`com.nube.mqtt.publish`, `com.acme.mqtt.publish`);
collisions are impossible by construction.

### R-flow-node-4 — Manifest declares the kind; the child implements the body; nothing else

The manifest entry carries:

```yaml
contributes:
  nodes:
    - kind: com.nube.mqtt.publish
      label_key: com.nube.mqtt.node.publish.label       # i18n key
      summary_key: com.nube.mqtt.node.publish.summary
      help_key: com.nube.mqtt.node.publish.help
      settings_schema: schemas/publish.settings.json    # JSON Schema, static
      input_slots:                                       # static metadata
        - { name: payload, type: bytes }
        - { name: topic,   type: string }
      output_slots:
        - { name: published_at, type: int }
      facets: []                                         # see NodeFacet enum in starter-flow-spi::node
      streaming: false                                   # true ⇒ uses stream.* protocol
      auth: {}                                           # AuthGate { require_role?, require_scope? } — reuses existing per-entry gate
```

Everything else is the child's job over the wire. The host knows
nothing about the kind's body — descriptors are the surface the editor
and registry consume; the body answers `flow.node.invoke` calls.

`deny_unknown_fields` everywhere (extensions R3). Schemas /
descriptions / i18n keys are static files in the bundle (R7). The
host serves them under stable per-kind paths so the editor fetches
them without going through the child; the exact URL shape
(`GET /api/node-kinds/<kind>/{settings_schema,description}` in
`flow-agent`) is an adapter concern, but the descriptor returned by
`GET /api/node-kinds` carries already-resolved absolute URLs so the
frontend never builds these paths itself — the host can move them
without a frontend change.

### R-flow-node-5 — Dispatch reuses `SupervisorHandle::call`

The kernel already has [`SupervisorHandle::call(method, params, timeout)`](../../../starter-extensions/crates/starter-ext-supervisor/src/supervisor.rs)
— synchronous request/response demultiplexer over the bidirectional
stdio channel. A node-kind proxy implementing `NodeBehavior` is a
thin wrapper:

```rust
async fn invoke(&self, ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
    let invocation_id = StreamId(format!("inv-{}", ulid::Ulid::new()));
    let params = serde_json::json!({
        "invocation_id": invocation_id,
        "kind": self.kind_id().as_str(),
        "node": ctx.node.as_str(),
        "run":  ctx.run.to_string(),
        "input": input,
        "deadline_ms": self.timeout.as_millis() as u64,
    });
    let cancel_guard = self.spawn_cancel_forwarder(ctx.cancel, invocation_id.clone());
    let resp = self.handle.call("flow.node.invoke", params, self.timeout).await?;
    drop(cancel_guard);
    serde_json::from_value(resp).map_err(...)
}
```

No new supervisor state machine; no second channel.

**Cancellation is keyed by `invocation_id`, not by the JSON-RPC `id`.**
The JSON-RPC `id` is short-lived (it correlates *one* response) and
is allocated by `SupervisorHandle::call`, invisible to the proxy.
`invocation_id` is the proxy's own handle that lives as long as the
node call, doubles as the `stream_id` for streaming nodes
(R-flow-node-9), and is what `stream.cancel` carries. The child
indexes in-flight invocations by `invocation_id` regardless of
streaming/non-streaming.

**`deadline_ms` is advisory, not authoritative.** It signals the
child's cooperative-cancellation budget so it can short-circuit
long operations without waiting for a `stream.cancel` round trip.
The hard bound is the host-side `SupervisorHandle::call` timeout
(typically equal to `deadline_ms` plus a small grace) — if the
child ignores `deadline_ms`, the host returns
`NodeError::Backend("timeout")` and emits a `stream.cancel` for
the `invocation_id`. A child that does not respect cancellation
gets killed by the supervisor's restart policy on the next health
ping miss.

The `Cancel` token in `NodeCtx` is wired through a forwarder task
(`spawn_cancel_forwarder`) that sends `stream.cancel { stream_id:
invocation_id }` on trip — matches the
[flow SCOPE R13 cancellation seam](../../flow/scope/SCOPE.md).

### R-flow-node-6 — Hot-reload is a registry swap with bounded-blast-radius shutdown

Reload is the single operation that makes extension installation feel
hot. The host exposes (in `flow-agent` and any consumer that opts in):

```
POST /admin/extensions/reload
```

Server-side:

1. Build a new `Loader::scan(EXTENSIONS_DIR)` + `validate_all` +
   `commit` into a fresh `ExtensionRegistry`. This is `O(bundles)`,
   no I/O beyond reading `block.yaml`s.
2. Compute the diff against the live supervisor set:
   **added**, **unchanged** (same id + same manifest content hash),
   **replaced** (same id, different hash), **removed**.
   - **added**: spawn a new `Supervisor` (existing code path).
   - **unchanged**: keep the existing `SupervisorHandle`; proxies
     in the new registry reuse it.
   - **replaced**: spawn the new supervisor; **defer shutdown of
     the old one** until every `ProcessNodeProxy` referencing its
     handle is dropped (see refcount note below).
   - **removed**: enqueue for shutdown with the **same defer**
     rule.
3. Rebuild the `CompositeNodeKindRegistry` from `Static.with_builtins() +
   Dynamic::from_extension_registry(&new_registry)`. New proxies
   for replaced extensions reference the **new** supervisor handle;
   the old handle is reachable only through old proxies still held
   by in-flight runs.
4. Atomically swap the engine's `Arc<dyn NodeKindRegistry>` for the new
   one (`ArcSwap`). In-flight runs hold the previous `Arc` and finish
   against the old proxies; new runs see the new registry.
5. Emit `extensions.reload` on the SSE bus the frontend already
   subscribes to (parallel to `flow-created` / `flow-renamed`) so the
   palette refetches.

**Deferred shutdown mechanism.** Each `SupervisorHandle` already has
an internal `Arc`-shared state. `ProcessNodeProxy` holds a clone, so
`Arc::strong_count` on the handle is a natural drop guard. The
reload path moves the old handle into a small `shutdown_pending`
queue that polls `Arc::strong_count == 1` (only the queue holds it)
before awaiting `handle.shutdown()`. A per-handle grace-window cap
(default 5 min, configurable) bounds the wait — if a run hangs
beyond it, the queue calls `shutdown()` anyway and in-flight
invocations against that handle complete with
`NodeError::Backend("supervisor shut down during reload grace")`.
The grace cap is an operator knob, not a guarantee.

**Guarantee table:**

| Diff bucket | In-flight calls against the old handle |
|-------------|----------------------------------------|
| unchanged   | Complete normally (same handle).       |
| replaced    | Complete normally up to grace cap; then fail with `NodeError::Backend`. |
| removed     | Complete normally up to grace cap; then fail with `NodeError::Backend`. |
| added       | N/A (no prior handle).                 |

**No partial state.** Two-phase commit at the registry boundary plus
`ArcSwap` at the engine boundary mean the kernel either has the old
world or the new world; never both. Old supervisors that the new
world no longer references are reaped only when safe.

### R-flow-node-7 — Settings validation belongs to the engine, not to the kernel

The `settings_schema` file is fetched by the editor and by the
publish-time validator in [`starter-flow`](../../../crates/starter-flow)
(`DefinitionManager::publish`, [`DOCS/flow/scope/hot-reload.md`](../../flow/scope/hot-reload.md)
HR1). The extensions kernel does **not** validate node settings — it
serves the schema file and trusts the engine to gate publication.
This keeps `starter-ext-host` free of `schemars`/`jsonschema`
dependencies and keeps validation in the one crate that already does
it for built-in kinds.

A future override hook (`NodeBehavior::validate_settings` with
cross-field rules) is reachable over the wire as a second optional
method (`flow.node.validate_settings`) that the engine calls only if
the manifest's node entry sets `validates_settings: true`. Not wired
in slice A; documented here so the seam is obvious.

### R-flow-node-8 — Builtin and process flavours are interchangeable from the engine's perspective

The engine reaches kinds through `Arc<dyn NodeBehavior>`. Built-in
kinds construct their behaviour directly
(`Arc::new(Log::new())`); process-flavour kinds construct a
`ProcessNodeProxy { handle, kind, timeout }`. Same trait, same
dispatch path, same observability spans.

This is the parent SCOPE R1 ("one trait, three flavours, one source")
applied to the node-kind contribution. An extension author writes the
same `NodeBehavior` impl either way; `starter-ext-sdk` chooses the
glue at compile time. The host neither knows nor cares which flavour
served a given invocation.

## Wire format

One new request method, no new notifications:

### Request — `flow.node.invoke`

```jsonc
// host → child
{
  "jsonrpc": "2.0",
  "id": 1234567,
  "method": "flow.node.invoke",
  "params": {
    "invocation_id": "inv-01KS9Q...", // proxy-minted; doubles as stream_id
    "kind":  "com.nube.mqtt.publish",
    "node":  "flow-agent.nodes.mqtt-out-1",
    "run":   "01KS9Q...",          // engine RunId
    "input": {                      // SlotMap
      "topic":   { "type": "string", "value": "sensors/lab/temp" },
      "payload": { "type": "bytes",  "value": "..."             }
    },
    "deadline_ms": 5000             // advisory; host enforces hard timeout
  }
}
```

### Success response — tagged enum

The `result` field is a `#[serde(tag = "kind")]`-style discriminated
union. The discriminator is `kind` so a non-streaming response and
a streaming-start response are unambiguous at the deserializer:

```jsonc
// non-streaming success
{
  "jsonrpc": "2.0",
  "id": 1234567,
  "result": {
    "kind": "output",
    "output": {                    // SlotMap
      "published_at": { "type": "int", "value": 1747961234567 }
    }
  }
}
```

```jsonc
// streaming start — only valid for kinds whose descriptor has streaming: true
{
  "jsonrpc": "2.0",
  "id": 1234567,
  "result": {
    "kind": "stream_started",
    "stream_id": "inv-01KS9Q..."   // MUST equal request.params.invocation_id
  }
}
```

A child that returns `"kind": "output"` for a `streaming: true`
descriptor (or vice versa) is a protocol violation; the proxy fails
the call with `NodeError::Backend("protocol violation: ...")`. The
`stream_id` echoing `invocation_id` is what lets the proxy demux
subsequent `stream.event` notifications without a second mapping
table.

### Error response

```jsonc
{
  "jsonrpc": "2.0",
  "id": 1234567,
  "error": {
    "code":    -32050,
    "message": "MQTT publish failed: connection refused",
    "data":    { "kind": "Backend" }   // maps to NodeError variant
  }
}
```

Error-code allocation: `-32050..-32099` is reserved for
`flow.node.invoke` errors. The exact subdivision is defined in
[`starter-ext-spi::jsonrpc`](../../../starter-extensions/crates/starter-ext-spi/src/jsonrpc.rs)
as `pub mod flow_node_error_codes { ... }` alongside the existing
`stream_methods`. This range is below the JSON-RPC reserved server
floor (`-32099`) and above the `stream.*` errors the kernel uses;
adding a new sub-code is a one-line change in that module.

Error mapping table (`data.kind` → `NodeError`):

| `data.kind`         | `NodeError` variant            | `message` source                    |
|---------------------|--------------------------------|-------------------------------------|
| `"InvalidInput"`    | `NodeError::InvalidInput(s)`   | `error.message`                     |
| `"Backend"`         | `NodeError::Backend(s)`        | `error.message`                     |
| `"Cancelled"`       | `NodeError::Cancelled`         | discarded                           |
| `"Domain"`          | `NodeError::Domain { code, message }` | `error.data.code` → `code` (must be a stable interned `&'static str`; proxy interns via `ProcessNodeProxy`'s arena), `error.message` → `message` |
| any other / missing | `NodeError::Other`             | `error.message`                     |

`error.message` is always the human-readable string. `error.data`
carries machine-readable structure; `data.code` is only consulted
for `"Domain"`. Unknown `data.kind` values are downgraded to
`Other` rather than rejected — forwards-compatible with future
`NodeError` variants.

### Streaming nodes

For kinds whose descriptor sets `streaming: true` the initial response
returns `{ "kind": "stream_started", "stream_id": invocation_id }`
and the child emits the existing
[`stream.event` / `stream.end` / `stream.error`](../../../starter-extensions/crates/starter-ext-spi/src/jsonrpc.rs)
notifications tagged with the same `stream_id`. Cancellation is
`stream.cancel { stream_id }` from host → child on `Cancel` trip
(R-flow-node-5). Stream payloads are `SlotMap`s the engine writes
through `GraphStore::write_slot` (flow SCOPE R2).

## File layout — example extension bundle

```
~/.local/share/flow-agent/extensions/com.nube.mqtt/
├── block.yaml
├── bin/
│   └── mqtt-driver            # spawned binary (runtime.bin)
├── schemas/
│   └── publish.settings.json
├── docs/
│   ├── extension.md
│   └── publish.md
└── i18n/
    ├── en.json
    └── es.json
```

`block.yaml`:

```yaml
v: 1
id: com.nube.mqtt
version: 0.1.0
display_name: MQTT
description_file: docs/extension.md
authors: [ "ops@nube.io" ]
requires:
  - { id: starter.flow.node, version: "^1" }
runtime:
  kind: process
  bin:  bin/mqtt-driver
supervision:
  restart: on_crash
  max_restarts: 5
  within_seconds: 60
capabilities:
  - { kind: net_out, allowlist: [ "mqtt://broker.lan:1883" ] }
config_schema: schemas/config.json
config:
  broker_url: "mqtt://broker.lan:1883"
contributes:
  i18n:
    catalogs: { en: i18n/en.json, es: i18n/es.json }
  nodes:
    - kind: com.nube.mqtt.publish
      label_key:   com.nube.mqtt.node.publish.label
      summary_key: com.nube.mqtt.node.publish.summary
      help_key:    com.nube.mqtt.node.publish.help
      settings_schema: schemas/publish.settings.json
      description_file: docs/publish.md
      input_slots:
        - { name: topic,   type: string }
        - { name: payload, type: bytes  }
      output_slots:
        - { name: published_at, type: int }
      facets: []
      streaming: false
    - kind: com.nube.mqtt.subscribe
      label_key:   com.nube.mqtt.node.subscribe.label
      summary_key: com.nube.mqtt.node.subscribe.summary
      help_key:    com.nube.mqtt.node.subscribe.help
      settings_schema: schemas/subscribe.settings.json
      description_file: docs/subscribe.md
      input_slots:
        - { name: topic_filter, type: string }
      output_slots:
        - { name: payload, type: bytes  }
        - { name: topic,   type: string }
      facets: [ IsTrigger ]
      streaming: true
```

## Where each piece lands

### `starter-ext-spi` (kernel contracts)

- `manifest.rs` — add `Contributes::nodes: Vec<ContributeNode>`,
  `ContributeNode { kind, label_key, summary_key, help_key,
  settings_schema, description_file, input_slots, output_slots,
  facets, streaming, auth }` (every field with
  `deny_unknown_fields`; `description_file`, `facets`, `streaming`,
  `auth` all `#[serde(default)]`). `auth: AuthGate` reuses the
  existing per-entry `AuthGate` struct that already governs every
  other `contributes.*` entry — see
  [`AuthGate`](../../../starter-extensions/crates/starter-ext-spi/src/manifest.rs)
  (`require_role`, `require_scope`). The flow-engine adapter on the
  boundary applies the gate before invoking the proxy, same posture
  as `contributes.tools`. Reverse-DNS `kind` validated against
  extension id in `starter-ext-host::validate`.
- `jsonrpc.rs` — add `FLOW_NODE_INVOKE: &str = "flow.node.invoke"`
  and a `flow_node_error_codes` module reserving `-32050..-32099`,
  colocated with `stream_methods::*` so the vocabulary lives in one
  file.

### `starter-flow-spi`

- `node.rs` — add `DynamicNodeKindRegistry` (owned descriptors +
  factory closures) and `CompositeNodeKindRegistry` (delegates first
  to a `Static`, then to a `Dynamic`). Existing
  `NodeKindRegistry` trait unchanged.

### `starter-ext-host`

- `validate.rs` — extend the namespace walker to validate
  `contributes.nodes[].kind` against the extension id (same rule as
  `contributes.tools[].id`).
- No new dep; reuses existing reverse-DNS walker.

### `starter-ext-supervisor`

- No wire change; `SupervisorHandle::call` already serves the
  request/response shape.
- One small addition: a `stream.cancel` helper on the handle so
  proxies don't construct envelopes inline.

### `starter-ext-flow` (the adapter)

- Add `contributed_node_kinds(manifest, extension_root, handle) ->
  Vec<DynamicNodeKindEntry>` mirroring `contributed_skills`. Each
  entry carries the descriptor + an `Arc<dyn NodeBehavior>` built
  from `ProcessNodeProxy::new(handle.clone(), kind, timeout)`.
- For builtin-flavour node-kind extensions (future), the adapter
  takes a `&dyn BuiltinNodeKindFactory` lookup; out of scope for
  slice A.
- New module: `process_proxy.rs` — the `ProcessNodeProxy`
  `NodeBehavior` impl plus its `Cancel` → `stream.cancel` plumbing.

### `examples/flow-agent`

- `flow_engine.rs` — replace the hard-coded
  `match n.kind { "trigger" | "ai-agent" | "log" => ... }` with a
  lookup through `Arc<dyn NodeKindRegistry>`. UI-kind → reverse-DNS
  kind mapping moves into a small `ui_kind_table.rs` so adding
  `mqtt-publish` to the palette is a one-line entry until the editor
  speaks reverse-DNS natively.
- New REST: `GET /api/node-kinds` returns descriptor + fetched
  i18n-resolved label/summary/help + absolute URLs for the
  per-kind settings schema and description (see R-flow-node-4).
  The editor calls this on mount and on `extensions.reload` SSE.
- New REST: `GET /api/node-kinds/<kind>/settings-schema` and
  `GET /api/node-kinds/<kind>/description` serve the static bundle
  files; the URLs in the descriptor point at these routes.
- New REST: `POST /admin/extensions/reload` triggers the reload
  sequence (R-flow-node-6).
- New SSE event: `extensions.reload` published on the existing flows
  bus so the React palette refetches.

### `examples/flow-agent/frontend`

- `state/node-kinds-store.ts` — react-query over `GET
  /api/node-kinds`, invalidated by the `extensions.reload` SSE event.
- `pages/FlowEditor.tsx` — palette reads from the store; the
  per-node settings panel fetches the schema URL the descriptor
  carries and renders the form with the existing JSON-Schema form
  renderer.

### `examples/flow-agent/extensions/com.nube.mqtt/` (new demo bundle)

- `block.yaml` per the example above.
- `bin/mqtt-driver` — small Rust binary using `rumqttc` for the
  MQTT client and `starter-jsonrpc-stdio` for the wire framing. One
  `tokio::main`, one `flow.node.invoke` dispatcher, one persistent
  MQTT connection per child (managed across invocations — extensions
  are stateful processes; nodes are stateless behaviours on top).
- Two markdown description files, one JSON Schema, one i18n catalog
  per language.

## Slicing

The work is two slices that ship independently. Slice A is a
prerequisite for slice B.

### Slice A — manifest + dynamic registry + flow-agent wiring

1. `starter-ext-spi`: `Contributes.nodes` + `ContributeNode`.
2. `starter-ext-host`: namespace validation extension.
3. `starter-flow-spi`: `DynamicNodeKindRegistry` +
   `CompositeNodeKindRegistry`.
4. `starter-ext-flow`: `contributed_node_kinds()` walker that returns
   *descriptors only* (no proxy yet) and a unit-test fixture
   exercising the schema.
5. `flow-agent` backend: registry composition, `GET /api/node-kinds`,
   manual builtin → reverse-DNS mapping.
6. `flow-agent` frontend: palette driven by the registry; no MQTT yet.

**Property at end of slice A:** a `block.yaml` fixture under
`crates/starter-ext-flow/tests/fixtures/` declares a
`contributes.nodes` entry; the `contributed_node_kinds()` walker
turns it into a `DynamicNodeKindEntry`; the composite registry
surfaces the descriptor through `NodeKindRegistry::all()`; and
`flow-agent`'s `GET /api/node-kinds` returns it with the resolved
schema URL. The kind is **not** yet invokable (no proxy), so
attempting to fire a flow that uses it returns a typed
"no behaviour bound" error — enough to prove the dynamic-registry
path end-to-end without the supervisor wire.
### Slice B — process-flavour proxy + supervisor wiring + MQTT demo

7. `starter-ext-spi`: `FLOW_NODE_INVOKE` constant + error-kind table.
8. `starter-ext-flow`: `ProcessNodeProxy` impl, `Cancel`-to-cancel
   wiring, streaming-node support over `stream.event`.
9. `flow-agent`: `POST /admin/extensions/reload`, `ArcSwap` of the
   registry, SSE event publication, supervisor lifecycle on reload.
10. `examples/flow-agent/extensions/com.nube.mqtt/` — full bundle plus
    a small `rumqttc`-backed driver binary.
11. End-to-end test: drop bundle into a tempdir, `POST
    /admin/extensions/reload`, fetch palette, create a flow, fire it,
    assert the published_at slot populated and the MQTT broker (a
    `mosquitto` test container) received the message.

**Property at end of slice B:** `cp -r com.nube.mqtt
$EXTENSIONS_DIR/ && curl -XPOST .../admin/extensions/reload` makes
MQTT nodes appear in the running flow-agent's palette and usable
inside flows — no host rebuild, no restart.

## What this explicitly does **not** add

- **No supervisor groups.** Each extension supervisor stays its own
  subtree. (parent SCOPE R9.)
- **No new wire format.** All node dispatch reuses the existing
  request/response + four `stream.*` notifications.
- **No second `Ctx` API.** Extensions receive the same `Ctx` shape
  today's process-flavour extensions get; node kinds are behaviour
  not lifecycle.
- **No bypass of the secret store.** A `com.nube.mqtt` node that
  needs broker credentials reads them via the existing `secrets.get`
  capability gate, same as any other extension wire method.
- **No WASM support yet.** WASI-p2 can host process-style node kinds
  later (the `wasi:http` analogue for "outgoing JSON-RPC" is the
  natural extension); writing it down here so future contributors
  know the seam is intentional.

## Acceptance criteria

- An MQTT bundle at `examples/flow-agent/extensions/com.nube.mqtt/`
  validates, loads, and surfaces two node kinds on `flow-agent` boot.
- The palette in the React frontend shows the MQTT kinds with i18n'd
  label / summary / help text — fetched, not built-in.
- A flow wired with `trigger → mqtt.publish → log` fires
  end-to-end against a local broker.
- `cp -r ... && curl POST /admin/extensions/reload` adds a second
  extension's kinds to the palette without restarting the host or
  closing in-flight runs.
- Killing the MQTT child triggers supervisor restart per
  `block.yaml.supervision`; in-flight `mqtt.publish` calls fail with
  `NodeError::Backend` and the engine surfaces a `NodeFailed`
  exactly as it does for built-in kind failures.
- Cancelling a flow run mid-`mqtt.subscribe` causes the child to
  receive `stream.cancel { stream_id: invocation_id }` and stop
  emitting `stream.event`s within 250 ms; the proxy returns
  `NodeError::Cancelled` and the engine emits `NodeCancelled`.
- Replacing an installed extension via `POST
  /admin/extensions/reload` while a streaming subscribe is mid-run
  leaves the in-flight stream alive (against the old handle) until
  it terminates normally or until the grace cap elapses, per
  R-flow-node-6's guarantee table.
- `cargo clippy --workspace --all-features -- -D warnings` clean.
- New crate / module docstrings cite the rule numbers in this file.
