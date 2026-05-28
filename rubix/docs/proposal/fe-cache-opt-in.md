# Proposal: Opt-In Caching for SDUI Pages, Extension Kinds, and Core Routes

**Status:** Proposed
**Date:** 2026-05-29
**Author:** NubeDev

## Summary

Promote [crates/starter-cache](../../../crates/starter-cache/) from a hand-rolled primitive into a **declarative caching layer at every read boundary the platform owns** — SDUI resolve / table / action endpoints, extension kind dispatcher, and arbitrary core HTTP routes. One config language, one invalidation model, three integration points.

A page author opts into caching by adding a `cache:` block to a SDUI page, a kind sidecar, or a route attribute. They do not write cache code, they do not derive keys, they do not wire invalidation. The platform does all three.

The driving examples are:

- The rubixos **usage** page (`/extensions/com.nubeio.rubixos/usage`) — an extension kind serving heavy aggregated warehouse reads.
- Any **SDUI main-dashboard page** authored via the [SDUI substrate](../../../DOCS/frontend/sdui/SCOPE.md) — e.g. a building-overview page whose `kpi` and `table` bindings hit the warehouse every resolve.
- Any **core HTTP route** that reads from the DB and is not SDUI- or extension-bound.

## Motivation

Most slow pages in this system are **read-mostly aggregations over warehouse data**: usage by meter, summaries, dashboards, alarm rollups. Three things are true of them:

1. They are expensive to compute (large scans, joins, aggregations).
2. They are queried far more often than the underlying data changes (a dashboard refreshes every 30s; readings batch every minute or longer).
3. The same answer is valid for many users in the same tenant — or even globally — for tens of seconds at a time.

That is the textbook caching case. Today, getting it requires:

- Holding a `Cache` handle from somewhere
- Picking a key shape (and remembering to mix in user / tenant scope)
- Wrapping the load site with `get_or_insert_with`
- Wiring invalidation by hand to every write path that could change the answer
- Doing the same five decisions correctly in every page that wants it

Every adopter gets at least one of those wrong. The platform should make caching a declarative property of the page — visible in source, reviewable in PRs, enforced by the runtime — not a hidden behaviour buried in a handler.

A second motivation is **uniformity across authoring modes**. The [SDUI scope doc](../../../DOCS/frontend/sdui/SCOPE.md) is explicit that `ComponentTree`s can be authored four ways (Rust DSL, JSON / YAML, AI, drag-drop) and the renderer does not care which. Caching has to be the same way: it must work whether the page came from a Rust `building_overview()` builder, a hand-written YAML file, an AI-emitted tree, or an extension kind. The cache layer cannot be coupled to extensions specifically — it sits at the resolve / dispatch boundary, where all four authoring modes converge.

## Goals

- A page author adds caching to a SDUI page, an extension kind, or a core route by editing one block of declarative config — no Rust, no key derivation, no invalidation plumbing.
- The same machinery covers **SDUI main dashboards**, **extension kinds**, and **arbitrary core HTTP routes** — one cache layer, three call sites.
- Invalidation is **event-driven first, TTL second** — a new ingest batch invalidates derived reads, it does not wait for them to expire.
- Per-user / per-tenant / global scope is a one-word choice, defaulting to the safe answer.
- One `Cache` trait, multiple backends (in-process moka today; Valkey or `foyer` later) — backend is config, not code.
- Stampede-safe by default (we already have `get_or_insert_with`).
- Caching never changes a page's correctness — only its freshness window. A bug in the cache config means stale data, never wrong data.
- **First-class support for time-windowed reads** (1d / 7d / 30d / 90d / 12-month). The dashboard workload is overwhelmingly "give me the last N of X" with a sliding `now`. The cache layer must handle that shape natively or it will look like a cache and behave like a thin TTL on a query that misses every time the wall clock ticks.

## Non-goals

- Caching writes, or write-through semantics. This is a read cache.
- Distributed coherence beyond "invalidate the same tag everywhere". No cross-node read-your-write guarantees in v1.
- Replacing the database. This is not a query store; it is a memoisation layer in front of one.
- Offline-first SDUI. [SDUI R-section "What does NOT land"](../../../DOCS/frontend/sdui/SCOPE.md) lists this as out of scope; the cache layer is the substrate that *could* support it later, but offline is not the v1 goal.

## Design

The cache layer is **one crate** (`starter-cache`, already exists) integrated at **three call sites**. Each call site reads the same declarative config language and feeds it to the same runtime.

```
                ┌────────────────────────────────┐
                │   starter-cache                │
                │   Cache trait + moka backend   │
                │   tag-based invalidator        │
                │   stats + observability        │
                └──────────┬─────────────────────┘
                           │
        ┌──────────────────┼─────────────────────┐
        │                  │                     │
 ┌──────▼──────┐    ┌──────▼──────┐       ┌──────▼──────┐
 │ SDUI        │    │ Extension   │       │ Core HTTP   │
 │ resolve/    │    │ kind        │       │ route       │
 │ table/      │    │ dispatcher  │       │ tower layer │
 │ action      │    │             │       │             │
 └─────────────┘    └─────────────┘       └─────────────┘
        ▲                  ▲                     ▲
        │                  │                     │
   page.cache:        kind.cache.yaml      route attribute /
   block in           sidecar              tower layer config
   ComponentTree
```

### Layer 1 — declarative config (the author surface)

The same shape, in three places.

**A — SDUI page (`page.cache` block on the ComponentTree root):**

```rust
// starter-ui-builder DSL
page("building-overview", "{{$target.name}} Overview", [...])
    .cache(PageCache::builder()
        .ttl(Duration::from_secs(60))
        .scope(CacheScope::User)
        .stale_while_revalidate(Duration::from_secs(30))
        .invalidate_on_tables(["com_nubeio_rubixos__readings"])
        .build())
```

Or, equivalently, in a hand-authored / AI-emitted JSON tree:

```json
{
  "ir_version": 1,
  "page_id": "building-overview",
  "cache": {
    "ttl_seconds": 60,
    "scope": "user",
    "stale_while_revalidate_seconds": 30,
    "invalidate_on": {
      "tables": ["com_nubeio_rubixos__readings"],
      "events": ["ingest.batch.committed"]
    },
    "tags": ["dashboard", "rubixos"]
  },
  "root": { ... }
}
```

The `cache:` block lives on the IR root, beside `ir_version` and `page_id`. It is **optional** — omit it and the page is uncached, same as today. It is **part of the IR**, so the SDUI resolver reads it for free, AI-emitted trees can include it, and the JSON Schema validates it. (This is a minor IR bump per [SDUI R2](../../../DOCS/frontend/sdui/SCOPE.md).)

**B — Extension kind (sidecar file):**

```yaml
# rubix/extensions/com.nubeio.rubixos/kinds/usage_bucketed.cache.yaml
cache:
  ttl: 60s
  stale_while_revalidate: 30s
  scope: user
  invalidate_on:
    tables:
      - com_nubeio_rubixos__readings
      - com_nubeio_rubixos__meters
    events:
      - ingest.batch.committed
  tags: [usage, rubixos]
```

The dispatcher reads this when the kind is registered.

**C — Core HTTP route (axum attribute or tower layer):**

```rust
Router::new()
    .route("/api/v1/dashboards/home", get(home_handler))
    .layer(CacheLayer::builder()
        .ttl(Duration::from_secs(60))
        .scope(CacheScope::User)
        .invalidate_on_tables(["dashboards", "user_prefs"])
        .build());
```

For ad-hoc programmatic use, the existing `get_or_insert_with` primitive on the `Cache` trait stays as-is — call it directly.

All three forms compile down to the same `CacheSpec` struct in `starter-cache`. The three call sites differ only in where the `CacheSpec` comes from.

### Layer 2 — the three call sites

**SDUI integration (the new piece):**

`starter-sdui-routes` (per [SDUI surface](../../../DOCS/frontend/sdui/SCOPE.md)) owns `POST /ui/resolve`, `POST /ui/action`, `GET /ui/table`. The resolver wraps its core work in the cache layer when the resolved `ComponentTree` carries a `cache:` block:

```rust
// pseudo-code, in starter-sdui-routes
let key = CacheKey::derive_sdui(page_ref, target_ref, &stack, &page_state, &auth, spec.scope);

cache_layer
    .get_or_insert_with(key, spec, || async {
        resolve_page(page_ref, target_ref, stack, page_state, ctx).await
    })
    .await
```

Cached objects are `ResolveResponse` values — the rendered tree plus the subscription plan. The subscription plan is keyed per-target by the binding grammar (`{{$target/temp.value}}` produces a target-scoped subject), so caching a resolved tree never crosses target boundaries the way a naive cache would. The `scope: user` default further isolates per-user `EvalContext` (user prefs, AuthZ) into the key.

The `/ui/table` endpoint caches independently — its key is `(source_id, page, sort, filter, scope-vars)`. Table pages are an obvious cache target and are not bundled into the resolve cache.

The `/ui/action` endpoint is **never cached** — actions are writes by definition. Action handlers instead call `invalidate_tags` on the way out (see Layer 3).

**Extension kind integration:**

```rust
// pseudo-code, in starter-extensions/crates/starter-ext-server/src/rest/dispatcher.rs
let key = CacheKey::derive_kind(ext_id, kind, &params, &auth, spec.scope);

cache_layer
    .get_or_insert_with(key, spec, || async {
        execute_kind(ext_id, kind, params, ctx).await
    })
    .await
```

The wrapper is a no-op when the kind has no `cache:` sidecar. Existing call paths unchanged.

**Core HTTP route integration:**

A small `tower::Layer` (`CacheLayer`) that derives a key from `(route_path, query_string, scope-vars)` and wraps the inner service. For routes whose response is a pure function of `(URL, auth scope)`, this is the lowest-effort opt-in.

### Layer 3 — invalidation (shared across all three call sites)

This is the part most cache layers get wrong. Two complementary mechanisms.

**(a) Tag-based, write-path hooks (v1)**

`starter-cache` exposes:

```rust
cache_layer.invalidate_tags(&["table:com_nubeio_rubixos__readings"]).await;
cache_layer.invalidate_tags(&["event:ingest.batch.committed"]).await;
cache_layer.invalidate_tags(&["user:42"]).await;  // for action handlers that touch only one user
```

Write paths call `invalidate_tags` once per commit:

- The warehouse ingest path hooks `table:<name>` and — for time-series tables — also `bucket:<table>:<floor(t, bucket)>` for each row's timestamp. Bucket tags are emitted at the same bucket granularities any cache spec on that table declares (the registry knows which granularities exist).
- SDUI action handlers (`/ui/action`) hook the tags they declare in their handler registration metadata. A handler with `affects_tables = ["dashboards"]` calls `invalidate_tags(&["table:dashboards"])` after success.
- The mart writers hook the L3 table tags they emit into.
- Retention sweepers hook the table tags they trim.

**Bucket-level invalidation is mandatory for any windowed time-series spec.** Without it, a single reading invalidates every cached `usage_bucketed` entry for every user in the tenant — the cache will not pay for itself on the dashboard workload. With it, a reading written for meter 42 at `t` invalidates exactly `bucket:readings:<floor(t, 1h)>`; closed historical buckets and unrelated meters' partial buckets are untouched. Combined with [Layer 4b](#layer-4b--time-windowed-reads-the-dashboard-workload), a 90d query's body is essentially permanent in cache.

Dimension-scoped tags (`table:readings:meter=42`) are *not* in v1. They are the next granularity if bucket-level invalidation still over-invalidates in practice — but bucket-level alone should cover the dashboard workload.

A small registry maps `table:X` / `bucket:X:Y` / `event:Y` / `user:Z` strings → the set of cache keys that subscribed via their `invalidate_on:` block. Built at page / kind registration time; lookup is O(1). The registry supports prefix matches on bucket tags so a tenant-wide rebuild can invalidate `bucket:readings:*` in one call.

**Invalidation-race safety.** Naive `get_or_insert_with` has a race: reader misses → starts load → writer commits and invalidates → reader stores the stale value, which now lives until TTL. The fix is a per-tag invalidation token (monotonic u64), bumped on every `invalidate_tags` call. Readers snapshot the token set their key depends on at load-start, and the store is dropped at load-end if any token moved. Cheap, correct, no locking — and necessary, not optional.

This works the same across all three integration points — the SDUI resolver, the extension dispatcher, and the tower layer all share one cache layer and one tag registry.

**(b) Event bus (v2, when we go multi-node)**

When we run more than one server replica, write-path hooks need to fan out. `invalidate_tags` then publishes to an event bus; each replica subscribes and invalidates its local cache. The author-facing config does not change — only the `Invalidator` impl does.

We pick (a) now, swap later. The `Invalidator` trait is the seam.

### Layer 4 — scope and key derivation

Three scopes, picked in one word, applied identically at every call site:

| Scope | Key includes | Use when |
|---|---|---|
| `user` | tenant + user + locale + unit-prefs hash + page/kind id + params hash | Result depends on user identity, AuthZ, units, language |
| `tenant` | tenant + page/kind id + params hash | Result is the same for everyone in a tenant |
| `global` | page/kind id + params hash | Result is the same for the whole world (rare; reference data) |

The author picks the scope; the layer derives the key. Mixing user / tenant / locale / unit-prefs in is automatic — and important. The platform already tracks [per-user unit preferences and i18n](../../../) (the warehouse stores canonical values and converts on read), which means **the same kind query produces different rendered output for two users in the same tenant**. `scope: user` is the safe default whenever the result passes through a unit or locale conversion.

For SDUI specifically, the per-target dimension of the binding grammar (`{{$target/...}}`) goes into the key automatically — `target_ref` is part of the resolve request and falls out of `CacheKey::derive_sdui` for free.

**The default scope is `tenant`, not `global`.** A page author has to actively pick `global` to share results across tenants. The registration-time validator warns when a page with `scope: global` declares `invalidate_on.tables` containing a tenant-scoped table (which is almost always wrong).

### Layer 4b — time-windowed reads (the dashboard workload)

Treating every query as an opaque blob keyed by params hash is wrong for the workload this proposal exists to serve. A query like `usage_bucketed(meter=X, from=now-7d, to=now)` has:

- **Infinite key cardinality** if `now` is mixed in raw — every wall-clock tick produces a new key. A 30-second dashboard refresh effectively never hits the cache; it just hits the TTL.
- **Massive overlap with sibling queries** — `now-1d` is a strict suffix of `now-7d` is a strict suffix of `now-90d`. With opaque keys, the 1d, 7d, 90d queries share nothing in cache. Each cold view pays full miss cost.
- **Mixed staleness requirements** — the partial bucket containing `now` is volatile; closed historical buckets (yesterday, last week, March 2026) are immutable. One TTL cannot be right for both.

The cache layer addresses this with a **first-class time-series block** that authors opt into when their query is windowed:

```yaml
cache:
  scope: tenant                # the heavy query is usually tenant-shared
  time_series:
    time_param: to             # which param carries "now"
    range_param: from          # which param carries the window start
    bucket: 1h                 # snap boundary; closed buckets are immutable
    tail_ttl: 30s              # current partial bucket (the one containing `to`)
    body_ttl: 24h              # closed buckets (effectively-infinite for our purposes)
    align_to: utc              # bucket boundaries align to UTC, not request time
  invalidate_on:
    tables: [com_nubeio_rubixos__readings]
```

The runtime does three things behind that block:

1. **Snap `from` and `to` to bucket boundaries before they enter the key.** `to = now` becomes `to = floor(now, bucket)`. Infinite cardinality collapses to one entry per bucket.
2. **Decompose the window into per-bucket sub-queries.** A 7d query becomes `union(bucket[-7d], ..., bucket[-1d], partial(today))`. Each bucket is cached independently. The 1d, 7d, 90d queries reuse the same daily buckets.
3. **Apply two TTLs.** Closed buckets get `body_ttl`. The tail (the open bucket containing `now`) gets `tail_ttl`. Closed-bucket entries are almost never re-fetched in steady state.

Bucket decomposition is also what makes invalidation tractable (see [Layer 3 — bucket-level tags](#layer-3--invalidation-shared-across-all-three-call-sites)): an ingest at time `t` invalidates `bucket:<table>:<floor(t, bucket)>`. Closed historical buckets are never invalidated by new writes, only by retention sweeps.

**Where the windowed-fetch logic lives.** Not in `starter-cache`. The cache layer should not know about time, ranges, or SQL — it knows about keys, TTLs, and tags. The bucket decomposition + stitching logic lives in a new **`starter-windowed` crate** (see [Companion crate](#companion-crate--starter-windowed) below), which any windowed-read author can use independently of caching. The cache integration is a thin adapter: the SDUI / dispatcher / tower layer sees `time_series:` in the config, hands the windowed-fetch crate the spec, and the windowed-fetch crate's `fetch_range(from, to)` is what the cache wraps.

This separation matters: a flow node or agent step that fetches a windowed range without rendering a page should also benefit from delta fetch (Gap 8 below), and it should not have to depend on the cache to do it.

### Layer 5 — interaction with the existing AuthZ surface

SDUI pages can be user-defined and gated per-user (per the platform's [AuthZ scope](../../../) — dynamic resources, not static routes). The cache layer respects this in two ways:

1. **The AuthZ check runs before the cache lookup, not after.** A user without permission for a page must never get a cached body served — even a stale one. The cache wrapper sits *inside* the authorised handler, not in front of it.
2. **The user's effective permission set goes into the key when `scope: user`** — so a user whose role changes does not see the old role's cached view. Permission-set hash is part of the user-scope key derivation.

### Layer 6 — programmatic API (unchanged)

`starter-cache` already exposes `Cache::get_or_insert_with`. For code that does not sit at one of the three integration points (background jobs, agent steps, etc.), call it directly. This proposal does not change the trait surface.

### Layer 6b — stale-while-revalidate semantics (spelled out)

`stale_while_revalidate: D` means: when a cache hit is within `D` of expiry **or** has expired but is still within the SWR window:

1. **Serve the stale value to this caller immediately.**
2. **Kick off exactly one background refresh** for that key (single-flight, same primitive as `get_or_insert_with`).
3. **All concurrent callers receive stale** until the refresh completes; then subsequent callers see the new value.
4. **If the refresh errors**, keep serving the stale value up to `max_stale` (defaults to `2 × ttl`). After that, callers receive the error — the cache does not silently serve unbounded-age data.
5. **An invalidation tag firing while SWR is in flight** is honoured: the in-flight load's store is dropped (the invalidation-token check from Layer 3), and the next caller pays a fresh miss.

SWR is what makes dashboards feel instant. Without (1) and (2) being explicit, an implementer could reasonably read SWR as "extend TTL by `D` seconds" — which serves correctness-equivalent results but loses the latency property that justifies the feature.

### Layer 6c — tenant-shared coalescing (the cost lever)

`scope: user` is the safe default whenever rendered output depends on user identity, units, or locale. But for warehouse-backed queries, the **SQL plan** is usually identical across users in the tenant — only the unit / locale conversion at render differs. Caching at `scope: user` pays the DB cost N times for what is one query.

The recommended pattern is a **two-layer cache** for these queries:

- **Inner layer** — `scope: tenant`, caches the canonical-units query result (the heavy DB cost).
- **Outer layer** — `scope: user`, caches the rendered output (cheap convert-on-read + locale).

The runtime supports this directly: a cache spec may set `inner_scope: tenant` alongside `scope: user`. The wrapper does a tenant-scope lookup first, runs convert-on-read against the user's prefs, then stores in the user-scope cache. One DB hit serves the whole tenant; per-user rendering is paid once per user per TTL.

This composes naturally with the warehouse's canonical-storage + convert-on-read model. Worked example D below uses this pattern.

For a tenant with 50 concurrent users on the same dashboard, this is the difference between 50× warehouse load and 1× warehouse load. It's the single largest cost win in the design.

### Layer 7 — backends

`Cache` is already a trait. The default impl is moka (in-process, TinyLFU, async). When we need cross-node sharing we add a Valkey backend behind the same trait. When we need larger-than-RAM, `foyer`. The author-facing config does not change; the backend is picked by server config.

## Worked example A — rubixos `usage` extension kind

1. Author adds `rubix/extensions/com.nubeio.rubixos/kinds/usage_bucketed.cache.yaml` with `ttl: 60s`, `scope: user`, `invalidate_on.tables: [com_nubeio_rubixos__readings]`.
2. Dispatcher, on next kind reload, registers `usage_bucketed` as cacheable and subscribes its tag to the readings table.
3. First request runs the query, stores the result keyed by `(tenant, user, ext, kind, params hash)`.
4. Subsequent requests from the same user with the same params return in microseconds.
5. Next ingest batch commits → `invalidate_tags(&["table:com_nubeio_rubixos__readings"])` → next request runs fresh.
6. 60s TTL bounds staleness if no writes arrive.

## Worked example B — SDUI building-overview dashboard

A core dashboard authored as a SDUI page via [the builder DSL](../../../DOCS/frontend/sdui/SCOPE.md):

```rust
pub fn building_overview() -> ComponentTree {
    page("building-overview", "{{$target.name}} Overview", [
        kpi_grid([ /* KPIs bound to warehouse slots */ ]),
        table("alarms", rsql().parent_path_prefix("{{$target.path}}/alarms").kind("alarm.active"))
            .live()
            .build(),
    ])
    .cache(PageCache::builder()
        .ttl(Duration::from_secs(30))
        .scope(CacheScope::User)
        .invalidate_on_tables(["readings", "alarms_active"])
        .build())
}
```

1. The Rust builder produces a `ComponentTree` with a `cache:` block at the root.
2. `POST /ui/resolve` for `page_ref = "building-overview"`, `target_ref = "building-7"` consults the cache: key is `(tenant, user, "building-overview", "building-7", stack_hash, page_state_hash, units_hash)`.
3. First resolve runs the full bind-and-render, stores the `ResolveResponse` (rendered tree + subscription plan).
4. Subsequent resolves for the same `(user, target)` pair return cached.
5. The `table` source for alarms is *not* part of the resolve cache — it has its own per-page cache via `/ui/table`. The resolve cache serves the empty table shell + subscription plan; the table page cache serves the rows.
6. An alarm write fires `invalidate_tags(&["table:alarms_active"])`. Both the resolve cache (which declared the tag) and the `/ui/table` cache (which subscribes by source) drop affected entries.
7. Live updates over the existing subscription plan continue to flow regardless of cache state — the cache changes *resolve* latency, not live-update latency.

The author wrote ten lines of cache config. The rest is the platform.

## Worked example C — a core HTTP route

A non-SDUI, non-extension JSON endpoint that serves a slow dashboard summary:

```rust
Router::new()
    .route("/api/v1/dashboards/home", get(home_handler))
    .layer(CacheLayer::builder()
        .ttl(Duration::from_secs(60))
        .scope(CacheScope::User)
        .invalidate_on_tables(["dashboards"])
        .build());
```

No handler change. The layer intercepts the response, caches the bytes by `(route, user, query)`, serves cached bytes on hit, invalidates on tag.

## Worked example D — `usage_bucketed` with windowed reads and tenant coalescing

The full canary. `usage_bucketed` is the kind powering `/extensions/com.nubeio.rubixos/usage`. It's queried with `from / to / meter / bucket` and supports 1d / 7d / 30d / 90d / 12-month windows. This is where the cache has to actually work.

```yaml
# usage_bucketed.cache.yaml
cache:
  scope: user                   # rendered output depends on user units/locale
  inner_scope: tenant           # raw query (canonical units) is tenant-shared
  ttl: 30s                      # outer (rendered) TTL
  stale_while_revalidate: 30s
  time_series:
    time_param: to
    range_param: from
    bucket: 1h
    tail_ttl: 30s
    body_ttl: 24h
    align_to: utc
  invalidate_on:
    tables: [com_nubeio_rubixos__readings]
  tags: [usage, rubixos]
```

What happens on a 7d query from user A in tenant T:

1. Outer (user-scope) cache check: key includes `(T, A, units_hash, locale_hash, "usage_bucketed", meter, bucket=1h, from=floor(now-7d, 1h), to=floor(now, 1h))`. Miss.
2. Inner (tenant-scope) cache check via the windowed-fetch crate (see [Companion crate](#companion-crate--starter-windowed)): the 7d window is decomposed into 7×24 hourly buckets + 1 partial tail.
3. For each closed bucket, the inner cache lookup hits (after the first user has populated them). The tail bucket hits the warehouse with a single 1h scan.
4. Stitched canonical-units result is rendered for user A's unit prefs; outer cache stores the rendered output.
5. User B in tenant T loads the same 7d view: outer miss, inner all-hit (including the tail if within `tail_ttl`), render-only cost.
6. User A switches to the 90d view: outer miss; inner reuses 7×24 of the hourly buckets they already populated, fetches the remaining 83 days from warehouse (one query, range scan).
7. Next ingest commits a row at `t`: `invalidate_tags(&["bucket:readings:<floor(t, 1h)>"])`. Exactly one bucket per affected granularity is dropped. The 90d body is otherwise untouched.

Result: the second user on the same dashboard pays render cost, not query cost. The 90d view reuses 7×24 buckets from the 7d view. The 1d view reuses 24 of those same buckets. Closed historical buckets are essentially permanent.

This is the shape the proposal exists to deliver. The author wrote one config block; the runtime did bucket decomposition, tenant coalescing, tail-vs-body TTL, single-flight refresh, and surgical invalidation.

## Companion crate — `starter-windowed`

The bucket decomposition + stitching logic does not belong in `starter-cache`. The cache layer knows about keys, TTLs, and tags — it does not know about time, ranges, or SQL. Mixing the two crates the time-series logic into the cache and makes it unreachable for callers that want delta fetch without caching (background jobs, agent steps, flow nodes, CLI tools).

We add a new crate:

```
crates/starter-windowed/         (NEW)
   ├── src/spec.rs               WindowedSpec { time_param, bucket, align_to, ... }
   ├── src/bucket.rs             snap_to_bucket(), decompose(from, to) -> Vec<BucketRange>
   ├── src/fetch.rs              WindowedFetcher<T> trait: fetch_bucket(BucketRange) -> T
   ├── src/stitch.rs             stitch(Vec<T>) -> T (T: Stitchable)
   └── src/delta.rs              extend(cached_range, requested_range) -> missing_ranges
```

Surface:

```rust
use starter_windowed::{WindowedSpec, WindowedFetcher, Bucket};

#[async_trait]
trait WindowedFetcher<T: Stitchable> {
    async fn fetch_bucket(&self, bucket: Bucket) -> Result<T, Error>;
}

// Caller side
let spec = WindowedSpec::hourly()
    .tail_ttl(Duration::from_secs(30))
    .body_ttl(Duration::from_secs(86400));

let buckets = spec.decompose(from, to);                  // Vec<Bucket>
let parts   = stream::iter(buckets)
    .map(|b| fetcher.fetch_bucket(b))
    .buffer_unordered(8)
    .try_collect::<Vec<_>>().await?;
let result  = T::stitch(parts);
```

**The delta-fetch property.** When a caller already holds a cached result for `(meter, from=A, to=B)` and asks for `(meter, from=A, to=C)` where `C > B`, the crate computes `extend(cached_range, requested_range)` and returns only the missing `(B, C]` buckets to fetch. The 12-month-extending-a-6-month case from the spec is exactly this:

```rust
let cached: CachedRange<T> = state.get(meter);            // 6 months
let want   = Range::last_n_months(12);
let missing = spec.delta(cached.range(), want);           // 6 months of new ranges
let new_parts = fetcher.fetch_many(missing).await?;
let combined  = cached.extend(new_parts);
state.put(meter, combined);
```

This works whether or not `starter-cache` is in the picture — a flow node holding state on disk, a background aggregation job, or a CLI tool can all use it.

**Why a separate crate, not warehouse or postgres:**

- It is **engine-agnostic.** The `WindowedFetcher` trait is implemented by whatever knows how to fetch one bucket — could be `starter-store-warehouse` (TimescaleDB), `starter-store-postgres` (regular tables), an in-memory mock, or a remote API. Putting the logic in either store crate forces the other to either reimplement it or take a dep on its sibling.
- It is **read-shape, not storage-shape.** The bucket/range/stitch concepts describe how a *caller* wants to read time-series data. The store crates describe how *data sits at rest*. Wrong layer to couple them.
- It is **reusable beyond caching.** Flow nodes, agent steps, exports, charts, and the cache layer all use the same delta-fetch logic. If it lives in the cache crate, none of those reach for it.
- It **composes with the cache layer**, doesn't depend on it: the cache integration is a thin adapter that registers a `Cache`-backed `WindowedFetcher` impl which (per-bucket) does `cache.get_or_insert_with(bucket_key, || real_fetcher.fetch_bucket(bucket))`.

**Where the per-engine implementations live:**

- `starter-store-warehouse` provides a `TimescaleWindowedFetcher` constructor that wraps a `WarehouseClient` + a SQL template, implementing `WindowedFetcher<RowSet>` for hypertables. Authors writing extension kinds against Timescale call this directly.
- `starter-store-postgres` provides the equivalent for regular Postgres tables (non-Timescale), for callers that have time-bucketed data outside the warehouse.

Both store crates pick up a thin dep on `starter-windowed` (for the trait + `Bucket` type); `starter-windowed` itself has no engine deps.

## Cache vs materialised view — drawing the line

For long windows (90d / 12-month aggregates), at some point a cache stops being the right tool and an L3 mart / continuous aggregate is. The cache is the right answer when:

- Hit rate is moderate-to-high but the working set fits in RAM.
- The "freshness" requirement is order-of-seconds, not order-of-milliseconds.
- The set of queries is open-ended (many shapes, many params).

The L3 mart / continuous aggregate is the right answer when:

- The same aggregation is queried at very high rate by many tenants.
- It can be incrementally maintained by the warehouse (Timescale continuous aggregates, mart rules).
- The query shape is fixed.

If an author finds themselves reaching for `body_ttl: 7d` on a 12-month aggregate, that's the signal — they have invented a worse materialised view. Promote it to an L3 mart per the [warehouse architecture](../../../). The cache layer is for the medium-rate, open-shape case; the mart is for the high-rate, fixed-shape case. `starter-windowed` is what both have in common — it's used to *read* the mart or the raw hypertable equally.

This boundary is in the rollout: when we adopt `usage_bucketed`, we measure. If the body-TTL'd 90d query dominates load, we promote the daily aggregate to a mart and the cache layer serves the now-cheap mart query instead of the now-cheap raw query. The author's `cache.yaml` does not change.

## Failure modes and how we handle them

- **Stale results after a write that did not call `invalidate_tags`.** Mitigated by the TTL ceiling. Caught by adding the hook on every writer; we lint for it at the dispatcher/handler-registration boundary.
- **Cache stampede on a hot key after invalidation.** `get_or_insert_with` is single-flight. `stale_while_revalidate` serves the old value to one cohort while a single loader refreshes.
- **Cross-tenant or cross-user leak via wrong scope.** Default is `tenant`, never `global`. Registration-time validator rejects `scope: global` on pages whose `invalidate_on.tables` include tenant-scoped tables.
- **AuthZ regression by serving cached body to a user who lost access.** The cache wrapper sits inside the authorised handler. Permission-set hash is part of the user-scope key.
- **Locale / unit preference change shows stale conversions.** Locale + units hash is part of the user-scope key. Changing the user's prefs implicitly invalidates their cached pages.
- **Memory blow-up from per-user keys on a high-cardinality kind.** moka has size-based eviction; we configure per-spec max-entries caps. Authors see this in stats.
- **Cache poisoning from a buggy loader.** Loader errors are not cached (`get_or_insert_with` returns `Err` without storing).
- **A SDUI page with `cache:` set but a `target_ref` the cache hasn't seen.** Cold miss, runs the resolver, populates — same as any cold cache.

## Observability

The cache layer emits per-spec counters at every call site:

- `cache.hits{site, id, scope}` — `site` ∈ `{sdui_resolve, sdui_table, kind, route}`, `id` is the page/kind/route id
- `cache.misses{site, id, scope}`
- `cache.invalidations{tag}`
- `cache.entries{site, id}` (gauge)
- `cache.load_seconds{site, id}` (histogram of wrapped loader latency)

These appear in the existing admin introspection console (see [admin-introspection-and-test-console.md](admin-introspection-and-test-console.md)). A page or kind author can see whether caching is paying for itself for their specific surface.

## Rollout

Sequenced so each step is independently shippable and reversible.

1. **`CacheSpec` + `Invalidator` trait + tag registry in `starter-cache`.** No behaviour change anywhere.
2. **Extension dispatcher integration + `kind.cache.yaml` parser.** First real call site. Opt in `usage_bucketed` as the canary. Measure.
3. **Warehouse write-path `invalidate_tags` hooks.** Ingest, mart writers, retention. Each writer's PR includes the tag list.
4. **SDUI IR `cache:` block + resolver / table-endpoint integration.** Minor IR bump per [SDUI R2](../../../DOCS/frontend/sdui/SCOPE.md); existing pages with no `cache:` are unaffected. Opt in `building-overview` as the canary.
5. **Core HTTP `CacheLayer` (tower).** For non-SDUI, non-extension routes.
6. **Author docs in the SDUI dev guide and the extension dev guide.**
7. **Multi-node fan-out invalidator (v2).** Swap the `Invalidator` impl when we deploy more than one replica.

## Open questions

- **Cache config on the IR root vs. per-component.** v1 is page-level only. Per-component caching (e.g. cache a `table` independently of the `kpi`s on the same page) is plausible later; defer until a real page wants it.
- **AI-emitted `cache:` blocks.** The ai-builder will be able to emit cache config the same way it emits any other IR field. Should we constrain AI-emitted TTLs or scopes? Lean: register `cache:` as a sensitive field that AI emission either omits (default) or requires explicit author sign-off on.
- **Per-spec memory budgets:** server config or page metadata? Leaning server config — operator owns RAM, not the author.
- **Invalidation granularity below the table level.** A write to one meter invalidates all `usage_bucketed` for everyone in the tenant. Fine for v1; revisit if overinvalidation shows up in stats.
- **Negative caching.** Default yes (cache "no rows" results at the same TTL); opt-out via `cache_empty: false`.

## Why not just tell authors to use moka directly

Because we tried that. The current `Cache` trait and `get_or_insert_with` are good primitives, but every adopter has to make the same five decisions (key shape, scope, TTL, invalidation, observability) and most of them get at least one wrong — and worse, they have to make those decisions *separately* for extension kinds, SDUI pages, and core routes, because the integration boundaries are different.

Moving the decisions into a single declarative config language, applied at three integration points, means:

- The author makes the decisions they care about (TTL, what invalidates) and skips the ones the platform should handle (key shape, scope mixing, stampede handling, AuthZ interaction, locale/units).
- The same `cache:` shape works whether the page came from a Rust builder, a YAML file, an AI session, or an extension kind sidecar — matching the [SDUI authoring-mode plurality](../../../DOCS/frontend/sdui/SCOPE.md).
- Caching becomes a property visible in source, reviewable in PRs — not a hidden runtime behaviour.
- One observability surface for every cached read in the system.
