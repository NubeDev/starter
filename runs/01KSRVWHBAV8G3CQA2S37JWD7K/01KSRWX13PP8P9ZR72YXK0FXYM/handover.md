## Done

- `claude` did not emit a structured handover; final message follows.

## Next

- Read the diff (Files changed tab) and the Timeline before deciding what to do next.

## What you need to know

- …ormats.
  
  ## Next
  
  - Stage 2 work session: `starter-windowed` crate, `time_series:` CacheSpec block, `TimescaleWindowedFetcher`, two-layer `inner_scope:` caching.
  
  ## What you need to know
  
  - Stage-1 work landed at commit 5643b51; commit 62961ab is a stage-0 handover-only commit on top.
  - New crate deps: rubix-agent → starter-cache, starter-ext-server → starter-cache. Direction is higher→lower; R1 holds.
  - openapi.json is unchanged; admin SpecRow grew only additive optional fields (`stale_while_revalidate_seconds`, `empty_ttl_seconds`, `cache_empty`, `invalidate_on_events`, `invalidate_on_buckets`).
  - Per the stage-1 handover: SWR refresh is caller-driven not background-spawned (intentional, deferred to v3 alongside the WarehouseWriter chokepoint which wants the same `'static` refresher refactor). Empty entries hard-expire at `empty_ttl` (skip SWR + max_stale).
  - Baseline `cargo test --workspace --all-features` and `cargo clippy ... -D warnings` are red on main for pre-existing unrelated reasons (process+wasm flavour marker conflict in starter-ext-sdk; `io_other_error` lints in starter-ext-server/src/lifecycle.rs); stage 1 added no new failures.
  
  ## Open questions
  
  - Stage-1 handover flags one open question for stage 3: should the dispatcher fire write-path invalidation when `cache.handlers` is unwired but a spec exists? Currently a strict no-op; revisit at the SDUI/tower-layer integration site.
  - Stage-2 needs to decide the per-bucket fan-out wire shape for `bucket:<table>:<granularity>` (currently coarse, one tag per spec).
  
  ## Verdict
  
  - PASS: stage-1 changes keep crate deps flowing higher→lower, add no new transport, tighten (not loosen) the trust boundary via the writing-handler declaration rule, and leave openapi.json + SPI wire types untouched.
  ```
  
  PASS: stage-1 changes keep crate deps flowing higher→lower, add no new transport, tighten (not loosen) the trust boundary via the writing-handler declaration rule, and leave openapi.json + SPI wire types untouched.

## Open questions

- (none)
