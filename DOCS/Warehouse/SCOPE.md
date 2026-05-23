# Warehouse — Scope

> ⚠ Read these first:
> - [ADR-003 — ClickHouse warehouse, Postgres OLTP](../storage/ADR-003-clickhouse-warehouse.md) — the storage decision.
> - [Tags SCOPE](../Tags/SCOPE.md) — the tag language this capability consumes (two compilation targets: Postgres + ClickHouse).
> - [Flow SCOPE](../flow/scope/SCOPE.md) — the engine that runs warehouse nodes.
> - [Insights SCOPE](../Insights/SCOPE.md) — the verdicts/rules consumer downstream.
>
> The warehouse owns: a three-layer data model on ClickHouse
> (raw → curated → marts), four flow node kinds, a marts catalog
> stored in Postgres, and the read contract dashboards and Insights
> bind to. It does **not** own: orchestration (flows), rule
> evaluation (insights), the LLM loop (agent), or the dashboard
> renderer (SDUI).

## One-line summary

`starter-warehouse` is the workspace's **tag-driven analytics layer**.
History (`raw_events`, `samples`, `events`, `documents`) lives in
**ClickHouse**. Dimensions (`entities`, `entity_refs`, `marts`
catalog, `tag_definitions`) live in **Postgres** with FKs. Rollup
marts are **ClickHouse incremental materialized views** backed by
`AggregatingMergeTree` and refreshed per-insert. Dashboards open in
<50 ms.

The AI agent and operators compose dashboards by **picking marts and
writing tag queries** through a catalog row, never by hand-writing
SQL the renderer has to trust.

## Hard rules (load-bearing)

### W1 — Two stores, one capability

`starter-warehouse` is the root crate. Storage is split across two
crates:

- `starter-store-postgres["dimensions"]` — adds the `entities`,
  `entity_refs`, `marts`, `tag_definitions` tables to the existing
  Postgres schema. FKs enforced. `_sqlx_migrations_dimensions`
  version table.
- `starter-store-clickhouse` — new crate. Owns `raw_events`,
  `samples`, `events`, `documents`, and the generated `mart_*`
  materialized views. ClickHouse over HTTP via the official
  `clickhouse` Rust crate.

There is no parallel storage crate beyond these two. Tests use a
ClickHouse testcontainer for the history side and the existing
Postgres testcontainer for the dimensions side.

### W2 — ClickHouse for history, Postgres for dimensions

Per [ADR-003](../storage/ADR-003-clickhouse-warehouse.md). The
seam is one-directional: ClickHouse reads Postgres dimensions
through a `Dictionary(SOURCE(POSTGRESQL(...)))`. Nothing in
Postgres reads ClickHouse.

### W3 — Layers, separated by write rate

The model is the medallion pattern, adapted to the split, with one
extra optional layer (L1.5) for analyst exploration:

| Layer | Store      | Table(s)                                                                   | Write pattern         | Read latency target | Who reads it                |
|-------|------------|-----------------------------------------------------------------------------|------------------------|----------------------|------------------------------|
| L1    | ClickHouse | `raw_events`                                                                | Append, indiscriminate | rarely               | Curation flows, debugging    |
| L1.5  | ClickHouse | `sandbox_<name>` (optional, per [`sandbox.define`](#sandboxdefine))         | Append, analyst-owned  | <500 ms              | Analysts iterating on schema |
| L2    | ClickHouse | `samples`, `events`, `documents`                                             | Append                 | <500 ms              | Insights, ad-hoc reads, AI   |
| L2-dim| Postgres   | `entities`, `entity_refs`                                                    | Low-rate UPSERT         | <50 ms               | OLTP joins, authz, AI tools  |
| L3    | ClickHouse | `mart_*` materialized views over `AggregatingMergeTree` targets             | Per-insert incremental | <50 ms               | Dashboards, SDUI             |

Retention: L1 short (14 days, hard), L1.5 analyst-controlled (default
30 days, max 1 year), L2 governed by per-table TTL (default `samples`
= 2 years with S3 tiering after 90 days), L3 derived (covers whatever
the source parts cover).

L1.5 is **not** part of the read seam — `mart.read` does not read
from sandboxes, and SDUI bindings do not target sandboxes. The
sandbox is an analyst workbench that produces a *promoted* cleaner
(L1.5 → L2 `samples`), at which point the workflow returns to the
normal L2/L3 path. See [`sandbox.define`](#sandboxdefine) for the
intended workflow.

### W4 — Tags are the only filter language at the read seam

`mart.read`, `GET /api/entities`, `GET /api/history`, and authz rules
**all accept a [`TagQuery`](../Tags/SCOPE.md) and nothing else**. Raw
SQL is never exposed to the dashboard, the AI tool surface, or the
authz layer. If a question can't be expressed as a `TagQuery`, it
goes through [Insights](../Insights/SCOPE.md)'s `rule.sql` where the
operator has accepted full SQL semantics.

### W5 — Marts are declarative; ClickHouse DDL is generated

A mart is a row in the Postgres `marts` catalog table:

```sql
CREATE TABLE marts (
  name             TEXT PRIMARY KEY,        -- "mart_energy_hourly", ^mart_[a-z0-9_]+$
  description      TEXT,
  source_table     TEXT NOT NULL,           -- usually 'samples'
  filter           JSONB NOT NULL,          -- serialised TagQuery
  time_bucket      INTERVAL NOT NULL,       -- '1 hour'
  group_by         TEXT[] NOT NULL,         -- promoted tag keys; first entry leads ORDER BY
  aggregations     JSONB NOT NULL,          -- [{"fn":"sum","col":"value_num","as":"kwh"}]
  definition_hash  TEXT NOT NULL,           -- SHA-256 of (filter,time_bucket,group_by,aggregations)
  created_by       TEXT NOT NULL,           -- 'user:…' | 'agent:…' | 'ext:<id>'
  created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  status           TEXT NOT NULL,           -- 'pending'|'live'|'quarantined'|'failed'
  CONSTRAINT marts_status_valid CHECK (status IN ('pending','live','quarantined','failed')),
  CONSTRAINT marts_created_by_valid CHECK (
    created_by LIKE 'user:%' OR created_by LIKE 'agent:%' OR created_by LIKE 'ext:%'
  )
);
```

`mart.define` reads a row and emits two ClickHouse DDL statements:

1. A target table `mart_<name>_state` of engine `AggregatingMergeTree`
   ordered by `(<first group_by column>, bucket, <remaining group_by
   columns>)`. The first `group_by` entry leads the `ORDER BY` so that
   equality filters on the primary dimension prune granules before the
   time range scan. A mart definition with `group_by = ['building',
   'tenant']` produces `ORDER BY (building, bucket, tenant)`, not
   `(bucket, building, tenant)`.
2. An incremental materialized view `mart_<name>` that reads from
   `samples` (or the declared source), buckets by `time_bucket`,
   promotes the `group_by` keys from `tags` into real columns, and
   inserts aggregate states into the target table.

`mart.read` queries the target table with `*Merge()` functions to
finalise the aggregates. **Group-by keys live as columns in the
target table, never as `tags['k']` at read time** — this is what
makes <50 ms reads reachable.

`mart.define` is idempotent when called with the same spec. If a
mart with the same name already exists in the catalog, `definition_hash`
is compared: identical hash → no-op; different hash → hard error. To
change a mart's spec, explicitly `mart.drop` it first and redefine.
This prevents silent schema drift between the catalog row and the live
ClickHouse MV.

If `mart.define` creates the target table but the MV DDL fails, the
catalog row is set to `status='failed'` and neither object is left in
an ambiguous state. The operator must inspect the error, drop any
leaked `mart_<name>_state` table manually, and rerun `mart.define`
after correction.

The mart name regex is enforced at catalog INSERT time and again at
DDL generation time. The DDL builder quotes identifiers; user input
never reaches the SQL string raw.

### W6 — Refs are FKs in Postgres; tags on ClickHouse rows are an optimisation

Per [Tags T4](../Tags/SCOPE.md#t4--refs-are-not-tags). The Postgres
`entity_refs(from_id, rel, to_id)` table is the source of truth with
`PRIMARY KEY (from_id, rel, to_id)` and FKs in both directions to
`entities(id)`. Tag-shaped refs on ClickHouse rows
(`tags['equipRef'] = 'equip_…'`) are an optimisation for fast tag
filtering inside ClickHouse; they are not enforced.

### W7 — Ingestion never refuses

L1 (`raw_events`) accepts any payload. Unknown tags pass through
(per [Tags T5](../Tags/SCOPE.md#t5--tagdefinition-is-advisory-not-a-schema-migration)).
Malformed values get a `quality` flag and a log line. Schema
mismatches land in L1 with a parse error in `tags['parse_error']`.
**Nothing is dropped on the floor.** Civilising chaotic ingest is the
*curation* job downstream, not the *tap*.

### W8 — Insert discipline: `async_insert=1` on every write connection

The store crate sets `async_insert=1, wait_for_async_insert=1` on
every ClickHouse connection. The server buffers ~1 s / 1 MB / 450
queries and flushes one part. Consequences:

- Writes from the flow engine are acked after the flush, so the
  per-row commit latency is bounded by ~1 s.
- `tap.write` and `curate.write` write one row per call. This is
  intentional and safe — `async_insert=1` means the server batches
  the flush, so one-row writes do not produce one part per row.
  **Row-at-a-time `INSERT` without `async_insert=1` is forbidden**;
  that is the invariant, not "row-at-a-time" itself.
- A `tap.write` or `curate.write` node MUST go through the store
  crate. Raw `INSERT` string construction in node code is forbidden
  and rejected by lint. This rule applies to extension-contributed
  SQL as well: the `warehouse_write` capability grants the extension
  access to the store crate API; the extension role has no direct
  `INSERT` permission on history tables.
- Bulk imports (file → samples) can override with `async_insert=0`
  and batch in-engine to N=10_000 — see W8a.

#### W8a — Bulk paths batch in-engine

A `bulk.import` flow node accumulates rows in memory and flushes
batches of 10k to ClickHouse with `async_insert=0`. This is the only
sanctioned path that bypasses server-side async buffering.

### W9 — The flow engine is the orchestrator

Warehouse nodes (`tap.write`, `curate.write`, `mart.define`,
`mart.read`, `mart.promote`, `mart.drop`, `bulk.import`,
`cleaner.define`, `cleaner.drop`, `sandbox.define`, `sandbox.drop`)
implement `NodeBehavior` from `starter-flow-spi` and register through
`starter-flow-nodes`'s descriptor list, behind a `warehouse` cargo
feature. **Mart and cleaner MV refresh is not orchestrated by the
flow engine** — ClickHouse MVs refresh themselves on every insert.
Cleaner backfill (`cleaner.define { backfill: 'async' }`) is a
flow-orchestrated `INSERT … SELECT` and *is* a flow run, but the
steady-state per-row refresh is the MV's own job.

### W10 — Apache-2.0 ClickHouse OSS features only

No ClickHouse-Cloud-specific functions (SharedMergeTree, ClickPipes).
Replication via `ReplicatedMergeTree` + `clickhouse-keeper` is
available but off the day-one path. Any future need for a Cloud-only
feature gets its own ADR.

### W11 — Dimension staleness is bounded, documented, and surfaced

The ClickHouse `entities_dict` dictionary is configured
`LIFETIME(MIN 300 MAX 600)`, so dimension reads on the ClickHouse
side trail Postgres by **up to 10 minutes (configurable via
`LIFETIME(MIN/MAX)`)**. Dashboards that need fresher dimensions read
directly from Postgres via the REST entity endpoints and join
client-side. The lag bound is part of the contract; tighten or relax
via the dictionary definition, not by ad-hoc refresh calls.

**The lag is surfaced at every read envelope.** A documented unknown
that the caller cannot observe is not a contract — it is a footnote
the caller has to remember. So every response that depends on
dimensions carries a `dimension_freshness` block:

```json
{
  "rows": [ … ],
  "dimension_freshness": {
    "entities_dict": {
      "loaded_at":   "2026-05-23T09:04:17Z",
      "age_seconds": 612,
      "lifetime_max_seconds": 600
    }
  }
}
```

Populated from `SELECT name, loading_start_time, last_successful_update_time,
lifetime_min, lifetime_max FROM system.dictionaries WHERE name='entities_dict'`,
cached on the server side for ≤ 5 s so that a burst of `mart.read`
calls does not re-query `system.dictionaries` per request.

Returned by:

- `GET /api/marts/:name/data` (the `mart.read` HTTP surface) — top
  level of the JSON envelope.
- `GET /api/warehouse/status` — under a `dimensions` key, alongside
  ingest lag and async-insert backlog.
- The `read_mart` MCP tool result — same shape as the HTTP envelope.

SDUI binds an optional "as of HH:MM" badge to `dimension_freshness`;
the agent reads `age_seconds > lifetime_max_seconds` as the signal
that a recent entity rename or delete may not yet be visible. This
is the contract; W13's `dictGetOrNull` rule is the floor that
prevents corrupted output during the lag window.

**Dictionary refresh and deletes.** `invalidate_query` polls
`max(updated_at)` — it detects updates but not deletes. An entity
deleted from Postgres stays in `entities_dict` until the `MAX 600`
lifetime expires. This is not a bug that `invalidate_query` fixes;
it is a structural property of the LIFETIME-based eviction. See W13
for the join rule that prevents deleted-entity rows from producing
corrupt dashboard output.

**Orphaned history.** Deleting an entity in Postgres leaves orphaned
rows in `samples` / `events` / `documents` (no FK on the ClickHouse
side). The TTL policy is the eventual GC — orphans expire with the
table's retention window. For deployments that need earlier cleanup,
the operator primer below includes a query to count and drop orphaned
`entity_id` values. This is an accepted operational cost of the
split-store design.

### W12 — Mart lifecycle is governed by author type

The `status` column transitions differ by who created the mart. Three
author types are recognised (enforced by the `CHECK` constraint in W5):

| `created_by` prefix | Initial status | Promotion to `live`          |
|---------------------|---------------|-------------------------------|
| `user:…`            | `pending`     | automatic on DDL success      |
| `agent:…`           | `quarantined` | explicit `mart.promote` (admin-only) |
| `ext:<id>`          | `pending` if the extension's manifest hash matches a previously-approved hash for this extension; otherwise `quarantined` | automatic on DDL success only for previously-approved manifests; new or changed manifests require explicit `mart.promote` (admin-only) |

Additional lifecycle rules:

- `mart.define` moves `pending` → `live` (for user, and for ext with
  approved manifest) or leaves `quarantined` (for agent, and for ext
  with new/changed manifest) after DDL succeeds. On DDL failure the
  row moves to `failed` — see W5 for cleanup.
- `mart.drop` moves any status → `quarantined`, executes
  `DROP VIEW IF EXISTS mart_<name>` and `DROP TABLE IF EXISTS
  mart_<name>_state`, and emits `mart.dropped` on the flow bus.
- `mart.promote` moves `quarantined` → `live` (admin-only). This is
  the only path for agent-authored marts to become live, and for
  ext-authored marts whose manifest changed since the operator last
  approved.
- Per-deployment quota: max N live marts (default 50). The catalog
  enforces with a partial-index-backed trigger that scans only
  `status = 'live'` rows on INSERT/UPDATE (`CREATE INDEX
  marts_live_count_idx ON marts (status) WHERE status = 'live'`),
  not the full catalog.

**Ext: trust seam — manifest hash approval.** An extension's manifest
hash is recorded in a small `ext_manifest_approvals` table:

```sql
CREATE TABLE ext_manifest_approvals (
  ext_id        TEXT NOT NULL,
  manifest_hash TEXT NOT NULL,            -- SHA-256
  approved_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  approved_by   TEXT NOT NULL,            -- 'user:<id>' or 'install:initial'
  PRIMARY KEY (ext_id, manifest_hash)
);
```

The extension installer seeds a row with `approved_by =
'install:initial'` for the manifest the operator approved at install
time. When the extension later ships an update, the new manifest hash
is *not* in the table; any mart the upgraded extension defines lands
`quarantined` until the operator runs `mart.promote` (which also
inserts an approval row for the new hash). The operator UI surfaces
"this extension's manifest changed; review N pending mart definitions
before promoting" as a single workflow.

This closes a real asymmetry: `agent:` is quarantined because the
LLM is untrusted; `ext:` was previously auto-live on the assumption
the manifest was reviewed at install and never again. In practice
extensions ship updates, so the re-approval gate brings `ext:`
in line with how operators actually think about extension trust.

**Extension-authored marts do not touch the `status` field directly.**
`starter-ext-warehouse` calls `mart.promote` via the warehouse
capability API after DDL succeeds (and after the approval check) —
the warehouse owns the state machine; the extension adapter is a
caller, not a writer.

### W13 — Dimension joins must surface missing entries explicitly

All dimension lookups in generated mart SQL and in `mart.read` filters
**must** use `dictGetOrNull` (ClickHouse 23.4+), not `dictGet`.
`dictGet` returns the column's default (empty string for `String`,
zero for numeric) when the key is missing — from a stale dictionary
after an entity delete, or during the up-to-10-minute lag window.
Silent empty-string joins corrupt dashboards without any error signal.

The DDL builder emits:

```sql
dictGetOrNull('entities_dict', 'display', entity_id) AS display
```

Callers (dashboards, mart reads) that receive a `NULL` display column
must render it as a visible sentinel — e.g. `"[unknown entity]"` — not
silently drop the row or coerce to empty string. The REST layer
filters `WHERE isNotNull(display)` only when the caller passes
`?hide_unknown=true` on `GET /api/marts/:name/data` or
`GET /api/entities/:id/history`; the default (`hide_unknown=false`,
or the parameter absent) surfaces the sentinel so operators can see
staleness rather than getting silently incomplete results. The
`read_mart` MCP tool accepts the same flag as a boolean argument with
the same default.

### W14 — `mart.read` filters must reference only promoted columns

Mart targets are `AggregatingMergeTree` tables; the per-sample
`tags` map does not survive aggregation. A `TagQuery` filter passed
to `mart.read` that references a key the mart did not promote into
a column cannot be evaluated against the target rows — the data is
literally not there.

The rule:

- Every key referenced by `mart.read`'s `filter` MUST be a column on
  the mart's target table — i.e., an entry in the catalog row's
  `group_by` list, or a column the catalog row's `aggregations`
  produced.
- Filters referencing keys outside that set are rejected with HTTP
  400 and a structured body naming the unsupported keys:

  ```json
  {
    "error": "mart_filter_unsupported_keys",
    "mart": "mart_energy_hourly",
    "unsupported_keys": ["floor"],
    "promoted_keys": ["building", "tenant"],
    "hint": "To filter by `floor`, redefine the mart with `floor` in `group_by`, or query `GET /api/entities/:id/history` for single-entity sample-level reads."
  }
  ```
- There is **no transparent fallback** to a `samples` scan. A
  sample-level read is a different shape with a different SLA
  (W3: <500 ms vs W3: <50 ms) and a different result schema (raw
  rows vs aggregated bucket rows). Silently switching one for the
  other turns a documented contract into a surprise.

Sample-level reads are reached deliberately via
`GET /api/entities/:id/history` (per-entity, range-scanned on
`ORDER BY (entity_id, ts)`) or, in a future iteration, a
`samples.read` flow node with its own catalog of allowed
projections.

The catalog row's pre-applied `filter` (the JSONB column on `marts`)
is **not** subject to W14 — it is applied at MV-fire time over the
raw `samples` rows, where `tags['k']` is still available. Only the
read-time `filter` argument is restricted.

### W15 — Catalog GC: terminal-state rows are pruned at 90 days

The catalog tables (`marts`, `cleaners`, `sandboxes`) accumulate
rows over time. Live-mart count is bounded by the W12 quota, but
nothing bounds the long tail of `quarantined` and `failed` rows —
an AI agent iterating on `define_mart` proposals will generate
hundreds over a year, each carrying a non-trivial filter JSONB.

A daily background job (registered by the warehouse capability,
runnable manually via `POST /api/warehouse/gc`) deletes catalog
rows matching:

```sql
WHERE status IN ('quarantined','failed')
  AND created_at < now() - INTERVAL '90 days'
```

The interval is configurable per deployment via the
`warehouse.catalog_gc_age_days` config key. A deployment that wants
to keep agent experiments forever sets this to a large value or
disables the job entirely. `live` and `pending` rows are never
auto-pruned — they require an explicit `mart.drop` /
`cleaner.drop` / `sandbox.drop`.

GC emits a `warehouse.gc.completed` flow event with counts per
status so operators can see what was reaped.

## Data model

### L1 — Raw landing zone (ClickHouse)

```sql
CREATE TABLE raw_events (
  id           UInt64,
  source       LowCardinality(String),       -- 'mqtt' | 'bacnet' | 'webhook:…' | 'flow:…'
  received_at  DateTime64(3) DEFAULT now64(3),
  payload      String,                        -- JSON text
  tags         Map(String, String) DEFAULT map(),
  INDEX tags_bloom tags TYPE bloom_filter GRANULARITY 1
)
ENGINE = MergeTree
PARTITION BY toYYYYMMDD(received_at)
ORDER BY (source, received_at)
TTL received_at + INTERVAL 14 DAY;
```

L1 is a buffer, not a museum. Default retention 14 days. Compression
is ZSTD on parts older than 3 days (set via column codecs in the
migration).

### L2 — Curated history (ClickHouse)

```sql
CREATE TABLE samples (
  entity_id    String,
  ts           DateTime64(3),
  value_num    Nullable(Float64),
  value_str    Nullable(String),
  value_bool   Nullable(UInt8),
  quality      UInt8 DEFAULT 0,               -- 0=good 1=stale 2=fault 3=override
  tags         Map(String, String) DEFAULT map(),
  INDEX tags_bloom tags TYPE bloom_filter GRANULARITY 1,
  INDEX entity_bloom entity_id TYPE bloom_filter GRANULARITY 1
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(ts)
ORDER BY (entity_id, ts)
TTL ts + INTERVAL 90 DAY TO VOLUME 's3_cold',
    ts + INTERVAL 2 YEAR DELETE;

CREATE TABLE events (
  id           UInt64,
  entity_id    String,
  ts           DateTime64(3),
  kind         LowCardinality(String),        -- 'alarm' | 'state-change' | 'note' | …
  payload      String,
  tags         Map(String, String) DEFAULT map(),
  INDEX tags_bloom tags TYPE bloom_filter GRANULARITY 1
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(ts)
ORDER BY (kind, entity_id, ts)
TTL ts + INTERVAL 1 YEAR;

-- documents: blob index rows. The blob itself lives in BlobStore.
CREATE TABLE documents (
  id           String,
  entity_id    String,
  ts           DateTime64(3) DEFAULT now64(3),
  blob_ref     String,
  mime         LowCardinality(String),
  tags         Map(String, String) DEFAULT map(),
  INDEX tags_bloom tags TYPE bloom_filter GRANULARITY 1
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(ts)
ORDER BY (entity_id, ts);
```

Notes:

- `samples.entity_id` is **not** a foreign key. Integrity is enforced
  upstream by `tap.write` / `curate.write`, which look up the entity
  in Postgres before inserting. Cleaner MVs cannot enforce this — see
  the `cleaner.define` entity-integrity caveat below.
- `Map(String, String)` is the canonical tag bag. `TagValue::Bool`
  serialises to `"true"`/`"false"`; `TagValue::Str` is stored as-is.
  Tags carry no float values by design — measurements live in
  `value_num`. See [Tags T2](../Tags/SCOPE.md#t2--tags-are-a-flat-mapstring-tagvalue-tag-values-are-bool--str).
- `bloom_filter` skip index on `tags` accelerates equality
  containment (`tags['k'] = 'v'`) — the dominant query shape for
  low-to-medium-cardinality tag values. **For high-cardinality tag
  values** (e.g. `tags['equipRef'] = 'equip_xyz'` against `events`
  or `documents`, where the equip-ID space can be in the millions),
  the bloom filter saturates and skip-index pruning degrades. The
  `samples` table is fine because `ORDER BY (entity_id, ts)` carries
  the per-entity lookup; `events` (`ORDER BY (kind, entity_id, ts)`)
  and `documents` (`ORDER BY (entity_id, ts)`) similarly rely on the
  ORDER BY for entity-keyed lookups. Operators who find a high-card
  tag-key filter becoming a hot path should add a per-key
  `set(N)` skip index in a follow-up migration; the doc does not
  add one preemptively.

### L2-dim — Dimensions (Postgres)

```sql
CREATE TABLE entities (
  id          TEXT PRIMARY KEY,              -- "ent_01H…"
  kind        TEXT NOT NULL,
  display     TEXT,
  tags        JSONB NOT NULL DEFAULT '{}',
  created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX entities_tags_gin ON entities USING GIN (tags jsonb_path_ops);
CREATE INDEX entities_kind     ON entities (kind);

CREATE TABLE entity_refs (
  from_id  TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
  rel      TEXT NOT NULL,                    -- 'equipRef' | 'siteRef' | 'floorRef' | 'pointOf'
  to_id    TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
  PRIMARY KEY (from_id, rel, to_id)
);
CREATE INDEX entity_refs_to   ON entity_refs (to_id, rel);

CREATE TABLE tag_definitions ( ... );        -- see Tags SCOPE T5
CREATE TABLE marts ( ... );                  -- see W5 above
```

Exposed to ClickHouse via:

```sql
-- inside ClickHouse
CREATE DICTIONARY entities_dict (
    id         String,
    kind       String,
    display    String,
    tags       String
)
PRIMARY KEY id
SOURCE(POSTGRESQL(
    port 5432 host 'pg' user 'warehouse_ro' password '…' db 'app'
    table 'entities' invalidate_query 'SELECT max(updated_at) FROM entities'
))
LIFETIME(MIN 300 MAX 600)
LAYOUT(HASHED());
```

Dashboards join `samples` to `entities_dict` via
`dictGetOrNull('entities_dict', 'display', s.entity_id)`. See W11 for
the staleness bound and W13 for the join rule.

### L3 — Marts (ClickHouse incremental MVs)

Catalog row `mart_energy_hourly` with `group_by = ['building',
'tenant']` and one `sum` aggregation generates:

```sql
-- target
CREATE TABLE mart_energy_hourly_state (
  bucket     DateTime,
  building   String,
  tenant     String,
  kwh_sum_state    AggregateFunction(sum, Float64),
  kwh_peak_state   AggregateFunction(max, Float64),
  kwh_avg_state    AggregateFunction(avg, Float64),
  samples_state    AggregateFunction(count, UInt64)
)
ENGINE = AggregatingMergeTree
PARTITION BY toYYYYMM(bucket)
ORDER BY (bucket, building, tenant);

-- incremental MV
CREATE MATERIALIZED VIEW mart_energy_hourly
TO mart_energy_hourly_state AS
SELECT
  toStartOfHour(ts)            AS bucket,
  tags['building']             AS building,
  tags['tenant']               AS tenant,
  sumState(value_num)          AS kwh_sum_state,
  maxState(value_num)          AS kwh_peak_state,
  avgState(value_num)          AS kwh_avg_state,
  countState()                 AS samples_state
FROM samples
WHERE tags['kind'] = 'energy'
GROUP BY bucket, building, tenant;
```

Reads at the seam look like:

```sql
SELECT
  bucket,
  building,
  sumMerge(kwh_sum_state)  AS kwh,
  maxMerge(kwh_peak_state) AS peak
FROM mart_energy_hourly_state
WHERE bucket >= ? AND bucket < ?
  AND building = ?
GROUP BY bucket, building
ORDER BY bucket;
```

The DDL is **generated** from the catalog row; operators do not write
it by hand. The catalog row is the artefact under version control.

## Node kinds

Each implements `starter-flow-spi::NodeBehavior` and is registered via
`starter-flow-nodes`'s descriptor list, behind a `warehouse` cargo
feature.

### `tap.write`

Appends one row to `raw_events` via `async_insert=1`. Input slots:
`payload` (string/JSON), `source` (string), `tags` (TagSet). Output:
`event_id` (uint64). Used at every ingestion edge. One row per call is
intentional — `async_insert=1` batches on the server side.

### `curate.write`

Writes typed curated rows to `samples` / `events` / `documents`.
Resolves `entity_id` against Postgres `entities` before writing
(rejects unknown entities — caller must create the entity in Postgres
first). Merges entity-level tags into the row's `tags` map at write
time per [Tags T6](../Tags/SCOPE.md#t6--one-reserved-namespace-documented).

### `bulk.import`

The only sanctioned non-async-insert path. Accumulates 10k rows in
memory, then `INSERT … FORMAT RowBinary` with `async_insert=0`. Used
for CSV/Parquet file imports.

Input slot `target` selects the destination:

- `target: 'samples'` — typed write into `samples`. The caller is
  asserting they know the schema and the rows pass the typed
  contract. Engineer-grade default for known imports.
- `target: 'sandbox:<name>'` — write into the analyst sandbox table
  `sandbox_<name>` (see [`sandbox.define`](#sandboxdefine)). Schema
  is inferred from the first batch if the sandbox is empty;
  subsequent imports must conform. Analyst-grade default for
  "I have a CSV and want to poke at it."
- `target: 'raw_events'` — write into the L1 buffer with a `source`
  tag identifying the bulk import. Use when the goal is to feed an
  existing cleaner.

If `target` is omitted, the node errors. There is no implicit
default — the choice between the three is meaningful enough that it
must be made explicitly.

### `sandbox.define`

Creates a user-owned L1.5 sandbox table `sandbox_<name>` for analyst
exploration of unfamiliar data. The catalog row in Postgres
(`sandboxes` table) carries the analyst's chosen TTL, the columns
(inferred from first batch or declared explicitly), the owner, and
a `promoted_to_cleaner` field that points at the cleaner row once
the analyst is happy with the shape.

```sql
CREATE TABLE sandboxes (
  name                  TEXT PRIMARY KEY,    -- ^sandbox_[a-z0-9_]+$
  description           TEXT,
  owner                 TEXT NOT NULL,       -- 'user:<id>' | 'agent:<id>'
  columns               JSONB NOT NULL,      -- inferred or declared schema
  ttl_days              INT  NOT NULL DEFAULT 30 CHECK (ttl_days BETWEEN 1 AND 365),
  promoted_to_cleaner   TEXT,                -- nullable; FK-like ref to cleaners(name)
  created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  status                TEXT NOT NULL,       -- 'pending'|'live'|'promoted'|'failed'
  CONSTRAINT sandboxes_status_valid CHECK (status IN ('pending','live','promoted','failed'))
);
```

The generated ClickHouse DDL is a plain `MergeTree` table — **no
materialized view, no aggregation**, so the analyst can drop columns,
add columns, and re-import without invalidating dependents:

```sql
CREATE TABLE sandbox_<name> (
  ts            DateTime64(3) DEFAULT now64(3),
  <inferred or declared columns>,
  tags          Map(String, String) DEFAULT map(),
  INDEX tags_bloom tags TYPE bloom_filter GRANULARITY 1
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(ts)
ORDER BY (ts)
TTL ts + INTERVAL <ttl_days> DAY;
```

Workflow (the named analyst-CSV story):

1. `sandbox.define { name: 'utility_bills_2025', ttl_days: 60 }`
   creates an empty table.
2. `bulk.import { target: 'sandbox:utility_bills_2025', file: '…' }`
   lands the CSV. Schema is inferred and recorded on the catalog row.
3. The analyst queries the sandbox via the
   `GET /api/sandboxes/:name/peek` endpoint (returns up to 1000
   rows; not a dashboard surface — explicitly an exploration tool).
4. Once the analyst knows the target shape:
   `cleaner.define { source: 'sandbox:utility_bills_2025',
   target: 'samples', backfill: 'sync', … }` promotes the data.
   The sandbox catalog row's `status` moves to `promoted` and
   `promoted_to_cleaner` is set.
5. `sandbox.drop` removes the sandbox once the promotion is
   verified (the catalog row is retained with `status = 'promoted'`
   for traceability, but the ClickHouse table is dropped to reclaim
   storage).

A sandbox is **not** the read seam for dashboards. The W4 rule still
holds: dashboards bind to marts on L3, not to L1.5. A sandbox that
never promotes expires with its TTL and the catalog row is GC'd by
W15 once `status = 'failed'` or after the operator manually drops it.

### `sandbox.drop`

Drops the ClickHouse sandbox table, sets the catalog row's `status`
to `promoted` (if a cleaner was promoted from it) or `failed` (if
not). Idempotent.

### `mart.define`

Idempotent. Input: a `marts` catalog row (by name or inline).
Behaviour:

1. Upsert the catalog row in Postgres (`status='pending'`).
2. Generate the ClickHouse target-table + MV DDL from the row.
3. Execute the DDL on ClickHouse. On success, transition the catalog
   row to `status='live'`.
4. Emit `mart.defined` on the flow bus.

Authored once per mart, then re-run is a no-op. AI-authored marts
start in `quarantined` per W12 — admin promotion is a separate node
(`mart.promote`) gated on the user's role.

### `mart.read`

Reads from a `live` mart by name. Input: `name`, `filter` (TagQuery),
`range`, optional `group_by` override (must be a subset of the
catalog row's `group_by`). Output: a `Dataset` with a top-level
`dimension_freshness` block per [W11](#w11--dimension-staleness-is-bounded-documented-and-surfaced).

Filter validation: every key referenced by `filter` MUST be either
(a) a column promoted into the mart target (`group_by` entry or a
column produced by an aggregation), or (b) already pre-applied by the
catalog row's `filter`. Filters referencing tag keys that the mart
did not promote are rejected with HTTP 400 and a message naming the
unsupported keys — see [W14](#w14--martread-filters-must-reference-only-promoted-columns).

Settings include `materialise_if_missing: false` (W12). A read of a
non-`live` mart errors loudly. Mart creation never happens on the
read path.

### `mart.drop`

Marks the catalog row `quarantined`, drops the ClickHouse view +
target table, emits `mart.dropped`. Required for AI experiment
cleanup.

### `cleaner.define`

Defines a user-authored L1 → L2 curation MV. A cleaner is a catalog
row in Postgres (`cleaners` table, parallel to `marts`) describing a
materialized view that reads from `raw_events` (or a sandbox table —
see [`sandbox.define`](#sandboxdefine)) and writes typed rows into
`samples`, `events`, or `documents`. The DDL is generated from the
catalog row; operators do not write it by hand.

This is the first-class path for bulk curation. Users and operators
can define cleaners from the UI or the REST API without shipping an
extension. Extension-contributed cleaners (see extensions
`contributes.warehouse`) go through the same catalog and the same
DDL builder — the difference is `created_by = 'ext:<id>'` and the
trust seam is the manifest hash, not an admin promotion.

`cleaner.define` follows the same lifecycle as `mart.define`: pending
→ live on DDL success, failed on DDL error, quarantined on drop.
Agent-authored cleaners start quarantined.

**Backfill.** ClickHouse incremental MVs only fire on *new* inserts;
they do not retroactively process rows already in the source table.
A naïve `cleaner.define` against a `raw_events` (or sandbox) table
that already contains rows produces a live MV that sees zero data
until the next insert — a footgun the analyst-CSV workflow will hit
on its first run.

The catalog row carries a `backfill` field:

| `backfill` value | Behaviour                                                                                              |
|------------------|---------------------------------------------------------------------------------------------------------|
| `'none'` (default) | Create MV only. No historical processing. Safe for cleaners on streams where pre-existing rows are intentionally ignored. |
| `'sync'`           | Create MV, then run `INSERT INTO <target> SELECT … FROM <source> WHERE <source.ts column> < now()` in the same transaction as DDL success. Blocks the node until backfill completes. Use for analyst CSV imports where the source is bounded. |
| `'async'`          | Create MV, then enqueue the same `INSERT … SELECT` as a background flow run. Status visible on `GET /api/cleaners/:name`. Use when the source is large enough that synchronous wait is impractical. |

`POPULATE` (ClickHouse's built-in MV backfill keyword) is **not
used**: it inserts during DDL with the inserting block locked, which
under our `async_insert=1` regime blocks ingest for the duration of
the backfill and can mask new rows that arrive mid-backfill. The
explicit `INSERT … SELECT` path is bounded by a `WHERE ts < now()`
predicate and runs after the MV is live, so the MV catches anything
that arrives during the backfill window without double-counting
(idempotency relies on the cleaner's `INSERT` selecting deterministic
target keys; cleaners that produce non-deterministic IDs must declare
`backfill='none'`).

**Entity-integrity caveat.** `curate.write` resolves `entity_id`
against Postgres before inserting into `samples` / `events` /
`documents`, so it can reject unknown entities at write time. A
cleaner MV cannot do this — it is pure ClickHouse SQL and the
Postgres `entities` table is reachable only through the
`entities_dict` dictionary, which lags by up to 10 minutes (W11)
and is by design unreliable for "does this entity exist *right
now*" checks.

The consequence: a cleaner that promotes `raw_events.tags['entityRef']`
into `samples.entity_id` *will* produce dangling `entity_id`s when
ingest races entity creation, when an entity is deleted, or when
the source row carries a typo. The W13 `dictGetOrNull` rule
guarantees those rows surface as `[unknown entity]` rather than
silent empty-string joins, and the orphan audit query in the
operator primer finds them after the fact.

If a deployment needs strict pre-write entity validation, use
`curate.write` (one row per call, Postgres roundtrip in the path)
instead of `cleaner.define`. The two paths are deliberately
asymmetric: `curate.write` trades throughput for integrity;
`cleaner.define` trades integrity for throughput. The doc names the
trade rather than papering over it.

## REST and SSE surface

REST (mounted under `/api`):

| Method | Path                              | Notes                                                              |
|--------|-----------------------------------|--------------------------------------------------------------------|
| GET    | `/api/entities`                   | `?query=<TagQuery>` → Postgres, GIN-accelerated                    |
| GET    | `/api/entities/:id`               | Full entity + tags + refs                                          |
| POST   | `/api/entities`                   | Create (admin/agent)                                               |
| PATCH  | `/api/entities/:id/tags`          | Merge tags (advisory validation)                                   |
| DELETE | `/api/entities/:id/tags/:key`     | Remove a single tag key                                            |
| GET    | `/api/entities/:id/history`       | `?from&to&downsample` → ClickHouse `samples` for one entity        |
| GET    | `/api/marts`                      | List catalog (Postgres)                                            |
| GET    | `/api/marts/:name`                | Catalog row + last-flush timestamp + status                        |
| POST   | `/api/marts`                      | Insert catalog row, fire `mart.define`                             |
| DELETE | `/api/marts/:name`                | Fire `mart.drop`                                                   |
| POST   | `/api/marts/:name/promote`        | Move quarantined → live (admin only)                               |
| GET    | `/api/marts/:name/data`           | `?filter=<TagQuery>&from&to&group_by=…&hide_unknown=` → ClickHouse rollup |
| GET    | `/api/cleaners`                   | List cleaners catalog (Postgres)                                   |
| GET    | `/api/cleaners/:name`             | Catalog row + backfill status (`pending` / `running` / `complete` / `failed`) |
| POST   | `/api/cleaners`                   | Insert catalog row, fire `cleaner.define`                          |
| DELETE | `/api/cleaners/:name`             | Drop the MV and quarantine the catalog row                         |
| POST   | `/api/cleaners/:name/promote`     | Move quarantined → live (admin only)                               |
| GET    | `/api/sandboxes`                  | List sandbox catalog (Postgres)                                    |
| GET    | `/api/sandboxes/:name`            | Catalog row + row count + last write timestamp                     |
| POST   | `/api/sandboxes`                  | Create a sandbox table                                             |
| GET    | `/api/sandboxes/:name/peek`       | `?limit=` (≤ 1000) → recent rows. **Not a dashboard surface.**     |
| DELETE | `/api/sandboxes/:name`            | Drop the sandbox table; retain catalog row for traceability        |
| POST   | `/api/warehouse/gc`               | Run catalog GC immediately (W15); admin only                       |

SSE (event push — use for things that happen, not for polled scalars):

| Path                          | Stream                                                              |
|-------------------------------|---------------------------------------------------------------------|
| `GET /api/marts/events`       | `mart.defined`, `mart.promoted`, `mart.dropped`                     |
| `GET /api/entities/events`    | `entity.created`, `entity.tagged`, `entity.deleted`                 |

All SSE handlers use `starter-server`'s `sse::keepalive` (15 s).

Pull (poll at dashboard cadence — use for slowly-changing scalars):

| Path                          | Response                                                            |
|-------------------------------|---------------------------------------------------------------------|
| `GET /api/warehouse/status`   | JSON: ingest lag, async-insert backlog, `dimensions` block per W11   |

`/api/warehouse/status` is a plain JSON GET, not SSE. Ingest lag and
dictionary staleness change on the order of seconds to minutes; a
long-lived SSE connection per dashboard client is wasteful for this
use case. The dashboard polls at its own cadence (suggested 30 s).

## Consumers

### Insights

[`starter-insights`](../Insights/SCOPE.md) rules consume `Dataset`s
from `mart.read` (rolled up, fast) or from a `rule.sql` directly
against ClickHouse (raw, slower but exact). Verdicts carry their own
tags per Insights R-ins-8.

### Pages / SDUI

SDUI components bind to a `MartQuery { mart, filter: TagQuery, range,
group_by? }`. The renderer calls `GET /api/marts/:name/data`. Page
authorship is the AI agent's job (see below); operators can
hand-author pages too.

### Authz

Page-level and row-level access control are expressed as `TagQuery`
predicates per principal. The authz layer evaluates the tag query
against the entity row's tag set (Postgres, GIN). For history-level
authz (rare — most reads go through marts), the same tag query
compiles to the ClickHouse target via
[Tags T8b](../Tags/SCOPE.md#t8--two-canonical-compilation-targets).

### AI agent / MCP

Tools:

1. `query_entities(q: TagQuery, limit?: u32) -> Vec<Entity>` (Postgres)
2. `tag_entity(id, tags: TagSet) -> Entity` (Postgres)
3. `define_mart(spec: MartSpec) -> Mart` (Postgres catalog → CH DDL, lands `quarantined`)
4. `drop_mart(name) -> ()` (admin-gated)
5. `read_mart(name, filter, range, group_by?, hide_unknown?) -> Dataset` (ClickHouse; result carries `dimension_freshness` per W11)
6. `define_sandbox(spec: SandboxSpec) -> Sandbox` (Postgres catalog → CH DDL; analyst exploration)
7. `peek_sandbox(name, limit?) -> Vec<Row>` (read-only, ≤ 1000 rows; not a dashboard surface)

A `propose_dashboard(question)` skill lives in `starter-skills`, not
the warehouse — it composes the tools above plus the SDUI authoring
tools.

## Migrations

```
crates/starter-store-postgres/migrations/
  dimensions/
    0001_entities.sql
    0002_entity_refs.sql
    0003_tag_definitions.sql
    0004_marts_catalog.sql
    0005_cleaners_catalog.sql
    0006_sandboxes_catalog.sql
    0007_ext_manifest_approvals.sql

crates/starter-store-clickhouse/migrations/
  0001_raw_events.sql
  0002_samples.sql
  0003_events.sql
  0004_documents.sql
  0005_entities_dict.sql        -- CREATE DICTIONARY entities_dict
```

Postgres version table: `_sqlx_migrations_dimensions`. ClickHouse
migrations run via a small in-crate runner (the ecosystem has no
`sqlx::migrate` equivalent). Migration ordering is linear; each file
is one DDL statement (ClickHouse DDL is non-transactional, so each
file must be safely re-runnable — `IF NOT EXISTS` everywhere).

Cargo features: `warehouse` on `starter-store-clickhouse`,
`dimensions` on `starter-store-postgres`. Both default-off.

## Operator primer (ClickHouse shortlist)

A consumer running this in production needs to know:

- `SELECT table, formatReadableSize(total_bytes), total_rows FROM
  system.tables WHERE database='default';` — table sizes including
  compressed parts.
- `SELECT partition, name, rows, bytes_on_disk FROM system.parts
  WHERE table='samples' AND active;` — part count and size; many small
  parts = trouble.
- `SELECT * FROM system.asynchronous_inserts;` — current async-insert
  buffer state.
- `SELECT * FROM system.dictionaries WHERE name='entities_dict';` —
  dictionary load status, last update time, and whether the last
  invalidation check fired.
- `OPTIMIZE TABLE mart_<name>_state FINAL;` — force merge of an
  aggregate target during incident triage. Costly; do not schedule.
- `ALTER TABLE samples DROP PARTITION 'YYYYMM';` — emergency drop.

**Orphan audit** — entity deleted in Postgres but samples still in
ClickHouse. Run periodically or before a storage audit:

```sql
-- Count orphaned entity_ids in samples (no matching entity in dict)
SELECT entity_id, count() AS rows
FROM samples
WHERE dictGetOrNull('entities_dict', 'id', entity_id) IS NULL
GROUP BY entity_id
ORDER BY rows DESC
LIMIT 100;
```

To drop orphaned rows for a specific entity:

```sql
ALTER TABLE samples DELETE WHERE entity_id = '<orphaned_id>';
```

Lightweight DELETE is the correct tool here; use it sparingly and
only after confirming the entity is genuinely gone from Postgres.

Anything beyond this list is escalation territory.

## Phases

| Phase | Deliverable                                                                                                |
|-------|------------------------------------------------------------------------------------------------------------|
| 1     | `starter-tags` crate lands with PG + CH compile targets (see Tags SCOPE T8).                               |
| 2     | `starter-store-postgres["dimensions"]` migrations 0001–0007 land (entities, refs, tag_defs, marts, cleaners, sandboxes, ext_manifest_approvals). |
| 3     | `starter-store-clickhouse` crate lands with migrations 0001–0005; CH testcontainer wired.                  |
| 4     | `starter-warehouse` with `tap.write` + `curate.write` + `bulk.import` nodes; flow-agent ingests to L2.     |
| 5     | `mart.define` + `mart.read` + `mart.drop` + `mart.promote` + `cleaner.define` + sandbox nodes; catalog endpoints; example mart, cleaner, and analyst-CSV sandbox workflow. `dimension_freshness` envelope wired (W11). W14 filter validation enforced. |
| 6     | SDUI binding to `MartQuery`; example dashboard page driven by tags. SDUI surfaces `dimension_freshness` as an "as of" badge. |
| 7     | AI MCP tools (`query_entities`, `tag_entity`, `define_mart`, `drop_mart`, `read_mart`, `define_sandbox`, `peek_sandbox`) + `propose_dashboard`. |
| 8     | Authz tag-query predicates wired into the server.                                                          |
| 9     | Catalog GC daemon (W15) and `ext_manifest_approvals` re-approval workflow in the operator UI.              |

Each phase ends with `cargo test -p starter-warehouse` green + a
manual smoke note in the flow-agent README.

## Non-goals

- ❌ Putting OLTP tables in ClickHouse. Flows, agents, users,
  sessions, entities, entity_refs, marts catalog all stay in Postgres.
- ❌ ClickHouse Cloud or any Cloud-only feature. Self-hosted OSS only.
- ❌ Streaming ingest (Kafka, Pulsar, CDC). Flow engine writes via HTTP.
- ❌ Raw-SQL dashboard bindings. Tag queries only at the read seam.
- ❌ Streaming aggregates (windowed tap.write → ring buffer). Insights'
  `window.tumble` / `window.slide` are the streaming primitives;
  warehouse marts are incremental on insert.
- ❌ Multi-tenant DB-level isolation. Tenancy is a tag (`tenant:…`)
  enforced via the authz layer.
- ❌ Cross-database replication of marts. Single ClickHouse instance
  by design.

## File-size budget

Per workspace R1 (≤ 400 lines per file):

| File                                              | Target |
|---------------------------------------------------|--------|
| `src/lib.rs`                                      | < 80   |
| `src/nodes/tap_write.rs`                          | < 200  |
| `src/nodes/curate_write.rs`                       | < 350  |
| `src/nodes/bulk_import.rs`                        | < 300  |
| `src/nodes/mart_define.rs`                        | < 350  |
| `src/nodes/mart_read.rs`                          | < 300  |
| `src/nodes/mart_drop.rs`                          | < 150  |
| `src/nodes/cleaner_define.rs`                     | < 350  |
| `src/nodes/sandbox_define.rs`                     | < 250  |
| `src/catalog/marts.rs`                            | < 300  |
| `src/catalog/cleaners.rs`                         | < 250  |
| `src/catalog/sandboxes.rs`                        | < 200  |
| `src/catalog/gc.rs` (W15)                         | < 150  |
| `src/dim_freshness.rs` (W11)                      | < 120  |
| `src/ddl/mart.rs`                                 | < 300  |
| `src/ddl/cleaner.rs`                              | < 250  |
| `src/ddl/sandbox.rs`                              | < 150  |
| `src/store/clickhouse/samples.rs`                 | < 300  |
| `src/store/clickhouse/raw_events.rs`              | < 200  |

If a file approaches the limit, split by concept.

## Decisions

### D1 — ClickHouse for history, Postgres for dimensions

Pinned by [ADR-003](../storage/ADR-003-clickhouse-warehouse.md). Not
re-litigated here.

### D2 — Marts are declarative catalog rows in Postgres; CH DDL is generated

`mart.define` reads a Postgres catalog row, generates the ClickHouse
target table + MV DDL, executes it. The catalog row is the artefact
users own; the DDL is an implementation detail.

### D3 — Refresh is the MV's own job, not the flow engine's

ClickHouse incremental MVs fire on every insert. There is no refresh
policy to schedule, no backfill window, no missed chunks.
Flow-scheduled rollups are not used.

### D4 — Ingestion never refuses

Per W7. Cost of dropping a malformed row at the tap is unbounded
(lost data, blind debugging). Cost of accepting it into L1 with a
parse-error tag is bounded.

### D5 — Apache-2.0 ClickHouse OSS only

Per W10. Keeps the licensing story trivial.

### D6 — Mart materialisation never happens on the read path

`mart.read` rejects non-`live` marts. AI-authored marts start
`quarantined` and need explicit promotion. Mart creation is always
an explicit, recorded decision.

### D7 — Group-by keys are promoted to columns in the mart target

The mart's MV pulls `tags['k']` into a real column in the
`AggregatingMergeTree` target. Dashboards never filter on
`tags['k']` at read time on a mart — they filter on the promoted
column, which is part of the `ORDER BY` and therefore index-pruned.
This is the structural reason <50 ms reads are reachable.

### D8 — `async_insert=1` is the default, `bulk.import` is the override

Per W8 / W8a. Two write paths, both clearly named.
