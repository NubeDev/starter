## Done

- Added `examples/blobs/` — axum app with PUT `/attachments/{name}` → SQLite `attachments(id, name, blob_ref_json)` → GET `/attachments/by-id/{id}` returning a presigned-GET envelope; in-memory SQLite + `starter-blob-memory` so it stands up with no external services. Round-trip integration test (`tests/round_trip.rs`) binds a real port and follows the presigned URL.
- Added 5 workspace-level SCOPE smokes under `crates/smoke-tests/tests/`: `blob_swap_test.rs`, `blob_compose_test.rs`, `blob_test_without_network.rs`, `blob_cost_to_skip.rs`, `blob_r8_doc_comments.rs`. All five pass.
- Wired `examples/blobs` into workspace `members` and added blob crate deps to `crates/smoke-tests/Cargo.toml`.
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` both clean across the new crate and the smoke-tests crate.
- Committed as `6d7db56 stage 5 — examples/blobs walkthrough + the five SCOPE smoke tests assembled at workspace level`.

## Next

- (none) — stage 9 of 9; the blob-storage job is complete.

## What you need to know

- Axum 0.8 path syntax is `{name}` not `:name`; the example originally hit "conflict with previously registered route" so the fetch route is `/attachments/by-id/{id}` to disambiguate from the upload route at the same prefix.
- `MemoryBlobStore` is `Clone` (internally `Arc`'d); the example clones it once so the consumer routes hold one handle and the engine's `/blobs` router holds another with the same HMAC key.
- The CostToSkip smoke shells out to `cargo tree -p starter-server --no-default-features` from `repo_root()`. It is therefore sensitive to the workspace layout; if a future commit adds a `default = ["blobs"]` feature to `starter-server`, this test correctly turns red.
- The R8 smoke is a *syntactic* floor (immediately-preceding `///` line). A reviewer still has to verify the doc-comment explains *why* — the smoke catches the regression where the doc-comment is deleted entirely.
- The pre-existing `smoke_1_no_dep_leakage` test was already red on `HEAD~1` (futures was added to starter-spi in stage 1 without the `DOCS/tools/scope/starter-spi-deps.baseline.txt` update). Not introduced by this stage; deliberately left as-is to avoid mixing stages.

## Open questions

- (none)
