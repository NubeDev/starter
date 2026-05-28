# Extension → Dashboard data path (long-term plan)

**Status:** proposal
**Author:** rubix-agent
**Date:** 2026-05-28
**Reference implementation:** `rubix/extensions/com.nubeio.rubixos` (Nube-iO Rubix-OS BMS dump, 164M history rows)

## Problem

An extension owns a large, evolving dataset in the warehouse
(`com_<ext_id>__*` tables). The dashboard needs to render charts, KPIs,
tables, and selectors over that data **without** the UI knowing SQL,
schemas, or table names. Today the path works but is uneven:

- Raw queries from the UI would couple panels to columns.
- The 4 host-builtin analytics templates assume `public.samples(metric_id,
  value, ts)` — extension schemas don't fit.
- `TimescaleAnalyticsBridge` only sees `TemplateRegistry::builtin()`, so
  the 9 named templates already contributed by the rubixos extension
  (`points_list`, `points_search`, `points_by_device`, `devices_overview`,
  `networks_overview`, `hosts_overview`, `history_recent`,
  `history_bucketed`, `histories_summary` — see
  [`block.yaml:146-196`](rubix/extensions/com.nubeio.rubixos/block.yaml#L146-L196))
  can't drive `analytics_template:` chart sources yet.
- 164M-row hypertables make ad-hoc dashboard SQL slow.

We want one durable answer: **the extension is the data product, the
dashboard is a declarative consumer.** Dashboards reference data by
**named template** — never by SQL — and the named-template catalog is
the warehouse's growing public API.

## Principles

1. **SQL stays in templates, code stays in tools.**
   Anything expressible in SQL is a named `warehouse_templates[]` entry
   in `block.yaml`. Anything else is a `tools[]` entry implemented in
   the extension process.
2. **Dashboards reference data by name, never by SQL.**
   Every panel uses `source.kind = "analytics_template"` with a template
   name and params. Rewriting the SQL (raw → 1m → 5m rollup) never
   touches the UI.
3. **The extension YAML is the data contract.**
   Schema, allowlist, templates, and tools all live in one reviewable
   file. The host enforces tenant scoping, audit, and capability gates
   around it.
4. **Performance is a dial, not a rewrite.**
   Continuous aggregates per cadence (1m / 5m / 1h / 1d) let a panel
   change resolution by editing one template.

## Architecture — three layers in the extension, one contract to the host

```
┌────────────────────────── extension (com.nubeio.rubixos) ──────────────────────────┐
│                                                                                   │
│  Layer 1  Storage                                                                 │
│    raw hypertables:  com_nubeio_rubixos__histories  (164M rows, hypertable)       │
│                      com_nubeio_rubixos__points / devices / hosts / *tags         │
│    rollups (continuous aggregates):                                               │
│      com_nubeio_rubixos__histories_1m / _5m / _1h / _1d                           │
│                                                                                   │
│  Layer 2  Named queries (warehouse_templates[] in block.yaml)                     │
│    Already shipping (block.yaml:146-196):                                         │
│      points_list, points_search, points_by_device                                 │
│      devices_overview, networks_overview, hosts_overview                          │
│      history_recent, history_bucketed, histories_summary                          │
│    Planned (rollups, see Layer 1):                                                │
│      history_bucketed_1m / _5m / _1h / _1d                                        │
│                                                                                   │
│  Layer 3  Rust tools (tools[] in block.yaml)  — only when SQL can't               │
│    bacnet_decode, anomaly_detect, remote_api_sync, ...                            │
│                                                                                   │
└──────────────────────────────────────────┬────────────────────────────────────────┘
                                           │
                       JSON-RPC stdio (audit + tenant + capability gate)
                                           │
┌──────────────────────────────────────────▼────────────────────────────────────────┐
│                                       rubix host                                  │
│                                                                                   │
│   TemplateRegistry  ◄─── merged: builtin() + extension_registry.templates()       │
│        │                                                                          │
│        ▼                                                                          │
│   TimescaleAnalyticsBridge  ── resolves analytics_template:<name> → SQL → rows    │
│        │                                                                          │
│   ┌────┴──────────────────┬──────────────────────────┐                            │
│   ▼                       ▼                          ▼                            │
│   SDUI panels       Warehouse Explorer        warehouse.query JSON-RPC            │
│   (declarative)     (operator ad-hoc)         (agent / power-user)                │
└───────────────────────────────────────────────────────────────────────────────────┘
```

### Layer 1 — Storage (raw + rollups)

Keep raw tables exactly as the host created them. Add Timescale
**continuous aggregates** for the heavy ones and register them in
`contributes.warehouse_tables[]` so the host allowlist covers them.

```sql
-- scripts/post-load.sql, idempotent
CREATE MATERIALIZED VIEW IF NOT EXISTS public.com_nubeio_rubixos__histories_1m
WITH (timescaledb.continuous) AS
SELECT tenant_id, point_uuid, host_uuid,
       time_bucket('1 minute', "timestamp") AS bucket,
       avg(value) AS avg, min(value) AS min, max(value) AS max, count(*) AS n
FROM public.com_nubeio_rubixos__histories
GROUP BY 1,2,3,4
WITH NO DATA;

SELECT add_continuous_aggregate_policy('com_nubeio_rubixos__histories_1m',
  start_offset      => INTERVAL '7 days',
  end_offset        => INTERVAL '1 minute',
  schedule_interval => INTERVAL '1 minute',
  if_not_exists     => true);

-- repeat for _5m, _1h, _1d with progressively wider start_offset / schedule_interval
```

Rationale: a 24-hour chart of one point hits ~1,440 rows in `_1m`
instead of potentially millions in `histories`. Backfill is automatic.

**Three operational notes that bite on day 1:**

1. **Tenant isolation lives on the query side, not the CAGG.** The
   `tenant_id` column is in the materialized view (it's in `GROUP BY`),
   but the CAGG itself has no RLS. The host's named-template binding of
   `$caller_tenant_id` is what keeps tenants apart. Don't add RLS to
   the CAGG — it'll break the refresh.
2. **Storage is not free — and the obvious arithmetic is wrong.** A
   CAGG's row count is `unique(group_by_keys) × buckets_in_window`,
   not `raw_rows ÷ bucket_size`. BMS data is COV-triggered, not
   periodic, so per-point sampling cadence varies wildly: one point
   might emit 100K samples in a day, another might emit 3. Before
   committing storage, run on the actual dump:
   ```sql
   SELECT count(DISTINCT point_uuid)                 AS points,
          max("timestamp") - min("timestamp")        AS span
   FROM   com_nubeio_rubixos__histories;
   -- _1m row estimate ≈ points × span_in_minutes
   ```
   Depending on the dump this can land anywhere from single-digit
   millions to tens of billions. Pick the cadences you'll actually
   chart against; adding all four "just in case" is the wrong default.
3. **`end_offset INTERVAL '1 minute'` means a "now" panel sees a
   1-minute hole.** Either accept the lag and document it on the panel,
   or expose a `history_live` template that `UNION ALL`s the most-recent
   minute from raw `histories`. Without one of those, "live" charts
   will look stuck.
4. **CAGG refresh is global, not per-tenant.** Timescale's continuous
   aggregate policy runs once for the whole hypertable; a tenant with
   1% of the data still inherits the refresh latency of a job sized for
   the busiest tenant. In a one-big-tenant + many-small-tenants
   deployment, the small tenants' visible lag is dictated by the big
   tenant's bucket size. If this matters in practice, the options are
   (a) accept it and surface the lag in the panel chrome, or (b) shard
   the CAGG by tenant via `add_continuous_aggregate_policy` per-tenant
   chunks — significantly more complex; don't reach for it pre-emptively.

### Layer 2 — Named queries (the data tools)

Every dashboard query is a `warehouse_templates[]` entry. Joins,
aggregations, window functions, jsonb shaping — all here. **9 templates
are already shipping** in
[`block.yaml:146-196`](rubix/extensions/com.nubeio.rubixos/block.yaml#L146-L196)
and live on disk as
[`kinds/*.sql`](rubix/extensions/com.nubeio.rubixos/kinds/) — they're
reachable today via the `com.nubeio.rubixos.warehouse_query` tool and
will become reachable via the SDUI `analytics_template:` source as soon
as the boot reorder below lands.

**Parameter convention (load-bearing — don't reinvent):**

- **Named params** (`$point_uuid`, `$limit`, `$from`), not positional
  `$1..$N`. The host's `WarehouseReadHandle` binds by name, the SDUI
  `analytics_template` source already passes `params` as a JSON object
  (see [dashboard-api-usage.md:106-109](rubix/docs/design/sdui/dashboard-api-usage.md#L106-L109)),
  and the per-template JSON Schema (`*_params.json`) describes the
  named set. Positional binding has no caller-visible advantage and
  drifts as columns are added.
- **`$caller_tenant_id` is injected by the host** from the operator
  session — never declared in `params_schema`, never passable by the
  caller. A template that needs tenant scoping just references
  `$caller_tenant_id` in the `WHERE` clause; the host fills it in.
  This is the only safe pattern: a missing positional tenant param
  would silently widen the query to all tenants.

**Working example** — [`kinds/history_bucketed.sql`](rubix/extensions/com.nubeio.rubixos/kinds/history_bucketed.sql), the chart workhorse:

```sql
-- com.nubeio.rubixos.history_bucketed
SELECT time_bucket($bucket::interval, "timestamp") AS bucket,
       min(value)::float8 AS min_value,
       max(value)::float8 AS max_value,
       avg(value)::float8 AS avg_value,
       count(*)           AS sample_count
FROM   com_nubeio_rubixos__histories
WHERE  tenant_id  = $caller_tenant_id          -- host-injected, never client-supplied
  AND  point_uuid = $point_uuid
  AND  "timestamp" >= $from::timestamptz
  AND  "timestamp" <  $to::timestamptz
GROUP  BY bucket
ORDER  BY bucket;
```

Contributed in `block.yaml` as:

```yaml
contributes:
  warehouse_templates:
    - name: com.nubeio.rubixos.history_bucketed
      params_schema: kinds/history_bucketed_params.json   # named-param JSON Schema
      sql_file:      kinds/history_bucketed.sql
      tables:        [com_nubeio_rubixos__histories]
```

New rollup templates added in this proposal follow the same shape,
swapping the source table for a continuous aggregate:

```sql
-- com.nubeio.rubixos.history_bucketed_1m  (new, see Layer 1 rollups)
SELECT bucket,
       min_value, max_value, avg_value, sample_count
FROM   com_nubeio_rubixos__histories_1m
WHERE  tenant_id  = $caller_tenant_id
  AND  point_uuid = $point_uuid
  AND  bucket >= $from::timestamptz
  AND  bucket <  $to::timestamptz
ORDER  BY bucket;
```

Why templates and not ad-hoc SQL from the dashboard:

- Audited via `TemplateRegistry` (single source of truth).
- Tenant scoping injected by the host before the SQL ever runs.
- Parameter binding by name — no injection surface, no positional drift.
- Reviewable diff in `block.yaml` for every data-contract change.
- UI never breaks when SQL is rewritten (raw → rollup).

### Layer 3 — Rust tools (optional)

Use `tools[]` only when the work is not SQL: BACnet decode, anomaly
detection, remote-API enrichment, file conversion. They appear in the
host as JSON-RPC `tools/<id>` and can be invoked from SDUI handlers
identically to host tools.

The two existing extension tools (`warehouse_query`, `warehouse_insert`)
stay as the operator/agent escape hatch — not for production panels.

## Host change — pass the existing registry to the SDUI bridge

> **Updated after peer review #2 (2026-05-28).** The earlier "boot
> reordering" framing was wrong. The merged registry IS already built
> in time — see `main.rs:553-580`, where
> `tmpl.extend_from_record(record)` is called for every validated
> extension and the resulting `Arc<TemplateRegistry>` is handed to
> `RubixHostMethods`, `RubixCapabilityFactory`, and the admin
> introspection state. The host-methods path
> (`ctx.warehouse_read().query(...)` and the `warehouse.query` JSON-RPC
> reverse-call) is unaffected — it already resolves contributed
> templates correctly today.
>
> The actual defect is narrower and weirder: `build_sdui_router` at
> `main.rs:685` never receives the registry, and inside
> [`TimescaleAnalyticsBridge::new`](rubix/crates/rubix-agent/src/sdui/analytics_bridge.rs#L44-L46)
> the bridge **constructs a fresh `Arc::new(TemplateRegistry::builtin())`
> and throws away** whatever the host built. So the SDUI plane sees
> the four `meter_*` builtins and nothing else — not because the host
> hadn't built the merged set, but because the bridge's default
> constructor silently discards it. This makes the fix smaller than
> the earlier framing implied — there's no reordering, only wiring.

### The change

Three coordinated edits. The first two are pure wiring; the third is
new public API on `TemplateRegistry` plus a small change to the
bridge.

**1. `main.rs` (~line 685) and `boot/sdui.rs` — thread the existing
`Arc<TemplateRegistry>` through `build_sdui_router` into the bridge.**

```rust
// before
pub fn build_sdui_router(client: WarehouseClient, /* … */) -> Router {
    let bridge: AnalyticsBridgeRef =
        Arc::new(TimescaleAnalyticsBridge::new(client));   // silently discards real registry
    // …
}

// after
pub fn build_sdui_router(
    client: WarehouseClient,
    template_registry: Arc<TemplateRegistry>,              // already constructed at main.rs:553-580
    /* … */
) -> Router {
    let bridge: AnalyticsBridgeRef =
        Arc::new(TimescaleAnalyticsBridge::with_registry(client, template_registry));
    // …
}
```

This alone is the "make dashboards see extension templates" change —
roughly 10 LOC across the two files. `with_registry` already exists at
[analytics_bridge.rs:50-52](rubix/crates/rubix-agent/src/sdui/analytics_bridge.rs#L50-L52).
No new types here.

**2. `TemplateRegistry` — add a name→owning-extension lookup.** *(new
public API.)*

Peer review caught that the original sketch for the SDUI allowlist
gate referenced `TemplateSpec::owning_extension` — a field that does
**not** exist on
[`TemplateSpec`](rubix/../starter-extensions/crates/starter-ext-spi/src/warehouse.rs#L61),
which has only `name`, `params`, `tables`, `sql`. The bridge needs
some way to recover "which extension owns this template" without
parsing the name prefix (fragile — builtin `meter_*` names don't
follow any prefix convention, and any future contributed template
that names itself off-convention would silently bypass the gate).

Pick **(b)** from the three reviewer-suggested options: keep
`TemplateSpec` unchanged and add a side map on `TemplateRegistry`.

```rust
// starter-extensions/crates/starter-ext-host/src/warehouse.rs
pub struct TemplateRegistry {
    specs:  HashMap<String, TemplateSpec>,
    owners: HashMap<String, ExtensionId>,                  // new — keyed identically to `specs`
}

impl TemplateRegistry {
    /// Single insertion choke point. Every path that registers a
    /// template — `builtin()`, `extend_from_record(record)`, and any
    /// future `register_one(...)` — funnels through here so `specs`
    /// and `owners` can never drift. Builtins pass `owner = None`;
    /// extension-contributed templates pass `Some(ext_id)`.
    fn insert_template(
        &mut self,
        name:  String,
        spec:  TemplateSpec,
        owner: Option<ExtensionId>,
    ) {
        self.specs.insert(name.clone(), spec);
        if let Some(id) = owner {
            self.owners.insert(name, id);
        } else {
            self.owners.remove(&name);                     // builtin shadowing an old entry clears the owner
        }
    }

    pub fn owning_extension(&self, name: &str) -> Option<&ExtensionId> {
        self.owners.get(name)
    }
}
```

`None` from `owning_extension` means **"no per-extension allowlist
applies — this is a host-trusted builtin"**, not "fail open". The SDUI
gate must treat the two cases distinctly: builtin (`None`) → skip the
intersection step entirely; contributed (`Some`) → require the
intersection to succeed. Spell this out in the gate's code comment so
a future reviewer doesn't read the early-return as a vulnerability.

This is a **new public method on a stable type**
(`TemplateRegistry::owning_extension`), so it has SPI versioning
consequences. Additive only — no breaking change — but budget the
review surface accordingly. The `insert_template` choke point itself
stays private to the crate; the SPI surface is just the lookup.

**3. `analytics_bridge.rs` — apply the per-template table allowlist on
the SDUI path.**

The host-methods path enforces
`warehouse_tables_for(registry, extension)` per call. The SDUI bridge
currently doesn't — see
[warehouse-templates-contribution.md:106-118](rubix/docs/design/extensions/warehouse-templates-contribution.md#L106-L118),
which describes this as "Deferred". Since SDUI is the primary
dashboard surface, deferring the gate makes the allowlist decorative
for the path that matters most.

`warehouse_tables_for` lives at
[rubix/crates/rubix-agent/src/extensions/backends.rs:531-543](rubix/crates/rubix-agent/src/extensions/backends.rs#L531-L543)
(reviewer-corrected path; the earlier draft cited the wrong crate).
Its signature is:

```rust
pub(crate) fn warehouse_tables_for(
    registry:  Option<&ExtensionRegistry>,
    extension: &ExtensionId,
) -> Vec<ContributeWarehouseTable>;
```

It's `pub(crate)` inside `rubix-agent`, so the SDUI bridge in the same
crate can call it directly — but it takes the **`ExtensionRegistry`**
(not the `TemplateRegistry` the bridge holds) and the **`&ExtensionId`**
(which the bridge recovers from step 2's lookup). So the bridge gains
two new fields:

```rust
pub struct TimescaleAnalyticsBridge {
    client:           WarehouseClient,
    template_registry: Arc<TemplateRegistry>,              // already there after step 1
    extension_registry: Arc<ExtensionRegistry>,            // new
}
```

…and `resolve` becomes: look up the spec by name → look up owning
`ExtensionId` via `template_registry.owning_extension(name)` → if
present, intersect `spec.tables` with
`warehouse_tables_for(Some(&extension_registry), ext_id)` → reject
on mismatch. Builtins (no owner) bypass — host-trusted by design.

**Honest LOC budget after peer review:**

- `main.rs` / `boot/sdui.rs` thread-through: ~10 LOC.
- `TemplateRegistry::owning_extension` + `owners` map +
  `extend_from_record` write-through: ~30 LOC plus tests.
- `TimescaleAnalyticsBridge` extra field + plumbing from
  `build_sdui_router`: ~20 LOC.
- Per-call gate in `resolve` + tests: ~40 LOC.
- **Total ~100 LOC + tests, one new public method on `TemplateRegistry`.**
  Still a single PR, but no longer "no new types, no new trait methods".

After these changes, any panel can resolve any extension template by
name, gated identically to every other warehouse-read path. No
further host changes are needed as the catalog grows.

## Dashboard contract

Panels declare a source; the bridge resolves it. The UI carries zero
SQL and zero schema knowledge. **Params are a named JSON object** —
the same shape the SDUI runtime already passes to `analytics_template:`
sources (see [dashboard-api-usage.md:106-109](rubix/docs/design/sdui/dashboard-api-usage.md#L106-L109)).
**`tenant_id` is never in the params** — the host injects
`$caller_tenant_id` from the session.

```jsonc
{
  "type": "line_chart",
  "title": "{{point.name}}",
  "source": {
    "kind": "analytics_template",
    "name": "com.nubeio.rubixos.history_bucketed",   // template that exists today
    "params": {
      "point_uuid": "{{route.point_uuid}}",
      "from":       "{{now-24h}}",
      "to":         "{{now}}",
      "bucket":     "1 minute"
    }
  }
}
```

KPIs (`com.nubeio.rubixos.histories_summary`), tables
(`com.nubeio.rubixos.points_list`,
`com.nubeio.rubixos.devices_overview`), selectors
(`com.nubeio.rubixos.points_search`), and detail panes use the same
`source` shape pointed at different templates from the same catalog.
Changing chart resolution from 1-minute raw buckets to a pre-rolled
1-hour aggregate is a one-line edit once the rollup templates land:
swap the template `name` (`history_bucketed` → `history_bucketed_1h`).

## Three planes, one mental model

| Surface | Audience | Path | When to use |
|---|---|---|---|
| **SDUI `analytics_template:`** | end users | `TemplateRegistry` → `WarehouseClient` | All production panels |
| **Warehouse Explorer** (`/api/warehouse/explorer/*`) | operators | direct `WarehouseClient` | Browse, sanity-check, prototype SQL |
| **`warehouse_query` tool** | agents / power users | extension JSON-RPC → host `warehouse.query` reverse call | One-off / agentic workflows |

All three read the same Postgres pool. The same row is the same row.

## What does NOT work, and why we're not fixing it

- **Host's 4 builtin `meter_*` templates** are hard-coded to
  `public.samples(metric_id, value, ts)`. We won't reshape rubixos data
  to fit them; instead the extension publishes its own templates. The
  builtins remain for tenants that genuinely use `samples`.
- **Cross-extension joins** are intentionally not a host feature. If
  two extensions need shared data, one of them publishes a template
  the other consumes through the extension SPI — explicit, audited.

## Growing the common toolkit (template promotion path)

The catalog should grow without exploding. Three tiers, with a defined
promotion path between them:

| Tier | Lives in | Naming | When |
|---|---|---|---|
| **Extension-private** | `block.yaml` of one extension | `com.nubeio.rubixos.history_bucketed` | The schema is owned by this extension. Default tier — every new template starts here. |
| **Shared via SPI** | Source extension's `block.yaml`; consumers list it under their `requires:` | `com.foo.shared_thing` | A second extension wants the *same* result. Don't copy-paste the SQL — depend on the owner. |
| **Host builtin** | `TemplateRegistry::builtin()` in `starter-ext-host` | `warehouse.bucketed` (`warehouse.*` reserved as a new convention — see below) | Two or more extensions need the *same shape* over different tables. The template is generalized (table name becomes a manifest-time parameter, not a runtime one) and promoted into code. |

**Reserving the `warehouse.*` prefix is a new commitment, not a
preexisting one.** The four current builtins
([starter-ext-host/src/warehouse.rs:88-174](rubix/../starter-extensions/crates/starter-ext-host/src/warehouse.rs#L88-L174))
are bare names — `meter_kwh_last_24h`, `meter_litres_last_24h`,
`meter_value_30d_15m`, `meter_value_24h_1m` — no namespace. Phase 1
adopts the `warehouse.*` prefix for *new* builtins going forward; the
existing `meter_*` names stay for back-compat. The reserved-prefix
check in `validate_manifest`
([warehouse-templates-contribution.md:52-55](rubix/docs/design/extensions/warehouse-templates-contribution.md#L52-L55))
gains `warehouse.` alongside `starter.` and `sys.` so an extension
can't accidentally squat the namespace.

**Reserved-prefix collisions are a hard reject; cross-extension name
collisions are a warning.** An extension declaring `warehouse.foo` is
almost always either a mistake or a takeover attempt — treat it the
same way `starter.*` and `sys.*` are treated today (validation fails,
extension lands in `Failed` lifecycle state). A collision with another
extension's contributed template, or with a non-reserved-prefix
builtin name, is preserved as warn-and-load — that's the existing
shadowing behavior and breaking it would force every existing
manifest through a migration. The warning is the new bit:
`extend_from_record` logs at `warn!` level when its insert would
overwrite an existing entry, so silent capture stops being silent.

**Promotion criteria** (all three must hold before lifting into the
host builtin):

1. ≥2 extensions have copies of the same SQL differing only in table
   names and column aliases.
2. The shape is stable — the SQL hasn't changed in ≥2 release cycles.
3. The generalized form can express both call sites without any
   runtime-templated SQL (R7 — `sql_file` is static; parameterization
   is `WHERE`-clause params, not string interpolation of identifiers).

**Rollout — coordinated PR, not shadowing.** The earlier draft argued
that "extension-contributed names shadow builtins, so an extension can
keep its private template through one release, then drop it." Peer
review caught two problems: (1) the private and promoted names *are
different* (`com.nubeio.rubixos.history_bucketed` vs
`warehouse.bucketed`), so shadowing doesn't apply — there's no name
collision to silence; (2) silent-replace
([warehouse-templates-contribution.md:96-104](rubix/docs/design/extensions/warehouse-templates-contribution.md#L96-L104))
is itself a footgun we'd rather close than rely on. Replace with:

- **Single coordinated PR** when criteria are met: add the builtin,
  update every consuming panel's `source.name` string, delete the
  private `kinds/*.sql` and its `warehouse_templates[]` entry. One
  release, atomic, reviewable.
- **Registry-time warning** when a contributed template name equals an
  existing builtin (and vice-versa). Cheap — implement alongside the
  `validate_manifest` change above. Catches the silent-capture case
  where a future builtin lands and an extension already declared the
  same name.

**What this gives us:** the catalog grows by addition, not silent
override. A panel built today against
`com.nubeio.rubixos.history_bucketed` keeps working forever. When the
shape generalizes into `warehouse.bucketed`, the migration is one
explicit PR per consuming panel — visible in the changelog, not
implicit in load order.

## Implementation order

1. **Boot reordering + SDUI allowlist gate** in `main.rs`,
   `boot/sdui.rs`, and `analytics_bridge.rs` (see "Host change" above).
   Single PR. **This is the only thing standing between the 9 already-
   contributed templates and the dashboard** — no extension authoring
   needed, just wire-up. Also fixes the per-call allowlist asymmetry
   between SDUI and host-methods paths.
2. **Continuous aggregates** for `histories` (start with `_1m` and
   `_1h`; add `_5m` / `_1d` only when a panel asks for them) via
   `scripts/post-load.sql`, registered in `contributes.warehouse_tables[]`.
   Makes time-series dashboards instant.
3. **Add rollup templates** alongside the existing 9: e.g.
   `history_bucketed_1m`, `history_bucketed_1h`. Each new template is a
   new `kinds/*.sql` + `*_params.json` pair plus a `warehouse_templates[]`
   entry — same shape as today.
4. **Build the UI** against `analytics_template:` sources only,
   referencing templates by name. Never call `warehouse_query` from a
   production panel; that path stays for operators and agents.
5. **Warehouse Explorer "save as template" affordance** — let an
   operator who prototyped a query in the Explorer write it back into
   the owning extension's `block.yaml` + `kinds/` as a draft template
   PR. Highest-leverage authoring win once panels start asking for
   queries that don't exist yet.
6. **Add Rust tools** only when SQL is insufficient (BACnet decode,
   anomaly detection, remote-API enrichment).

## What this buys us long-term

- **Schema can evolve** — only templates change, dashboards untouched.
- **Performance is a per-panel dial** — swap `_1m` for `_1h` in one line.
- **Multi-tenant safe by construction** — host injects tenant filter
  before every template runs.
- **Auditable** — every dashboard data pull = a `TemplateRegistry`
  audit event.
- **Portable** — the extension is a self-contained YAML + SQL + UI
  bundle; drop it into another rubix install and it works.
- **Agent-friendly** — the same named templates are callable by agents
  via `warehouse.query`, so the LLM and the dashboard share one
  vocabulary.

## Open questions

- **Template versioning + rename/removal UX.** The proposal claims the
  catalog grows monotonically and panels reference names forever, but
  doesn't define the lifecycle. Concretely:
  - **Error shape** when a name resolves to nothing. Today an unknown
    template in `TimescaleAnalyticsBridge::invoke` returns a string
    error; the SDUI renderer surfaces this as a blank panel with no
    diagnostic. We need a structured `template_not_found(name)` plus a
    renderer message ("This panel references a template that no longer
    exists — was it renamed?").
  - **`params_schema` evolution.** Additive changes (new optional
    field) are back-compat. Breaking changes (renamed/required field)
    should force a new template name (`history_bucketed_v2`) rather
    than mutating in place. State this as policy.
  - **Deprecation flag.** Add `deprecated_in: <version>` /
    `removed_in: <version>` fields on `warehouse_templates[]` so the
    admin introspection surface (`GET /api/v1/admin/registry/templates`)
    can warn dashboards that depend on a sunsetting name.
- **Agent (LLM) discoverability of the catalog.** The doc claims "the
  LLM and the dashboard share one vocabulary" but doesn't specify how
  an agent enumerates that vocabulary. Phase 1 should also expose
  `warehouse.list_templates` (or extend `tools/list` to include
  template metadata) returning `{name, params_schema, tables}` for
  every grant-visible template, so an agent can build valid
  `warehouse.query` calls without being hand-fed names. Without this,
  the "shared vocabulary" property is aspirational.

  **Filter policy — pin before shipping; changing it later is
  breaking.** Two choices, pick one:
  - **(a) Grant-filtered** — return only templates the caller can
    actually invoke (intersect with the caller's
    `warehouse_read.tables` grant). Safer: the LLM only sees what it
    can call, can't waste turns proposing forbidden queries, and
    least-privilege is preserved end-to-end.
  - **(b) Full catalog with `granted: bool`** — return everything,
    flag which entries are callable. More useful for planning ("an
    operator would have to grant me X for this dashboard"), but
    leaks the existence of templates the caller can't see.

  Recommend (a) for the v1 ship. Add (b) later as a separate
  `warehouse.list_templates_full` if operators ask — that keeps the
  default safe and the breakage surface zero.
- **Extension table schema migration.** An extension that ships
  `histories` v1 with a `value NUMERIC` column and later renames it to
  `reading NUMERIC` breaks every template — including the ones we
  promoted into the host builtin tier. The data contract is
  `block.yaml`, but `block.yaml` doesn't pin its own underlying DDL
  across upgrades. Minimum: state in the manifest contract that the
  extension owns keeping its `kinds/*.sql` compiling against its own
  table evolution; load-time SQL parse-check is a future hardening.
- **Bridge-layer result caching.** Should template results be cached
  at the bridge (TTL per template, keyed by params + tenant)? Likely
  yes for KPIs (`histories_summary`), probably not for time-series.
  Do we want a `warehouse_templates[].cache: { ttl: 30s }` field in
  `block.yaml` so the extension declares the policy per template?
- **Rollup authoring path — raw DDL vs. `mart.create` rule verb.**
  This proposal uses a raw `post-load.sql` for the continuous
  aggregates. The alternative is to author them through the
  `mart.create` rule verb so they participate in the warehouse-rules
  lifecycle. We're picking raw DDL because `mart.create` currently
  has the data-loss-on-undo caveat documented in
  `warehouse-rules/README.md` — continuous aggregates are expensive to
  rebuild (164M rows) and we don't want undo to drop them. Revisit if
  the undo story for marts improves.

## Appendix — file touch list

**Host (Phase 1, one PR):**

- `rubix/crates/rubix-agent/src/main.rs` — pass the already-built
  `Arc<TemplateRegistry>` (constructed at `main.rs:553-580`) into the
  `build_sdui_router(...)` call (currently `main.rs:685`). The
  registry is already shared with `RubixCapabilityFactory`,
  `RubixHostMethods`, and admin introspection state — this just adds
  the SDUI router to that set.
- `rubix/crates/rubix-agent/src/boot/sdui.rs` — add
  `template_registry: Arc<TemplateRegistry>` parameter to
  `build_sdui_router`; swap `TimescaleAnalyticsBridge::new(client)` for
  `TimescaleAnalyticsBridge::with_registry(client, template_registry)`.
  Also thread an `Arc<ExtensionRegistry>` for the per-call gate.
- `starter-extensions/crates/starter-ext-host/src/warehouse.rs` —
  **new public method** `TemplateRegistry::owning_extension(name)`
  plus an internal `owners: HashMap<String, ExtensionId>` populated by
  `extend_from_record`. Additive, but a stable-API change.
- `rubix/crates/rubix-agent/src/sdui/analytics_bridge.rs` — bridge
  gains `extension_registry: Arc<ExtensionRegistry>`. In `resolve`,
  recover owning `ExtensionId` via the new lookup, then intersect
  `spec.tables` with
  [`warehouse_tables_for(...)`](rubix/crates/rubix-agent/src/extensions/backends.rs#L531-L543);
  reject on mismatch. Brings SDUI to parity with the host-methods
  path.

**Extension (Phase 2-3):**

- `rubix/extensions/com.nubeio.rubixos/scripts/post-load.sql` — **new**,
  continuous aggregate DDL + policies for `_1m` (and `_1h` if/when a
  panel needs it).
- `rubix/extensions/com.nubeio.rubixos/scripts/load-dump.sh` — invoke
  `post-load.sql` at the end of the load.
- `rubix/extensions/com.nubeio.rubixos/block.yaml` — register the
  rollup tables under `warehouse_tables[]` and the new rollup templates
  under `warehouse_templates[]`. **The 9 existing templates at
  `block.yaml:146-196` are untouched — they already use the right
  shape.**
- `rubix/extensions/com.nubeio.rubixos/kinds/history_bucketed_1m.{sql,params.json}`
  — **new**, same shape as the existing `history_bucketed.{sql,params.json}`
  pair but sourced from the CAGG.
- `rubix/extensions/com.nubeio.rubixos/ui-src/` — panels reference
  templates by name only (`analytics_template:` source).

**New public surface (peer-review-corrected from the earlier "no new
types" claim):** one new method on `TemplateRegistry`
(`owning_extension`) plus one new field
(`owners: HashMap<String, ExtensionId>`). Everything else —
`extend_from_record`, `TimescaleAnalyticsBridge::with_registry`,
`warehouse_tables_for` — already exists on `main`.
