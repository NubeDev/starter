## Done

- Reviewed stage 2 diff (commit 7f63319) against Layer-1 invariants R1/R2/R4/R5 and wire-format stability.
- Confirmed starter-windowed has no starter-cache dependency; per-engine fetchers (TimescaleWindowedFetcher, PgWindowedFetcher) live in their store crates and consume the trait, satisfying the "reusable for non-cache consumers" criterion.
- Confirmed `inner_scope:` two-layer wiring uses a render closure into starter-i18n/starter-prefs rather than linking prefs into starter-cache — no dep cycle.
- Confirmed bucket decomposition + delta-fetch (`extend`) + body/tail TTL keying match the worked example D contract.
- PASS: stage 2 v2 diff preserves R1 (one-way dep: cache → windowed; stores → windowed), R2 (sqlx-only transport), R4/R5 ($1/$2 bound binds, operator-supplied SQL template), and adds only additive YAML keys.

## Next

- Stage 3 (v3): SDUI page-level `cache:`, tower `CacheLayer`, multi-node event-bus invalidator + Valkey backend, cold-start warming, dimension-scoped tags, WarehouseWriter chokepoint firing `invalidate_tags` automatically.

## What you need to know

- The per-engine fetcher seam is a SQL template with $1=bucket_start, $2=bucket_end (UTC, exclusive end). Any later positional binds are caller-supplied; this is the contract stage 3 should code against.
- Stitch contract: `Stitchable::stitch(Vec<T>) -> T` over buckets returned in chronological order from `decompose`; `RowSet` is the default carrier, specialised callers impl on their own row type.
- Two-layer caching takes `render: Fn(canonical_bytes, CallerScope) -> user_bytes`; site (dispatcher / SDUI route) supplies the closure bound to its `EvalContext` UserPrefsRow.
- No live Timescale/PG integration coverage of the fetchers exists yet; scenarios are unit-level. Worth landing testcontainers coverage at the canary level in stage 3.

## Open questions

- Confirm with the user that the `render` closure seam (rather than linking starter-prefs into starter-cache) is the intended Layer-6c shape before stage 3 starts wiring SDUI page caches that share this code path.

PASS: stage 2 preserves R1 (one-way deps cache→windowed, stores→windowed), R2 (sqlx-only transport), R4/R5 ($1/$2 parameter binds, operator-supplied SQL template), and only adds additive YAML keys.
