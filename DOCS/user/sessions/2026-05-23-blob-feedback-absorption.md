# 2026-05-23 — Blob storage feedback absorbed; follow-ups for next session

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

---

## Follow-ups for next session

### High priority — absorb the rest of the blob feedback

1. **Gap 4 — Quotas (0.2 work).**
   Scope shape is locked. Land in this order:
   - Add `BlobUsage { bytes, objects }` type + `BlobStore::approximate_usage(prefix)` default method (returns `Unsupported`) in `starter-spi`.
   - Implement `approximate_usage` authoritatively in `starter-blob-memory` and `starter-blob-fs` (cheap; walk the keyspace).
   - Implement eventually-consistent in `starter-blob-s3` / `starter-blob-garage` (list-based, document the lag).
   - Add `Namespaced::Quota { max_bytes, max_objects }` — `put_*` exceeding the cap returns `BlobError::PayloadTooLarge`. **Do not** maintain a counter inside `Namespaced`; ask the inner store via `approximate_usage`.

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
- Run `cargo test -p starter-spi -p starter-blob-memory -p starter-blob-fs -p starter-blob-compose -p starter-blob-axum` before the next blob-touching commit to keep the 60-test baseline green.
- Run `pnpm --filter @nube/starter-ui-blobs test` to keep the TS-side 7-test baseline green.
