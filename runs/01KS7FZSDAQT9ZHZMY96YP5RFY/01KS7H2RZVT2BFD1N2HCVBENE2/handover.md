## Done

- Added `crates/starter-spi/src/blob/` module: `mod.rs` (barrel + observability contract doc), `key.rs` (validated BlobKey + BlobKeyError + MAX_BLOB_KEY_LEN=1024), `blob_ref.rs` (BackendId, Etag, opaque BlobRef + BlobRefInternal engine-facing trait), `meta.rs` (BlobMeta, BlobRange w/ HTTP-inclusive semantics), `presigned.rs` (PresignOp, PresignedUrl with absolute expires_at), `error.rs` (full typed BlobError with Unauthorized vs Forbidden distinct, Throttled{retry_after}, Unsupported, Backend(BoxError) residual), `store.rs` (async BlobStore trait + PutOptions + ListPage + copy_via_client + default copy_server_side → Unsupported).
- `pub mod blob;` added unconditionally to `crates/starter-spi/src/lib.rs` (matches the `secrets` precedent — foundational seam, not a feature).
- Added `bytes` and `futures` workspace deps to `crates/starter-spi/Cargo.toml`.
- Verified zero `Blob[A-Z]\w*` symbol collisions across all `.rs` files in the workspace before landing.
- 12 new unit tests pass (BlobKey validation incl. serde re-validation; BlobRef debug-omits-locator, accessors, serde round-trip, with_locator).
- `cargo check --workspace`, `cargo clippy -p starter-spi --all-targets -- -D warnings`, `cargo test -p starter-spi --lib`, and `rustfmt --check --edition 2021` on new files all clean.
- Committed as `50926e5` on branch `codeless/blob-storage` with the stage-1 title.

## Next

- Stage 2: implement `starter-blob-memory` and `starter-blob-fs` engines against the frozen trait. Both must ship feature-gated axum routers that honour their own presigned URLs so the presign contract is testable without a real S3. The trait test suite (smoke fixtures) is to be written in stage 2 and reused by stages 3-4.
- REVIEW gate 1 (B2 opacity audit) should run before stage 2 starts: rustdoc check, `pub` field scan, attempted consumer-side raw-key extraction must fail to compile.

## What you need to know

- `BlobRef` opacity is enforced via `pub(crate)` fields + `BlobRefInternal` trait. Engines mint refs via `BlobRefInternal::mint(...)` and read the locator via `opaque_locator()`. Consumers cannot reach either.
- `BlobStore::copy_server_side` has a default `Err(Unsupported)` body; engines override only when their backend supports CopyObject. The companion free function `copy_via_client` lives in `starter-spi::blob::store` (re-exported from the module barrel).
- `ListPage.items` is `Vec<(BlobRef, BlobMeta)>` — never raw `BlobKey`. Combinators will rewrite the `BlobRef` on the way out via `BlobRefInternal::with_locator`.
- `BlobRange` uses HTTP-inclusive `start..=end` semantics (matches presigned-GET path); `BlobRange::from(start)` gives an unbounded tail via `end = u64::MAX`.
- `PutOptions` is `#[non_exhaustive]` and intentionally minimal (content_type, cache_control, if_absent). Backend-specific knobs (SSE, S3 storage class) belong on engines' concrete types — not on the SPI struct.
- spi emits zero `tracing` spans / metrics; engine crates own observability under `starter_blob::<engine>` targets per the module doc contract.
- Pre-existing `cargo fmt --check` failures in `crates/starter-spi/src/units/tests.rs` are unrelated to this stage; my new files all pass `rustfmt --check --edition 2021`.

## Open questions

- (none)
