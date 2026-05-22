## Done

- Added `crates/starter-blob-s3/` wrapping `aws-sdk-s3` 1.x: `S3BlobStore` impl of `BlobStore` with `force_path_style` configurable, two constructors (`open` for SDK credential chain, `open_with_credentials` for `SecretStore`-sourced creds via `S3Credentials`), multipart `put_stream` (single-shots streams that fit in 8 MiB; otherwise chunks with abort-on-failure), and `copy_server_side` (within-bucket only; cross-engine returns `Unsupported` per B3).
- Wired conditional writes: `PutOptions::if_absent` → `If-None-Match: *`; server 412 re-mapped to `BlobError::AlreadyExists` so the trait contract holds. `if_match` against a specific ETag is documented as a roadmap item (PutOptions is `#[non_exhaustive]`, additive later) — see README. Not adding it touched SPI in this stage.
- Error mapping module (`src/error.rs`): 404→NotFound, 403→Forbidden, 401→Unauthorized (never collapsed), 409→AlreadyExists, 412→PreconditionFailed, 413→PayloadTooLarge, 429/503→Throttled with `retry_after` parsed from `Retry-After` (capped at 300 s), SDK `TimeoutError`→Timeout, residual→`Backend(SdkErrorWrapper { op, .. })`. Internal `HasStatus` trait abstracts status/header extraction across SDK response types; `()` impl exists for synthetic timeouts used in unit tests.
- Added `crates/starter-blob-garage/`: `GarageBlobStore` delegates the data-plane `BlobStore` impl to `S3BlobStore`; `GarageAdmin` is a reqwest+rustls client over the v1 admin API covering `POST /v1/bucket`, `GET /v1/bucket?globalAlias=…`, `DELETE /v1/bucket/{id}`, `POST /v1/key`, `POST /v1/bucket/allow`, `GET /v1/health` (typed `ClusterStatus`: Healthy/Degraded/Unavailable/Unknown + connected node count + storage_used_pct), `GET /v1/layout` (typed `LayoutInfo`: version, node_count, raw JSON). `GarageBlobStore::open` probes health + layout at construction so a misconfigured cluster fails at startup rather than first put.
- `docker/garage.example.toml` (single-node reference config with placeholder tokens) + `docker/docker-compose.garage.yml` (Garage v1.0.1 container + `garage-init` one-shot that lays out the cluster, creates bucket `starter-test`, mints key `starter-test-key`, grants permissions, prints credentials).
- READMEs in both crates with the AGPL boundary note: starter-side is MIT/Apache-2.0; Garage is reached only over the HTTP wire; verify isolation with `cargo tree -p starter-blob-s3 | grep -i garage` (empty).
- Workspace members + workspace.dependencies extended; `cargo build --workspace` and `cargo clippy -p starter-blob-s3 -p starter-blob-garage --all-targets -- -D warnings` both green; `cargo test -p starter-blob-s3 -p starter-blob-garage` passes 4 unit tests + 2 compile-only swap fixtures.
- Integration tests for both crates live under `tests/integration.rs` gated by the `integration-tests` feature; they read `STARTER_S3_*` / `STARTER_GARAGE_*` env vars and `eprintln!("...skipping")` when unset so a partial config does not pretend to pass.

## Next

- Stage 4: `starter-blob-compose` with `Namespaced` / `Tiered` / `Mirrored` / `ReadThroughCache`. The TODO markers in the new swap-test fixtures reference this stage already. Stage 4 should also revisit whether `if_match: Option<Etag>` lands on `PutOptions` — the s3 README flags it as a roadmap item.
- Stage 5: workspace-level smoke tests (Swap / Compose / TestWithoutNetwork / CostToSkip / R8) + `examples/blobs/` round-trip.

## What you need to know

- **Toolchain.** `aws-sdk-s3` 1.x latest requires rustc 1.91+ (transitive crates pin it). The worktree's default toolchain was 1.90; I installed 1.91 via rustup and `cargo build --workspace` is green under `RUSTUP_TOOLCHAIN=1.91`. The workspace `rust-version = "1.80"` declaration was left intact — it is an MSRV hint, not an enforcer, and CI uses `dtolnay/rust-toolchain@stable` which now resolves to 1.91+. If a future stage needs strict 1.80 MSRV the s3 crate would need to pin older `aws-sdk-s3` sub-crates with `[patch]`.
- **PutOptions extension.** I deliberately did **not** add `if_match` / `if_none_match` fields to `PutOptions` despite the stage brief mentioning them, because that would re-open stage 1's SPI freeze. `if_absent` covers the `If-None-Match: *` case end-to-end. Stage 4 (or a separate review) can decide whether finer-grained conditions land as additive `#[non_exhaustive]` fields.
- **Multipart chunk size.** Hardcoded to 8 MiB in `MULTIPART_CHUNK`; S3 minimum is 5 MiB. Configurable on a future pass — left as a doc-commented constant rather than a `S3BlobStoreConfig` field to keep the config struct small.
- **`copy_server_side`.** Within-bucket only on the S3 engine; cross-engine deliberately returns `Unsupported` so callers reach for `copy_via_client` explicitly (B3). Returned `BlobRef` re-uses `head()` for the size; the trip is needed because `CopyObject` does not return Content-Length.
- **Health probe at startup.** `GarageBlobStore::open` refuses to construct when health reports `Unavailable`. Other statuses (Degraded / Unknown) construct successfully so a partially-down cluster is still usable for reads.
- **TLS.** `reqwest` and `aws-sdk-s3` are configured with `rustls` only (no native-tls) to match the workspace TLS choice.
- **Garage Rust crate isolation.** Verified by `cargo tree -p starter-blob-garage` — no `garage::*` crate appears. The compose mounts the upstream Garage binary inside a container; starter never links it.

## Open questions

- Whether to extend `PutOptions` with `if_match` (and possibly `if_none_match: Option<Etag>`) in stage 4 or leave conditional control where it is. Trade-off noted in `crates/starter-blob-s3/README.md`.
- Whether `GarageAdmin::allow_key` should expose finer-grained permissions (currently grants read+write, not owner). Punt to whoever ships the consumer that needs read-only keys.
- Whether the workspace should add a `.rust-toolchain.toml` pinning 1.91 now that `aws-sdk-s3` requires it transitively. Out of scope for this stage but worth raising on the next REVIEW gate.
