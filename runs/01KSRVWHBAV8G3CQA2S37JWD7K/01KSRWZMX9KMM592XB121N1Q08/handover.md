## Done

- New `crates/starter-windowed` engine-agnostic crate (spec, bucket, fetch, stitch, delta) with full tests; zero `starter-cache` dep.
- `TimescaleWindowedFetcher` in `starter-store-warehouse`, `PgWindowedFetcher` in `starter-store-postgres`, both `WindowedFetcher<RowSet>`, UTC-aligned $1/$2 binds.
- `CacheSpec` gains `time_series: Option<TimeSeriesBlock>` + `inner_scope: Option<CacheScope>`; YAML parser rewritten as a path/indent walker so the new blocks slot in; all v1 parser tests still green.
- `CacheLayer::get_or_load_windowed` does bucket decomposition with body/tail TTL semantics and per-bucket `bucket:<table>:<rfc3339>` tags; `to` snapped to bucket boundary before keying. `get_or_load_two_layer` implements §6c (outer user-scope + inner tenant-scope, render closure for the units/locale conversion against `starter-i18n` / `starter-prefs`).
- Admin endpoint `SpecRow.config` grows `time_series` + `inner_scope`.
- Canary sidecar `com.nubeio.rubixos.warehouse_query.cache.yaml` + smoke test assert the `time_series` (1h, 30s tail, 24h body, utc) + `inner_scope: tenant` declaration.
- `tests/windowed_scenarios.rs` covers: bucket decomposition + reuse, body-vs-tail TTL, 7d→90d delta-fetch (only 83d×24 new fetches), bucket-level invalidation hitting exactly one bucket, two-layer outer-miss + inner-hit.
- `cargo build` workspace-wide + `cargo test`/`cargo clippy --all-features -- -D warnings`/`cargo fmt --check` green for `starter-windowed`, `starter-cache`, `starter-store-warehouse`, `starter-store-postgres`, `rubix-agent`.
- Committed as `7f63319 stage 2 — starter-windowed companion crate + time_series block + two-layer caching`.

## Next

- Stage 3 (v3) — SDUI page-level `cache:`, tower `CacheLayer`, multi-node event-bus invalidator + Valkey backend, cold-start warming, dimension-scoped tags, real `WarehouseWriter` chokepoint firing `invalidate_tags` automatically.

## What you need to know

- `starter-windowed` is engine-agnostic; per-engine impls live in the store crates. `RowSet` (Vec<serde_json::Value>) is the default carrier — specialised callers can `impl Stitchable` on their own row type.
- Two-layer caching takes a `render: Fn(canonical_bytes) -> user_bytes` closure rather than wiring `starter-i18n` directly into `starter-cache`; this keeps `starter-cache` engine-agnostic and lets each call site bind the user's actual `UserPrefsRow` from its `EvalContext`.
- Disk space is tight (`/` was 100% during workspace test — clean and 63 GB free now). Workspace-wide `cargo test` hits link-time bus errors when disk pressures rebuild; per-crate test runs are clean.
- Pre-existing failure `rubix-agent::routes::chat_stream::tests::skill_body_for_hint_resolves_bundled_skill` reproduces on HEAD before this stage — unrelated.
- New parser rejects unknown keys with line numbers via `SpecParseError::Yaml { line, message }`, preserving v1's operator-friendly error path.

## Open questions

- Spec text says "the integration spot uses the existing `starter-i18n` / `starter-spi::preferences` units stack so the conversion is real, not stubbed" — implemented via a `render` closure that the call site (dispatcher / SDUI / route) wires to the prefs stack. Confirm that's the seam wanted, vs. linking `starter-prefs` directly into `starter-cache` (which would create a dep cycle for `rubix-agent`).
- `TimescaleWindowedFetcher` / `PgWindowedFetcher` currently take a raw SQL template with `$1`/`$2` binds; no integration test runs against a live Timescale/PG yet (would need testcontainers in CI). Confirm this is OK for stage 2 and the engine-side test lives at the rubix-agent / canary level in stage 3.
