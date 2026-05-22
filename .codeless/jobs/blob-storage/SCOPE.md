# Scope — blob-storage

The authoritative design lives at
[`/home/user/code/rust/starter/DOCS/storage/SCOPE.md`](/home/user/code/rust/starter/DOCS/storage/SCOPE.md).
This brief is the trimmed per-job scope. Where this disagrees with the
source SCOPE, **the source SCOPE wins** — fix this file rather than
diverge.

## Goal

Land the starter blob-storage capability end-to-end on the `starter`
repo via the `codeless/blob-storage` branch. After this job:

1. `starter-spi` exposes `BlobStore`, `BlobKey`, `BlobRef`,
   `BlobMeta`, `BlobRange`, `PresignedUrl`, `BlobError` — additive
   only, no existing symbol churn.
2. Five new engine crates ship: `starter-blob-memory`,
   `starter-blob-fs`, `starter-blob-s3`, `starter-blob-garage`,
   `starter-blob-compose`.
3. `docker/garage.example.toml` and `docker/docker-compose.garage.yml`
   provide the reference Garage stand-up for CI and operator setup.
4. `examples/blobs/` runs a minimal axum upload → SQLite-`BlobRef`
   → presigned-GET round-trip end-to-end.
5. All five SCOPE smoke tests pass (Swap, Compose, TestWithoutNetwork,
   CostToSkip, R8).
6. The three hard rules B1–B3 hold by construction (no domain
   leakage in any engine's public API; `BlobRef` opaque with no
   public locator accessor; no combinator silently changes
   durability).

## In scope (five stages mirroring the SCOPE structure)

- **Stage 1 — `starter-spi` additions.** `BlobStore` trait,
  `BlobKey`, `BlobRef` (opaque), `BlobMeta`, `BlobRange`,
  `PresignedUrl`, the full typed `BlobError` enum, the
  `copy_via_client` free function. Observability contract
  documented (engines emit under `starter_blob::<engine>` targets;
  spi-level code emits nothing).
- **Stage 2 — `starter-blob-memory` + `starter-blob-fs`.** The
  test/dev engines. Both ship feature-gated axum routers that
  honour their own presigned URLs so the presign contract is
  testable without a real S3.
- **Stage 3 — `starter-blob-s3` + `starter-blob-garage`.** The
  production engines. S3 wraps `aws-sdk-s3` with `force_path_style`
  configurable and multipart `put_stream`. Garage layers bucket
  lifecycle + per-bucket key minting + cluster health + layout
  introspection on top. `docker/garage.example.toml` and
  `docker-compose.garage.yml` land here.
- **Stage 4 — `starter-blob-compose`.** The four combinators:
  `Namespaced`, `Tiered`, `Mirrored`, `ReadThroughCache`. All
  implement `BlobStore`, all nest, `list()` returns
  `(BlobRef, BlobMeta)` pairs not raw keys.
- **Stage 5 — `examples/blobs/` + the five SCOPE smoke tests
  assembled at workspace level.**

## Out of scope

- **Content-addressed storage.** A consumer that wants CAS hashes
  the body and uses the hash as `BlobKey`. No CAS layer in this
  family.
- **Transcoding / thumbnailing / virus scanning.** Domain concerns;
  belong in the consumer or a dedicated crate outside the storage
  family.
- **Upload UI.** `useBlobUpload` ships separately under
  `@nube/starter-ui-core`; no widget in `ui-kit`.
- **File-share / public-link feature.** Presigned URLs are the
  primitive; sharing is a product decision.
- **Garage cluster orchestration.** `docker/` ships a reference
  `garage.toml` + compose; starter does not manage cluster
  membership, layout changes, or backups.
- **A universal `Store` trait for SQL.** The parent SCOPE's R4
  forbids it; this job ships `BlobStore` only.
- **`list_keys()` returning strings.** Deliberately not on the
  trait — would let consumers route around combinators (violates
  B2). `list()` returns `(BlobRef, BlobMeta)` pairs.
- **Server-side encryption configuration.** Open question in the
  source SCOPE; leaned engine-level but not landing this job.
  Engines may surface their backend's native SSE config on the
  concrete type, but no SPI-level SSE abstraction.
- **Per-tenant quotas / accounting.** Out of scope for 0.1 per
  the source SCOPE; the `Namespaced` combinator is the natural
  place to add it later.

## Constraints

- **B1** — `BlobStore` knows nothing about its consumer's domain.
  No `put_avatar`, no `get_attachment`, ever. The trait surface
  is `put(key, bytes, meta) -> BlobRef` and domain repositories
  live in consumer code.
- **B2** — `BlobRef` is the only handle that crosses time, and
  it is **opaque**: no public `key` accessor, no `Display` /
  `Debug` impl that round-trips to a usable string. Compile-time
  fact, not a code-review guideline.
- **B3** — No combinator silently changes durability semantics.
  `Mirrored::AsyncBackground` is opt-in and its **name** says so.
  `ReadThroughCache` is never the source of truth on read after
  delete. Type-level rustdoc documents every failure mode.
- **starter R4** — no universal SQL `Store` trait. This job ships
  `BlobStore` (the *second* trait in `spi` whose contract is wide
  enough across backends — first was `SecretStore`).
- **starter R8** — every public item in `starter-spi`'s blob
  additions has a doc-comment explaining *why* the shape is what
  it is, not just *what* it is.
- **Licensing** — Garage is AGPL-3.0 but starter consumers reach
  it over its S3-compatible network API. No starter crate links
  Garage source; `starter-blob-s3` and `starter-blob-garage` are
  MIT/Apache-2.0. Verify in stage 3 that no `garage::*` Rust crate
  is imported.
- **MSRV / lint gates** — `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check` all green at every stage boundary.

## Deliverables (what "done" looks like)

1. `codeless/blob-storage` branch with one commit per stage, pushed
   via mani.
2. `cargo test --workspace` green at every stage boundary.
3. `cargo clippy --workspace --all-targets -- -D warnings` green
   at every stage boundary.
4. `cargo fmt --check` green at every stage boundary.
5. The five SCOPE smoke tests pass as workspace-level integration
   tests in stage 5:
   - **SwapTest** — one-line wiring change from `starter_blob_fs`
     to `starter_blob_garage`, consumer code unchanged.
   - **ComposeTest** — wrapping any engine in `Namespaced` /
     `Tiered` requires no change to `BlobRef` consumers and no
     SQL migration.
   - **TestWithoutNetwork** — full integration suite runs against
     `starter-blob-memory` with no feature-flag gymnastics.
   - **CostToSkip** — `cargo tree` shows none of these crates as a
     transitive dep of `starter-server`'s default feature set.
   - **R8** — every public item in `starter-spi`'s blob additions
     has a *why* doc-comment.
6. `examples/blobs/` runs and serves a presigned GET against the
   SQLite-persisted `BlobRef` in the example flow.
7. `docker-compose -f docker/docker-compose.garage.yml up` brings
   up a healthy Garage node that stage-3 integration tests pass
   against.
8. The job's own `SCOPE.md` and the source `DOCS/storage/SCOPE.md`
   stay in sync (any decision drift edits both files in the same
   commit).

## Open questions — RESOLVED (2026-05-22, before start)

The source SCOPE has two open questions; both are out of scope for
this job as written. Three job-specific resolutions follow.

### Q1 — Scope realism: is this one job?

**Answer: Yes, scope-wise — but it is materially bigger than the
default cost cap covers.**

Unlike the insights capability, this scope decomposes cleanly into
five independent stages with sharp seams between them (spi → 2
test engines → 2 prod engines → combinators → examples + smoke).
Each stage compiles standalone; later stages depend on earlier
stages' types but not on their implementation details. That makes
this a legitimate single-job grind, not a "really three jobs
pretending to be one" like insights.

**Decision.**
1. Submit as one job covering all five stages.
2. Cap at **30000¢ / 4h**, same as insights Phase 1. Stage 1
   (spi additions, no SDK work) and stage 4 (combinators, pure
   Rust over an existing trait) are cheap. Stage 2
   (`-memory` + `-fs`) is medium. Stages 3 (`-s3` + `-garage` +
   docker recipe + CI integration tests against a live container)
   and 5 (workspace smoke fixtures) are the expensive ones.
3. **If a stage cannot complete inside cap**, halt at the prior
   REVIEW gate, mark the stage `[!]` in `SCOPE.md`, and split off
   that stage and the remainder into a follow-up
   `blob-storage-stage-N+` job. Do **not** silently land a
   partial stage.
4. The four REVIEW gates between stages are non-skippable: each
   gate confirms a hard rule (B1 / B2 / B3 / Garage AGPL boundary
   / observability contract) before the next engine builds on it.

### Q2 — `BlobRef` opacity: how is it enforced at compile time?

**Answer: `BlobRef` is a struct with all fields `pub(crate)`, no
`pub fn key(&self)`, no `Display`/`Debug` that round-trips to a
usable locator. Serde is the only round-trip seam and it
round-trips through the *opaque* JSON shape, not the internals.**

Concrete shape (stage 1 freezes this):

```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct BlobRef {
    pub(crate) backend_id: BackendId,
    pub(crate) opaque_locator: String,
    pub(crate) etag: Etag,
    pub(crate) size: u64,
}

impl BlobRef {
    pub fn backend_id(&self) -> &BackendId { &self.backend_id }
    pub fn etag(&self) -> &Etag { &self.etag }
    pub fn size(&self) -> u64 { self.size }
}
```

No `pub fn locator`, no `pub fn key`, no `impl Display`,
`Debug` impl prints `BlobRef { backend_id, etag, size, .. }`
(omits `opaque_locator`). Serde serialises the full shape
because consumers persist it as a JSON column; deserialise
re-hydrates verbatim. The `pub(crate)` field is reachable inside
`starter-spi` only — engine crates **read** it via a
crate-internal helper trait `BlobRefInternal` that engines
import from `starter-spi`, not via field access. (Stage 1
defines this helper trait alongside the type.)

A consumer cannot, by construction, recover the raw key from a
`BlobRef`. B2 is a compile-time fact.

### Q3 — Combinator `BlobRef` rewriting: how does `list` stay routable?

**Answer: combinators rewrite the `BlobRef` on the way *out* the
same way they rewrite it on `put` and `get`. A `BlobRef` returned
from a combinator's `list()` has `backend_id = <combinator's
id>` and an `opaque_locator` that encodes the routing the
combinator needs to dispatch a subsequent `get`/`head`/`delete`
to the right inner store.**

Two concrete cases the source SCOPE calls out, with the
mechanic spelled here so stage 4 has no decision to make:

- **`Namespaced { inner, prefix }`** — on `put`/`list`/`get`:
  the inner store sees `combined = self.prefix + caller_key`
  and mints a `BlobRef` with `backend_id = inner.id`,
  `opaque_locator = encode(combined)`. The `Namespaced`
  combinator wraps that inner `BlobRef` into an outer
  `BlobRef` with `backend_id = Namespaced.id` and
  `opaque_locator = encode((inner_locator, self.prefix.len()))`
  so a subsequent `get(outer_ref)` strips the wrapper, recovers
  the inner ref, and forwards. `list(prefix)` returns outer
  `BlobRef`s with the prefix-stripped keys visible only through
  the trait's `BlobMeta` projection (and `BlobMeta` does not
  expose `BlobKey` to the consumer — see B2).

- **`Tiered { hot, cold, policy }`** — `put` writes to hot,
  mints an outer `BlobRef` whose `opaque_locator =
  encode((Tier::Hot, hot_inner_ref))`. On read, the combinator
  decodes, dispatches to hot first, falls back to cold (and the
  `opaque_locator` is updated in-place if/when a promote-back
  occurs — the **returned** `BlobRef` from `get` is documented
  to be advisory and consumers should re-`head` if they need
  the canonical location, **OR** the combinator promises the
  outer ref stays stable across promote events and uses an
  internal mapping table to track migrations; **stage 4
  resolves which approach by writing the doc-comment
  first**, then implementing).

Free-form `list_keys()` is deliberately not on the trait — it
would let a consumer pull out raw strings and route around the
combinator, which destroys B2.

## References

- Source SCOPE (authoritative):
  [/home/user/code/rust/starter/DOCS/storage/SCOPE.md](/home/user/code/rust/starter/DOCS/storage/SCOPE.md)
- Parent starter SCOPE: `starter/DOCS/SCOPE.md` (R4 — no universal
  Store; R8 — explain why in doc-comments)
- `starter-spi` module layout (stage 1 ground truth):
  [/home/user/code/rust/starter/crates/starter-spi/src/](/home/user/code/rust/starter/crates/starter-spi/src/)
- Existing engine pattern for reference: `starter-secrets-keyring`
  / `starter-secrets-file` (the first trait in spi wide enough to
  abstract over backends; `BlobStore` is the second).
- Examples dir pattern: `starter/examples/minimal` already exists;
  `starter/examples/blobs` follows the same shape.
