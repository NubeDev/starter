# `examples/blobs/` — BlobStore + SQLite + presigned GET

Minimal axum app demonstrating the canonical blob-storage shape
that `DOCS/storage/SCOPE.md` settles on:

1. A consumer accepts an upload, hands the bytes to a
   `BlobStore` (`Arc<dyn BlobStore>` — never a concrete engine
   type), and persists the returned `BlobRef` in SQLite alongside
   a domain row (here: an `attachment` row carrying just `id`,
   `name`, `blob_ref_json`).
2. A subsequent fetch reads the `BlobRef` back from SQLite, calls
   `BlobStore::presign(.., PresignOp::Get, ttl)`, and hands the
   URL to the client. The client retrieves the bytes from the
   engine's own router — no proxy hop through the application.

The example wires `starter-blob-memory` so it stands up with no
external services and `cargo run -p starter-blobs-example` works
on a fresh checkout. The point is the trait surface: a deployment
swaps to `starter-blob-fs` or `starter-blob-garage` by changing
one constructor call and one feature flag in `Cargo.toml` — the
domain code below `BlobStore` does not move (this is the
**SwapTest** smoke fixture in `crates/smoke-tests/tests/`).

## Run

```
cargo run -p starter-blobs-example
```

The binary opens an in-process SQLite at `sqlite::memory:` (so the
example needs no migrations on disk), spins the memory engine and
its presign router mounted at `/blobs`, and binds the upload /
fetch routes on `127.0.0.1:8090` by default. Override with
`STARTER_BIND_ADDR=…`.

## Wire shape

- `PUT /attachments/:name` with the body as the blob bytes →
  inserts a row, returns `{"id":N}`.
- `GET /attachments/by-id/:id` → returns
  `{"name":..,"presigned_url":..,"expires_at_unix":..}`. The
  consumer follows `presigned_url` to fetch the bytes.
- `GET /blobs?token=…` → the engine's own presign router; tokens
  minted by `BlobStore::presign` resolve here.

A full end-to-end check (PUT then follow GET) is exercised by the
example's `tests/round_trip.rs`.
