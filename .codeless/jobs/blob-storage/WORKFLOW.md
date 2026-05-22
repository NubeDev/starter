# Workflow — blob-storage

How to drive the stages in `template.yaml`. Read this before every
stage alongside `SCOPE.md` and the authoritative source SCOPE at
[/home/user/code/rust/starter/DOCS/storage/SCOPE.md](/home/user/code/rust/starter/DOCS/storage/SCOPE.md).

## Sequencing

Five stages, four REVIEW gates between them. Strictly linear:

- Stage 2 (memory + fs) cannot start until stage 1's `BlobStore`
  trait and `BlobError` enum are frozen — the engine impls hang
  off them.
- Stage 3 (s3 + garage) cannot start until stage 2's smoke
  fixtures exist — they are the trait test suite the production
  engines plug into.
- Stage 4 (compose) cannot start until at least two engines from
  stages 2–3 exist — combinators need real wrappable stores to
  exercise.
- Stage 5 (examples + smoke tests) is the integration layer; it
  builds on every prior stage.

The four REVIEW gates exist to confirm a hard rule before the
next stage builds on it: B1 (no domain leakage in engines), B2
(BlobRef opaque), B3 (combinators name their durability), Garage
AGPL boundary (no source-level Garage imports). A silent
violation at gate N cascades to expensive rework at stage N+2.

## Per-stage discipline

Before writing any code in a stage:

1. Re-read the corresponding section in the source SCOPE. The
   SCOPE text is the contract; this WORKFLOW is the process.
2. Re-read `SCOPE.md` §"In scope" and §"Out of scope". The
   biggest risk on this job is silent scope creep — the source
   SCOPE explicitly carves out many tempting features
   (transcoding, CAS, sharing, quotas, SSE). Stay within the
   carve-outs.
3. For stages that touch `starter-spi` (only stage 1 should):
   `grep -rln 'starter-spi'` in stage 1 to enumerate the 30+
   downstream crates, then
   `rg 'struct (Blob[A-Z]\w*|Etag|BackendId|PresignedUrl)\b'` in
   those crates to confirm zero collisions before the trait
   lands.
4. `cargo check --workspace` from the `starter` repo root before
   any edit so the baseline is known-clean.

Before committing a stage:

1. `cargo fmt --all` clean.
2. `cargo clippy --workspace --all-targets -- -D warnings` green.
3. `cargo test --workspace` green.
4. For stages 2, 3, 4: the engine/combinator passes the trait
   test suite. For stage 5: the five SCOPE smoke tests all pass.
5. Update `SCOPE.md` §"Deliverables" with `[x]` against anything
   completed in the stage.

Commit + push via **mani** from the codeless-workspace root:

```
./bin/mani --config mani.yaml run commit --projects starter \
  MSG='stage N: <one-line title from template.yaml>'
./bin/mani --config mani.yaml run push --projects starter
```

No `--force`, no `--no-verify`. If a hook fails, fix the cause.

## Closing trio — the last three todos of every stage

1. `checks` — `cargo fmt --check` + `cargo clippy --workspace
   --all-targets -- -D warnings` + `cargo test --workspace`.
   Stages 3+ also run the relevant docker-compose / smoke fixture.
2. `docs` — update `handover.md` for the next stage, tick the
   relevant `[x]` in `SCOPE.md` §"Deliverables", and (for stages
   that change a public-API decision) update
   `starter/DOCS/storage/SCOPE.md` to match in the same commit.
3. `git` — stage the changes, commit with `stage N: <title>`,
   push to `codeless/blob-storage`. One phase, one commit.

A stage is not "done" until all three are green.

## REVIEW gates

Four gates, one after every stage except stage 5. At each gate
write a handover comment containing:

- One bullet per item the gate is checking (taken from the
  template.yaml `REVIEW stage N` description).
- For gate 1 (after stage 1): the `BlobRef` opacity audit
  transcript (rustdoc check, `pub` field scan, attempted
  consumer-side raw-key extraction must fail to compile).
- For gate 2 (after stage 2): the presign round-trip transcript
  for both `-memory` and `-fs` (mint URL → GET via the same
  crate's axum router → bytes match → expired URL is `403`).
- For gate 3 (after stage 3): the docker-compose Garage boot
  log + the SwapTest transcript (one-line wiring change from
  `-fs` to `-garage`, consumer compiles unchanged).
- For gate 4 (after stage 4): the four-combinator nest test
  transcript (`Namespaced("tenant-7", Tiered(Fs::local,
  Garage::remote))`) plus the B3 rustdoc audit (every
  combinator names its durability mode).

Do not proceed past a REVIEW gate without explicit approval.

## Anti-patterns specific to this job

- **Do not** add a domain-shaped method to `BlobStore`. No
  `put_avatar`, no `get_attachment`, no `put_with_user_context`.
  B1 is non-negotiable; the trait surface is bytes-in /
  bytes-out plus the standard metadata.
- **Do not** expose a `pub fn key(&self) -> &BlobKey` on
  `BlobRef`. B2 is enforced at compile time; an accessor that
  hands back the inner key destroys the property. The opaque
  locator is `pub(crate)`, period.
- **Do not** add a `list_keys() -> impl Stream<Item = BlobKey>`
  method to the trait — even tempting "for debugging". A raw-key
  stream is the same B2 violation as a `key()` accessor on
  `BlobRef`.
- **Do not** make `Mirrored` default to `AsyncBackground`. B3
  binds: the **name** must carry the durability signal. If a
  consumer wants async, they type `Mode::AsyncBackground`.
- **Do not** fold `ReadThroughCache` into `Tiered` as
  `policy = AlwaysPromote`. Write semantics differ (cache
  populates lazily on read; tier writes to hot first). Folding
  would force a `WriteTarget` enum no current consumer needs.
- **Do not** import any AGPL Rust crate from Garage in
  `starter-blob-garage`. The boundary is over the wire (S3 API
  + Garage admin HTTP API). If the temptation arises in stage 3,
  halt and surface — the licensing posture is non-negotiable.
- **Do not** collapse `Unauthorized` and `Forbidden` in
  `BlobError` mapping. The SCOPE is explicit: `403` →
  `Forbidden`, `404` → `NotFound`, never the lazy "anything 4xx
  → Forbidden" shape. The distinction matters for surface area
  the consumer renders to operators.
- **Do not** emit tracing spans from `starter-spi` blob code.
  The observability contract puts emission in the engine crates
  under `starter_blob::<engine>` targets. A span inside `spi`
  forces every consumer to filter it; the engine boundary is the
  right place.
- **Do not** create `examples/blobs/` as a copy of
  `examples/minimal` with the blob bits bolted on. Read
  `examples/minimal`'s shape first and follow the same conventions
  (workspace layout, axum router style, sqlite wiring) so the
  walkthrough reads as a delta against minimal, not a parallel.
- **Do not** ship `docker/garage.example.toml` as a tuned-for-prod
  config. It is a *reference* single-node config for getting
  started. Comment any non-obvious knob with the SCOPE link;
  point operators at the Garage docs for prod tuning.
- **Do not** start stage 5 without all four prior REVIEW gates
  green. The smoke tests assume stages 1–4 hold the invariants
  they tested for; running them against a partial implementation
  produces false greens.

## When to halt

- The Q3 decision in `SCOPE.md` — combinator `list()` returning
  an outer `BlobRef` whose `opaque_locator` encodes the inner
  ref — turns out to have a hidden cost (e.g. `opaque_locator`
  outgrows reasonable size limits when combinators nest 3+
  deep). Surface in chat at the start of stage 4; the resolution
  is either a fixed-size locator with an internal mapping table
  or a redesign of the wrapping shape, both of which are above
  this job's authority.
- `docker-compose up` of the reference Garage cannot reach a
  healthy state on the CI runner. Stage 3 needs a green Garage
  to run integration tests against; surface in chat, do not
  paper over with mocks.
- A `starter-spi` collision audit (stage 1 first action) finds a
  downstream crate with a colliding `pub struct Blob*` name.
  Surface; the resolution is to rename the new symbol (e.g.
  `BlobStore` → `ObjectStore`) before any further code lands,
  not to silently shadow the downstream.
- The B1 audit (REVIEW gate 2) finds a domain-shaped method
  signature in `-memory` or `-fs`. Halt and rework. B1 is
  load-bearing; a violation at stage 2 cascades to every later
  engine.
- The Garage AGPL boundary audit (REVIEW gate 3) finds a direct
  Rust-source import from Garage. Halt and rework. The licence
  posture is non-negotiable.
