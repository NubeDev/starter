# Warehouse read capability

> **Tier:** design (canonical). Lifetime: durable. Describes the
> shipped row-2 capability of the extensions north-star scope. The
> plan / "we will" framing lives in
> [docs/scope/extensions-north-star/](../../scope/extensions-north-star/README.md);
> Appendix A of
> [docs/proposal/extension-architecture-north-star.md](../../proposal/extension-architecture-north-star.md)
> covers the target shape.

## What

The `warehouse_read` capability lets an extension issue
**named-template** queries against the host's time-series store
(`samples`, `events`, …). The handle is not a SQL gateway: the
host owns a finite catalog of templates ([`TemplateRegistry`]);
extensions reference them by name and bind typed parameters.

## Why

Power-BI-style dashboard authoring needs to read tenant-scoped
history without either (a) the extension authoring SQL (audit
black hole) or (b) the extension going through the
`HttpOutHandle` → host-loopback shortcut (no enforced tenancy).
A typed handle backed by a server-defined template catalog
solves both.

## Capability grant

The manifest grants `warehouse_read` with a tables allowlist:

```yaml
# block.yaml
capabilities:
  warehouse_read:
    tables: [samples]
```

An empty `tables` vec is the legal neutralised form: the
extension loads, every query is denied. The supervisor's
capability gate cross-checks the invoked template's
[`TemplateSpec::tables`] against the grant — a template touching
`events` cannot be invoked by an extension whose grant was
`tables: [samples]`.

The wire-method namespace is `warehouse.*`; the only method shipped
in row 2 is the read path (the writer story is out of scope —
warehouse mutations stay on the host's existing ingestion path).

## SDK surface

`requires!(warehouse_read)` brings the `WarehouseReadHandle`
accessor onto the per-extension Ctx:

```rust
starter_ext_sdk::requires! {
    name = DashboardCtx,
    capabilities = [warehouse_read, tracing],
}

// inside a handler:
let rows = ctx.warehouse_read()
    .query("meter_value_30d_15m", json!({
        "tenant_id": ctx.caller().and_then(|c| c.tenant_id.as_deref()).unwrap_or(""),
        "meter_id":  meter_id,
    }))?;
```

The handle methods are:

- `query(template, params) -> Vec<Row>` — execute the template,
  validate `params` against the spec's schema, return rows.
- `count(template, params) -> u64` — row count without
  materialising the result set.
- `describe(template) -> Option<TemplateSpec>` — catalog
  introspection.

The v1 shape is **sync** (`Vec<Row>`). Appendix A's streaming
target (`BoxStream<Row>`) lands as a v2 (`query_stream`) when
the first extension's working set exceeds the row cap; the v1
method stays on the handle per the proposal's "add v2,
keep v1" deprecation rule.

`Row` is a `serde_json::Map` newtype — the kernel does not
interpret column types; the template's documented schema is the
contract.

## TemplateRegistry

`TemplateRegistry` is an in-process catalog of
[`TemplateSpec`](../../../../starter-extensions/crates/starter-ext-spi/src/warehouse.rs)
entries:

```rust
pub struct TemplateSpec {
    pub name:    String,
    pub params:  serde_json::Value,   // JSON Schema fragment
    pub tables:  Vec<String>,
    pub sql:     Option<String>,      // descriptive; not executed by this crate
}
```

The registry lives in
[`starter-ext-host::warehouse`](../../../../starter-extensions/crates/starter-ext-host/src/warehouse.rs).
It depends only on `starter-ext-spi` — no `sqlx`, no warehouse
client, no tenant-store crate. Resolvers (the code that binds
parameters and runs SQL) live in the host integration crate
(`rubix-agent` for the rubix product) and consult the registry
only for spec lookup.

### Builtin templates

`TemplateRegistry::builtin()` returns the four templates currently
hard-coded inside
[`rubix-agent/sdui/analytics_bridge.rs`](../../../crates/rubix-agent/src/sdui/analytics_bridge.rs):

| name                    | tables   | params                  |
|-------------------------|----------|-------------------------|
| `meter_kwh_last_24h`    | samples  | `tenant_id`             |
| `meter_litres_last_24h` | samples  | `tenant_id`             |
| `meter_value_30d_15m`   | samples  | `tenant_id`, `meter_id` |
| `meter_value_24h_1m`    | samples  | `tenant_id`, `meter_id` |

The SQL bodies are captured in `TemplateSpec::sql` for audit /
documentation; the bridge's `sqlx::query_as` calls still execute
the SQL (matched by name). This is the row 2 cut-over: the
catalog is centralised; resolver code is unchanged.

### Extension-contributed templates

Row 3 of the critical path adds the
`contributes.warehouse_templates[]` manifest slice. When that
lands, `Loader::commit` will call `TemplateRegistry::insert` for
each contributed spec so the registry is a single audit surface
across builtin + contributed entries.

## Caller-identity coupling

Per Appendix A: the host binds `$caller_tenant_id` from
`ctx.caller()` before executing a template — extensions cannot
override it. A frame with `caller().is_system()` will be refused
by the host-side `WarehouseReadBackend` impl with
`Error::Capability` once the resolver lands; until then the
process-flavour stub returns the same error verbatim.

This is the first real consumer of
[caller-identity.md](./caller-identity.md): if a tool is invoked
without a tenant-scoped caller, the warehouse read refuses
rather than silently leaking cross-tenant rows.

## What is not in row 2

- **Host-side wasm/process resolver**. Row 2 ships the SDK
  surface and the SPI/supervisor wiring. The concrete
  `WarehouseReadBackend` impl that bridges JSON-RPC
  `warehouse.query` to the `TemplateRegistry` lands when the
  host's dispatch table wires capability-gated host methods
  end-to-end (row 5 timeline — paired with `DashboardHandle`).
  Process-flavour calls today return
  `Error::Capability("warehouse_read not wired …")`.
- **Streaming results** — see SDK surface section above.
- **Contributed templates** — row 3.
- **Closed-grammar calculated fields** — deferred per the scope's
  open-questions list.

## Tests

- `starter-extensions/crates/starter-ext-spi/src/warehouse.rs` —
  `TemplateSpec` / `WarehouseReadRequest` / `Row` round-trip.
- `starter-extensions/crates/starter-ext-spi/src/capability.rs` —
  `Capability::WarehouseRead` round-trip.
- `starter-extensions/crates/starter-ext-supervisor/src/capability.rs` —
  `warehouse.query` gated under `warehouse_read` grant.
- `starter-extensions/crates/starter-ext-host/src/warehouse.rs` —
  `TemplateRegistry::builtin` registers all four templates; lookup
  returns the right spec; `with` replaces existing entries.
