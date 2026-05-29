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
        "invalidate_on_tables": ["com_nubeio_rubixos__histories"]
      },
      "hits": 124,
      "misses": 17,
      "hit_ratio": 0.879,
      "load_latency": {
        "le_10ms": 0, "le_100ms": 4, "le_1s": 12, "le_10s": 1, "gt_10s": 0,
        "count": 17, "sum_nanos": 8_421_330_000, "mean_ms": 495.4
      }
    }
  ]
}
```

Each row reflects one sidecar. Key fields:

- `config` — verbatim shape of the `<id>.cache.yaml` sidecar. If this
  looks wrong, the sidecar shipped wrong; don't go hunting in code.
- `hits` / `misses` / `hit_ratio` — counters since process start.
- `load_latency` — **miss-path only**. Hits don't pollute the
  histogram, so this measures what the cache is shielding callers
  from. `mean_ms` is the derived mean; the bucket counts let you
  spot bimodal distributions (fast and slow loaders in the same
  spec).
- `config: null` — appears when a counter row has no matching
  registered spec. Shouldn't happen in practice; treat as drift,
  worth filing.

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

This is the same mechanism the write-path *would* use if the
unified `WarehouseWriter` chokepoint existed (see the
`// TODO(cache-invalidation):` markers in the warehouse store
crates). Until then, this endpoint is the manual escape hatch for
"the data changed but the cache doesn't know".

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

## What this cache does NOT do (v0 scope)

To save you reading the proposal:

- **No SDUI integration.** Only the extension kind dispatcher is
  wrapped today (Builtin + Process flavours).
- **No SWR / stale-while-revalidate.** A miss waits for the loader;
  there is no "serve stale + refresh in background" path.
- **No time-windowed / bucket-level caching.** A sliding `now`
  produces a new cache key every wall-clock tick. See the deferred
  `starter-windowed` companion-crate proposal for the long-term fix.
- **No automatic write-path invalidation.** The unified
  `WarehouseWriter` chokepoint that would make this safe is a
  separate project (one of the proposal's un-defer conditions).
  Today it is best-effort via the manual `/invalidate` endpoint
  above.
- **No two-layer (inner-tenant + outer-user) caching.** Worth doing
  when a tenant runs ≥10 concurrent users on the same dashboard,
  per the proposal — not yet justified by traffic.

If you find yourself wanting any of the above, the right move is to
re-read [the v0 proposal](../proposal/fe-cache-opt-in.md) and the
[progress doc](../sessions/cache-v0-progress.md) — the conditions
that would un-defer the deferred features are spelled out there.

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
