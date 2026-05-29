# Cache v0 — Progress Log

## Status
2026-05-29 — v0 landed end-to-end (primitives + parser + dispatcher integration on both builtin and process dispatchers + canary sidecar + 5 scenario tests + 3 integration tests + warehouse-write `// TODO(cache-invalidation):` markers). Numbers TBD — needs a real workload replay on the rubix dev rig.

## Scope reminder
Building **only** the "Minimum viable v0" section of [fe-cache-opt-in.md](../proposal/fe-cache-opt-in.md) (the rest is Deferred). v0 = `CacheSpec` + tag-based invalidation primitives in `crates/starter-cache`, integration at the **extension kind dispatcher only** ([dispatcher.rs](../../../starter-extensions/crates/starter-ext-server/src/rest/dispatcher.rs)), a `kind.cache.yaml` sidecar parser, and `usage_bucketed` as the canary. Three knobs: `ttl`, `scope` (user|tenant|global), `invalidate_on.tables`. Non-negotiable: invalidation-token race fix + per-tenant weight caps. Explicit **non-goals** for v0: SDUI integration, `starter-windowed`, two-layer cache (`inner_scope:`), SWR, `time_series:` block, tower layer, multi-node fan-out, dimension-scoped tags, IR version bump.

## Decisions log
- 2026-05-29 — Integration site is `BuiltinRestDispatcher::dispatch` via a builder `.with_cache(...)` taking `DispatcherCache { layer, registry }`. Why: the proposal pins the v0 integration point at the extension kind dispatcher; the dispatcher trait is host-agnostic so a registry handed in at construction keeps starter-cache decoupled from extension internals. **Update later in session**: the rubixos `usage_bucketed` canary is a `runtime: process` extension, not a builtin — to actually exercise the canary, the same `.with_cache(...)` shape was added to `ProcessRestDispatcher` as well. `NotWiredDispatcher` is still not wrapped (nothing to cache there).
- 2026-05-29 — No unified `WarehouseWriter` chokepoint exists. Confirmed by search: `WarehouseWriteBackend::insert` is per-extension; mart writers and agent ingest are separate; `crates/starter-store-warehouse/src/tsdb/store/*.rs` have four independent `insert_many` paths. v0 ships best-effort write-path hooks behind `// TODO(cache-invalidation):` markers at all five sites (the four store sites + the rubix-agent `RubixWarehouseWriteBackend::insert`) and documents the gap here. Building the real chokepoint is a separate project per the proposal's un-defer conditions.
- 2026-05-29 — Per-tag invalidation token uses an `AtomicU64` per tag. The race fix is **token-on-read** (not subscribe-and-drop): every stored entry carries the snapshot it was loaded under, and `tokens_match()` is rechecked on every `get`. Drop-on-store-if-moved is also enforced in the layer wrapper. This avoids the unsafe code that subscribe-and-drop would have required to break the `Subscription → Invalidator → Subscription` cycle, and lets starter-cache stay `#![forbid(unsafe_code)]`. Cost: token recheck per read. Hot path is one mutex + N hashmap lookups for the per-tag tokens (N = number of tags on the spec). Profile if it becomes a problem.
- 2026-05-29 — Per-tenant weight caps land by **partitioning into one moka cache per tenant**, lazily created with `max_capacity = per_tenant_max_entries`. A noisy tenant evicts only its own entries because they live in a physically separate moka instance. Simpler than a single moka with a custom weigher + tenant accounting, and zero behavioural surprise. Tradeoff: one `moka::future::Cache` per active tenant — a tenant with no traffic costs nothing; a tenant under load costs one moka instance's overhead (small). Default cap: 10,000 entries / tenant — revisit if production data warrants.
- 2026-05-29 — Sidecar YAML parser is hand-rolled to avoid pulling `serde_yaml` into `starter-cache`'s dep tree. The shape it accepts is exactly the v0 shape: `cache: { ttl, scope?, invalidate_on?: { tables?: [...] } }`. Unknown fields are **rejected** at parse time so a v1+ shape (`time_series:`, `inner_scope:`, `stale_while_revalidate:`) fails loudly rather than silently degrading.
- 2026-05-29 — Cached values are `Arc<Vec<u8>>` (serialised JSON). The dispatcher wrapper serialises `serde_json::Value` on store and deserialises on hit. This keeps the cache layer typeless (one moka cache shape per tenant, not one per kind). Cost: an extra serialise/deserialise round trip on cache hits; for the rubixos usage workload this is dwarfed by the avoided warehouse round-trip.

## Sessions
### 2026-05-29 — v0 land
- **What landed:**
  - Commit `docs(cache): seed cache v0 progress log` — initial progress doc.
  - Commit `feat(starter-cache): CacheSpec + invalidator + layer for v0 opt-in caching` — new modules `spec`, `clock`, `invalidator`, `layer`, `tracing_cache`; sidecar parser; `MockClock` / `InMemoryInvalidator` / `TracingCache<C>` test infra.
  - Commit `test(starter-cache): the five v0 scenarios` — `crates/starter-cache/tests/scenarios.rs` covering each of the five "Test story" scenarios (v0-honest analogs where the original mentions post-v0 features). Also unconditional `run_pending_tasks` on `MokaCache` and `CacheLayer`.
  - Commit `feat(starter-ext-server): opt-in cache wrapping at the kind dispatcher` — `KindCacheRegistry`, `DispatcherCache`, `load_from_dir`, `with_cache` on `BuiltinRestDispatcher`, 3 end-to-end tests. Later in the session: same `.with_cache(...)` added to `ProcessRestDispatcher` because the rubixos canary is a process-flavour extension.
  - Commit `chore(warehouse): mark scattered write sites for cache invalidation` — `// TODO(cache-invalidation):` markers at the four `tsdb/store/*.rs` insert sites and the rubix-agent `RubixWarehouseWriteBackend::insert`.
  - Canary sidecar: [`rubix/extensions/com.nubeio.rubixos/kinds/usage_bucketed.cache.yaml`](../../extensions/com.nubeio.rubixos/kinds/usage_bucketed.cache.yaml) — `ttl: 60s`, `scope: user`, `invalidate_on.tables: [com_nubeio_rubixos__histories]`. Pinned against silent edits by `crates/starter-cache/tests/canary_sidecar.rs`.
  - Rubix-agent boot picks up every validated extension's `kinds/*.cache.yaml` and shares a single `CacheLayer` across the builtin and process dispatchers.
- **What's blocked:**
  - Numbers (hit rate, latency reduction) — needs the rubix dev rig with the rubixos extension enabled and a realistic dashboard refresh workload. Plan: drive `/extensions/com.nubeio.rubixos/usage` at the dashboard refresh cadence with `wrk` or a small replay script; record warm-vs-cold latency from the dispatcher `info!` log site we already have. Defer to next session because it needs an environment, not code.
  - Unified `WarehouseWriter` chokepoint — separate project (proposal un-defer condition #2). Until it lands, write-path invalidation is best-effort; the five `// TODO(cache-invalidation):` markers will guide the swap.
- **What's next:**
  1. Replay a realistic `usage_bucketed` load against the rubix dev rig; record hit rate + p50/p95 latency in this doc under "Numbers".
  2. Decide whether `ProcessRestDispatcher` caching shows the same hit-rate shape as the (unused) builtin path; the wire-bytes cost is identical from the cache's POV.
  3. If hit rate is low because each user sees a slightly different `to=now` value, the v1 `time_series:` block is the right next move — **do not** sneak it in as v0; that's a separate proposal slice.
- **Numbers:**
  - LoC: starter-cache +880 net, starter-ext-server +290 net, rubix-agent +85 net, warehouse markers +28 (no behavioural change).
  - Test count: starter-cache `+5 scenarios + 1 canary + 9 unit = +15`; starter-ext-server `+3 integration + 2 cache::tests`.
  - Hit rate / load-time reduction: TBD pending workload replay (see "What's blocked").

### 2026-05-29 — wiring tighten (same day)
- **What landed:** `ProcessRestDispatcher::with_cache` (process-flavour extensions need it for the canary to fire; rubixos is `runtime: process`). Rubix-agent boot sweeps each validated extension's `kinds/` dir for `*.cache.yaml` and hands one shared `CacheLayer` to both dispatchers. Sidecar parse errors are surfaced as `warn!` per file — one bad sidecar does not block the rest.
- **What's blocked:** Same as above — measurements pending workload replay.
- **What's next:** Workload replay + numbers.
- **Numbers:** TBD.
