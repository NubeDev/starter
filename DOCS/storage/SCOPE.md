# starter blob storage — Scope

## One-line summary

A small family of Rust crates that gives a starter-based product
**reusable object/blob storage** — upload, download, list, delete,
presign — backed by pluggable engines (filesystem, in-memory,
S3-compatible, and [Garage] as the recommended self-hostable default),
that a consumer can pick one of, or **compose several together**
(namespacing, tiering, mirroring) without rewriting domain code.

It is the **blob** counterpart to `starter-store-sqlite` /
`starter-store-postgres`: those crates ship typed SQL building blocks
for *rows*; the crates scoped here ship a typed seam for *bytes*.

[Garage]: https://garagehq.deuxfleurs.fr/

## Why this exists

Every non-trivial product handles user-supplied bytes: avatars,
attachments, exports, snapshots, generated reports, model artifacts,
cached LLM outputs. Today each starter consumer re-rolls the same
plumbing:

- Pick a backend (local disk in dev, S3/R2 in prod).
- Re-implement the same five operations (`put`, `get`, `delete`,
  `list`, `presign`).
- Re-derive a key-naming convention.
- Re-discover the same edge cases (range reads, content-type
  round-trip, conditional writes, retention).
- Couple their domain code to a specific SDK, making the
  laptop-vs-prod swap painful.

Garage is the right default engine for self-hostable starter products:
S3-API-compatible, multi-node, geo-aware, runs on a Raspberry Pi as
happily as in a rack, written in Rust. But the *consumer* should not
be coupled to Garage — they should be coupled to a small trait that
any reasonable blob backend satisfies, with Garage being the one we
test against and ship a docker-compose recipe for.

**Licensing note.** Garage is AGPL-3.0, but starter consumers reach
it over its S3-compatible network API; the AGPL boundary stops at
the wire. No starter crate links Garage source, and `starter-blob-s3`
/ `starter-blob-garage` are MIT/Apache-2.0 like the rest of the
workspace. Consumer code stays under whatever licence the consumer
picks.

## Why a `BlobStore` trait exists in `starter-spi` (and why this is not a violation of R4)

[`SCOPE.md`](../../SCOPE.md) R4 forbids a universal `Store` trait for
**SQL** storage, because that surface has unbounded shape (joins,
custom queries, transactions, schema per consumer) and any attempt to
hide it pushes either SQL into starter or a fork onto the consumer.

Object storage is the opposite case. The industry has converged for
fifteen years on a narrow, uniform interface — `PUT key bytes`,
`GET key [range]`, `DELETE key`, `LIST prefix`, `presign key`. AWS
S3, GCS, Azure Blob, R2, B2, MinIO, SeaweedFS, and Garage all expose
that same shape. There is no equivalent of "the consumer wants a
custom JOIN" for blobs.

Therefore `starter-spi` exposes a `BlobStore` trait. It is the second
trait in `spi` whose contract is wide enough to abstract over multiple
backends without leaking implementation detail (the first being
`SecretStore`). Any operation that does *not* fit (lifecycle policies,
bucket creation, IAM, multi-region tuning) lives on the concrete
engine type, not on the trait.

## Relationship to existing storage crates

```
starter-spi                         (BlobStore, BlobKey, BlobRef, BlobMeta,
                                     BlobRange, PresignedUrl, BlobError)
   ↑
   ├── starter-blob-memory          (in-process; tests, examples)
   ├── starter-blob-fs              (local filesystem; dev, single-node deploys)
   ├── starter-blob-s3              (any S3-compatible endpoint; AWS, R2, MinIO, Garage-via-S3)
   ├── starter-blob-garage          (Garage-native; uses S3 API + Garage admin API for buckets/keys)
   ├── starter-blob-compose         (combinators: Namespaced, Tiered, Mirrored, ReadThroughCache)
   └── starter-blob-axum            (authenticated GET proxy handler; presign router shared with fs/memory)
```

The proxy crate is broken out so `starter-spi` and the engine crates
stay free of an `axum` dependency. Consumers who don't need a proxy
(e.g. CLI tools, batch jobs) skip the crate entirely.

`starter-store-sqlite` / `starter-store-postgres` are **unaffected**.
Blob bytes do not go in SQL; SQL rows reference blobs by `BlobRef`
(an opaque, stable handle the `BlobStore` minted). The two storage
families do not depend on each other.

## What ships in each crate

### `starter-spi` — additions

- `trait BlobStore: Send + Sync` with the canonical operations:
  - `put_bytes(key, Bytes, meta) -> BlobRef` — small/known-size bodies.
  - `put_stream(key, impl Stream<Item = Result<Bytes>>, meta) -> BlobRef`
    — large or unknown-size bodies. Every engine implements both; see
    the streaming decision under "Resolved design choices" below.
  - `get(BlobRef, Option<BlobRange>) -> impl Stream<Item = Result<Bytes>>`.
  - `head(BlobRef) -> BlobMeta`, `delete(BlobRef)`, `list(prefix, cursor)`,
    `presign(BlobRef, op, ttl) -> PresignedUrl`.
  - `copy_server_side(src: BlobRef, dst: BlobKey) -> Result<BlobRef,
    BlobError::Unsupported>` — never falls back silently. A separate
    free function `copy_via_client(&store, src, dst)` exists for the
    streamed-through-process case; the caller picks explicitly.
- `BlobKey` — validated path-like key (no `..`, no leading `/`,
  UTF-8, length-bounded). Constructor returns `Result<BlobKey, _>`.
  Used only at the *call site* of an operation.
- `BlobRef` — **opaque** durable handle minted by `put_*`. Internally
  carries `{ backend_id, opaque_locator, etag, size }`, but the
  locator is **not** exposed as a public field; only `backend_id()`,
  `etag()`, `size()`, and serde round-trip are public. Consumers
  persist `BlobRef` (e.g. as a SQL JSON column) and pass it back to
  `get` / `head` / `delete`. They cannot, by construction, extract
  the raw key and store *that*, which is what makes B2 enforceable
  rather than aspirational.
  - When a combinator wraps an engine, the `backend_id` on `BlobRef`
    is the **combinator's** ID, and the opaque locator encodes
    whatever the combinator needs to route the call (e.g. which
    mirror leg holds the canonical copy). Unwrapping the combinator
    invalidates outstanding `BlobRef`s — documented, migration-tool
    territory, not silently broken.
- `BlobMeta` — content-type, content-length, etag, last-modified,
  user-defined string→string metadata (capped, validated). The
  user-metadata map reserves three conventional keys that every
  engine and combinator round-trips unchanged, so a `BlobRef` is
  portable between consumers without each one inventing its own
  spelling:

  | Key            | Meaning                                          |
  | -------------- | ------------------------------------------------ |
  | `filename`     | Original client-supplied filename, UTF-8         |
  | `uploaded_by`  | Opaque consumer-defined principal id             |
  | `uploaded_at`  | RFC3339 timestamp                                |

  `starter-blob-axum`'s proxy handler reads `filename` to populate
  `Content-Disposition: attachment; filename="…"`. Consumers may add
  their own keys freely; the reserved set is documented in
  `starter-spi::blob::meta` as constants so call sites don't
  stringly-type them.
- `BlobError` — enum, exhaustively matched by engines:
  - `NotFound`
  - `Unauthorized` / `Forbidden` (distinct — leaking which one is a
    real concern; engines map S3 `403` to `Forbidden`, `404` to
    `NotFound`, and never collapse them)
  - `AlreadyExists`
  - `PreconditionFailed` (conditional-write failure)
  - `PayloadTooLarge`
  - `Throttled { retry_after: Option<Duration> }` (S3 `SlowDown`,
    Garage backpressure; consumer retry policy differs from generic
    backend error)
  - `Timeout`
  - `Unsupported` (e.g. `copy_server_side` across heterogeneous
    backends)
  - `Backend(BoxError)` — strictly the residual; engines must map
    known cases to the typed variants first.
- `PresignedUrl` — `{ method, url, expires_at, headers }`.
- `BlobRange` newtype for `GET` partial reads.

These are the only types that cross the seam. No SDK type, no
`aws_sdk_s3::*`, no `garage::*` leaks into `spi`.

**Observability contract.** `BlobStore` operations do **not** emit
their own spans or metrics from inside `spi`. Each engine crate
exposes a `tracing` target named `starter_blob::<engine>` and emits
one span per operation with stable fields (`op`, `backend_id`,
`size`, `outcome`). Metrics are opt-in via a `metrics` feature on
each engine that registers histograms under `starter_blob_*` names
that match `starter-observability` conventions. Combinators in
`starter-blob-compose` emit their *own* spans, child of the inner
engine's, so a `Tiered` read shows hot-miss → cold-hit as two nested
spans.

### `starter-blob-memory`

In-process `HashMap<BlobKey, Bytes>` behind a `RwLock`. Implements
`BlobStore`. Zero external deps. Used by every other crate's tests
and by `examples/minimal`. Presign returns a process-signed token
the same crate's `axum::Router` (feature-gated) honours, so the
presign contract is testable without a real S3. The HMAC key is
process-local random and rotates per process restart — outstanding
links die with the process, which is the correct behaviour for an
in-memory store.

### `starter-blob-fs`

`BlobStore` over a local directory. Atomic writes via
`tempfile-rename`. Conditional writes via `O_EXCL`. Listing via
`walkdir` with a configurable max-depth. Presign issues HMAC-signed
URLs that a feature-gated `axum` router in this crate serves; the
consumer mounts the router under whatever path they like.

**Presign key management.** The HMAC key is required at
construction (`Store::open(path, PresignKey)`); the crate never
generates one implicitly. Recommended source is `starter-spi`'s
`SecretStore` (`keyring` on developer machines, `file` on headless
servers), so links survive process restarts and rotate when the
operator rotates the secret. A `PresignKey::ephemeral()` constructor
is provided for tests and explicitly documents that links die with
the process.

Target use: single-node deployments, dev machines, CI.
**Non-goal:** durability across hosts. The crate's README says so in
the first paragraph.

### `starter-blob-s3`

`BlobStore` over any S3-compatible endpoint, built on
[`aws-sdk-s3`](https://crates.io/crates/aws-sdk-s3) with
`force_path_style` configurable (required for MinIO / Garage / many
on-prem deployments). Credentials sourced via the SDK's normal chain
**or** via `starter-spi::SecretStore` for keys-at-rest hygiene.

This crate is sufficient by itself to talk to Garage over its S3
API. It is the "lowest-common-denominator" engine.

### `starter-blob-garage`

A thin layer on top of `starter-blob-s3` that adds Garage-specific
operations the plain S3 API does not expose:

- Bucket lifecycle via Garage's admin API (`POST /v1/bucket`,
  `DELETE /v1/bucket/{id}`).
- Per-bucket access-key minting (so a consumer can hand each tenant
  their own key without sharing the root one).
- Cluster health (`GET /v1/health`) surfaced as a typed status the
  consumer can wire into `starter-observability` readiness checks.
- Layout introspection (replication factor, zone awareness) for
  startup logging.

A consumer who does not need bucket/key provisioning at runtime can
skip this crate entirely and use `starter-blob-s3` against their
Garage cluster.

### `starter-blob-compose`

The "group them together" crate. Each combinator implements
`BlobStore` and wraps one or more inner `BlobStore`s. None of them
adds SQL; none of them adds a new trait.

- **`Namespaced { inner, prefix }`** — prepends a key prefix on every
  operation. `list(prefix)` queries the inner store with
  `combined = self.prefix + prefix` and **strips `self.prefix`** from
  every returned entry before yielding to the caller, so the consumer
  observes a clean view rooted at their namespace. The stripped form
  is the only form a consumer ever sees; the unprefixed `BlobRef`s
  returned by `list` are routed through the combinator on subsequent
  `get` calls, preserving B2.
- **`Tiered { hot, cold, policy }`** — write to `hot`, demote to
  `cold` per `policy` (size / age / on-eviction). Read tries `hot`
  first, falls back to `cold` and (optionally) promotes back.
- **`Mirrored { primary, mirrors, mode }`** — `mode = Sync`
  fails the write if any mirror fails; `mode = AsyncBackground`
  returns on primary success and best-effort fans out. Read always
  hits `primary` first.
- **`ReadThroughCache { source, cache, ttl }`** — kept separate from
  `Tiered` rather than folded in as `policy = AlwaysPromote` because
  its write semantics differ: it writes only to `source` and lets
  `cache` populate lazily on read, whereas `Tiered` writes to `hot`
  first. Conflating them would force a `WriteTarget` enum on `Tiered`
  that no current consumer needs.

Combinators are themselves `BlobStore`s, so they nest: a consumer can
build `Namespaced("tenant-7", Tiered(Fs::local, Garage::remote))` and
hand it to any code that takes `impl BlobStore`.

### `starter-blob-axum`

An optional crate that ships the **authenticated GET proxy** every
consumer would otherwise write themselves. Presign covers the
direct-PUT-from-browser case; it is the wrong primitive for inline
content (e.g. images embedded in a markdown body persisted in SQL):

- A markdown row is rendered at arbitrary times; presigned image
  URLs with a TTL would have to be refreshed on every render or
  rewritten into the body on every edit (lossy round-trip).
- Per-request auth is the right model: the GET handler decides
  whether *this* viewer can see *this* `BlobRef` based on the
  enclosing domain object's ACL — which only the consumer knows.

The crate exposes:

```rust
pub fn blob_proxy_handler<S: BlobStore + 'static>(
    store: Arc<S>,
    authz: impl Fn(&BlobRef, &BlobContext, &Request) -> Result<(), BlobError>
        + Send + Sync + 'static,
) -> axum::Router
```

`BlobContext` is a small struct the combinator stack populates as
the request is routed: it carries the **parsed namespace prefix**
(e.g. `Some("project-7")` for a `Namespaced` wrapper) alongside the
`BlobRef`. This is load-bearing: without it the consumer's authz
closure would have to re-parse the prefix out of an opaque
`BlobRef`, leaking the namespace scheme into auth code and
undermining B1/B2. With it, the authz closure receives a structured
value and stays domain-clean.

The handler is responsible for:

- Parsing `BlobRef` from the URL using the serde round-trip already
  defined in `starter-spi`.
- Resolving `BlobContext` by walking the combinator stack on `store`.
- Calling the consumer-supplied `authz` closure.
- Mapping `BlobError` variants to HTTP status codes per the table
  below — uniform across consumers, never re-derived:

  | `BlobError` variant     | HTTP                                  |
  | ----------------------- | ------------------------------------- |
  | `NotFound`              | 404                                   |
  | `Unauthorized`          | 401                                   |
  | `Forbidden`             | 403                                   |
  | `PreconditionFailed`    | 412                                   |
  | `PayloadTooLarge`       | 413                                   |
  | `Throttled { retry_after }` | 503 + `Retry-After` header        |
  | `Timeout`               | 504                                   |
  | `Unsupported`           | 501                                   |
  | `Backend(_)`            | 500                                   |

- Forwarding `Range`, `If-None-Match`, `If-Modified-Since`, and
  `Accept-Encoding` end-to-end where the engine supports it.
- Reading the reserved `filename` user-metadata key to populate
  `Content-Disposition: attachment; filename="…"` when the client
  sets `?download=1`.

The presign router that `starter-blob-fs` and `starter-blob-memory`
already ship moves here too, so all `axum` integration lives in one
crate and consumers wire one `Router` per surface.

The handler takes consumer authz as a closure; it knows nothing
about domain entities. B1 stays intact.

### React: `useBlobUpload` (in `@nube/starter-ui-core`)

The scope previously said this hook would be specified in a separate
doc that did not yet exist. The surface is locked here so the first
consumer doesn't have to invent it (and so future consumers don't
each re-roll a divergent version that leaks raw keys into
user-visible content):

```ts
function useBlobUpload(opts: {
    presignEndpoint: string;            // POST → { url, headers, ref }
    onUploaded: (ref: BlobRef, meta: BlobMeta) => void;
    maxBytes?: number;
    acceptedTypes?: string[];
}): {
    upload: (file: File) => Promise<BlobRef>;
    progress: number | null;
    error: Error | null;
};
```

Plus a markdown-editor adapter that composes with the hook so any
`@uiw/react-md-editor` (or tiptap, or codemirror) instance gets
paste-image / drop-image / toolbar-upload behaviour with one prop:

```ts
const onImageUpload = useBlobUploadForMarkdown({
    presignEndpoint,
    proxyUrlFor,   // (ref: BlobRef) => string — typically /api/blobs/{ref}
});
<MDEditor ... onImageUpload={onImageUpload} />
```

The adapter **always** writes `![](proxyUrlFor(ref))` into the
markdown body — never an engine-specific URL, never a presigned
URL, never a raw key. This is what makes a later
`Namespaced`/`Tiered` swap non-breaking for content already stored
in markdown rows. The hook is taxonomy-agnostic: it does not know
what a "project" or "user" is — the consumer's presign endpoint is
what binds an upload to a domain object.

## Choosing isolation — `Namespaced` vs Garage per-bucket keys

`starter-blob-compose::Namespaced` and `starter-blob-garage`'s
per-bucket access-key minting both isolate one consumer-defined
scope from another, but they sit at very different costs. Pick by
the **trust boundary** between scopes, not by the number of scopes:

| Need                                       | Use                                  |
| ------------------------------------------ | ------------------------------------ |
| Multi-scope, single trust boundary (e.g. multi-project SaaS where the app server is the only thing that can reach the bucket) | `Namespaced("scope-<id>", store)`    |
| Multi-tenant, separate trust boundaries (each tenant gets credentials they could in principle use directly against the engine) | Garage per-bucket key minting        |
| Multi-tenant, hosted on shared S3          | `Namespaced` + IAM bucket policy     |

The default for a starter consumer is row 1. Reach for row 2 only
when a tenant must be able to hold their own credential against the
engine — most consumers never do.

**On `list` and B2.** `list` returns `(BlobRef, BlobMeta)` pairs,
not raw keys. Combinators rewrite the `BlobRef`s on the way out the
same way they rewrite them on `put`, so a `list` result from a
`Namespaced(Tiered(...))` is routable back through the same
combinator stack with no consumer-visible key. A free-form
`list_keys()` that yields strings is deliberately not on the trait.

## Hard rules

"Load-bearing" in the parent [`SCOPE.md`](../../SCOPE.md) means: if
you break the rule, the modularity story collapses and downstream
consumers stop being able to mix-and-match. These three clauses
extend R1–R8 with the storage-specific invariants that make "swap
Garage for S3" or "wrap two stores into a tier" stay a one-line
change.

### B1 — A `BlobStore` knows nothing about its consumer's domain

No `put_avatar`, no `get_attachment`. The trait surface is
`put(key, bytes, meta) -> BlobRef`. Domain repositories are built by
the consumer on top.

### B2 — The `BlobRef` is the only handle that crosses time

The `BlobRef` type is opaque: no public `key` accessor, no
`Display`/`Debug` impl that round-trips to a usable string. Consumers
persist it as JSON (serde-stable) and pass it back. They *cannot*,
by construction, store a raw key and route around the combinator.
This turns the no-raw-keys rule from a code-review request into a
compile-time fact, which is what makes adding a `Namespaced` or
`Tiered` wrapper later a non-breaking change.

### B3 — No combinator silently changes durability semantics

`Mirrored::AsyncBackground` is opt-in and its name says so.
`ReadThroughCache` is read-only-ish: cache misses never block writes,
and the cache is never the source of truth on read after delete.
Every combinator documents its failure modes in the type-level
rustdoc, not in a separate guide.

## What does NOT ship here (non-goals)

- **No content-addressed storage / CAS layer.** A consumer who wants
  CAS hashes the body themselves and uses the hash as the `BlobKey`.
- **No transcoding, no thumbnailing, no virus scanning.** Those are
  domain concerns and belong in the consumer or in a dedicated crate
  outside the storage family.
- **No upload widget in `ui-kit`.** The React side gets a
  `useBlobUpload` hook in `@nube/starter-ui-core` whose surface is
  locked above under §"React: `useBlobUpload`". The hook calls
  `presign` and `PUT`s directly to the backend; no styled widget,
  picker, or drop-zone is shipped — the consumer composes those
  themselves.
- **No file-share / public-link feature.** Presigned URLs are the
  primitive; sharing is a product decision.
- **No Garage-cluster orchestration.** `docker/` will gain a
  reference `garage.toml` and a compose service, but starter does not
  manage cluster membership, layout changes, or backups.

## Smoke tests (this scope succeeds iff…)

1. **Swap test.** A consumer changes a single line in their wiring
   from `starter_blob_fs::Store::open(path)` to
   `starter_blob_garage::Store::connect(cfg)` and every domain
   function **compiles** and behaves identically *for the operations
   defined by the trait*. Engine-specific realities (eventual
   consistency, S3 multipart minimums, rate-limit shapes) are not
   abstracted; they surface as typed `BlobError` variants the
   consumer can react to uniformly.
2. **Compose test.** Wrapping any engine in `Namespaced` or `Tiered`
   requires no change to `BlobRef` consumers and no SQL migration.
3. **Test-without-network test.** A consumer can run their full
   integration suite against `starter-blob-memory` with no
   feature flag gymnastics.
4. **Cost-to-skip test.** A consumer who never touches blobs pays
   zero — none of these crates is a transitive dep of `starter-server`
   or `starter-spi`'s default feature set.
5. **R8 test.** Every public item in `starter-spi`'s blob additions
   has a doc-comment that explains *why* the shape is what it is,
   especially the `BlobRef`-vs-`BlobKey` split.

## Repo layout (additions)

```
crates/
  starter-blob-memory/
  starter-blob-fs/
  starter-blob-s3/
  starter-blob-garage/
  starter-blob-compose/
  starter-blob-axum/

docker/
  garage.example.toml           <- reference single-node config
  docker-compose.garage.yml     <- compose service + healthcheck

examples/
  blobs/                        <- minimal: axum route that accepts
                                   an upload, persists a BlobRef in
                                   sqlite, serves a presigned GET.
```

## Resolved design choices (in 0.1)

- **Streaming is on the trait, not a super-trait.** Report exports,
  model artifacts, and any non-trivial attachment will outgrow
  `Bytes`. Adding `put_stream` after the fact is a breaking change
  for every engine impl. 0.1 ships `put_bytes` and `put_stream` as
  peer methods; `fs` and `memory` implement `put_stream` by
  buffering, `s3` and `garage` implement it as multipart.
- **Conditional writes are on `put_*`.** `PutOptions::if_none_match`
  / `if_match` map to S3/Garage conditional headers; `fs` enforces
  via `O_EXCL` + etag compare; `memory` is trivial. Failure surfaces
  as `BlobError::PreconditionFailed`.
- **`copy` never silently falls back.** `copy_server_side` errors
  with `BlobError::Unsupported` when the engine (or a heterogeneous
  combinator pair) cannot do it natively; the free function
  `copy_via_client` is the explicit opt-in for streamed copy.

## Quotas / per-namespace accounting (shipped)

Driven by dev-pulse: per-project byte caps prevent one noisy
project from exhausting an org-wide budget.

Shape:

- `BlobUsage { bytes: u64, objects: u64 }` in `starter-spi::blob`.
- `BlobStore::approximate_usage(prefix: &BlobKey) ->
  Result<BlobUsage, BlobError>` with a default impl that returns
  `BlobError::Unsupported`. The word "approximate" is in the name
  deliberately: `fs` and `memory` answer authoritatively;
  `s3`/`garage` will answer from list/inventory and may lag (not
  yet implemented, tracked as 0.2 follow-up).
- `Namespaced::with_quota(Quota { max_bytes, max_objects })`.
  `put_*` exceeding the cap returns `BlobError::PayloadTooLarge`.
- Counter authority is engine-defined. `Namespaced` does **not**
  maintain its own counter — it asks the inner engine via
  `approximate_usage` on every write, so there is one source of
  truth per deployment.

Known limits (documented at `Quota`):

- Pre-flight race window: two concurrent writers can both pass the
  check and overshoot by one write. Closing this would require a
  global lock against the inner store, which is the wrong cost
  shape for a noisy-neighbour deterrent.
- `put_stream` does not know body length up-front, so the
  pre-flight only refuses a namespace that is already over the
  byte cap. Streaming bytes that cross the cap mid-flight are
  admitted.

## Planned for 0.2

- `approximate_usage` on `starter-blob-s3` and `starter-blob-garage`
  (list-/inventory-based; document the lag at the call site).

## Open questions (decide before 0.1)

- **Server-side encryption.** Engine-level config or `SecretStore`
  hook? Leaning engine-level: keys are an ops concern, not a domain
  one.
