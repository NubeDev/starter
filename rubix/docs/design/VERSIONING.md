# VERSIONING — R10 in full, with the breaking-change taxonomy

> Source: `rubix/SCOPE.md` R10 ("Versioning is add-only within a
> major"), §"What counts as a breaking change (R10)", §"Decisions
> made" (publishing-model + migrations-order bullets), R5 (contracts
> hub), R7 (block-author surface). Cross-refs: `KIND-MANIFEST.md`
> (per-kind manifest version), `MIGRATIONS.md` (forward-only SQL),
> `AUTH.md` (the AuthZ resource registry).

## The rule

> **Within a major, add-only. A breaking change bumps the major on
> the Rust crate, the npm package, the Dart package, and the agent
> binary — simultaneously.**

R10 is the contract that lets a third-party block depend on a
specific `rubix-extensions-sdk` major and trust that no minor or
patch breaks it. A breaking change is **coordinated** across every
consumer surface; an additive change ships independently.

## The contract surfaces R10 covers

1. **`rubix-spi` Rust API** — every public item: type, fn, trait,
   const, macro. The contracts hub (R5).
2. **`rubix-extensions-sdk` public Rust API** — the curated block-
   author Rust surface (R7).
3. **`rubix-agent-client` public Rust API** — the Rust HTTP client.
4. **`@rubix/agent-client` public TS API** — the codegen'd TS HTTP
   client.
5. **`@rubix/ui-kit` public TS API** — Shadcn primitives + tokens.
6. **`@rubix/ui-core` public TS API** — the portable brain.
7. **`@rubix/extension-ui-sdk` public TS API** — the curated block-
   author TS façade (R7).
8. **`rubix_agent_client` (Dart) public API** — the mobile client.
9. **`KindManifest` schema** — the wire shape (`KIND-MANIFEST.md`).
10. **Per-kind `manifest.version`** — see "Per-kind versioning"
    below.
11. **`Msg` shape** — the immutable wire envelope (R6).
12. **REST DTO surface** — every request/response shape
    decorated `#[derive(ToSchema)]`.
13. **`block.yaml` schema** — the YAML serialisation of
    `KindManifest`.
14. **gRPC `.proto` shape** — `rubix/contracts/proto/block.proto`
    and siblings.
15. **CLI command surface** — every `agent <subcommand>` shape
    consumers script against.
16. **Postgres schema referenced by external readers** — only
    columns explicitly documented as public-stable; rubix's internal
    tables are not a contract.

For each surface: additive → minor. Breaking → major. Patch is for
bug fixes that don't change shape.

## The breaking-change taxonomy

The SCOPE's master list, restated and expanded. Anything in the
**Breaking** column triggers a coordinated major bump.

### `rubix-spi` Rust API

| Change | Class |
|---|---|
| Add a new public type / fn / const / variant (variant on `#[non_exhaustive]` enum) | Additive |
| Add a method to a trait **with a default impl** | Additive |
| Add a method to a trait **without a default impl** | **Breaking** |
| Add a required field to a public struct | **Breaking** |
| Add an optional field to a public struct *that is `#[non_exhaustive]`* | Additive |
| Add an optional field to a struct that is **not** `#[non_exhaustive]` | **Breaking** (struct literals break) |
| Rename a public item | **Breaking** |
| Remove a public item | **Breaking** |
| Change a fn signature (param type, return type, lifetime bound) | **Breaking** |
| Tighten a trait bound | **Breaking** |
| Loosen a trait bound | Additive |
| Narrow an enum variant (remove or shrink a field) | **Breaking** |
| Add a new variant to an enum that is **not** `#[non_exhaustive]` | **Breaking** (match arms break) |
| Add a new variant to an `#[non_exhaustive]` enum | Additive |

**Rule of thumb:** every `pub struct` and `pub enum` in `rubix-spi`
is born `#[non_exhaustive]`. The cost is forcing call sites to
construct via builders; the benefit is that field additions are
additive instead of breaking.

### `KindManifest` schema (the struct itself)

| Change | Class |
|---|---|
| Add a new optional top-level field | Additive |
| Add a new required top-level field | **Breaking** |
| Rename a top-level field | **Breaking** |
| Tighten a field's type | **Breaking** |
| Add a new `SlotKind` variant | Additive (enum is `#[non_exhaustive]`) |
| Rename a `SlotKind` variant | **Breaking** |
| Add a new `FacetCategory` | Additive |
| Remove a `FacetCategory` | **Breaking** |

### Per-kind manifest version

A `KindManifest.version: u32` bumps per the matrix in
`KIND-MANIFEST.md` §"`version: u32`". Restated:

| Change to a specific kind's manifest | `version` bumps? | Consumer major? |
|---|---|---|
| Add an output slot | yes | no |
| Add an input slot with engine-supplied default | yes | no |
| Add an input slot without default | yes | **yes** |
| Rename / remove / retype a slot | yes | **yes** |
| Tighten a slot constraint | yes | **yes** |
| Loosen a slot constraint | yes | no |
| Add a permission | yes | **yes** |
| Remove a permission | yes | no |
| Tighten placement | yes | **yes** |
| Loosen placement | yes | no |
| Add a facet | yes | no |
| Remove a facet | yes | **yes** |

The "Consumer major" column drives the coordinated bump on
`rubix-spi`, every client crate, and the agent binary. The kind's
own `manifest.version` is the read-side breadcrumb for forward-only
manifest migrations (see `KIND-MANIFEST.md`).

### `Msg` shape (R6)

| Change | Class |
|---|---|
| Add an optional field | Additive |
| Add a required field | **Breaking** |
| Rename or remove a field | **Breaking** |
| Change a field's type | **Breaking** |
| Add an attribute on a child msg | Additive (child msgs are open) |

### REST DTO surface

| Change | Class |
|---|---|
| Add an optional response field | Additive |
| Add a required response field | Additive (consumers ignore unknown) **only if** OpenAPI snapshot still validates; otherwise **breaking** |
| Add an optional request field | Additive |
| Add a required request field | **Breaking** |
| Remove or rename any field | **Breaking** |
| Change a field's type (incl. narrowing) | **Breaking** |
| Tighten validation (e.g. `min: 0 → min: 1`) | **Breaking** |
| Loosen validation | Additive |
| Add a new endpoint | Additive |
| Remove an endpoint | **Breaking** |
| Change a status code from `200` to `201` | **Breaking** (callers check `== 200`) |
| Add a new error response variant | Additive **if** error type is `#[non_exhaustive]`; **breaking** otherwise |

### `block.yaml` schema

Any change requiring existing block manifests to be edited is a
major. Adding a new optional field that defaults sensibly when
omitted is additive.

### gRPC `.proto`

Protobuf compatibility rules apply on top of R10:

| Change | Class |
|---|---|
| Add a new field number | Additive |
| Reuse an old field number for a new type | **Breaking** |
| Change a field's wire type | **Breaking** |
| Add a new RPC | Additive |
| Remove an RPC | **Breaking** |
| Add a new enum value | Additive (clients must handle unknown) |
| Renumber an enum value | **Breaking** |

### CLI command surface

| Change | Class |
|---|---|
| Add a new subcommand | Additive |
| Add a new optional flag | Additive |
| Add a required flag to an existing command | **Breaking** |
| Rename a flag | **Breaking** |
| Change the default of a flag (silent behaviour change) | **Breaking** |
| Change exit code semantics | **Breaking** |
| Change output format (table → JSON by default) | **Breaking** |

CLI consumers script against output formats. A formatting change
that breaks `jq '.[] | .name'` is a major bump.

### `starter-spi` re-exports

If rubix re-exports a `starter-spi` type and `starter` bumps that
type's major, rubix bumps too. The re-export is the contract
surface; bumping `starter-spi` without bumping rubix would let
consumers depend on two incompatible majors of the same type.

## Coordinated bump procedure

When a breaking change lands:

1. **Open the major bump PR.** Title: `breaking: <one-line>`.
2. **Bump every consumer crate** in the same PR:
   - `rubix-spi`, `rubix-extensions-sdk`, `rubix-agent-client`
     (Rust majors).
   - `@rubix/agent-client`, `@rubix/ui-kit`, `@rubix/ui-core`,
     `@rubix/extension-ui-sdk` (npm majors).
   - `rubix_agent_client` (Dart major).
   - `rubix-agent` (binary major).
3. **Update the `CHANGELOG.md`** under a new major section listing
   every breaking change. The changelog format follows Keep a
   Changelog; the breaking section lives at the top of the major.
4. **Update consumer-facing docs** that quote the old shape.
5. **Land the migration path** alongside the bump: a transitional
   re-export, a `#[deprecated]` shim on the old API for one major,
   or a code-mod script when shape diverges enough to need one.
6. **Tag the binary's next release with the new major.** The
   agent binary's major matches the SPI's major; consumers verify
   compatibility by comparing majors at boot.

## When NOT to bump

- **Internal refactor that doesn't change a public shape.** Patch.
- **Adding a new domain crate that exposes new endpoints.** Minor
  on the binary; additive on every consumer surface.
- **Renaming a private item.** Not a bump.
- **Tightening a `pub(crate)` item.** Not a bump.
- **Adding a new test.** Not a bump.
- **Adding a doc comment.** Not a bump.
- **Reformatting (`cargo fmt`).** Not a bump.

## Add-only within a major — the additive checklist

A change is additive **if and only if**:

1. **No existing consumer needs to recompile** to upgrade.
   (Recompiling to *pick up* new functionality is fine; recompiling
   to *keep working* is not.)
2. **No existing consumer's existing call sites** observe a
   behaviour change.
3. **No existing persisted state** becomes unreadable.
4. **No existing manifest** becomes invalid.
5. **No existing permission grant** becomes insufficient.

If all five hold, ship the change as a minor. If any one fails,
it's a breaking change — line it up for the next major.

## `#[non_exhaustive]` discipline

Every `pub struct` and `pub enum` in `rubix-spi` is born
`#[non_exhaustive]`. The cost is forcing call sites to construct
via `::builder()` or `::default()` + setters; the benefit is that
field additions become additive instead of breaking.

The discipline is enforced by lint:

```rust
#![deny(missing_non_exhaustive_on_public_types)]   // hypothetical lint
```

In practice the lint is a Clippy-categorised check
(`clippy::exhaustive_structs` and `clippy::exhaustive_enums`)
enabled in `rubix-spi`'s `Cargo.toml`. PRs that add a public
struct/enum without `#[non_exhaustive]` fail lint.

## Publishing model (transitional)

Per SCOPE.md "Decisions made" §"Publishing model (R7)": while
`rubix` lives as a sibling tree to `starter/crates/`, extensions
path-dep `agent-sdk`, `agent-client-rs`, and `contracts/spi` only
— CI-enforced. **The R10 rules apply to the in-tree path-deps
exactly as they will to the registry-published versions.** A
breaking change to `rubix-spi` while in-tree is still a major bump
on every consumer; the only difference at the cut-over to registry
publishing is that `Cargo.toml` switches from `path = "../..."` to
`version = "X.Y.Z"`.

This is the load-bearing reason CI enforces the path-dep
restriction: it lets us iterate quickly without losing the contract.
A block written against `agent-sdk = { path = "../../agent-sdk" }`
compiles unchanged against
`agent-sdk = "1.0"` once we publish.

## Deprecation cycle

When removing a public item:

1. **Mark `#[deprecated(since = "X.Y.0", note = "use Y")]`** in a
   minor release of the *current* major.
2. Update consumer-facing docs to point at the replacement.
3. **Remove in the next major.** The deprecation lived for at
   least one major minor cycle before deletion.

The deprecation note must always name the replacement. A
`#[deprecated(note = "old API, use the new one")]` without naming
the new API is rejected at review — every deprecation is a pointer.

For TS / Dart consumers: use `@deprecated` JSDoc + a runtime
console warning the first time the deprecated path is hit.

## Manifest migrations — the forward-only shape

When a kind's `manifest.version` bumps in a way that retypes or
restructures persisted slot data, ship a **manifest migration**
alongside:

```rust
impl KindMigration for PointWritableV2 {
    fn from_version() -> u32 { 1 }
    fn to_version()   -> u32 { 2 }

    fn migrate(persisted: &mut PersistedSlots) -> Result<(), Error> {
        // Transform v1 shape → v2 shape on this row.
        if let Some(old) = persisted.remove("value_str") {
            let parsed: f64 = old.as_string()?.parse()?;
            persisted.insert("value", SlotValue::F64(parsed));
        }
        Ok(())
    }
}
```

The kinds-registry walks the persisted graph at boot and applies
the migration per row before the engine begins serving. Forward-
only: there is no `migrate_back`. If a v2-shape persisted row needs
to roll back, ship a v3 migration that re-derives the v1 shape.

## Binary version compatibility at boot

The agent binary verifies at boot:

- **The persisted graph's manifest versions are ≤ the registered
  manifests' versions.** A persisted v3 row against a v2 register
  is a refusal to boot with a clear error ("downgrade detected;
  re-deploy the v3 binary or run the downgrade migration").
- **The connected clients' major matches the binary's major.** A
  REST handler returns 426 Upgrade Required if the
  `X-Rubix-Client-Major` header (sent by `agent-client-*`) doesn't
  match.
- **The block-process manifests' R10 schema version matches the
  binary's.** Mismatched manifests refuse to register, the host
  comes up with the rest of the blocks (already the Phase 7
  pattern — broken extension refuses to mount, host stays alive).

## CHANGELOG conventions

Each PR that ships a public-surface change adds an entry under
`CHANGELOG.md`'s `[Unreleased]` section:

```markdown
## [Unreleased]

### Added (additive)
- `rubix_spi::manifest::Capabilities::sdui` — kinds can declare SDUI emission.

### Changed (breaking)
- `KindManifest::id` is now `KindId` (was `&'static str`). Construct via
  `KindId::parse`.

### Deprecated
- `rubix_spi::manifest::OldFacets` — use `Facets` instead. Removed in 2.0.

### Removed
- (nothing)

### Fixed
- Slot writes no longer race when two propagator ticks coalesce on the same slot.
```

At release time, `[Unreleased]` is renamed to the new version and
the date is filled in. The CHANGELOG is the single source of truth
for what each version added; PR descriptions reference the
CHANGELOG entry rather than restating the change.

## Phase 0 expectation

No public surface ships in Phase 0 beyond the skeleton crates +
the eight design docs. The first R10-relevant bump happens when
Phase 1 lands a non-empty `rubix-spi` with `KindManifest` and `Msg`.
That bump is `0.1.0 → 0.2.0` (minor — additive against an empty
`0.1.0`). The `1.0.0` tag is reserved for the day a third-party
block author can build against a registry-published
`rubix-extensions-sdk` and run against a deployed agent —
post-Phase 4, by the SCOPE's phase ordering.
