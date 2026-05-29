## Done

- Extended `CacheSpec` with `stale_while_revalidate`, `max_stale` (default 2*ttl), `empty_ttl` (default 5s, clamped to ttl), `cache_empty` (default true); `InvalidateOn` with `events: Vec<String>` and `buckets: Option<BucketTagSpec{table,granularity}>`.
- Hand-rolled YAML parser accepts every new key (line-numbered errors preserved); `derived_tags()` emits `table:`, `event:`, and `bucket:<t>:<g>` tags.
- Added `LoadOutcome::{Value, Empty}`; new `CacheLayer::get_or_load_labelled_outcome` is the canonical entry point. Bytes-only `get_or_load_labelled` adapts to it.
- SWR semantics implemented via the `Clock`: fresh-until = `ttl - swr`, stale-limit = `ttl + max_stale`; first stale-window caller is served from cache and marks the entry, next caller drives the refresh. Empty entries skip SWR and `max_stale` (hard expire at `empty_ttl`). Race-fix preserved.
- New `starter-ext-server::rest::cache` types: `HandlerMeta`, `HandlerCatalog`, `HandlerCatalogBuilder` — registering a writing handler without `affects_tables` is a hard error (`HandlerRegistrationError::WritingHandlerMissingTables`). `DispatcherCache::with_handlers(...)` wires them.
- `BuiltinRestDispatcher` and `ProcessRestDispatcher` fire `invalidate_tags(meta.invalidation_tags())` after every successful write call (both cached and uncached paths) via `fire_write_invalidation` helper.
- Admin `SpecRow.config` gains `stale_while_revalidate_seconds`, `empty_ttl_seconds`, `cache_empty`, `invalidate_on_events`, `invalidate_on_buckets`.
- `scenarios.rs` gains `swr_stale_served_while_refresh_pending` + `empty_result_respects_empty_ttl`; new `starter-ext-server/tests/handler_catalog.rs` covers read-only vs writing registration (3 tests, all green).
- Canary sidecar adds `stale_while_revalidate: 30s`; `canary_sidecar.rs` asserts it.
- Runbook gains "SWR explained (v1)" and "How to declare a writing handler (v1)" sections.
- All starter-cache + starter-ext-server + rubix-agent admin_cache_test tests pass. `cargo fmt` applied across the workspace.

## Next

- Stage 2 — companion `starter-windowed` crate, `time_series:` as a CacheSpec block, `TimescaleWindowedFetcher`, two-layer `inner_scope:` caching.

## What you need to know

- **SWR refresh is caller-driven, not background-spawned.** The proposal asks for "single-flight one background refresh per key"; v1 implements caller-driven refresh because true background spawn requires the loader to be `'static + Fn + Clone`, which would force an `Arc<Self>` refactor of both dispatchers. This is intentionally deferred to v3 — both true-background-SWR and the `WarehouseWriter` chokepoint want the same `'static` refresher abstraction. Doc-stringed inline on `get_or_load_labelled_outcome` and called out in the runbook.
- **Empty entries skip SWR + max_stale** — they expire hard at `empty_ttl`. This is a layer-level choice not literally in the proposal text but operationally required (otherwise a noisy empty-cell lingers up to `ttl + 2*ttl` = 180s for a 60s spec, defeating the point of `empty_ttl`).
- **`cargo test --workspace --all-features`** fails at baseline with a pre-existing `__STARTER_EXT_FLAVOUR_MARKER` symbol conflict in `starter-ext-sdk` when both `process` and `wasm` flavour features are simultaneously enabled. This is unrelated to stage 1 and predates this branch — verified by `git stash` + retest.
- **`cargo clippy --workspace --all-features -- -D warnings`** also fails at baseline due to pre-existing `io_other_error` lints in `starter-ext-server/src/lifecycle.rs` (Rust 1.91 lint, predates this branch). Stage 1 introduces no new clippy warnings.
- **One pre-existing test** (`rubix-agent::routes::chat_stream::tests::skill_body_for_hint_resolves_bundled_skill`) fails at baseline — verified by stash + retest.
- The repo has **three separate cargo workspaces**: root, `starter-extensions/`, `rubix/`. Cross-workspace `-p` flags don't work — `cd` into the right tree first.
- `starter-ext-server` is now `pub mod rest` (was `mod rest`) and `pub mod cache` (was `mod cache`) so external test crates can import `HandlerCatalog`, etc. directly. Existing re-exports at `lib.rs` line 78–82 are unchanged.

## Open questions

- Do we want the dispatcher to also fire write-path invalidation when `cache.handlers` is unwired but the spec exists? Currently `fire_write_invalidation` is a strict no-op without `HandlerCatalog`. Probably correct, but worth a re-read in stage 3 when the SDUI/tower-layer integration site lands.
- The `bucket:<table>:<granularity>` derived subscription tag is currently coarse (one tag per spec). Stage 2's `TimescaleWindowedFetcher` will need to teach the invalidator about per-bucket fan-out; the wire-shape for that fan-out is not yet decided.
