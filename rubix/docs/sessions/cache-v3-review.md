# Stage 6 — v3 Layer-1 review

Scope: review the cumulative cache v1+v2+v3 diff (commits `62961ab`
→ `b2206c2`) against the rulebook's Layer-1 invariants — R1 crate
dependency direction, R2 single transport, R4/R5 trust boundary,
wire-formats untouched.

## Verdict: PASS

## R1 — crate dependency direction

- `starter-cache` depends only on `starter-windowed` plus
  workspace utility crates (async-trait, chrono, serde, tokio,
  tracing, thiserror, optional moka / tower-layer / http). No
  `rubix-*` edge.
- `starter-windowed` depends only on async-trait / chrono /
  serde / thiserror. Engine-agnostic, zero `starter-cache` edge —
  flow nodes / agent steps can consume it without pulling the
  cache crate, exactly as the proposal specifies.
- `starter-sdui-routes` adds `starter-cache` (upstream → upstream).
- `starter-store-warehouse` and `starter-store-postgres` add
  `starter-windowed` (upstream → upstream) and host the
  `TimescaleWindowedFetcher` / `PgWindowedFetcher` impls there.
- `rubix-agent` consumes `starter-cache`, supplies the
  `RubixInvalidationBus` adapter that bridges `starter-cache`'s
  abstract `InvalidationBus` port to the existing `RubixEventBus`.
  Direction is `rubix-agent → starter-cache`, never the reverse.
- The Valkey backend lives in `crates/starter-cache/src/backends/
  valkey.rs` behind a feature flag; no new top-level crate, no
  `rubix-*` dep introduced.

## R2 — single transport

- The `EventBusInvalidator` does **not** introduce a second
  transport. It speaks to `starter_cache::InvalidationBus`, a
  trait. The host-side adapter publishes through the existing
  process-local `RubixEventBus` (the same bus extensions already
  use). Multi-node fan-out is one trait call away from the
  in-memory invalidator and re-uses whatever transport the
  event-bus is itself swapped onto later.
- `CacheLayer::tower()` produces an `axum`-compatible
  `tower::Layer` that wraps the *existing* axum app. No new HTTP
  listener, no second `Router` mount.
- SDUI cache integration wraps work inside the existing
  `POST /api/v1/ui/resolve` / `GET /api/v1/ui/table` handlers —
  cache lookup is in-process, not a side-channel.
- The Valkey backend is gated behind a `valkey` feature; the
  shipped impl is the shape-correct mock — no live `redis://`
  client compiled into the default build, so R2 is preserved in
  the default deployment too.

## R4 / R5 — trust boundary

- `/ui/action` is explicitly never cached. Action dispatch still
  goes through the existing tool registry; the SDUI cache
  integration only wraps resolve + table read paths.
- Cache keys carry `CallerScope { tenant, user }` plus the
  `units_hash`, so a tenant-scoped cache entry cannot leak to a
  user in another tenant, and per-user unit prefs yield distinct
  entries. AuthZ for the page itself runs at the route layer
  before cache lookup — `CacheLayer::get_or_load_labelled` is
  called from inside the already-authorized handler scope.
- The two-layer (`inner_scope:`) cache only relaxes scoping
  *down* the unit-conversion seam: the canonical-units payload is
  cached at tenant scope and converted on read for the requesting
  user. Authorization is unaffected — same user-scoped check
  upstream of both lookups.
- `WarehouseWriter::commit` requires the writer to be constructed
  with a registry of declared dim-tagged tables. The chokepoint
  emits only `table:`, `bucket:`, and registry-declared
  `table:<n>:<dim>=<v>` tags. No untrusted-input flows into the
  invalidator surface.

## Wire formats untouched

- `ComponentTree` gains an additive `cache: Option<PageCacheBlock>`
  root field with `#[serde(skip_serializing_if = "Option::is_none")]`
  — older IR readers ignore it. The IR_VERSION constant is
  unchanged, matching the proposal's "no IR version bump" clause.
- `ResolveRequest` / `ResolveResponse` / `ActionRequest` /
  `TableRequest` shapes unchanged.
- Admin `SpecRow` JSON grows additive fields
  (`config.stale_while_revalidate_seconds`, `config.empty_ttl_seconds`,
  `config.cache_empty`, `config.invalidate_on_events`,
  `config.invalidate_on_buckets`, `config.time_series`,
  `config.inner_scope`, `config.invalidator_kind`, and the warmer
  status fields). Existing v0 admin clients keep parsing the
  payload.
- `WarehouseWriteBackend::insert` is an internal trait — not a
  wire format. The chokepoint is type-system-enforced (write
  paths take `&dyn WarehouseWriter` and `commit()` deduplicates +
  fires the invalidator). The five v0 `// TODO(cache-invalidation):`
  comment sites are gone from source; only the runbook/session
  log retain historical references.

## Type-system enforcement of invalidation

The v0 posture ("lint-and-hope") is now compile-checked:

- Writing extension handlers cannot reach registration without
  declaring `affects_tables: Vec<String>`; the dispatcher hard-
  errors on a writing handler with no declaration (v1).
- SDUI action handlers carry the same `SduiActionMeta` declaration.
- The warehouse path goes through `WarehouseWriter::commit` which
  emits the deduped tag set automatically — write paths cannot
  forget to fire invalidation, because the only way to ship rows
  is `enqueue → commit`.

## Multi-node / Valkey wiring sanity

- `RUBIX_CACHE_INVALIDATOR=local|event-bus` selects the
  invalidator at boot. Both arms hand back
  `Arc<dyn Invalidator>` so the layer wiring is identical.
- A separate subscriber path on boot calls
  `EventBusInvalidator::apply_remote` for peer publishes — that
  bumps local tokens without re-publishing, so fan-out cannot
  storm.
- Single-node deployments stay on `InMemoryInvalidator` by default;
  the multi-node story is opt-in, matching the proposal's
  "ship-when-needed" framing.

## Functional gaps (flag for ramp step, not Layer-1 fail)

- The Valkey backend ships as a shape-correct in-memory mock. The
  swap to a real `redis://` client is documented as a one-file
  change inside `backends/valkey.rs`. This is honest and the
  feature flag prevents accidental "we're multi-node now" claims;
  flag for a follow-up ramp step.
- Cold-start warming replays top-N by hit count from in-process
  stats. After a process restart those stats are zero, so the
  *first* boot warms nothing; subsequent boots warm against the
  previous run's stats persisted to disk. Worth a sentence in the
  runbook ("warming is a no-op on the very first boot").
- Dimension-scoped tag cardinality is bounded only by the
  registry config; an operator who tags a high-cardinality column
  (e.g. `device_uuid`) will explode the tag table. Worth a
  cardinality warning in the runbook's "dimension-scoped tags"
  section.

PASS: R1 / R2 / R4 / R5 / wire-format invariants all hold —
v3 distribution layer, SDUI integration, Valkey backend, event-bus
invalidator, cold-start warmer, dimension-scoped tags, and the
unified `WarehouseWriter` chokepoint compose without crossing
crate-dep direction, adding a second transport, weakening the
trust boundary, or breaking any existing wire format.
