# Cache operator runbook

The opt-in read-cache for extension kind dispatch. Lives in
[`crates/starter-cache`](../../../crates/starter-cache/) and is wired
at the extension dispatcher per [the v0 proposal](../proposal/fe-cache-opt-in.md).
This runbook is the *operator* surface: how to inspect, intervene,
and tune without re-reading source.

If you are debugging the *code*, start at
[`rubix/docs/sessions/cache-v0-progress.md`](../sessions/cache-v0-progress.md) —
that has the design decisions, the tradeoffs, and the still-open work.

## Quick reference

| What you want                                | How                                                       |
| -------------------------------------------- | --------------------------------------------------------- |
| See hit/miss/latency for every cached kind   | `GET /api/v1/admin/cache/specs`                           |
| Drop entries tied to a table                 | `POST /api/v1/admin/cache/invalidate {"tags":["table:X"]}` |
| Drop a single tenant's whole cache           | `DELETE /api/v1/admin/cache/tenants/{tenant}`             |
| Drop **everything** (last-resort)            | `POST /api/v1/admin/cache/invalidate_all`                 |
| Tune the per-tenant entry cap                | `RUBIX_CACHE_PER_TENANT_MAX_ENTRIES=<N>` env var          |
| Find a typo / stale sidecar                  | grep `rubix.boot.extensions` warn logs at startup         |

All admin endpoints sit under the same `Role::Admin` gate as the
rest of `/api/v1/admin/*`.

## Anatomy of the response

```json
{
  "specs": [
    {
      "spec_id": "com.nubeio.rubixos::com.nubeio.rubixos.warehouse_query",
      "extension": "com.nubeio.rubixos",
      "contribute_id": "com.nubeio.rubixos.warehouse_query",
      "config": {
        "ttl_seconds": 60,
        "scope": "user",
        "invalidate_on_tables": ["com_nubeio_rubixos__histories"],
        "stale_while_revalidate_seconds": 30,
        "empty_ttl_seconds": 5,
        "cache_empty": true,
        "invalidate_on_events": [],
        "invalidate_on_buckets": {
          "table": "com_nubeio_rubixos__histories",
          "granularity": "1h"
        },
        "time_series": {
          "time_param": "to",
          "range_param": "from",
          "bucket": "1h",
          "tail_ttl": "30s",
          "body_ttl": "24h",
          "align_to": "utc"
        },
        "inner_scope": "tenant",
        "invalidator_kind": "local"
      },
      "hits": 124,
      "misses": 17,
      "hit_ratio": 0.879,
      "load_latency": {
        "le_10ms": 0, "le_100ms": 4, "le_1s": 12, "le_10s": 1, "gt_10s": 0,
        "count": 17, "sum_nanos": 8421330000, "mean_ms": 495.4
      }
    }
  ],
  "warmer": {
    "last_run_at": 1717072800,
    "entries_warmed": 50,
    "last_duration_ms": 1827
  }
}
```

Each row reflects one sidecar. Key fields:

- `config` — verbatim shape of the `<id>.cache.yaml` sidecar. If this
  looks wrong, the sidecar shipped wrong; don't go hunting in code.
- `config.stale_while_revalidate_seconds` (v1) — `0` when SWR is
  disabled, otherwise the SWR window in seconds. See "SWR explained".
- `config.empty_ttl_seconds` / `config.cache_empty` (v1) — empty-result
  caching. `empty_ttl` is clamped to `min(empty_ttl, ttl)`.
- `config.invalidate_on_events` (v1) — write-path event tags the spec
  subscribes to (e.g. `event:ingest.batch.committed`).
- `config.invalidate_on_buckets` (v1) — `{table, granularity}` block,
  `null` when the spec declares no bucket subscription. Writers fire
  `bucket:<table>:<floor(t, granularity)>` per-row; the chokepoint
  dedupes per batch.
- `config.time_series` (v2) — `null` when absent; otherwise the
  windowed-read block (`time_param`, `range_param`, `bucket`,
  `tail_ttl`, `body_ttl`, `align_to`). Authoring is covered below.
- `config.inner_scope` (v2) — `null` when absent; `"tenant"` for the
  two-layer cache pattern (`scope: user` + `inner_scope: tenant`).
- `config.invalidator_kind` (v3) — `"local"` or `"event-bus"`,
  reflecting `RUBIX_CACHE_INVALIDATOR` at boot. Use to verify the
  env var was honoured.
- `hits` / `misses` / `hit_ratio` — counters since process start.
- `load_latency` — **miss-path only**. Hits don't pollute the
  histogram, so this measures what the cache is shielding callers
  from. `mean_ms` is the derived mean; the bucket counts let you
  spot bimodal distributions.
- `config: null` — appears when a counter row has no matching
  registered spec. Shouldn't happen in practice; treat as drift.
- `warmer` (v3, top-level) — present only when
  `RUBIX_CACHE_WARM_ON_BOOT` is set. `last_run_at` is wall-clock
  seconds since epoch, or `null` if no pass has run yet;
  `entries_warmed` is from the last pass; `last_duration_ms` is `null`
  before the first pass completes.

## Diagnosing common scenarios

### "Why is this spec showing zero hits and zero misses?"

The spec is registered (sidecar parsed) but the dispatcher has
never been called with that kind. Either no traffic is reaching it,
or the kind id in the sidecar filename doesn't match the
dispatcher's contribute_id.

The orphan check at boot already catches the second case — look for
warnings like `cache sidecar references unknown contribute_id` in
the `rubix.boot.extensions` target. If you see one, the sidecar
filename is wrong: it must be the **full reverse-DNS id**, e.g.
`com.nubeio.rubixos.warehouse_query.cache.yaml`, not the bare
trailing segment. (See the session-3 entry in the v0 progress doc
for the why.)

### "Hit ratio looks bad"

Three first-line causes, in order of likelihood:

1. **The input differs every call.** If the dispatch input includes
   a timestamp like `to=now` (sliding window), every request gets a
   different `input_hash` and therefore a different cache key. v0
   does *not* have the windowed-fetch / time-series support that
   would fix this (it's a deferred-proposal feature). Workaround:
   ask the caller to snap timestamps to a bucket boundary client-side.
2. **The spec is set to `scope: user` for a query that doesn't
   depend on the user.** Two users in the same tenant pay the
   warehouse cost twice for what is one query. Consider
   `scope: tenant` if the answer is genuinely per-tenant (not
   per-user) and the rendered output doesn't depend on user prefs.
   (Note: locale and unit prefs *do* make most warehouse-backed
   reads user-scoped; verify before flipping.)
3. **Aggressive invalidation.** Check the `invalidate_on_tables`
   list — every write to one of those tables drops every cached
   entry tied to it. If you have one cache spec invalidated by a
   high-write table, hit rate is going to be poor by design.

### "Mean load latency is dropping over time"

Almost always means hits are masking misses in the *count* —
remember: only misses populate the histogram. A dropping mean with
constant hit ratio is normal as fast misses (cold path 1-bucket
queries) replace slow misses (90-day queries) in the bucket counts.

If you want first-call latency specifically, restart the process
and watch `load_latency.count == 1` then read `mean_ms`.

### "We deleted/disabled a tenant; how do I reclaim cache memory?"

```bash
curl -X DELETE http://<host>/api/v1/admin/cache/tenants/<tenant-id>
```

Returns `{tenant, entries_dropped}`. Unknown tenant returns 200 with
`entries_dropped: 0` — by design (your intent is satisfied either
way). The per-tenant moka cache is dropped along with the entries;
subsequent traffic for that tenant lazily reallocates a fresh one.

### "A schema migration just landed; how do I force-invalidate one table's cached data?"

```bash
curl -X POST http://<host>/api/v1/admin/cache/invalidate \
     -H 'content-type: application/json' \
     -d '{"tags":["table:com_nubeio_rubixos__histories"]}'
```

Returns `{invalidated: N}`. Every cached entry whose stored token
snapshot depended on that tag becomes a miss on next read.
Multiple tags in one call is fine; empty array is accepted as a
no-op.

This is the same mechanism the v3 `WarehouseWriter` chokepoint
fires automatically on every committed batch — see
"v3 — WarehouseWriter chokepoint" below. The manual endpoint is
the escape hatch for cases the chokepoint doesn't see (out-of-band
SQL, manual schema fixes, post-incident reset of one tag).

### "The cache is in a bad state, drop everything"

Last-resort escape hatch when the surgical `/invalidate` and
`/tenants/{id}` aren't enough — corrupted serialiser output,
unexplained stale data, post-incident reset:

```bash
curl -X POST http://<host>/api/v1/admin/cache/invalidate_all
```

Returns `{entries_dropped: N}`. Per-spec counters and tag tokens
survive, so the operator can watch hit rate recover from a known
baseline. Prefer the surgical endpoints when you know what
specifically is stale; reach for this only when you don't.

### "The cache is eating all our RAM"

Per-tenant entries are capped — default 10,000 per tenant. Tune via:

```
RUBIX_CACHE_PER_TENANT_MAX_ENTRIES=2000
```

Lower numbers = less RAM, lower hit rate. The cap is enforced by
moka's weight-based eviction (one cache per tenant, each sized to
the cap), so a noisy tenant cannot evict its neighbours.

If individual entries are too large (large JSON responses), tuning
the entry cap helps proportionally — entries are stored as
`Arc<Vec<u8>>` of the serialised JSON.

## What this cache does (post-v3 scope)

As of v3 the cache covers every integration point the proposal pinned:

- **Extension kind dispatcher** (v0; Builtin + Process flavours).
- **SDUI** (`POST /ui/resolve`, `GET /ui/table`, `POST /ui/action`) via
  `starter-sdui-routes` — v3. See "v3 — SDUI integration" below.
- **Core HTTP routes** via `CacheLayer::tower()` — v3. See "v3 — tower
  layer" below.
- **SWR / `empty_ttl`** — v1. See "SWR explained".
- **Time-windowed / bucket-level caching** via the `time_series:`
  sidecar block, backed by [`starter-windowed`](../../../crates/starter-windowed/)
  + `TimescaleWindowedFetcher` / `PgWindowedFetcher` — v2.
- **Two-layer (`scope: user` + `inner_scope: tenant`) caching** via
  the canonical-units inner layer + convert-on-read outer layer — v2.
- **Automatic write-path invalidation** via the unified
  `WarehouseWriter` chokepoint — v3. Replaces the v0
  `// TODO(cache-invalidation):` markers entirely. See "v3 —
  WarehouseWriter chokepoint".
- **Multi-node fan-out** via `EventBusInvalidator` — v3. See
  "v3 — multi-node deployment".
- **Valkey backend** behind the `valkey` feature — v3.
- **Cold-start warming** via `RUBIX_CACHE_WARM_ON_BOOT=<N>` — v3.
- **Dimension-scoped tags** via opt-in `BucketTagSpec.dimensions` —
  v3.

For history on what landed when, see
[cache-v0-progress.md](../sessions/cache-v0-progress.md) and
[cache-v1-v2-v3-progress.md](../sessions/cache-v1-v2-v3-progress.md).
The authoritative design is [the proposal](../proposal/fe-cache-opt-in.md).

## Adding a new cached kind

1. Create `<extension-id>.kinds/<full-reverse-dns-contribute-id>.cache.yaml`.
   Filename **must** match the dispatcher's contribute_id; the bare
   trailing segment will not match and is caught by the orphan check
   at boot.
2. Body:

   ```yaml
   cache:
     ttl: 60s            # required; <number><s|m|h>
     scope: user         # optional; user (default tenant), or global
     invalidate_on:
       tables:
         - some_warehouse_table
   ```

3. Restart rubix-agent; check the boot log for `opt-in cache:
   registered kind specs` (info) — your spec should be in the
   count. Watch for `cache sidecar failed to load` (warn) — that's
   a parse error; the message includes the 1-indexed line number
   (e.g. `yaml: line 3: unknown cache field: "  swr: 30s"`), so
   fix the named line.
4. After traffic flows, `GET /api/v1/admin/cache/specs` will show
   the spec with counters.

The `ttl` ceiling bounds staleness: if no write hits one of the
declared tables, the entry refreshes when its TTL expires. Pick a
TTL no larger than the worst-case staleness your callers can
tolerate; pick a TTL no smaller than the loader cost / refresh
amortisation.

### Authoring a windowed sidecar (v2 `time_series:`)

Use the `time_series:` block for queries shaped like "give me the
last N of X" where `to=now` slides. The block tells the cache to
snap `from` and `to` to bucket boundaries before keying, to
decompose the window into per-bucket sub-fetches that hit the
cache independently, and to apply two TTLs (the open tail bucket
gets `tail_ttl`; closed historical buckets get `body_ttl`).

```yaml
cache:
  scope: user                       # rendered output depends on user units/locale
  inner_scope: tenant               # raw query is tenant-shared (see below)
  ttl: 30s                          # outer (rendered) TTL
  stale_while_revalidate: 30s
  time_series:
    time_param: to                  # which input param carries "now"
    range_param: from               # which input param carries the window start
    bucket: 1h                      # closed buckets are effectively immutable
    tail_ttl: 30s                   # the open bucket containing `now`
    body_ttl: 24h                   # closed buckets
    align_to: utc                   # only `utc` is supported in v2
  invalidate_on:
    tables: [com_nubeio_rubixos__histories]
    buckets:
      table: com_nubeio_rubixos__histories
      granularity: 1h               # must match `time_series.bucket`
```

A 7d query then decomposes into 7×24 hourly buckets + 1 tail.
A follow-up 90d query for the same params reuses those 7×24
buckets and fetches only the missing 83 days. An ingest at time
`t` invalidates exactly `bucket:com_nubeio_rubixos__histories:<floor(t, 1h)>`.

### Authoring two-layer caching (v2 `inner_scope:`)

When the SQL plan is identical across users in the tenant but the
rendered output depends on per-user units / locale (the
canonical-storage + convert-on-read pattern):

```yaml
cache:
  scope: user                  # outer: rendered output keyed per-user
  inner_scope: tenant          # inner: canonical-units query keyed per-tenant
  ttl: 30s
  invalidate_on:
    tables: [com_nubeio_rubixos__histories]
```

The cache wrapper looks the tenant-scope (canonical-units) entry
up first, runs the existing `starter-i18n` /
`starter-spi::preferences` conversion against the caller's prefs
on inner miss, then stores in the user-scope cache. One DB hit
serves the whole tenant; per-user rendering pays once per user per
TTL. For a tenant with 50 concurrent users on the same dashboard
this is the single largest cost win available — see proposal
§Layer 6c.

### Authoring dimension-scoped tags (v3)

When bucket-level invalidation over-invalidates (e.g. one meter's
ingest drops every other meter's cached entries on the same hour),
extend the bucket subscription with `dimensions:`:

```yaml
cache:
  invalidate_on:
    buckets:
      table: com_nubeio_rubixos__histories
      granularity: 1h
      dimensions: [meter_id]
```

The `WarehouseWriter` chokepoint then emits
`table:com_nubeio_rubixos__histories:meter_id=42` in addition to
the per-row bucket tag. Specs subscribe by listing the
dimensional tag literally in `invalidate_on.tables`. Use sparingly
— dimensional cardinality is unbounded by definition.

## SWR explained (v1)

`stale_while_revalidate` (SWR) is the **opt-in** mechanism that lets
a cached entry keep answering while a follow-up refresh runs. It is
a sidecar key:

```yaml
cache:
  ttl: 60s
  stale_while_revalidate: 30s   # serve cached values from age 30s..60s
  max_stale: 120s               # ...and up to 120s past TTL (default 2*ttl)
```

Reading the timeline for a `ttl: 60s` + `stale_while_revalidate: 30s`
spec:

| Entry age          | What happens                                          |
| ------------------ | ----------------------------------------------------- |
| `0s..30s`          | Fresh hit. Loader is not called.                      |
| `30s..(60s+120s)`  | **Stale hit.** Cached value returned. Entry is marked for refresh. |
| `(60s+120s)..`     | Hard miss. Loader runs synchronously.                 |

The first call inside the SWR window serves stale and **marks** the
entry. The **next** call for the same key sees the marker and drives
the refresh — it pays the loader cost so subsequent callers see a
fresh entry. This is the v1 "caller-driven" refresh model; v3 will
upgrade it to true background-spawned single-flight once the
`WarehouseWriter` chokepoint ships (the two share the same `'static`
refresher abstraction).

Operator-facing implications:

- A spec with `stale_while_revalidate: 30s` is **observably as fresh
  as** a spec with `ttl: 30s`, but **two-thirds less loader pressure**
  — the loader runs at the refresh cadence, not the hit cadence.
- `max_stale` is the **safety net**, not the target. If callers
  observe answers older than `ttl + max_stale`, the loader has been
  failing — check error logs from the underlying handler.
- An invalidation (`POST /admin/cache/invalidate` or a write-path
  fire) drops the entry regardless of SWR window; the next read is
  a hard miss.

Empty results are caught by `empty_ttl` and **do not** participate in
SWR/`max_stale`: an empty entry expires hard at `empty_ttl` (default
`5s`, clamped to `min(empty_ttl, ttl)`). This is intentional —
"the warehouse came back empty" is a transient answer that should
not linger.

## How to declare a writing handler (v1)

The kind dispatcher fires `invalidate_tags(["table:T", ...])` after
**every successful write call** through a registered handler.

Wire it at host boot:

```rust
use starter_ext_server::rest::cache::{
    DispatcherCache, HandlerCatalog, HandlerCatalogBuilder, HandlerMeta,
};

let mut h = HandlerCatalogBuilder::new();
h.register(ext.clone(), "list_things", HandlerMeta::read_only())?;
h.register(
    ext.clone(),
    "create_thing",
    HandlerMeta::writing(["things", "thing_audits"]),
)?;
let dispatcher_cache = DispatcherCache::new(layer, kind_registry)
    .with_handlers(h.build());
```

Rules the registration API enforces:

- A writing handler **must** name at least one table in
  `affects_tables`. Registering a writer without tables is a hard
  error (`HandlerRegistrationError::WritingHandlerMissingTables`) —
  silent best-effort wiring is the #1 way caches rot in production.
- A read-only handler (`HandlerMeta::read_only()`) does no
  invalidation. Use this for `list`/`get`/`describe` handlers.
- A writing handler whose `affects_tables` is missed at registration
  time will silently fail to invalidate. Pin the catalog with a unit
  test the way you would pin a sidecar's contents.

Handlers that route writes through the **warehouse** still rely on
the (best-effort) `// TODO(cache-invalidation):` markers until the
v3 `WarehouseWriter` chokepoint lands. The v1 dispatcher-fired path
covers handlers the dispatcher owns directly — that is a real
guarantee, not best-effort.

## v3 — multi-node deployment (event-bus invalidator)

Pick the invalidator at boot via `RUBIX_CACHE_INVALIDATOR`:

| Value | Meaning |
|---|---|
| `local` (default) | Single-process tag tokens. Use for one-replica deployments. |
| `event-bus` | Local tokens + fan-out via `RubixEventBus` on the `__cache.invalidate` topic. Every replica subscribes on boot and applies remote tag fires through `EventBusInvalidator::apply_remote` — no double-publish loops. |

A successful tag fire on replica A publishes a JSON `[tag, …]` array
on `__cache.invalidate`. Every other replica reads it and bumps its
**local** token store via `apply_remote` (which does **not**
re-publish). Cache entries stored under those tags become misses on
the next read.

Operational checks:

- `GET /api/v1/admin/cache/specs` returns
  `specs[*].config.invalidator_kind` — `"local"` or `"event-bus"`.
  Use to confirm the env var is honoured at boot.
- Replica B's hit rate dropping at the moment replica A is observed
  publishing on `__cache.invalidate` is the wire-level signal the
  fan-out is alive.

## v3 — Valkey backend

Compile the agent with the `valkey` feature on `starter-cache` to
enable `crates/starter-cache/src/backends/valkey.rs` — a second
`Cache` impl behind the same trait. The author-facing `CacheSpec`
does **not** change; the backend is picked at the host's wiring
site.

The v3 cut ships a shape-correct in-memory shared-handle model
(handles cloned from one parent share the underlying store — the
network-shared shape a real Valkey client needs). The protocol-level
swap to the `redis` crate is a one-file change.

## v3 — Cold-start warming

After a deploy every cache key is cold. SWR doesn't help. Set
`RUBIX_CACHE_WARM_ON_BOOT=<N>` to replay the top-N spec ids (by
prior `hits + misses`) at boot. The result is surfaced as:

```
GET /api/v1/admin/cache/specs
{
  "specs": [...],
  "warmer": {
    "last_run_at": 1717072800,
    "entries_warmed": 50,
    "last_duration_ms": 1827
  }
}
```

`warmer` is `null` when the env var is unset.

## v3 — Dimension-scoped tags

`BucketTagSpec` grows an optional `dimensions: [<column>, …]` list.
When set, the `WarehouseWriter` chokepoint emits
`table:<name>:<dim>=<value>` in addition to the per-row bucket tag.
Specs subscribe by listing the dimensional tag literally in
`invalidate_on.tables`. Use this when bucket-level invalidation
over-invalidates — e.g. when a meter-43 write should not touch
cached meter-42 entries on the same hour.

## v3 — WarehouseWriter chokepoint

The five scattered write sites have **all** been routed through one
chokepoint, [`starter_cache::DefaultWarehouseWriter`]:

| Site | Status |
|---|---|
| `crates/starter-store-warehouse/src/tsdb/store/samples.rs` | callers wrap via `WarehouseWriter::enqueue` + `commit` |
| `crates/starter-store-warehouse/src/tsdb/store/raw_events.rs` | same |
| `crates/starter-store-warehouse/src/tsdb/store/events.rs` | same |
| `crates/starter-store-warehouse/src/tsdb/store/documents.rs` | same |
| `rubix/crates/rubix-agent/src/extensions/warehouse_write.rs` | wired directly — `RubixWarehouseWriteBackend::with_writer(registry, invalidator)` enqueues per row and commits once per `insert` |

Batched dedup: a 500-row batch spanning 12 buckets fires *one*
`invalidate_tags` call carrying ≤13 tags (1 table + ≤12 buckets), not
500. The dedup is at the chokepoint, not the invalidator; write
paths with no cache wired pay nothing.

Rollback semantics: `discard()` drops the batch without firing.
Wire it on transaction rollback paths.

## v3 — SDUI integration

`crates/starter-sdui-routes/src/cache_integration.rs` ships the
helpers `/ui/resolve`, `/ui/table`, and `/ui/action` consume:

- `wrap_resolve(layer, spec_id, tree, caller, base_key, load)` —
  no-op when the tree has no `cache:` block; otherwise wraps the
  resolver in `layer.get_or_load_labelled` with the spec derived
  from the IR block. The resolve cache key mixes
  `(tenant, user, page_id, target_ref, stack_hash, page_state_hash,
  units_hash, ir_version, page_content_hash)` per `resolve_base_key`.
- `table_base_key(source_id, page, sort, filter, scope_vars)` — the
  `/ui/table` key shape; cached independently of the resolve cache.
- `SduiActionMeta { read_only, affects_tables }` + `fire_action_invalidation`
  — `/ui/action` is never cached; writing handlers fire
  `invalidate_tags` after success, read-only handlers are a no-op.

The IR `cache:` block on `ComponentTree` is additive — no IR version
bump (§"What changed in this revision").
