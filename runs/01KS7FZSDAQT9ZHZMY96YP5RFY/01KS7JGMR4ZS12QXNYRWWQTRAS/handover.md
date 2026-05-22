## Done

- Reviewed stage 2 against the gate criteria: trait test suite green for both engines (12 fs + 14 memory unit, 1+1 swap, 1 network), clippy `-D warnings` clean under `--all-features`.
- Confirmed presign router contract per crate: memory router accepts tokens minted by the same store and rejects tokens from a different instance / forged tokens; fs router streams bytes back for a `PresignKey`-signed URL.
- Confirmed B1: zero matches for avatar/attachment/upload/photo/image/document/file_share/profile_pic in either crate's `**/*.rs` (case-insensitive).
- Confirmed B2 in the stage-2 smoke tests: every `BlobRef` in `swap_test.rs` (memory + fs) and `test_without_network.rs` comes from `store.put_bytes(...)`; no test imports `BlobRefInternal` or constructs a ref from a string. `BlobRef` still has `pub(crate)` fields, no `Display`, no `key()` / `locator()` accessors, and `Debug` uses `finish_non_exhaustive` so it cannot leak `opaque_locator`.
- Confirmed R1/R2/R4-R5: both engines depend only on `starter-spi` + utility crates; presign uses a single HTTP transport (axum, feature-gated); additions are purely additive — no existing wire format changed shape.

## Next

- Stage 3 (`starter-blob-s3` + `starter-blob-garage` + docker recipe) per the SCOPE. A fresh session picks it up.

## What you need to know

- `BlobRefInternal::mint` is `pub` (engines are downstream crates and need a sanctioned door). A non-engine consumer who imports it could mint a `BlobRef` from raw parts — that is a stage-1 design call (SCOPE Q2 resolution), not a stage-2 regression. The gate question scopes B2 to smoke-test consumer code, which never imports it.
- Pre-existing baseline `starter_flow_spi_baseline_holds` failure noted in prior handover is upstream Cargo.lock drift, not caused by stage 2.
- PASS: stage-2 trait suite + per-engine presign router contract + B1/B2/R1/R2/R4-R5 all hold, no patches required.

## Open questions

- (none)
