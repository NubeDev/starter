# NODE-AUTHORING — how to write a NodeBehavior

> Source: `rubix/SCOPE.md` R2, R3, R4, R5, R6, R10, R11. Cross-refs:
> `EVERYTHING-AS-NODE.md` (when to promote to a node), `KIND-MANIFEST.md`
> (manifest schema). The Rust types this doc references live in
> `rubix-spi`, which itself re-uses `starter-spi` (R5) and the
> `starter-flow-spi` types.

This doc is the contributor-facing recipe. Read it before adding a
built-in node kind to a `domain-*` crate (Q2 in the SCOPE decision
tree) or a third-party block kind via `rubix-extensions-sdk` (Q3).

## What a node is, in code

A node is the union of:

1. A **`KindManifest`** — declarative metadata: kind id, version, slot
   schema, facets, permissions, capabilities, placement rules. The
   manifest is the wire-level contract (R5) and lives in `rubix-spi`.
   It is what kindergartens, REST clients, the kind picker, and authz
   read to know the kind exists.
2. A **`NodeBehavior`** — the runtime: what the node does when it
   receives a `Msg`, how it propagates writes downstream, how it
   transitions through its own lifecycle, how it surfaces status via
   slot writes. Lives in a `domain-*` crate (built-in) or an
   `extensions/<id>/process/` crate (third-party).
3. **Tests** that mirror `src/` 1:1 per R11 (`TESTS.md`).

The manifest is **data**; the behaviour is **code**. They are
versioned together — bumping one without the other is the most
common Phase 0/1 bug to watch for.

## The two surfaces to know

```text
rubix-spi (R5)              starter-flow-spi (consumed)
├── KindManifest            ├── NodeBehavior trait
├── SlotSchema              ├── NodeCtx
├── SlotValue               ├── Msg / Msg::new / Msg::child
├── Msg envelope re-export  ├── Propagator
└── Facets / Permissions    └── Lifecycle FSM (Init → Ready →
                                Running → Draining → Stopped)
```

`rubix-spi` re-exports the `starter-flow-spi` types a kind author
needs, so a third-party block depends only on `rubix-extensions-sdk`
(R7) — never directly on `starter-flow-spi`.

## File layout for one kind

A built-in kind lives in a `domain-*` crate. The R1 file-size limit
(400 lines) and naming rule (name files after the concept they own)
mean one kind ≈ one file pair:

```
agent/crates/domain-points/
  src/
    manifest.rs           ← one fn per kind returning a KindManifest
    point_writable.rs     ← NodeBehavior for sys.point.writable
    point_read_only.rs    ← NodeBehavior for sys.point.read_only
    slots.rs              ← slot-key constants used across kinds in this crate
  tests/
    point_writable_test.rs
    point_read_only_test.rs
```

A third-party block under `extensions/com.<org>.<name>/process/`
mirrors the same shape. A block can ship multiple kinds (e.g. an
MQTT block ships `mqtt.broker` and `mqtt.subscription`) — one file
per behaviour, one entry per manifest.

## Writing the manifest

The manifest is a `KindManifest` struct (full schema in
`KIND-MANIFEST.md`). At minimum:

```rust
use rubix_spi::manifest::{KindId, KindManifest, SlotSchema, SlotKind,
    Facets, PlacementRule, Capabilities, Permission};

pub fn manifest() -> KindManifest {
    KindManifest {
        id: KindId::parse("sys.point.writable").unwrap(),
        version: 1,
        title: "Writable point",
        description: "Operator- or driver-writable scalar slot.",
        slots: SlotSchema::builder()
            .input ("set_value", SlotKind::F64,     "Operator or driver write")
            .output("value",     SlotKind::F64,     "Current value (last write wins)")
            .output("ts",        SlotKind::DateTime,"Wall-clock of last write")
            .build(),
        facets: Facets::scalar_writable(),
        permissions: Permission::list(&["points:write"]),
        capabilities: Capabilities::status_slots(),
        placement: PlacementRule::child_of(&["sys.device"]),
    }
}
```

Notes:

- **`id`** is a stable reverse-DNS path. `sys.*` for built-in.
  `com.<org>.<name>.*` for blocks. Renaming an id is a breaking
  change (`VERSIONING.md`).
- **`version`** is the manifest version, not the Rust crate version.
  It bumps under the R10 rules in `VERSIONING.md` (add-only within a
  major).
- **`slots`** declare typed inputs and outputs. The
  `SlotSchema::builder` API is fluent so the schema reads
  declaratively. Slot names are stable; renaming is breaking.
- **`facets`** are coarse-grained capability tags read by Studio
  (which icon, which inspector tab). See `KIND-MANIFEST.md`.
- **`permissions`** are the `starter-authz` resource:action strings
  required to mutate the node. The transport-rest layer wraps
  handlers in `with_permission(...)` derived from this list.
- **`placement`** declares which parents this kind can be a child of.
  `placement_allowed(parent, candidate)` lives in `graph` and is the
  single chokepoint — R4 forbids re-implementing it in transport.

## Writing the behaviour

`NodeBehavior` is the runtime trait. The author implements three
hooks: `on_init`, `on_msg`, `on_shutdown`. Each hook receives a
`NodeCtx` (the slot writer + propagator handle + tracing span) and
returns a `Result<Outcome, Error>`.

```rust
use rubix_spi::node::{NodeBehavior, NodeCtx, Outcome};
use rubix_spi::msg::Msg;
use rubix_spi::Error;
use chrono::Utc;

pub struct PointWritable;

impl NodeBehavior for PointWritable {
    fn manifest(&self) -> KindManifest { manifest() }

    fn on_init(&self, ctx: &mut NodeCtx) -> Result<Outcome, Error> {
        // Seed status slots from persisted state. The graph store
        // hands us the persisted value via ctx.persisted_slots().
        if let Some(v) = ctx.persisted_slot("value")? {
            ctx.write_slot("value", v.clone())?;
            ctx.write_slot("ts", SlotValue::DateTime(Utc::now()))?;
        }
        Ok(Outcome::Idle)
    }

    fn on_msg(&self, ctx: &mut NodeCtx, msg: Msg) -> Result<Outcome, Error> {
        // R6: msg is immutable. Read; produce child msgs via Msg::child.
        let v = msg.payload::<f64>()?;
        ctx.write_slot("value", SlotValue::F64(v))?;     // R3: through the graph
        ctx.write_slot("ts",    SlotValue::DateTime(Utc::now()))?;
        ctx.propagate(msg.child().with_payload(v))?;     // R6: child, not mutate
        Ok(Outcome::Idle)
    }

    fn on_shutdown(&self, _ctx: &mut NodeCtx) -> Result<Outcome, Error> {
        // No-op for a passive point. A driver would close its socket here.
        Ok(Outcome::Idle)
    }
}
```

### Slot writes — R3 chokepoint

`ctx.write_slot(name, value)` is the only call that mutates
observable state. Internally it delegates to `GraphStore::write_slot`
in `agent/crates/graph` (the chokepoint). The propagator picks up
the write and walks downstream subscribers. Authoring a node:

- **Always** write slots via `ctx.write_slot`. Do not stash slot
  state in a struct field "for performance" — the graph store is
  the source of truth and stale local copies cause divergence bugs.
- **Status slots are first-class.** A driver's connection state is a
  slot (`connection` = `online | offline | reconnecting`), not a
  field. Studio renders it, alarms fire on it.
- **Coalesce at the graph boundary, not in the node.** A driver
  bursting writes to the same slot in a single propagator tick is
  coalesced to the last value by the graph layer (SCOPE.md
  §"Concurrency"). Don't pre-debounce in the node.

### Propagator interaction — R6 immutable msgs

The `starter-flow` propagator delivers a `Msg` to `on_msg`. The node
**must not** mutate the received msg. Produce children with
`Msg::child` and propagate via `ctx.propagate(child_msg)`. The Rhai
Function node is the one place `msg` *feels* mutable to the author —
the runtime snapshots it on exit (Arc-CoW; deep-clone only at the
boundary that escapes the Function scope). See SCOPE.md R6 and the
`NODE-RED-MODEL.md` design doc when it lands.

### FSM placement

The engine runs each node through the FSM:

```
Init  →  Ready  →  Running  ⇄  Draining  →  Stopped
                      ↑                 │
                      └────── reload ───┘
```

- `on_init` runs in `Init`. Allocate, read persisted state, declare
  initial slot values.
- `on_msg` runs in `Running`. The propagator delivers msgs only here.
- `on_shutdown` runs in `Draining`. Drain in-flight work, close
  external connections, flush buffers. The outbox flushes after
  `on_shutdown` returns; do not write slots here expecting downstream
  delivery — downstream nodes have already drained.

The engine's graceful-shutdown protocol (SIGTERM → drain in-flight →
outbox flush → exit) is in `agent/crates/engine`. A node author does
not call into it directly; the lifecycle hooks above are the seam.

### Error handling

Return `Err(...)` from a hook and the engine transitions the node to
`Stopped` and emits a node-error slot write (which an alarm rule can
fire on). Recoverable errors (transient driver disconnect) **must**
be handled inside the node and surfaced via status slots —
**returning `Err` is for unrecoverable conditions only**.

Use `starter-spi::Error` for the error type so authz, transport, and
client surfaces handle it uniformly. Never panic in a hook —
`#![deny(clippy::unwrap_used)]` is on for `domain-*` crates.

### Logging

A `NodeCtx` carries a `tracing::Span` linked to the originating
request (R-Observability rule 3 — trace propagation). Use
`tracing::info!`/`warn!`/`error!` directly; don't take your own
logger. Span fields already include the principal, tenant, and node
path.

## Registering the kind

The agent binary (`agent/crates/apps/agent/src/main.rs`) is the
single call site that wires every built-in kind into the registry:

```rust
fn register_kinds(registry: &mut KindsRegistry) {
    use domain_points::{PointWritable, PointReadOnly};
    registry.register(PointWritable);
    registry.register(PointReadOnly);
    // …every other built-in…
}
```

A third-party block registers via the supervisor — the block process
emits its manifests at startup; the agent's kinds-registry merges them
in. The block author does not edit `apps/agent/src/main.rs`.

## Tests (R11)

Per SCOPE.md R11 and `TESTS.md`, each behaviour file has a sibling
test file in `tests/`:

```rust
// tests/point_writable_test.rs
use starter_server::testing::TestApp;
use rubix_agent_client::Client;

#[tokio::test]
async fn writes_propagate_to_downstream() {
    let app = TestApp::spawn().await;          // testcontainer-backed
    let client = Client::new(app.url(), app.token());

    let point = client.create_point("/sys/dev1/p1", 0.0).await.unwrap();
    client.write_point(&point.path, 42.0).await.unwrap();

    let snap = client.read_point(&point.path).await.unwrap();
    assert_eq!(snap.value, 42.0);
    assert!(snap.ts > point.created_at);
}
```

Unit tests for pure helpers live `#[cfg(test)] mod tests` next to
the function under test. Integration tests requiring Postgres are
`#[ignore = "needs-docker"]` and run via the `RUBIX_E2E=1` lane in
CI. See `TESTS.md` for the full convention.

## Cross-cutting reminders

- **R4 layer arrow.** A `NodeBehavior` lives in `domain-*`. It does
  not import `transport-*`. If you find yourself needing a REST type,
  the type belongs in `rubix-spi` (R5).
- **R7 block-author surface.** A third-party block consumes
  `rubix-extensions-sdk` — which re-exports `NodeBehavior`, `NodeCtx`,
  `Msg`, slot types — never `agent/crates/*` directly.
- **R10 manifest versioning.** Adding a slot or an optional manifest
  field is additive (minor). Renaming, removing, or retyping a slot is
  a major bump (`VERSIONING.md`). Plan the manifest carefully on
  first land — every renamed slot becomes a coordinated breaking
  change across Rust + TS + Dart clients.
- **R11 tests-with-code.** A new behaviour without a sibling test is
  a CI failure, not a "follow-up PR."

## When the SDK doesn't expose what you need

For a third-party block: **add it to `rubix-extensions-sdk` and
bump**, then consume the new surface. Do **not** path-dep
`agent/crates/*` or copy types out of `rubix-spi`. The R7 fence is
load-bearing; a single block reaching behind the SDK becomes the
precedent that ten more cite, and the platform/product boundary
rots.

For a built-in kind: if you need a primitive the graph layer doesn't
expose, add it to `graph` (the chokepoint already owns the
consistency contract). New primitives in `graph` get their own R11
tests against the testcontainer fixture.
