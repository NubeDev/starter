# starter-cache

Reusable in-process async cache for the starter workspace. Two
distinct layers ship side-by-side:

1. **Primitive: `Cache` trait + `MokaCache` impl** — generic
   `<K, V>` cache with `get`/`insert`/`invalidate`/single-flight
   `get_or_insert_with`. Use this when you need a typed cache for a
   specific data shape (a `MokaCache<UserId, Profile>` etc).
2. **Opt-in `CacheLayer`** — higher-level cache surface designed
   around the [opt-in caching proposal][1]: per-tenant moka caches,
   declarative `CacheSpec` (TTL + scope + invalidate tags), tag
   invalidation with race-safe per-tag monotonic tokens, per-spec
   stats with a load-latency histogram. Use this when you want
   declarative caching for an HTTP / dispatcher boundary.

The opt-in `CacheLayer` is what the rubix extension dispatcher
wires today; see the [v0 caching proposal][1] and the
[operator runbook](../../rubix/docs/operations/cache-runbook.md)
for the integration story.

[1]: ../../rubix/docs/proposal/fe-cache-opt-in.md

## Crate layout

| Module             | What's there                                              |
| ------------------ | --------------------------------------------------------- |
| `cache`            | The generic `Cache<K, V>` trait                           |
| `backends::moka`   | Default in-process backend (TinyLFU, TTL, async)          |
| `spec`             | `CacheSpec`, `CacheScope`, sidecar YAML parser            |
| `layer`            | `CacheLayer` — the opt-in higher-level surface            |
| `invalidator`      | `Invalidator` trait + `InMemoryInvalidator` impl + tokens |
| `per_spec_stats`   | Per-spec hit/miss counters + load-latency histogram       |
| `clock`            | `Clock` trait + `SystemClock` + `MockClock` (test helper) |
| `tracing_cache`    | `TracingCache<C>` — records every cache event, test-only  |

## Why moka

- Concurrent, async-aware, TinyLFU eviction (better hit rates than
  plain LRU).
- TTL, time-to-idle and size-based weights out of the box.
- Pure Rust, no external service to operate — the right default for
  a small team.

## When to add a second backend

- Need a cache **shared across server instances** → add a Valkey
  (BSD-3 Redis fork) backend behind the `Cache` trait.
- Need a cache **larger than RAM** → look at the `foyer` crate.

Either way, only the *primitive* `Cache` trait surfaces the
backend choice. `CacheLayer` always uses moka in v0; multi-node
sharing is a v2 concern.

## Examples

### Primitive `Cache<K, V>`

```rust,no_run
use starter_cache::{Cache, backends::moka::MokaCache};
use std::time::Duration;

# async fn demo() {
let cache: MokaCache<String, String> = MokaCache::builder()
    .max_capacity(10_000)
    .time_to_live(Duration::from_secs(60))
    .build();

cache.insert("home".into(), "<html>…</html>".into()).await;
let page = cache.get(&"home".to_string()).await;
# let _ = page;
# }
```

### Opt-in `CacheLayer` with a sidecar

```rust,no_run
use starter_cache::{CacheLayer, CacheSidecar, CallerScope, LayerConfig};

# async fn demo() {
// Configure: env-driven so operators can tune the per-tenant cap.
let layer = CacheLayer::new(LayerConfig::from_env("MYAPP_CACHE"));

// Parse one sidecar from YAML.
let spec = CacheSidecar::from_yaml(r#"
cache:
  ttl: 60s
  scope: user
  invalidate_on:
    tables: [readings]
"#).unwrap().into_spec().unwrap();

// Use the layer at a dispatch boundary.
let caller = CallerScope::new("tenant-A", "user-1");
let bytes = layer.get_or_load(
    &spec,
    &caller,
    "extension-id::tool-name::input-hash",
    || async {
        // run the underlying query
        Ok::<_, std::convert::Infallible>(std::sync::Arc::new(b"result".to_vec()))
    },
).await.unwrap();
# let _ = bytes;
# }
```

The labelled variant `get_or_load_labelled(..., Some(spec_id), ...)`
additionally records per-spec hit/miss + load-latency counters
keyed by the spec id, which the operator surface in
`rubix-agent` reads back through `GET /api/v1/admin/cache/specs`.

## Test surface

- **Unit tests** in every module.
- **`tests/scenarios.rs`** — five proposal-level scenarios (mid-load
  invalidation, post-invalidate read pays miss, surgical
  invalidation, loader errors not cached, per-tenant cap isolation).
- **`tests/canary_sidecar.rs`** — pins the shipped rubix canary
  sidecar against the v0 shape; a v1+ field (`time_series:`,
  `inner_scope:`, …) trips this test before it can silently degrade.

Run them all with `cargo test -p starter-cache`.
