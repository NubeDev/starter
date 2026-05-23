# KIND-MANIFEST — the manifest schema and its versioning semantics

> Source: `rubix/SCOPE.md` R5 (contracts hub), R10 (add-only within a
> major), R7 (block-author surface), §"What counts as a breaking
> change (R10)". Cross-refs: `NODE-AUTHORING.md` (how to author the
> behaviour the manifest describes), `VERSIONING.md` (the breaking-
> change taxonomy in full).

The manifest is the kind's **wire contract**. It lives in `rubix-spi`
(R5) because every consumer — `kinds-registry`, Studio's kind picker,
the REST kind catalogue, MCP's tool surface, third-party blocks, the
TS/Dart clients via codegen — reads the same shape. There is no
parallel manifest format; there is no transport-side embellishment.

## Top-level shape

```rust
pub struct KindManifest {
    pub id:           KindId,
    pub version:      u32,
    pub title:        &'static str,
    pub description:  &'static str,
    pub slots:        SlotSchema,
    pub facets:       Facets,
    pub permissions:  Vec<Permission>,
    pub capabilities: Capabilities,
    pub placement:    PlacementRule,
}
```

Every field is required at the struct level. Wire-format
representations (JSON for REST + OpenAPI, protobuf for gRPC,
YAML for the `block.yaml` author surface) follow the same shape via
`serde` + `utoipa::ToSchema`.

The struct lives in `rubix-spi/src/manifest/`; the wire
representations are derived. **Hand-rolling a JSON or protobuf shape
that differs from the Rust struct is forbidden** — that creates two
sources of truth and R5 collapses.

## Field-by-field

### `id: KindId`

A stable reverse-DNS string. Constructed via `KindId::parse(&str)`
which validates the lexical grammar:

```text
<segment>(.<segment>)+
<segment> = [a-z][a-z0-9_]{0,30}
```

- `sys.*` — built-in kinds shipped in the agent binary. Reserved
  namespace; only the rubix maintainers add kinds here.
- `com.<org>.<name>.*` — third-party blocks. The `com.<org>.<name>`
  prefix matches the `block.yaml` block id; the suffix names the kind
  within the block.

Examples: `sys.point.writable`, `sys.alarm.rule`,
`com.rubix.mqtt_client.subscription`,
`com.acme.bacnet.device`.

**Renaming an id is breaking** (`VERSIONING.md`). Choose carefully.

### `version: u32`

The manifest version, separate from the Rust crate's semver. Starts at
`1`. Bumps under R10:

| Change | Bumps `version` | Bumps major (consumers) |
|---|---|---|
| Add an optional manifest field | no | no — additive |
| Add an output slot | yes | no — additive |
| Add an input slot **with a default the engine can supply** | yes | no — additive |
| Add an input slot **with no default** | yes | **yes** — pre-existing flows lack the wire |
| Rename a slot | yes | **yes** |
| Remove a slot | yes | **yes** |
| Change a slot's `SlotKind` | yes | **yes** |
| Tighten a slot's value constraints (range, enum subset) | yes | **yes** |
| Loosen a slot's value constraints | yes | no — additive |
| Add a permission | yes | **yes** — pre-existing principals lose access |
| Remove a permission | yes | no — strictly liberalising |
| Tighten the placement rule | yes | **yes** — pre-existing trees may violate it |
| Loosen the placement rule | yes | no — additive |
| Add a facet | yes | no — additive |
| Remove a facet | yes | **yes** — Studio rendering changes |

`version` is the **read-side breadcrumb** for migrations: a kind in
the persisted graph stores the `version` it was created against; the
registry's loader compares and runs forward-only manifest migrations
where defined. `VERSIONING.md` covers the manifest-migration shape.

### `title` / `description`

i18n-keyed strings rendered in Studio. The catalogues live in
`rubix-i18n/catalogues/{en,es}/kinds.json` and ship per-feature from
Phase 1 (SCOPE.md "Decisions made (locked)" — i18n bullet). Keys
are stable; renaming a key is a breaking documentation change.

The Rust strings in the manifest are `&'static str` defaults; the
client-side renderer prefers the catalogue.

### `slots: SlotSchema`

The typed declaration of every input and output slot the kind owns.

```rust
pub struct SlotSchema { /* private builder; access via builder() */ }
pub enum SlotDirection { Input, Output, Bidirectional }
pub enum SlotKind {
    Bool, I64, F64, String, DateTime, Duration, Bytes,
    Enum(&'static [&'static str]),
    Json,                 // escape hatch; tightening to a typed kind is additive
    Slot(KindId),         // a reference to another node
}
pub struct SlotDecl {
    pub name:        &'static str,            // stable id; rename = breaking
    pub direction:   SlotDirection,
    pub kind:        SlotKind,
    pub description: &'static str,
    pub constraints: SlotConstraints,         // range, enum subset, regex
    pub default:     Option<SlotValue>,       // engine-supplied default
}
```

Rules:

- **Slot names are stable.** Renaming is breaking.
- **Adding a slot is additive.** Removing or retyping is breaking.
- **Constraints**: tightening is breaking; loosening is additive.
- **Defaults**: adding a default to a previously-required input is
  additive (existing principals get the default); changing an existing
  default is **breaking** (changes runtime behaviour of pre-existing
  flows).

The `SlotKind::Json` escape hatch is allowed but discouraged — the
typed kinds (`Bool`, `I64`, `F64`, etc.) carry the constraints the
SDUI renderer and unit converter rely on. A `SlotKind::Json` slot is
opaque to those layers.

### `facets: Facets`

Coarse-grained tags read by Studio and the kind picker:

```rust
pub struct Facets {
    pub category:    FacetCategory,     // Device / Point / Logic / …
    pub icon:        IconRef,           // resolved against the icon set
    pub render_as:   RenderHint,        // ScalarTile / Gauge / List / …
    pub editable:    bool,              // operator can set values?
    pub historized:  bool,              // values land in the warehouse?
    pub tags:        SmallVec<[Tag; 4]>,// free-form labels for search
}
```

Facets do **not** affect runtime semantics — they are purely the
Studio + kind-picker hints. A facet drift between manifest and code
is a Studio bug, not a correctness bug. That makes facets safe to
iterate on per-feature without touching consumer pinning.

### `permissions: Vec<Permission>`

The `starter-authz` resource:action strings required to mutate the
node. `transport-rest` reads this list and wraps each write handler
in `with_permission(resource, action)` (the Phase 7 pattern landed in
this repo already).

```rust
pub struct Permission {
    pub resource: ResourceKind,         // Device, Point, Dashboard, …
    pub action:   Action,               // Read, Write, Configure, …
}
```

Adding a permission is **breaking** (pre-existing principals lose
access). Removing is additive (strictly liberalising). See `AUTH.md`
for the resource/action taxonomy and the layer order
(`with_role → with_scope → with_permission → handler`).

### `capabilities: Capabilities`

A bitset of optional engine features the kind opts into:

```rust
pub struct Capabilities {
    pub status_slots:   bool,           // node writes its own health slot
    pub history:        bool,           // engine ships slot writes to warehouse
    pub schedule_aware: bool,           // engine wakes the node on schedule fire
    pub alarm_source:   bool,           // kind can be an alarm rule target
    pub sdui:           bool,           // kind emits a UiIr fragment for Studio
}
```

Capabilities are additive: turning one on is a minor bump, turning
one off is breaking (consumers may have relied on the side effect).

### `placement: PlacementRule`

Containment rule — where in the graph this kind can live.

```rust
pub enum PlacementRule {
    Root,                                       // top-level only
    ChildOf(SmallVec<[KindId; 4]>),             // must parent on one of these
    Anywhere,                                   // unrestricted (rare)
}
```

`placement_allowed(parent_kind, parent_manifest, candidate) -> bool`
in `agent/crates/graph` is the pure function consumers call. Both
`GraphStore::create_child` and the REST/CLI handlers call the same
chokepoint (R4). There is **no transport-side placement check**;
that would create two sources of truth and the smoke test "Swap
REST for gRPC" would catch the drift.

Tightening a `PlacementRule` is breaking — pre-existing trees may
violate it. Loosening is additive.

## The `block.yaml` wrapper (third-party blocks)

A block author authors manifests in `block.yaml`, transformed at
load time into `KindManifest` instances:

```yaml
# extensions/com.acme.bacnet/block.yaml
id: com.acme.bacnet
version: 1
kinds:
  - id: com.acme.bacnet.device
    version: 1
    title: BACnet device
    slots:
      - name: address
        direction: input
        kind: string
        description: BACnet device address (e.g. "10.0.0.5:47808")
      - name: connection
        direction: output
        kind: enum
        constraints: { enum: [online, offline, reconnecting] }
        description: Last observed link state
    permissions:
      - { resource: device, action: configure }
    placement: { child_of: [sys.network] }
```

The YAML is **a serialisation of `KindManifest`**, not a parallel
schema. The block-loader deserialises into the same Rust struct the
built-in kinds use. Adding a YAML-only field that has no Rust
counterpart is forbidden (R5 — single contracts hub).

The `block.yaml` schema's own version follows R10 too: any change
requiring existing block manifests to be edited is a major bump.

## Codegen surface

`rubix-spi` is the source of the OpenAPI snapshot
(`rubix-spi/openapi/spec.json`). The codegen lanes:

- **Rust** — direct dep on `rubix-spi`.
- **TS** — `mani run codegen` regenerates `agent-client-ts/src/generated/`.
  Codegen runs from the OpenAPI snapshot, not from a hand-typed mirror.
- **Dart** — same path, into `agent-client-dart/lib/generated/`.

A change to `KindManifest` lands in `rubix-spi`, then `mani run
codegen`, then every downstream consumer picks it up on next rebuild.
**Do not copy types by hand** (Q1 of the SCOPE decision tree).

## Manifest version vs. crate version

Two version axes; do not confuse them:

- **Manifest `version: u32`** — the schema of one kind. Bumps on
  any change to that kind's manifest fields per the table above.
  Persisted graph rows reference this number.
- **Crate semver** — `rubix-spi` itself follows semver. A
  manifest-shape change (adding a field to `KindManifest` itself, not
  to a specific kind's manifest) bumps `rubix-spi` per R10 and forces
  every consumer crate's major.

`VERSIONING.md` is the master reference; this doc just specifies the
per-kind dimension.

## Smoke tests for a new or changed manifest

Before merging a manifest change:

1. **`cargo test -p rubix-spi`** — the manifest's `serde`,
   `utoipa::ToSchema`, and prost round-trips pass.
2. **`mani run codegen --dry-run`** — the OpenAPI snapshot has not
   diverged from the Rust struct (CI fails on a divergence).
3. **Decision-tree check** — if you added a slot, is it nameable in
   stable reverse-DNS? If you added a permission, does
   `ResourceRegistry::lookup` find the resource kind? (The Phase 7
   extension-permission landing in this repo already enforces the
   second.)
4. **Migration check** — if you bumped a kind's `version`, is there a
   manifest-migration step that maps old persisted slots to new? See
   `VERSIONING.md` §"Forward-only manifest migrations".
5. **i18n catalogues** — every new `title` / `description` key has
   English and Spanish entries (SCOPE.md "Decisions made" — i18n EN+ES
   from Phase 1).

## Common pitfalls

- **Adding a "convenience" field to the JSON-side that doesn't exist
  in the Rust struct.** R5 collapse. Two sources of truth diverge by
  the third PR.
- **"Just rename this slot, we'll fix the migrations later."** No.
  Renaming is a major bump on every consumer crate (Rust + TS + Dart).
  Either ship the rename in a coordinated major or add the new name
  alongside the old (the additive path) and deprecate the old name
  over a major boundary.
- **Tightening a constraint to "fix a bug."** If a pre-existing
  persisted value violates the tighter constraint, the boot fails.
  Either loosen the constraint and add a runtime alarm, or coordinate
  the major bump.
- **Permission added to a manifest without an authz seed.** The
  Phase 7 audit log now records the deny. Coordinate the seed update
  with the manifest bump or pre-existing principals silently lose
  access at the next deploy.
