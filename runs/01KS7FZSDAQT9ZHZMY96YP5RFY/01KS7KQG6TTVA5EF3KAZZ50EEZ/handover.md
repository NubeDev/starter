## Done

- created `crates/starter-blob-compose/` with `Namespaced`, `Tiered { TieredPolicy }`, `Mirrored { MirrorMode, MirroredBuilder }`, and `ReadThroughCache` — all implement `BlobStore` and nest; each combinator wraps inner `BlobRef`s with a versioned JSON envelope locator (see `src/codec.rs`).
- `Namespaced` prepends/strips its prefix on every op including `list()`; outer refs only round-trip through the wrapper.
- `Tiered` honours `demote_above_bytes` at put time, hot-then-cold read fallback, deletes both tiers (B3), and walks hot-then-cold in `list()` via a cursor prefix.
- `Mirrored::Sync` fails on any mirror error; `Mirrored::AsyncBackground` returns on primary success and spawns the fan-out — variant names are the B3 contract.
- `ReadThroughCache` writes source-only, populates cache lazily on `get`, and clears the cache on `delete` so stale bytes cannot out-live a source delete.
- Type-level rustdoc on every combinator documents failure modes per B3.
- compose-test smoke fixture landed at `crates/starter-blob-compose/tests/compose_test.rs`; the TODO markers in `starter-blob-memory/tests/swap_test.rs` and `starter-blob-fs/tests/swap_test.rs` are replaced with pointers to the new fixture.
- Workspace `Cargo.toml` lists `starter-blob-compose` as a member and as a workspace dep alias.
- `cargo test -p starter-blob-compose` (14 unit + 4 integration) green; `cargo clippy -p starter-blob-compose --all-targets -- -D warnings` clean; `cargo fmt` clean for the new crate.
- Committed on `codeless/blob-storage` as `8d6d889 stage 4 — starter-blob-compose (combinators)`.

## Next

- Stage 5 — `examples/blobs/` (axum upload → SQLite-`BlobRef` → presigned-GET) plus assemble the five SCOPE smoke fixtures (Swap, Compose, TestWithoutNetwork, CostToSkip, R8) at workspace level in `crates/smoke-tests` and hoist `compose_test.rs` from the compose crate into that smoke-tests crate.

## What you need to know

- `BlobKey` is `Clone` (derived); combinators rely on this in the async-mirror spawn path. `BlobRef` is `Clone, Serialize, Deserialize` — codec uses `serde_json` round-trip directly through it.
- `Tiered::put_stream` always lands in hot because the size-demotion policy needs `put_bytes` size up front; this is documented in the type-level rustdoc.
- `Tiered::promote_back_on_read` deliberately *logs and skips* rather than fabricating a `BlobKey` from the inner ref — the inner ref has no recoverable key by B2. Operator-driven promotion via `list()` + `copy_via_client` is the supported path; documented in the in-source comment.
- `Mirrored` does **not** propagate `delete` to mirrors; the rustdoc explains why (use `Mirrored<Tiered<...>>` if you want cross-mirror delete coordination).
- `ReadThroughCache` looks up cache entries by `BlobKey` via `cache.list(Some(key), None)` rather than persisting a separate cache-ref — the cache is treated as a side-effect store that may have evicted.
- The TieredPolicy.demote_above_age field is shipped but currently advisory — there is no background sweeper in stage 4; the field is reserved so on-eviction sweepers can layer on without a SPI change.
- The previous-stage `pre-check-failed` flagged paths-not-in-diff for `starter-blob-s3` / `starter-blob-garage`; the auto-bypass advanced through stage 3 with those crates already on disk from earlier work. This stage 4 commit's diff matches the handover exactly.

## Open questions

- (none)
