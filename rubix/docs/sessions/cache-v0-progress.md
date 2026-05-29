# Cache v0 — Progress Log

## Status
2026-05-29 — In progress: v0 primitives + sidecar parser + dispatcher integration + canary landing in initial session.

## Scope reminder
Building **only** the "Minimum viable v0" section of [fe-cache-opt-in.md](../proposal/fe-cache-opt-in.md) (the rest is Deferred). v0 = `CacheSpec` + tag-based invalidation primitives in `crates/starter-cache`, integration at the **extension kind dispatcher only** ([dispatcher.rs](../../../starter-extensions/crates/starter-ext-server/src/rest/dispatcher.rs)), a `kind.cache.yaml` sidecar parser, and `usage_bucketed` as the canary. Three knobs: `ttl`, `scope` (user|tenant|global), `invalidate_on.tables`. Non-negotiable: invalidation-token race fix + per-tenant weight caps. Explicit **non-goals** for v0: SDUI integration, `starter-windowed`, two-layer cache (`inner_scope:`), SWR, `time_series:` block, tower layer, multi-node fan-out, dimension-scoped tags, IR version bump.

## Decisions log
- 2026-05-29 — Integration site is `BuiltinRestDispatcher::dispatch` via a builder `.with_cache_registry(...)` taking `Arc<KindCacheRegistry>`. Why: the proposal pins the v0 integration point at the extension kind dispatcher; the dispatcher trait is host-agnostic so a registry handed in at construction keeps starter-cache decoupled from extension internals. The `NotWiredDispatcher` and `ProcessRestDispatcher` are not wrapped in v0 (canary is builtin-only).
- 2026-05-29 — No unified `WarehouseWriter` chokepoint exists. Confirmed by search: `WarehouseWriteBackend::insert` is per-extension; mart writers and agent ingest are separate; `crates/starter-store-warehouse/src/tsdb/store/*.rs` have four independent `insert_many` paths. v0 ships best-effort write-path hooks behind `// TODO(cache-invalidation):` markers and documents the gap here. Building the real chokepoint is a separate project per the proposal's un-defer conditions.
- 2026-05-29 — Per-tag invalidation token uses an `AtomicU64` per tag, snapshotted at load-start and re-checked before `insert`. Drop-on-store-if-moved is implemented inside the `CacheLayer::get_or_insert_with` wrapper, not inside `MokaCache` — that keeps the moka backend untouched and the race fix testable against any backend.
- 2026-05-29 — Per-tenant weight caps land via `moka`'s `weigher()` + `max_capacity()` configured on `CacheLayer::new`. Tenant id is mixed into the cache key under `scope: tenant|user`; the weigher charges every entry weight 1 with a tenant-aware bucket cap enforced by a small wrapper map of per-tenant moka caches. Simpler implementation in v0: one moka cache per tenant, lazily created, each with a configured `max_capacity`. This naturally bounds a noisy tenant without cross-tenant interference.

## Sessions
### 2026-05-29 — v0 land
- What landed: see commits in this session.
- What's blocked: none in v0 scope. The unified `WarehouseWriter` chokepoint is a separate project (proposal un-defer condition #2).
- What's next: collect canary numbers from a real `usage_bucketed` workload once the rubix dev rig is up.
- Numbers: TBD — captured under "## Canary measurements" once a real workload is replayed.
