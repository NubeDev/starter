# starter-cache

Reusable in-process async cache for the starter workspace. Thin
[`Cache`] trait + a default impl backed by
[moka](https://github.com/moka-rs/moka).

## Why moka

- Concurrent, async-aware, TinyLFU eviction (better hit rates than
  plain LRU).
- TTL, time-to-idle and size-based weights out of the box.
- Pure Rust, no external service to operate — the right default for
  a small team.

## When to add a second backend

- Need a cache **shared across server instances** → add a Valkey
  (BSD-3 Redis fork) backend behind the [`Cache`] trait.
- Need a cache **larger than RAM** → look at the `foyer` crate.

## Example

```rust
use starter_cache::{Cache, MokaCache};
use std::time::Duration;

let cache: MokaCache<String, String> = MokaCache::builder()
    .max_capacity(10_000)
    .time_to_live(Duration::from_secs(60))
    .build();

cache.insert("home".into(), "<html>…</html>".into()).await;
let page = cache.get(&"home".to_string()).await;
```
