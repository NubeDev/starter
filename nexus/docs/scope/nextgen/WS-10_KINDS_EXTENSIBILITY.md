# WS-10 — "Kinds": Declarative Query & Datasource Extensibility

> **Status:** Proposal · **Wave:** 0 (contract) + 1 (query-kinds) + 2 (datasource-kinds) · **Owner:** _unassigned_
> **Depends on:** C2 macro/param engine (WS-03) — *and reshapes it* · **Reshapes:** WS-03, WS-08, WS-09
> **Migration:** block `14xx` (e.g. `1401_query_kinds.sql`; optional — manifest-only kinds need no table) · **Read first:** GAP_ANALYSIS §2.3/2.8/2.9, ROADMAP §0
> **Verified:** `82a6a19a` on 2026-06-09 — re-grep this WS's file:line claims (incl. rubix `kinds/`) before building (ROADMAP §0).
>
> **Origin:** ported from the rubix `kinds/` convention in
> `rubix/extensions/com.nubeio.rubixos/` (manifest: `block.yaml`; templates: `kinds/*.sql` +
> `kinds/*_params.json`; cache sidecars: `kinds/*.cache.yaml`). This is the single change that
> makes nexus's API surface *genuinely extensible* — add a queryable API by **dropping files**,
> not recompiling — and it folds three separate workstreams (query authoring, connectors, caching)
> into one coherent mechanism.

---

## 1. The idea in one paragraph

A **kind** is a unit of API surface declared by **convention + manifest**, not code. Today, adding
a queryable thing to nexus means raw SQL pasted into a panel (unsafe, unshareable, uncacheable) or
a Rust code change (a new connector). The rubix pattern offers a third, better path: a **named,
JSON-Schema-validated, parameterized query** whose SQL is a file, whose params are a schema, whose
tenant isolation is **structural** (a host-bound `$caller_tenant_id` the caller cannot supply), and
whose caching is a **declarative sidecar**. Panels call a kind by id with validated params instead
of pasting SQL. We adopt this in two flavours — **query-kinds** (high value, do first) and
**datasource-kinds** (extensible connectors) — and make the existing raw-SQL path the *advanced
escape hatch* rather than the default.

---

## 2. What rubix does (the pattern we're porting) — evidence

A kind = a triple of files wired by **naming convention**, declared once in the manifest:

```yaml
# rubix block.yaml
warehouse_templates:
  - name: com.nubeio.rubixos.bc_devices_list      # reverse-DNS id
    params_schema: kinds/bc_devices_list_params.json   # JSON Schema, additionalProperties:false
    sql_file:      kinds/bc_devices_list.sql           # the query
    tables: [com_nubeio_rubixos__bc_devices]           # capability grant + cache invalidation
```

The SQL uses **named params + a host-bound token**:
```sql
-- kinds/bc_devices_list.sql
SELECT device_id, name, status, site_id
FROM   com_nubeio_rubixos__bc_devices
WHERE  tenant_id = $caller_tenant_id        -- host-bound; caller CANNOT supply it
  AND  ($site_id = '' OR site_id = $site_id)
ORDER  BY site_id NULLS LAST, device_id
LIMIT  $limit;
```
The params schema validates input *before* execution (defaults, min/max, `additionalProperties:
false`). The time-series template `history_bucketed.sql` shows the same pattern doing a Timescale
`time_bucket($bucket::interval, ...)` with host-bound `$caller_tenant_id` + caller `$from/$to`.

A **single proxy tool** (`warehouse_query`) takes `{ template, params }`, refuses any template
outside the extension's namespace, validates params host-side, binds the tenant, and runs it via a
`WarehouseReadHandle`. Optionally, a **sidecar** `kinds/<id>.cache.yaml` adds declarative caching:
`ttl`, `scope: user|tenant`, `invalidate_on.tables: [...]`, even a `time_series` bucket decomposition
(closed buckets cache 24h, the open tail 30s). Caching is **config, not code**.

**Five conventions doing the work:**
1. **kind = file triple** (`<name>.sql` + `<name>_params.json` + `<name>.md`) — naming *is* the wiring.
2. **host-bound security tokens** (`$caller_tenant_id`) — isolation is structural, not a forgettable check.
3. **JSON Schema as the contract** — validates input, documents the API, codegen-friendly.
4. **`tables:` = capability + cache key** — drives read/write grants *and* invalidation.
5. **cache sidecar** — opt-in declarative TTL/scope/invalidation/bucketing.

The whole barcode feature is removable by deleting its `bc_*` files + manifest lines. **Features are
additive file-drops, removable by deletion, with zero engine changes.**

---

## 3. Why it fits nexus — and what it trims

Nexus already has the primitives this pattern is a front-end over: host-bound tenant context
(`SET LOCAL app.tenant_id`, `tenant_tx.rs`), the read-only/timeout/row+byte guards
(`nexus-store/src/query/run.rs`), and OpenAPI-DTO discipline. Kinds are the **declarative surface
over exactly those.**

| Gap / workstream | What it specced | What kinds give instead |
|---|---|---|
| **WS-03** query authoring | raw SQL + a macro engine | **named validated query-kinds** as the safe default; raw SQL = advanced escape hatch. The macro engine *becomes* the kind param-binder (one engine, two front doors) |
| **WS-08** connectors | a Rust `DatasourceKind` enum + scattered forms | **datasource-kinds**: `{config_schema, secret fields, test query, dialect}` declared in a manifest → new source = file-drop + thin builder |
| **WS-09** caching | invent an LRU keyed by sql+vars | **port rubix's `.cache.yaml` sidecar** wholesale — ttl/scope/invalidate-on-tables + time-series bucketing, a designed spec not a blank page |

It also de-risks two product bets: **AI-generated panels** ("Ask Nexus" picks a *kind + params*, never
arbitrary SQL — bounded, safe, cacheable) and **GitOps dashboards** (kinds are files in a repo).

---

## 4. Proposed design for nexus

### 4.1 Two kinds of `kind`

**A. Query-kinds (Wave 1 — the high-value one).**
A named, parameterized, validated query a panel/variable invokes by id instead of pasting SQL.

```yaml
# a "core kinds" pack shipped with nexus, OR an extension manifest
query_kinds:
  - name: nexus.energy.usage_bucketed
    params_schema: kinds/usage_bucketed_params.json   # JSON Schema, additionalProperties:false
    sql_file:      kinds/usage_bucketed.sql
    datasource_kind: postgres        # which datasource shape this targets (see §4.4 binding)
    tables: [meters, histories]      # read-capability + cache invalidation
    cache: kinds/usage_bucketed.cache.yaml   # optional sidecar
```
```sql
-- kinds/usage_bucketed.sql  (host-bound + caller params + time/interval injected)
SELECT time_bucket($__interval::interval, ts) AS bucket,
       site_id, sum(kwh)::float8 AS kwh
FROM   meters
WHERE  tenant_id = $caller_tenant_id      -- host-bound, structural isolation
  AND  ts >= $__timeFrom AND ts < $__timeTo   -- host-injected from WS-01 time range
  AND  ($site_id = '' OR site_id = $site_id)  -- caller param, schema-validated
GROUP  BY bucket, site_id
ORDER  BY bucket;
```
Panel query model becomes a **discriminated union** (extend `PanelQuery` in `ui/src/data/types.ts`):
```ts
type PanelQuery =
  | { mode: "sql";  datasourceId: string; sql: string; params?: ... }   // existing escape hatch
  | { mode: "kind"; datasourceId: string; kind: string; params?: Record<string, unknown> }
```

**B. Datasource-kinds (Wave 2 — WS-08 expressed declaratively).**
A datasource type declared as files: a config JSON-Schema (host/port/db…), which fields are secrets,
a `test` query, and the dialect's time-bucket/`$__timeFilter` mapping. Adding Prometheus/InfluxDB/
HTTP-REST becomes a manifest entry + a thin builder, not enum edits scattered across DTOs+forms+registry.

### 4.2 Parameter model — three tiers (the security spine)

1. **Host-bound tokens** (caller can never supply; rejected if present in input):
   `$caller_tenant_id`, `$caller_user_id`. Bound from `Principal`. *This is the structural isolation.*
2. **Host-injected context**: `$__timeFrom`, `$__timeTo`, `$__interval` (from WS-01),
   and dashboard variables `$site_id` etc. (from WS-02) — all flow through the **same binder**.
3. **Caller params**: validated host-side against `params_schema` (`additionalProperties: false`,
   defaults, min/max, patterns) **before** the SQL runs.

> **This is the keystone:** the WS-03 "macro engine" and the kinds "param binder" are the **same
> component**. It injects time/vars/host-tokens, validates caller params, and **emits every value as a
> bound `$N` parameter** (returning `BoundQuery {sql, args, validated_identifiers}` — NOT a finished
> SQL string; see WS-03 §"Macro engine") — for *both* raw-SQL panels and kinds. Build it once. (C2 in
> the roadmap — its signature carries the param map + host-bound token set; freeze that in Wave 0.)

### 4.3 Dispatch & governance
- A single guarded entry: `POST /api/v1/query` accepts either `sql` or `{kind, params}`.
- For a kind: resolve name → registered kind → validate params against schema → bind host tokens →
  inject time/vars → run under the **existing** read-only/timeout/caps guards (`query/run.rs`,
  unchanged). `tables:` is checked against the bound datasource's read-capability.
- Namespacing: kind names are reverse-DNS (`nexus.energy.*`, `com.acme.*`); a request can only run
  a kind its tenant/extension is allowed to see.

### 4.4 The multi-datasource delta (the one real design problem)
Rubix is **single-warehouse**; nexus is **multi-datasource**. So a query-kind must declare *what it
runs against*, and the host must resolve it:
- **`datasource_kind: postgres`** — the kind is valid against any postgres datasource the caller
  picks (panel still carries `datasourceId`). Most flexible; the kind is dialect-bound, not
  instance-bound.
- **`datasource_binding: <id>`** (optional) — pin a kind to a specific datasource (e.g. a curated
  "energy warehouse"). Useful for core packs.
- **`tables:` resolution** — the capability check resolves the declared tables against the *bound*
  datasource's allowed read surface. (Nexus's data-side DBs don't have RLS — §5.2 NEXUS.md — so for
  a *shared* datasource the `$caller_tenant_id` predicate in the kind SQL is what isolates rows. Make
  that predicate **mandatory-by-lint**: a query-kind whose SQL omits `$caller_tenant_id` on a
  tenant-scoped table fails validation at load.)

### 4.5 Where kinds live (open question — recommend a default)
Two hosting options; **recommend (a) for v1, design for (b)**:
- **(a) Built-in "core kinds" pack** — a `nexus/kinds/` (or `nexus-api/kinds/`) directory loaded at
  boot. Ships curated, reviewed kinds. No extension machinery needed. Fastest path.
- **(b) Extension-contributed kinds** — kinds declared in an extension manifest (rubix `block.yaml`
  style) loaded via the federation/extension system (NEXUS.md §7). Gated behind the extension
  security model (allowlist/signature/capability) before any out-of-repo kind loads.
- **(c) Tenant-authored kinds** (later) — saved kinds in the DB (`1401_query_kinds.sql` (WS-10 `14xx` block)) so an admin
  promotes a working Explore query into a reusable named kind via the UI. The natural "save this
  query as a template" feature; reuses the same registry + governance.

Start with (a) — a loader + registry + the param binder — and make the registry source-agnostic so
(b)/(c) plug in without re-architecting.

---

## 5. Relationship to other workstreams (read before building)

- **WS-03** — owns the **param-binder/macro engine** (C2). WS-10 widens its contract to carry the
  param map + host-bound tokens and to be invoked by the kind dispatcher. **Coordinate as one effort;
  WS-10's query-kinds and WS-03's macro engine should land together or in lockstep.**
- **WS-01 / WS-02** — `$__timeFrom/$__timeTo/$__interval` and `$var`s are injected into kind SQL by
  the *same* binder. Kinds get time + variables "for free."
- **WS-08** — datasource-kinds **are** WS-08 expressed declaratively. Decide early whether WS-08
  registers connectors imperatively (Rust builders) or declaratively (datasource-kinds) — recommend
  declarative config + a thin builder per protocol.
- **WS-09** — adopt the **`.cache.yaml` sidecar** as the WS-09 cache spec; `tables:` drives
  invalidation. The cache key = `tenant + datasource + kind + bound-params + resolved-time` (C3).
- **WS-04 panel editor** — gains a "**pick a kind**" mode: a dropdown of available kinds + a
  schema-driven params form (json-schema → rhf/zod), alongside the raw-SQL tab.
- **WS-05** — kinds make the dashboard JSON model **portable**: a panel references a kind by id +
  params, not an opaque SQL blob bound to one schema. Better for export/AI-generation.
- **WS-11 (units & prefs)** — a kind can **declare the `quantity` (+ stored unit) of each output
  column** in its manifest, so a kind-backed panel **converts to the viewer's units automatically**
  with no per-panel tagging. This makes kinds *self-describing* for units — a strong reason to land
  the kind output-schema and WS-11's quantity model together. The `.cache.yaml` two-layer scope
  (canonical@tenant / converted@user, WS-09) is the matching cache shape.

---

## 6. Scope (this workstream)

1. **Kind registry + loader** (`nexus-api` or `nexus-engine`): parse manifest entries, load
   `sql_file` + `params_schema` + optional `cache` + `description`, validate at boot (schema parses,
   SQL references only declared params, tenant-predicate lint §4.4). Reverse-DNS namespacing.
2. **Param binder (with WS-03)**: host-bound tokens, host-injected time/vars, schema-validated caller
   params — **all emitted as bound `$N` args** (returns `BoundQuery`, not a SQL string). One engine for
   sql-mode and kind-mode.
3. **Dispatch**: extend `POST /query` (and `QueryRequest` DTO) to accept `{kind, params}`; run under
   existing guards; resolve `datasource_kind`/binding + `tables` capability.
4. **Core kinds pack**: a `kinds/` dir + 4–6 starter kinds (a list, a detail, a time-bucket
   aggregate, a top-N) to prove the path and seed dashboards.
5. **UI (with WS-04)**: a "kind" query mode — kind picker + schema-driven params form.
6. **Cache sidecar (with WS-09)**: port the `.cache.yaml` schema; wire `invalidate_on.tables`.
7. **Datasource-kinds (Wave 2, with WS-08)**: declarative datasource type manifest + per-kind config
   form + test path + dialect mapping.
8. **(Later)** tenant-authored kinds: "save Explore query as a kind" + `1401_query_kinds.sql` (WS-10 `14xx` block).

## 7. Acceptance criteria
- [ ] **C6 (audit/undo):** if tenant-authored kinds are persisted (§4.5c), the `query_kind` resource
  has a `Reversible` impl + `record_if_reversible` in its handlers + is in WS-12's mutable-kinds
  manifest. *(Manifest-only / built-in kinds are immutable config — no recording needed; note that.)*
- [ ] A core query-kind runs via `POST /query` with `{kind, params}`; params validated against schema;
  bad params rejected pre-execution.
- [ ] `$caller_tenant_id` is host-bound and **cannot** be overridden by caller input (test it).
- [ ] A query-kind whose SQL omits the tenant predicate on a tenant-scoped table **fails to load**
  (lint).
- [ ] Time range (WS-01) and a variable (WS-02) inject into a kind via the shared binder.
- [ ] Adding a new kind requires **only** a file-drop + manifest line (no recompile for built-in pack
  is acceptable to defer; the *registry* must not need code per kind).
- [ ] A `.cache.yaml` sidecar caches a kind; a write to a declared `tables:` entry invalidates it.
- [ ] WS-04 panel editor can pick a kind and fill its params form.
- [ ] Raw-SQL mode still works unchanged (escape hatch preserved).
- [ ] Tests: registry load + lints, param binding + host-token rejection, schema validation, cache
  invalidation, multi-datasource binding resolution.

## 8. Out of scope (hand off / defer)
- The extension *security* model for out-of-repo kinds (allowlist/signature/CSP) → reuse NEXUS.md §7
  / the federation security workstream; v1 ships the **built-in core pack** only.
- Full datasource-kind connector implementations → **WS-08** (this WS defines the *declaration*; WS-08
  supplies the builders/dialects).
- Tenant-authored-kinds UI → later phase (registry must be source-agnostic so it slots in).

## 9. Open questions to settle in Wave 0
1. **Hosting:** built-in `kinds/` dir vs extension-manifest vs DB — recommend built-in first (§4.5).
2. **Manifest format:** reuse the rubix `block.yaml` shape, or a nexus-native `kinds.yaml`? (Lean
   nexus-native + small, since nexus isn't loading the rubix extension here — but keep the field
   names aligned so the mental model transfers.)
3. **Binder ownership:** confirm WS-03 owns the single binder and WS-10 calls it (vs two engines).
4. **`tables:` semantics** on a datasource with no RLS (shared DB) — mandatory `$caller_tenant_id`
   predicate lint is the proposed answer (§4.4); confirm.
