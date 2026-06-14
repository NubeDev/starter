# WS-17 — Extension Data Access: own tables in the nexus DB + full datasource CRUD

> Status: **IMPLEMENTED** (Wave A + Wave B). See §10 for what shipped.
> (Original design below kept for context.)
> Extends [WS-14 §4.3 Capability host-methods](WS-14_EXTENSIONS_RUNTIME.md).
> Relates to [WS-08 Datasources](WS-08_DATASOURCES.md), [WS-10 Kinds](WS-10_KINDS_EXTENSIBILITY.md).
> Reference impl to port from: `rubix-agent` (`boot/extension_tables.rs`,
> `extensions/warehouse_write.rs`, `extensions/host_methods.rs`).

---

## 1. The idea in one paragraph

A nexus extension is currently a **read-only** data citizen: it can run a
contributed query-kind over the nexus Postgres (`warehouse.query`) but it cannot
**persist** anything of its own. This workstream makes an extension a **first-class
data citizen** with two clearly-separated capabilities:

1. **Own tables in the nexus database.** An extension declares tables in its
   manifest (`contributes.warehouse_tables[]`); nexus creates them at boot as
   `<extension_id>__<table>` (the extension-id prefix is the ownership + isolation
   boundary) and exposes a tenant-stamped **write** host-method. This is the same
   model rubix already ships — ported to nexus Postgres.
2. **Access datasources.** An extension can run **full CRUD against any configured
   nexus datasource** through new `datasource.*` host-methods, tenant-scoped and
   capability-gated — mirroring the human `POST /api/v1/datasources/{id}/query`
   path. If an extension wants to *create* a table inside a datasource (rather
   than just CRUD existing data), that table must also carry the
   `<extension_id>__<table>` prefix, so ownership is unambiguous wherever the
   bytes live.

> **Naming + database note (read this first).**
> - **nexus is Postgres, period.** Both `AppState` pools are `PgPool` over
>   `postgres://` (`state.rs:33,37`; `main.rs:42,48`). **There is no ClickHouse
>   and no separate warehouse database in nexus** — `grep -rn clickhouse
>   nexus/backend/src` finds only a doc-comment example in `dialect.rs`. Every
>   table in this doc is a plain **Postgres** table in the nexus DB.
> - **"Warehouse" is a rubix leftover.** `warehouse.query` already just runs SQL
>   against the nexus Postgres `metadata` pool (`host_methods.rs:297-298`,
>   `self.state.metadata`). This doc keeps the `warehouse_tables[]` manifest
>   **field name** only because it is the shared kernel SPI schema (renaming
>   would fork the kernel) — but the mental model is **"an extension owns a
>   Postgres table in the nexus DB."**
> - The SPI manifest's column-type examples (`Float64`, `MergeTree`, …) are
>   **ClickHouse-flavoured doc-comments written for rubix** — they do **not**
>   apply to nexus. In nexus, `warehouse_tables[].columns[].ty` are **Postgres
>   types** (`text`, `timestamptz`, `double precision`, …). See Q1.

---

## 2. What already exists — evidence

**Kernel / SPI (shared, done):**
- `contributes.warehouse_tables[]` manifest schema — `ContributeWarehouseTable`
  (`starter-ext-spi/src/manifest.rs:641`): `name`, `columns`, `order_by`,
  `engine`, `partition_by`, `ttl`, `kind` (`Table` ⇒ host issues
  `CREATE TABLE IF NOT EXISTS`; `ContinuousAggregate` ⇒ host skips DDL). The host
  **prepends a `tenant_id` column** at position 0; the extension must not declare
  it. (Type strings are passed verbatim to the engine DDL — written
  ClickHouse-first; nexus must emit **Postgres** types.)
- `WarehouseReadHandle` is a **named-template catalog, NOT a SQL gateway**
  (`starter-ext-host/src/warehouse.rs`). `starter-ext-host` is I/O-free — no
  sqlx. So all execution lives in the host integration crate.

**Rubix (the reference implementation — port from here):**
- `rubix-agent/src/boot/extension_tables.rs` — creates each
  `contributes.warehouse_tables[]` as `<sanitized_ext_id>__<name>` at boot.
- `rubix-agent/src/extensions/warehouse_write.rs` — `full_table_name`,
  `sanitize_extension_id`, the per-call `warehouse_tables[]` allowlist.
- `rubix-agent/src/extensions/host_methods.rs` — `warehouse.write` ⇒
  `RubixWarehouseWriteBackend::insert` (tenant-stamped).

**Nexus (what's wired today):**
- Host methods routed (`nexus-api/src/extensions/host_methods.rs:129`): only
  `authz.check`, `dashboard.read`, `warehouse.query`/`warehouse.read`,
  `ingest.write`. **No `warehouse.write`. No `datasource.*`.**
- `warehouse.query` runs a contributed query-kind's SQL against
  `self.state.metadata` (nexus Postgres), tenant-scoped by `$caller_tenant_id`
  (`host_methods.rs:280-306`). Read works.
- `ingest.write` (`extensions/ingest.rs`) pushes rows into a **running flow's**
  source channel — it requires a pre-existing flow whose sink targets a table.
  It is **not** a general row-write.
- Datasources: human CRUD + `POST /api/v1/datasources/{id}/query`
  (`routes/datasources/mod.rs`), which builds a `QueryIdentity` from the
  principal (`routes/datasources/query.rs:85`). `AppState.datasource_pools`
  (`datasource_pools.rs:23`) resolves a per-datasource connection.

---

## 3. The actual gaps — evidence

| Capability | Designed in SPI? | Wired in nexus? | Gap |
|---|---|---|---|
| Read via query-kind (`warehouse.query`) | ✅ | ✅ | none |
| **Own a table** (`warehouse_tables[]` → DDL at boot) | ✅ | ❌ | nexus never reads `warehouse_tables[]`, never issues DDL |
| **Write a row** (`warehouse.write`) | ✅ (rubix) | ❌ | no host method in nexus |
| **Datasource CRUD** (`datasource.query/execute`) | ⚠️ partial | ❌ | no host method at all |
| Create a table **in a datasource** | ❌ | ❌ | needs the prefix rule + a DDL-allowed write path |

Net: an extension can read nexus Postgres but cannot persist anything, and cannot
touch a datasource at all from a node/tool body.

---

## 4. Proposed design

### 4.1 Own tables in the nexus DB (Wave A — port rubix, Postgres-flavoured)

**4.1.1 Boot-time DDL.** In `extensions/boot.rs` (alongside the existing
contributed-query-kind materialisation), for each enabled extension read
`manifest.contributes.warehouse_tables[]` and issue
`CREATE TABLE IF NOT EXISTS <sanitized_ext_id>__<name> (...)` against
`state.metadata`, **Postgres-typed**:
- `<sanitized_ext_id>` = extension id with `.`/`-` → `_` (e.g.
  `com.acme.devices` → `com_acme_devices`), so a table is
  `com_acme_devices__devices`. The prefix is the ownership boundary.
- Prepend `tenant_id TEXT NOT NULL` at column 0 (host-owned; extension must not
  declare it). All rows are tenant-stamped on write and tenant-filtered on read.
- `columns[].ty` are **Postgres types**, used verbatim in the DDL (`text`,
  `timestamptz`, `double precision`, `boolean`, `jsonb`, …). nexus is Postgres;
  a nexus bundle author writes Postgres types directly. (The SPI's ClickHouse
  type examples are rubix doc-comments and do not apply — see Q1 for the one
  cross-product portability wrinkle.)
- `order_by` → a `PRIMARY KEY`/`UNIQUE` or index. The `engine` / `partition_by`
  / `ttl` fields are **ClickHouse-only and irrelevant to nexus**; if a (ported
  rubix) bundle sets them, nexus **ignores them and logs once at boot** so the
  degradation is visible, never silent.
- `kind: ContinuousAggregate` ⇒ skip DDL (extension owns creation), exactly as
  rubix.

**4.1.2 `warehouse.write` host method.** Route it in `host_methods.rs` beside
`warehouse.query`, gated by the same `warehouse` capability category:
- Params: `{ table, rows }` where `table` must be one of the **calling
  extension's own** declared `warehouse_tables[]` (per-call allowlist — an
  extension cannot write another's table or an arbitrary nexus table).
- Stamp the caller's `tenant_id` onto every row (overwrite any client-supplied
  value — same rule as `ingest.write`, `ingest.rs:29`).
- Insert via the `metadata` pool. Support an **upsert** mode keyed on a declared
  natural key (so idempotent nodes — DOCS §8c — don't double-insert on resume).
- A caller with no tenant is a hard deny (mirrors every tenant-scoped method).

**4.1.3 Read-back.** No new mechanism — the existing `warehouse.query` +
a contributed query-kind over `<ext>__<name>` already works, tenant- and
team-scoped (`$caller_tenant_id` / `$caller_team_ids`). The query-kind is the
read API; the owned table is the storage.

**4.1.4 Cleanup.** `DELETE /extensions/<id>?purge=true` must `DROP TABLE` the
extension's owned tables (a new cleanup provider beside the query-kind one in
`extensions/cleanup.rs`). Idempotent; dry-run lists the tables to be dropped.

### 4.2 Datasource access (Wave B — the nexus-native half)

New host methods, gated by a new `datasource` capability category, each taking a
**datasource id** the extension is authorised for and binding the caller's tenant
identity exactly like the human route (`routes/datasources/query.rs:85`):

- **`datasource.query { datasource_id, sql | kind, params }`** → read. Runs
  through the same guarded path the human `/datasources/{id}/query` uses
  (read-only, timeout, caps; host tokens `$caller_tenant_id` / `$caller_team_ids`
  bound from the caller). Returns rows.
- **`datasource.execute { datasource_id, statement, params }`** → write
  (INSERT/UPDATE/DELETE/DDL). **Guard rails:**
  - The datasource must be in the extension's declared, operator-approved
    datasource allowlist (a new manifest field, **Q2** — or reuse a capability
    grant naming the datasource id/kind).
  - Any **table the extension CREATEs** in a datasource must be
    `<extension_id>__<name>` (validated by parsing the DDL target, or by a
    convention check) — ownership is unambiguous wherever bytes live, per the
    user's rule. CRUD against **existing** (non-prefixed) tables is allowed only
    with an explicit, broader operator grant (**Q3** — "full CRUD of any data" is
    powerful; gate it deliberately).
  - Writes are tenant-stamped where the target table has a `tenant_id` column;
    for arbitrary external tables the extension is responsible (document the
    sharp edge).

**Datasource resolution** uses `AppState.datasource_pools`
(`datasource_pools.rs`) — the per-datasource connection already exists; the host
method is a thin, gated wrapper.

### 4.3 Capability model (both waves)

Extend the manifest's `Capability` set / the supervisor gate:
- `warehouse` (exists) — now covers `warehouse.query` **and** `warehouse.write`
  over the extension's own `warehouse_tables[]`.
- `datasource` (new) — covers `datasource.query` / `datasource.execute`, scoped
  to declared datasource ids/kinds. An extension capability is **never broader
  than the caller's grants** (the WS-14 §4.3 invariant) — a `datasource.execute`
  is additionally bounded by the prefix rule for CREATE and the operator grant
  for non-owned tables.

---

## 5. What an extension looks like (the deliverable shape)

`block.yaml`:
```yaml
contributes:
  warehouse_tables:                       # owns a table in the nexus DB
    - name: devices                       # → com_acme_devices__devices
      columns:
        - { name: device_id, ty: text }   # tenant_id prepended by the host
        - { name: barcode,   ty: text }
        - { name: location,  ty: text }
        - { name: sensor_id, ty: text }
        - { name: owner,     ty: text }
        - { name: created_at, ty: timestamptz }
      order_by: [device_id]               # → PRIMARY KEY on (tenant_id, device_id)
  warehouse_templates:                    # the read API over that table
    - name: com.acme.devices.devices_list
      sql_file: kinds/devices_list.sql    # SELECT … WHERE tenant_id = $caller_tenant_id
      tables: [devices]
```

Node body (process child), using the SDK host-call:
```rust
// device_create — now PERSISTS the device (idempotent upsert on device_id).
ctx.warehouse().write("devices", json!([{
    "device_id": device_id, "barcode": barcode,
    "location": location, "owner": owner,
}]))?;   // tenant_id stamped by the host
```

The panel's new **Devices** page calls the read kind via `warehouse.query` (or a
thin REST/tool wrapper) and renders the real rows — tenant- and team-scoped.

---

## 6. Scope (this workstream)

**Wave A — own tables in the nexus DB (port rubix to Postgres):**
1. Boot-time `CREATE TABLE IF NOT EXISTS <ext>__<name>` from
   `warehouse_tables[]`, Postgres-typed, `tenant_id` prepended (§4.1.1).
2. `warehouse.write` host method — own-table allowlist, tenant-stamp, upsert
   (§4.1.2).
3. `DROP TABLE` cleanup provider on purge (§4.1.4).
4. SDK Rust `ctx.warehouse().write(...)` helper (Postgres backend).

**Wave B — datasource access:**
5. `datasource.query` host method (read, guarded path).
6. `datasource.execute` host method (write/DDL) with the prefix rule + grant gate
   (§4.2).
7. `datasource` capability category + manifest allowlist field.
8. SDK Rust `ctx.datasource(id).query/execute(...)` helpers.

**Demo (proves both):**
9. `com.acme.devices` declares + owns `devices`, `device_create` persists,
   `devices_list` reads back, the panel gets a **Devices table page**, and a
   `hvac-ops` user sees only their team's rows (P3a scoping). A dashboard +
   nav + user/team walkthrough ties it together.

## 7. Acceptance criteria

- [ ] At boot, `com_acme_devices__devices` exists in nexus Postgres with a
      host-prepended `tenant_id` column; re-boot is idempotent (no error).
- [ ] `device_create` running the live automation **inserts a real row**; the
      same barcode upserts (no duplicate) — idempotency proven against the table,
      not just a derived id.
- [ ] `warehouse.write` refuses a table the calling extension does not own, and
      refuses a caller with no tenant.
- [ ] `devices_list` query-kind returns only the caller's tenant's rows; a
      `hvac-ops` user sees their team's rows via `$caller_team_ids`.
- [ ] `DELETE …?purge=true` drops the owned table; dry-run lists it first;
      re-purge is idempotent.
- [ ] `datasource.query` runs read-only under the caller's tenant against a named
      datasource; `datasource.execute` creating a non-`<ext>__` table is rejected;
      CRUD against a non-owned table requires the explicit operator grant.
- [ ] The panel's Devices page lists real persisted devices; existing nexus tests
      stay green.

## 8. Out of scope (defer / hand off)

- **Per-extension migrations beyond `warehouse_tables[]`** (arbitrary multi-stmt
  DDL packs like rubix's `scripts/post-load.sql`) — start with the declarative
  table schema; add a migration-source seam only when a bundle needs it.
- **Columnar/analytics engines** (ClickHouse `MergeTree`, Timescale hypertables,
  `partition_by`, `ttl`) — **not part of nexus.** nexus owns plain Postgres
  tables; the ClickHouse-flavoured SPI fields are accepted-but-ignored with a
  boot log purely so a ported rubix manifest doesn't error. If a columnar store
  is ever wanted, it arrives as a separate **datasource** (WS-08), not as a
  nexus-owned warehouse.
- **Cross-extension table sharing / foreign keys** — each extension owns its own
  prefix; sharing is via query-kinds, not direct table refs.
- **Datasource *connector* breadth** — owned by [WS-08](WS-08_DATASOURCES.md);
  this WS only adds the extension *access* path over whatever connectors exist.

## 9. Open questions to settle first

- **Q1 — column types (nexus = Postgres types directly).** Recommended and
  assumed throughout this doc: `warehouse_tables[].columns[].ty` are **Postgres
  types** in nexus, used verbatim. The only open wrinkle is *cross-product
  portability* — a bundle authored for rubix (ClickHouse types) won't create on
  nexus and vice-versa. Options: (a) accept that bundles are product-specific
  (simplest, recommended — nexus authors write Postgres types); or (b) add an
  optional per-engine type map later if a single bundle must target both. **No
  ClickHouse support is implied either way.**
- **Q2 — datasource allowlist shape.** A new `contributes.datasources[]` /
  `requires.datasources[]` manifest field naming ids/kinds, or a capability grant
  string (`datasource:<id>`)? The latter reuses the existing capability gate.
- **Q3 — "full CRUD of any data" gate.** The user wants extensions to be able to
  CRUD any datasource data, but that is broad. Default to: **owned-prefix tables
  freely; non-owned tables only under an explicit, operator-visible grant** shown
  in the install/cleanup UI. Confirm the default.
- **Q4 — upsert key declaration.** Where does the natural key for idempotent
  `warehouse.write` upserts come from — a new `warehouse_tables[].upsert_key`
  field, or inferred from `order_by`/the PRIMARY KEY? Lean: reuse `order_by` as
  the conflict target.

---

## 10. What shipped (implementation notes)

**Decisions taken** (the §9 open questions, resolved as recommended):
- **Q1** — `columns[].ty` are **Postgres types**, used verbatim in the DDL.
- **Q2** — datasource allowlist is a **capability grant**:
  `Capability::Datasource { datasources: [<uuid>…], allow_foreign_tables: bool }`
  (no new `contributes`/`requires` manifest field — it reuses the existing
  capability gate).
- **Q3** — `datasource.execute` allows **owned-prefix tables freely**; a CREATE
  must target `<ext>__<table>`; CRUD against a **non-owned** table requires the
  explicit `allow_foreign_tables: true` flag on the grant.
- **Q4** — the upsert conflict target is the table's **`order_by`** (which the
  boot DDL makes the `PRIMARY KEY (tenant_id, order_by…)`).

**Wave A — own tables in the nexus DB** (`nexus-api/src/extensions/warehouse.rs`):
- Boot DDL: `extensions::boot` calls `warehouse::create_extension_tables`, which
  `CREATE TABLE IF NOT EXISTS <ext>__<name>`s every validated extension's
  `warehouse_tables[]`, `tenant_id TEXT NOT NULL` prepended, Postgres-typed, with
  `PRIMARY KEY (tenant_id, order_by…)`. Idempotent. Non-`Table` kinds
  (continuous aggregates) are deferred to the extension. If the runtime DB role
  differs from the DDL/owner role, set `NEXUS_RUNTIME_ROLE` so the boot DDL also
  `GRANT`s CRUD to it (single-role deployments leave it unset).
- `warehouse.write` / `.update` / `.delete` host methods (`host_methods.rs`)
  route to `warehouse::WriteExecutor`: own-table allowlist (manifest +
  `warehouse_write` grant), tenant clamp, column validation, and **upsert** on
  the `order_by` PK so a repeated natural key updates in place.
- `DROP TABLE` cleanup on purge: `cleanup::WarehouseTableCleanupProvider`
  (dry-run lists the owned tables; `purge` drops them; idempotent).
- Capability gate: `warehouse.write/.update/.delete` map to the `warehouse_write`
  category (a per-method-name override in `starter-ext-supervisor`), so a read
  grant cannot write.
- SDK side (`ctx.warehouse_write().insert/update/delete`) + the SPI DTOs already
  existed; this WS wired the **nexus host** end.

**Wave B — datasource access** (`nexus-api/src/extensions/datasource.rs`):
- `datasource.query` — read in a `READ ONLY` txn under the `statement_timeout`
  guard (writes phrased as queries are rejected by Postgres).
- `datasource.execute` — write/DDL, gated by the ownership-prefix rule + the
  `allow_foreign_tables` flag.
- Both resolve the datasource within the **caller's tenant** (the same
  `datasource::get(tenant, id)` the human route uses) and reuse
  `AppState.datasource_pools`.
- New `Capability::Datasource` SPI variant; `ctx.datasource().query/execute`
  SDK handle + `RealDatasourceBackend` (process flavour) wired through every
  adapter's stub.

**Demo** (`nexus/extensions/com.acme.devices`): declares + owns the `devices`
table, `device_create` **persists** an upserted row via `ctx.warehouse_write()`,
the `devices_list` query-kind reads it back tenant-/team-scoped
(`$caller_tenant_id` / `$caller_team_ids`), and the panel renders a live
**Provisioned devices** table.

**Tests**: `nexus-api` unit tests for the SQL builders, the prefix rule, and the
capability gate; a docker-gated e2e
(`tests/extensions/warehouse_write_e2e_test.rs`) proving boot DDL → tenant-stamped
persist → upsert idempotency → tenant isolation → allowlist refusal →
tenant-scoped delete → `DROP TABLE`.

**AppState**: gained `extensions: Arc<ExtensionRegistry>` so a host method can
read the **calling** extension's manifest at request time (the own-table
allowlist and the datasource grant).
