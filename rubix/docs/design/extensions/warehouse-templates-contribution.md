# `contributes.warehouse_templates[]`

> **Tier:** design (canonical). Lifetime: durable. Describes the
> shipped row-3 manifest slice of the extensions north-star scope.
> Plan / "we will" framing lives in
> [docs/scope/extensions-north-star/](../../scope/extensions-north-star/README.md);
> the row-2 handle the slice feeds is
> [warehouse-read.md](./warehouse-read.md).

## What

Extensions can grow the host's warehouse-read catalog by listing
templates under `contributes.warehouse_templates[]` in
`block.yaml`. Each entry produces a
[`TemplateSpec`](../../../../starter-extensions/crates/starter-ext-spi/src/warehouse.rs)
which `starter-ext-host::TemplateRegistry` inserts alongside the
four host builtins.

```yaml
# block.yaml
id: com.acme.charts
requires:
  - { id: cap.warehouse_read, version: "^1" }
capabilities:
  - kind: warehouse_read
    tables: [samples]
contributes:
  warehouse_templates:
    - name: com.acme.charts.daily_kwh_by_site
      params_schema: schemas/daily_kwh.json
      tables: [samples]
      sql_file: sql/daily_kwh.sql
```

## Schema

[`ContributeWarehouseTemplate`](../../../../starter-extensions/crates/starter-ext-spi/src/manifest.rs):

| field           | required | shape                          |
|-----------------|----------|--------------------------------|
| `name`          | yes      | reverse-DNS string             |
| `params_schema` | yes      | bundle-relative path to JSON   |
| `tables`        | yes      | `Vec<String>`                  |
| `sql_file`      | no       | bundle-relative path to text   |

Both file references are static (R7 — never templated at runtime).

## Load-time rules

`starter-ext-host::validate_manifest` enforces, in order:

1. **Reserved-prefix.** `name` may not start with `starter.` or
   `sys.` — host-owned. Reported as a distinct error so an
   operator can act ("rename the template").
2. **Namespace ownership** (R4). `name` must equal the extension
   id or be a dotted descendant. `com.acme.charts.daily_kwh`
   is legal for an extension whose id is `com.acme.charts`;
   `com.other.thing` is not.
3. **Capability compatibility** (R6). If the extension's
   `requires:` block lists `cap.warehouse_read`, the manifest's
   `capabilities:` block must include a `warehouse_read` grant
   (an empty `tables: []` allowlist is the legal neutralised
   form, matching every other category — see
   [warehouse-read.md](./warehouse-read.md)).

A failure on any of these puts the extension in the `Failed`
lifecycle state — the rest of the registry is unaffected
(SCOPE "Bad manifest is isolated to its own extension").

## Post-commit wiring

After `Loader::commit` has populated the registry the host
integration crate (`rubix-agent` for the rubix product) folds
each validated record's contributions into the registry:

```rust
let mut registry = TemplateRegistry::builtin();
for rec in ext_registry.iter().filter(|r| r.is_validated()) {
    registry.extend_from_record(rec)?;
}
let registry = Arc::new(registry);
```

[`TemplateRegistry::extend_from_record`](../../../../starter-extensions/crates/starter-ext-host/src/warehouse.rs):

- reads `params_schema` from `record.bundle_dir`, parses it as
  JSON (syntactically — schema *meaning* is not validated here;
  the eventual `warehouse.query` path will validate the call's
  `params` against this fragment);
- reads `sql_file` if present, stored verbatim into
  `TemplateSpec::sql` for audit / admin surfaces;
- inserts the spec under its `name`.

I/O is bounded to the bundle directory.

### Shadowing

A contributed template whose name matches an existing entry
(builtin or another extension's contribution) silently
**replaces** the prior entry — matching every other contribute
slice for row 3. The row-3 follow-up is a `name:` collision
checker that emits a load-time error unless the entry carries
an explicit `override:` flag (deferred until the first
collision is reported in practice).

## Per-template table allowlist

The supervisor's `warehouse` namespace is gated under the
`WarehouseRead` capability today; the grant's `tables: [...]`
allows the call onto the namespace.

**Deferred**: per-call cross-check that the invoked template's
`TemplateSpec::tables` is a subset of the grant's `tables`.
This lands when row 5's host dispatch wiring turns the stubbed
`WarehouseReadBackend` into a real resolver — at that point the
backend already needs the `TemplateRegistry` for spec lookup,
so the allowlist check is "look up the spec, intersect with the
grant" with no extra plumbing.

## What is not in row 3

- **Override-or-collision-error semantics.** Today: silent
  replace. Becomes a hard error + explicit `override:` flag in
  a follow-up.
- **Schema-meaning validation.** The JSON Schema is parsed only
  as JSON; it is run against call-site `params` later (row 5
  resolver work).
- **SQL execution.** `sql_file` is descriptive. The resolver
  that runs SQL still lives in `rubix-agent`; row 5 wires
  contributed templates into that path.
- **Wasm flavour support.** Builtin + process only for row 3;
  wasm follows once the wasm capability backend lands.

## Tests

- `starter-ext-spi` (`manifest.rs`): unchanged round-trip tests
  cover the new contribute field via `Contributes::default()`
  and `deny_unknown_fields`.
- `starter-ext-host` (`validate.rs`): `warehouse_templates`
  namespace-ok / reserved-prefix / namespace-mismatch /
  `cap.warehouse_read` grant-satisfies / grant-missing.
- `starter-ext-host` (`warehouse.rs`): `extend_from_record`
  reads schema + sql; optional sql; missing schema errors;
  invalid JSON errors; no-manifest is a no-op.
