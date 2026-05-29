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
