# Scope — flow-nodes

The authoritative design lives at
[`/home/user/code/rust/starter/DOCS/extensions/scope/FLOW-NODES.md`](/home/user/code/rust/starter/DOCS/extensions/scope/FLOW-NODES.md).
This brief is the trimmed per-job scope. Where this disagrees with the
source SCOPE, **the source SCOPE wins** — fix this file rather than
diverge.

## Goal

Land hot-loadable flow node kinds via extensions on the `starter` repo
via the `codeless/flow-nodes` branch. After this job:

1. An extension declares one or more flow node kinds in `block.yaml`
   via the new `contributes.nodes` field.
2. The kernel routes `NodeBehavior::invoke` calls to the extension's
   child process over the existing stdio JSON-RPC channel using the
   new `flow.node.invoke` wire method.
3. `examples/flow-agent`'s palette is driven by the host's
   `NodeKindRegistry` — new kinds appear when the operator triggers
   `POST /admin/extensions/reload`, without rebuilding the host.
4. The `com.nube.mqtt` demo bundle hot-loads, surfaces
   `mqtt.publish` and `mqtt.subscribe` in the palette, fires
   end-to-end against a `mosquitto` test container, survives a
   cancel-mid-subscribe, survives a supervisor restart on child
   crash, and survives a reload-while-streaming with the
   guarantee-table semantics from R-flow-node-6.
5. R-flow-node-1 through R-flow-node-8 hold by construction.

## In scope (two slices mirroring the SCOPE structure)

- **Slice A (stage 1) — manifest + dynamic registry + flow-agent
  wiring (descriptors only, no proxy):**
  - `Contributes.nodes` + `ContributeNode` on
    `starter-ext-spi::manifest`.
  - Reverse-DNS namespace validator extension in
    `starter-ext-host::validate`.
  - `NodeDescriptor` `Cow<'static, str>` widening per R-flow-node-2,
    with a `const fn new(&'static str)` shim that keeps every
    existing built-in call site source-compatible.
  - `DynamicNodeKindRegistry` + `CompositeNodeKindRegistry` in
    `starter-flow-spi::node`.
  - `contributed_node_kinds()` walker in `starter-ext-flow` returning
    `DynamicNodeKindEntry` values; in slice A the entry's
    `Arc<dyn NodeBehavior>` is a placeholder that returns a typed
    "no behaviour bound" error.
  - `flow-agent` backend: registry composition via `ArcSwap`,
    `GET /api/node-kinds`, `GET /api/node-kinds/<kind>/settings-schema`,
    `GET /api/node-kinds/<kind>/description`.
  - `flow-agent` frontend: `state/node-kinds-store.ts` (react-query),
    `FlowEditor` palette reads from the store.
  - Test fixture `starter-ext-flow/tests/fixtures/` with a
    `block.yaml` exercising the schema.

- **Slice B (stage 2) — process-flavour proxy + supervisor wiring +
  MQTT demo:**
  - `FLOW_NODE_INVOKE` constant + `-32050..-32099` error-code range
    on `starter-ext-spi::jsonrpc`.
  - `ProcessNodeProxy` in `starter-ext-flow::process_proxy`
    implementing `NodeBehavior::invoke` with `invocation_id`
    correlation per R-flow-node-5, advisory `deadline_ms`, and
    `Cancel`→`stream.cancel` forwarding.
  - `stream.cancel` helper on `SupervisorHandle` in
    `starter-ext-supervisor`.
  - Streaming node support over existing
    `stream.event`/`stream.end`/`stream.error` notifications (no
    new streaming shape).
  - `POST /admin/extensions/reload` on `flow-agent` implementing
    the full R-flow-node-6 reload algorithm: diff + deferred
    shutdown with `Arc::strong_count == 1` drop guard + grace-window
    cap + `ArcSwap` registry swap + `extensions.reload` SSE event.
  - `examples/flow-agent/extensions/com.nube.mqtt/` full bundle:
    `block.yaml`, `bin/mqtt-driver` (`rumqttc` + `starter-jsonrpc-stdio`),
    settings schemas, description docs, i18n catalogs.
  - End-to-end test covering: bundle drop + reload + palette
    refresh + flow fire + broker assertion, cancel-mid-subscribe,
    supervisor restart on crash, replace-while-streaming with the
    R-flow-node-6 guarantee-table behaviour.

## Out of scope

- **Supervisor groups.** Each extension supervisor stays its own
  subtree (parent SCOPE R9).
- **A new wire format.** All node dispatch reuses the existing
  request/response shape + the existing four `stream.*`
  notifications. Adding a new streaming variant is an explicit
  R-flow-node-1 violation.
- **A second `Ctx` API.** Extensions receive the same `Ctx` shape
  today's process-flavour extensions get.
- **Bypass of the secret store.** MQTT broker credentials route
  through the existing `secrets.get` capability gate, same as any
  other extension wire method.
- **WASM/WASI-p2 support.** The seam is intentional and noted
  in the source SCOPE; landing it is a separate job.
- **A second supervisor process per extension.** Nodes are
  behaviours, not separate processes; they live inside the
  extension's existing child.
- **`NodeBehavior::validate_settings` override hook.** The seam
  for `flow.node.validate_settings` is documented in R-flow-node-7
  but not wired in either slice.
- **Builtin-flavour node-kind extensions** (a
  `BuiltinNodeKindFactory` lookup in the adapter). Out of scope
  for slice A per the source SCOPE; not picked up in slice B.
- **Editor support for reverse-DNS kind ids natively.** The
  `ui_kind_table.rs` workaround in `flow-agent` ships in slice A;
  a deeper editor refactor is a follow-up.
- **A CLI wrapper for `POST /admin/extensions/reload`.** The
  REST endpoint ships; the CLI shim is TBD per the source SCOPE.

## Constraints

- **R-flow-node-1** — exactly one new manifest field
  (`contributes.nodes`), one new adapter walker
  (`starter-ext-flow::contributed_node_kinds`), one new wire method
  (`flow.node.invoke`). No new streaming shape; reuse `stream.*`.
- **R-flow-node-2** — `NodeDescriptor` widens to
  `Cow<'static, str>`. No `Box::leak`. The `const fn new(&'static
  str)` shim keeps existing call sites source-compatible.
- **R-flow-node-3** — reverse-DNS namespace ownership. The loader
  rejects extensions contributing `starter.*` or a non-descendant
  of the extension id.
- **R-flow-node-4** — manifest declares the kind; the child
  implements the body; nothing else. `deny_unknown_fields`
  everywhere.
- **R-flow-node-5** — dispatch reuses `SupervisorHandle::call`.
  Cancellation is keyed by `invocation_id` (proxy-owned),
  **not** by the JSON-RPC `id`. `deadline_ms` is advisory; the
  host timeout is authoritative.
- **R-flow-node-6** — hot-reload is a registry swap with
  bounded-blast-radius shutdown. `Arc::strong_count == 1` is
  the drop guard; per-handle grace cap is an operator knob, not
  a guarantee. The guarantee table in the source SCOPE is the
  contract for `unchanged` / `replaced` / `removed` / `added`.
- **R-flow-node-7** — settings validation belongs to the engine
  (`starter-flow::DefinitionManager::publish`), not the kernel.
  `starter-ext-host` stays free of `schemars`/`jsonschema` deps.
- **R-flow-node-8** — builtin and process flavours are
  interchangeable from the engine's perspective. Same trait,
  same dispatch path, same observability spans.
- **Parent extensions R3** — `deny_unknown_fields` on every new
  manifest type.
- **Parent extensions R4** — reverse-DNS namespace.
- **Parent extensions R13** — `contributes.nodes` is the missing
  column; the adapter mechanism is the same as every other
  contribution kind.
- **MSRV / lint gates**: `cargo test --workspace`,
  `cargo clippy --workspace --all-features -- -D warnings`,
  `cargo fmt --check` green at every stage boundary.

## Deliverables (what "done" looks like)

1. `codeless/flow-nodes` branch with one commit per stage (two
   stages = two commits, plus one for the REVIEW handover),
   pushed via mani.
2. `cargo test --workspace` green at every stage boundary.
3. `cargo clippy --workspace --all-features -- -D warnings` green
   at every stage boundary.
4. `cargo fmt --check` green at every stage boundary.
5. **Slice A acceptance:** the fixture `block.yaml` under
   `starter-ext-flow/tests/fixtures/` validates and round-trips
   through `contributed_node_kinds()`; `flow-agent`'s
   `GET /api/node-kinds` returns the fixture kind with resolved
   i18n labels and absolute URLs; attempting to fire a flow that
   uses the kind returns the typed "no behaviour bound" error
   rather than a panic.
6. **Slice B acceptance** (per the source SCOPE §"Acceptance
   criteria", verbatim):
   - MQTT bundle at `examples/flow-agent/extensions/com.nube.mqtt/`
     validates, loads, and surfaces two node kinds on `flow-agent`
     boot.
   - Palette in the React frontend shows the MQTT kinds with
     i18n'd label / summary / help text — fetched, not built-in.
   - A flow wired `trigger → mqtt.publish → log` fires end-to-end
     against a local broker.
   - `cp -r ... && curl POST /admin/extensions/reload` adds a
     second extension's kinds to the palette without restarting
     the host or closing in-flight runs.
   - Killing the MQTT child triggers supervisor restart per
     `block.yaml.supervision`; in-flight `mqtt.publish` calls
     fail with `NodeError::Backend` and the engine surfaces a
     `NodeFailed` exactly as it does for built-in kind failures.
   - Cancelling a flow run mid-`mqtt.subscribe` causes the child
     to receive `stream.cancel { stream_id: invocation_id }` and
     stop emitting `stream.event`s within 250 ms; the proxy
     returns `NodeError::Cancelled` and the engine emits
     `NodeCancelled`.
   - Replacing an installed extension via `POST
     /admin/extensions/reload` while a streaming subscribe is
     mid-run leaves the in-flight stream alive (against the old
     handle) until it terminates normally or until the grace cap
     elapses, per R-flow-node-6's guarantee table.
   - New crate / module docstrings cite the rule numbers from
     the source SCOPE.

## Open questions — RESOLVED (2026-05-23, before start)

The source SCOPE is unusually well-resolved — slicing is explicit,
acceptance is precise, every hard rule has a load-bearing reason.
Three job-specific resolutions follow.

### Q1 — Scope realism: one job or two?

**Answer: One job, two stages, one REVIEW gate. The slicing in
the source SCOPE already does the decomposition work.**

Unlike the insights capability (4 phases requiring approval
between each), this scope is two slices with a clean property at
the end of slice A: the dynamic-registry path is fully wired and
testable via the "no behaviour bound" error, but no MQTT, no
supervisor wire, no real child process. Slice A is small and
mechanical (manifest field + `Cow` widening + registry plumbing +
descriptor REST). Slice B is the bulk (proxy + reload algorithm +
MQTT demo + end-to-end tests with a real broker).

**Decision.**
1. One job, two stages.
2. Cap at **30000¢ / 4h**, same as the other two queued starter
   jobs. Slice A is cheap (~30% of cap budget); slice B is the
   load-bearing remainder.
3. One REVIEW gate between slices. The gate exists because slice
   B's `ProcessNodeProxy` replaces slice A's placeholder behaviour
   — if slice A's descriptor shape is wrong, slice B compounds the
   mistake.
4. If slice B blows the cap, halt at the gate, split off slice B
   as `flow-nodes-slice-b`. Do not silently land a partial slice B
   (the MQTT acceptance test is all-or-nothing — a half-wired
   reload is worse than no reload).

### Q2 — `NodeDescriptor` `Cow` widening: how source-compatible can it stay?

**Answer: fully source-compatible via a `const fn new(&'static
str)` shim on `NodeDescriptor`. The internal field type changes
from `&'static str` to `Cow<'static, str>`, but every existing
call site that passes a string literal compiles unchanged.**

Concrete shape (stage 1 freezes this):

```rust
pub struct NodeDescriptor {
    label_key: Cow<'static, str>,
    summary_key: Cow<'static, str>,
    // ... etc
}

impl NodeDescriptor {
    pub const fn new(label_key: &'static str, summary_key: &'static str, /* ... */) -> Self {
        Self {
            label_key: Cow::Borrowed(label_key),
            summary_key: Cow::Borrowed(summary_key),
            // ... etc
        }
    }

    pub fn new_owned(label_key: String, summary_key: String, /* ... */) -> Self {
        Self {
            label_key: Cow::Owned(label_key),
            summary_key: Cow::Owned(summary_key),
            // ... etc
        }
    }
}
```

Existing call sites in `starter-flow-nodes` (one `NodeDescriptor::new`
per built-in kind, ~13 sites) compile without edit. New extension
descriptors call `new_owned`. The `const fn` constructor preserves
the zero-allocation form for built-ins so `&'static NodeDescriptor`
construction in `StaticNodeKindRegistry` is byte-for-byte unchanged.

If a built-in call site fails to compile under the new signature
(missing field, type mismatch), surface in chat — that's a sign
the public-API audit missed a field, not a sign to widen the
shim.

### Q3 — `invocation_id` vs JSON-RPC `id` correlation

**Answer: `invocation_id` is the proxy's correlation key; the
JSON-RPC `id` is `SupervisorHandle::call`'s internal demux key.
They are different lifecycles and must not be conflated.**

Per R-flow-node-5:
- **JSON-RPC `id`** is allocated by `SupervisorHandle::call` per
  request/response pair, invisible to the proxy, short-lived
  (released the moment the response lands).
- **`invocation_id`** is allocated by `ProcessNodeProxy::invoke`
  as `StreamId(format!("inv-{}", ulid::Ulid::new()))`, included
  in the `flow.node.invoke` params, and lives as long as the
  node call. For streaming nodes it doubles as the `stream_id`
  for `stream.event`/`stream.end`/`stream.error`/`stream.cancel`.
- The child indexes in-flight invocations by `invocation_id`
  regardless of streaming/non-streaming, so the same `stream.cancel
  { stream_id }` envelope cancels both shapes.
- `Cancel` token trips are forwarded by
  `spawn_cancel_forwarder(ctx.cancel, invocation_id.clone())`
  which sends `stream.cancel { stream_id: invocation_id }`. The
  forwarder is dropped on successful response (the `cancel_guard`
  drop in the R-flow-node-5 example).

The implementation choice the source SCOPE leaves open — *what
the host does if the child ignores `deadline_ms`* — is also
resolved: host-side `SupervisorHandle::call` timeout is the hard
bound (typically `deadline_ms + small grace`); on expiry the host
returns `NodeError::Backend("timeout")` AND emits
`stream.cancel { stream_id: invocation_id }`. A child that does
not respect cancellation gets killed by the supervisor's restart
policy on the next health ping miss. This is the
defence-in-depth path the source SCOPE describes.

## References

- Source SCOPE (authoritative):
  [/home/user/code/rust/starter/DOCS/extensions/scope/FLOW-NODES.md](/home/user/code/rust/starter/DOCS/extensions/scope/FLOW-NODES.md)
- Parent extensions SCOPE:
  `starter/DOCS/extensions/scope/SCOPE.md`
- Flow engine SCOPE:
  `starter/DOCS/flow/scope/SCOPE.md`
- Existing extension crates layout (ground truth for stage 1):
  `/home/user/code/rust/starter/starter-extensions/crates/`
- `starter-flow-nodes` `NodeDescriptor` call sites (Q2 ground truth):
  `/home/user/code/rust/starter/crates/starter-flow-nodes/src/`
- `examples/flow-agent` shape:
  `/home/user/code/rust/starter/examples/flow-agent/`
