## Done

- Landed `starter-blob-memory` and `starter-blob-fs` crates with full `BlobStore` impls (put_bytes/put_stream/get/head/delete/list/presign), feature-gated `axum` routers in the same crate, swap-test fixtures (both crates) plus a `test_without_network` fixture against the memory engine. All B1/B2/B3 properties hold by construction: no domain words on the public surface, BlobRef opacity preserved (engines mint via `BlobRefInternal::mint`, never reconstruct from strings), no silent durability shift (FS refuses to mint a `PresignKey` implicitly, memory engine documents per-restart rotation).
- Memory engine: 32-byte HMAC key randomised at `MemoryBlobStore::new()`, signs JSON claims with HMAC-SHA256, URL_SAFE_NO_PAD base64, axum router validates against the same in-memory store instance.
- FS engine: atomic writes via `tempfile::NamedTempFile::persist`, conditional via `OpenOptions::create_new` (O_EXCL), walkdir with configurable `max_depth` (default 32), `.meta.json` sidecar with content-type / cache-control / timestamps. `FsBlobStore::open(path, PresignKey)` is the canonical constructor; `PresignKey::ephemeral()` exists for tests with documented die-with-process semantics.
- Compose-test smoke is a TODO marker in each crate's `swap_test.rs` referencing stage 4 (the combinator crate doesn't exist yet, per the brief).
- Operator-policy durable fix: stage 1 marked `BlobMeta`, `ListPage`, `PutOptions` `#[non_exhaustive]` but shipped no constructors, leaving engines outside the spi crate unable to literal-construct them. Added `BlobMeta::new` + `with_*` setters, `ListPage::new`, `PutOptions::if_absent()` / `.cache_control()` chain setters in `starter-spi`. Purely additive — existing spi internals continue to compile, downstream crates are unaffected.
- `cargo clippy -p starter-spi -p starter-blob-memory -p starter-blob-fs --all-targets --all-features -- -D warnings` clean. Also clean without `--all-features`. 47 stage-2 tests pass (12 fs lib + 1 fs swap + 14 memory lib + 1 memory swap + 1 memory test_without_network + the spi/secrets unchanged baseline).
- Committed as `c57119f` on `codeless/blob-storage`.

## Next

- Stage 3 — `starter-blob-s3` + `starter-blob-garage`. Wrap `aws-sdk-s3` with `force_path_style`, multipart `put_stream`, conditional `If-Match`/`If-None-Match` → `PreconditionFailed`, distinct 401/403 mapping, `SlowDown` → `Throttled` with `Retry-After`. Garage layers bucket lifecycle / key minting / cluster health / layout introspection on top via its admin API (HTTP only, no source-level import — AGPL boundary). `docker/garage.example.toml` + `docker/docker-compose.garage.yml` ship here.

## What you need to know

- The two engines emit tracing spans under `starter_blob::memory` / `starter_blob::fs` per stage 1's observability contract — `starter-spi` itself still emits nothing.
- `MemoryBlobStore`'s presign router is bound to a single store instance: presigned URLs do not cross store instances (different HMAC keys). The swap_test in `starter-blob-memory` exploits this to assert process-local rotation.
- `FsBlobStore` uses a `tokio::sync::Mutex` per-key to serialise put/delete ops on the same key; the map of locks grows during contention and is otherwise cheap. The data path on disk is `<root>/<key>`; the sidecar is `<root>/<key>.meta.json` — `list()` filters sidecars out. Keep this in mind when stage 4 wraps the engine in `Namespaced` (the prefix is applied at the trait layer; the engine sees the combined key).
- Stage 1's `BlobMeta` / `ListPage` / `PutOptions` are now constructible from outside the spi crate via the new `new` / `with_*` helpers. Stage 3+ engines should use the same pattern rather than `Struct { … }` literals.
- The `axum` feature on both crates pulls `axum + http + tower` (+ `tokio-util` on the fs crate). Default features are empty, so a consumer who never needs to test the presign loop pays nothing — CostToSkip property preserved.
- Pre-existing baseline test `starter_flow_spi_baseline_holds` (in `crates/starter-flow/tests/workspace_dep_tree_gates.rs`) fails on the prior commit too — not caused by this stage. Likely a Cargo.lock drift from utoipa/indexmap upstream. Worth fixing in a separate clean-up commit so the next REVIEW gate has a green workspace.

## Open questions

- (none)
