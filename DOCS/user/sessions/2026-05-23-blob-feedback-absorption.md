# 2026-05-23 — Blob storage feedback absorbed; Gap 4 quotas landed

## What landed this session

Driven by [dev-pulse's SCOPE-STORAGE-FEEDBACK.md](../../../../dev-pulse/SCOPE-STORAGE-FEEDBACK.md).
Gaps 1, 2, 3, 5 are real code; Gap 4 is a named 0.2 scope item.

### Scope changes
- [DOCS/storage/SCOPE.md](../../storage/SCOPE.md) — added `starter-blob-axum` crate, locked `useBlobUpload` surface, isolation guidance table, reserved `BlobMeta` keys, quotas section under "Planned for 0.2".

### Rust (60 tests passing across touched crates)
- `starter-spi`: new `BlobContext` type, `meta_keys` module (`FILENAME` / `UPLOADED_BY` / `UPLOADED_AT`), `BlobMeta.user_metadata`, `PutOptions.user_metadata`, `BlobStore::context_for` default method.
- `starter-blob-memory`: propagates `opts.user_metadata` through `head`.
- `starter-blob-compose`: `Namespaced::context_for` peels its prefix onto inner store's context (nests correctly).
- `starter-blob-axum`: **new crate** — `blob_proxy_handler(Arc<dyn BlobStore>, authz)`, `BlobError`→HTTP mapping, base64url-encoded `BlobRef` path param, Range/If-None-Match/HEAD/Content-Disposition support, `Retry-After` on `Throttled`.

### TypeScript (7 tests passing)
- `@nube/starter-ui-blobs`: **new package** — `useBlobUpload` + `useBlobUploadForMarkdown` at the `/markdown` subpath. Locked surface from the scope.

### Gap 4 — quotas (now shipped, 86 tests across touched crates)
- `starter-spi`: new `BlobUsage { bytes, objects }` type, new trait method `BlobStore::approximate_usage(prefix)` defaulting to `BlobError::Unsupported`.
- `starter-blob-memory`: authoritative `approximate_usage` (walks the in-memory map under the read lock).
- `starter-blob-fs`: authoritative `approximate_usage` via `walkdir` on a blocking pool; skips `.meta.json` sidecars.
- `starter-blob-compose::Namespaced`: new `Quota { max_bytes, max_objects }` + `Namespaced::with_quota(...)`. `put_bytes` / `put_stream` / `copy_server_side` pre-flight via the inner store's `approximate_usage`; over-cap returns `BlobError::PayloadTooLarge`. No combinator-local counter — one source of truth per deployment.
- `Namespaced::approximate_usage` overrides the default to combine + forward, so `Namespaced<Namespaced<...>>` stacks compose.
- Documented limits at the `Quota` type: race window (two concurrent writers can overshoot by one write), and `put_stream` pre-flight only refuses an already-over namespace (mid-stream overflow is admitted).
- `DOCS/storage/SCOPE.md`: Quotas moved from "Planned for 0.2" to "shipped"; new 0.2 entry tracks s3/garage `approximate_usage`.
- `dev-pulse/SCOPE-STORAGE-FEEDBACK.md` status header refreshed.

---

## Follow-ups for next session

### High priority — finish the blob feedback

1. **`approximate_usage` on s3 + garage.**
   Scope shape is locked (see SCOPE.md "Planned for 0.2"). List/inventory-based — `aws-sdk-s3::list_objects_v2` paginated with `prefix=` + summing `Contents[].Size`. Document the lag (list lags multipart-completion by seconds). Without it, a `Namespaced::with_quota(...)` over an s3-backed store always returns `Unsupported` on writes, which silently breaks quota enforcement in production deployments. Add a test that runs `Namespaced::with_quota` against the s3 engine behind localstack and verifies overshoot is rejected.

2. **Propagate `user_metadata` through s3 + garage engines.**
   Memory + fs already round-trip it. S3/garage `put_object` accepts `metadata: HashMap<String, String>` — wire `opts.user_metadata` through and read it back in `head_object`. Without this, the `Content-Disposition: filename=…` behaviour from `starter-blob-axum` silently degrades to `"download"` for s3/garage-backed stores.

3. **Move the existing presign router from `starter-blob-fs` + `starter-blob-memory` into `starter-blob-axum`.**
   Scope says all axum integration lives in one crate. Current state: presign routers still live in their engine crates (working, feature-gated). Move them; delete the `axum` feature from `starter-blob-fs` and `starter-blob-memory`; rewire `examples/blobs`. **Risk:** breaking the example — test before/after.

### Architectural — `starter-ui-core` breakdown

`i18n` extraction was attempted this session and deferred. The blocker is real:
- `i18n/provider.tsx` imports `usePreferences()` from `preferences/`.
- `preferences/SettingsPage.tsx` imports `useIntlContext` + `useTranslate` from `i18n/`.

These are mutually coupled. A clean extraction needs a holistic plan for the whole package family. Suggested ordering for next session:

1. Map all cross-module imports in `starter-ui-core/src/` (already partially done — see commit log).
2. Decide what stays in "core" — recommendation: `query/` + `testing/` only.
3. Decide what becomes siblings — candidate packages: `@nube/starter-ui-auth`, `@nube/starter-ui-i18n` + `preferences` (combined, given coupling), `@nube/starter-ui-theme-editor`.
4. Execute as one PR with all renames. ~30 i18n imports + similar for auth/preferences. Mechanical but high-volume; typecheck after each workspace (`packages/`, `starter-extensions/packages/`, `examples/*/frontend/`).

**Do not extract one piece at a time.** Doing so creates dep arrows that the next extraction has to fight.

### Lower priority — debt from this session

- `mapping.rs` in `starter-blob-axum` ends with `_ => 500` because `BlobError` is `#[non_exhaustive]`. When new variants are added, add explicit arms there or the mapping silently defaults to `500`.
- `useBlobUpload`'s `progress` reports only `0`/`1` because we used `fetch`. A future revision can switch to `XMLHttpRequest` for granular progress without breaking the surface.
- No tests yet for `starter-blob-axum` against a `Tiered`/`Mirrored` combinator — only `Namespaced` is exercised for `BlobContext`. Add one.

---

## Don't forget

- The dev-pulse feedback doc has a status header noting which gaps landed where: [/home/user/code/rust/dev-pulse/SCOPE-STORAGE-FEEDBACK.md](../../../../dev-pulse/SCOPE-STORAGE-FEEDBACK.md). Update it again after Gap 4 lands.
- Run `cargo test -p starter-spi -p starter-blob-memory -p starter-blob-fs -p starter-blob-compose -p starter-blob-axum` before the next blob-touching commit to keep the 86-test baseline green.
- Run `pnpm --filter @nube/starter-ui-blobs test` to keep the TS-side 7-test baseline green.
