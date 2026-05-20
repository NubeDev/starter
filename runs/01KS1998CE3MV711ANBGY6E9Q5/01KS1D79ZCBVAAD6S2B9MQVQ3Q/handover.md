## Done

- Implemented `accept_language_layer(Arc<MessageBundle>)` tower Layer/Service in `crates/starter-i18n/src/middleware.rs` with `LocaleCtx` (language + is_fallback) request extension, `Content-Language` + `Vary: Accept-Language` response headers, `tracing::debug!` fallback event, and opt-in `.with_fallback_header(true)` → `X-I18n-Fallback: <lang>`.
- Implemented `router(Arc<MessageBundle>)` in `crates/starter-i18n/src/routes.rs`: `GET /v1/i18n/manifest` (BTreeMap of lang→16-char fp, ETag, revalidate Cache-Control) and `GET /v1/i18n/catalogs/{spec}` handling both un-fingerprinted (ETag/304) and content-hashed `{lang}-{fp}.json` (immutable Cache-Control when fp matches; downgrades to revalidate on stale fp). Stale-fp downgrade and BCP-47 subtag (`en-GB-{fp}.json`) parsing covered.
- Added optional `futures` + `async-trait` deps gated on `routes`, plus `http-body-util` dev-dep.
- 52 unit/integration tests pass with `cargo test -p starter-i18n --features routes`; default-feature `cargo build -p starter-i18n` clean.
- Committed as `stage 14 — Phase 3 Accept-Language middleware + routes.`

## Next

- Stage 15 picks up the next item in `DOCS/user/scope/SCOPE.md` (likely Phase 3 seed catalogs at `catalogs/starter/` for en/es and wiring into `platform.rs` / `starter-server`).

## What you need to know

- `LocaleCtx::is_fallback` is derived by checking whether the chosen tag matches any exact entry in the parsed Accept-Language list, so family / wildcard / static-fallback all count as "fallback" and emit the debug event (and the `X-I18n-Fallback` header when enabled).
- The fingerprinted catalog URL form serves the current bytes even when the path-embedded fingerprint is stale, but downgrades Cache-Control to revalidate — keeps the immutable contract on the URL only.
- The route parser peels the fingerprint suffix as `-<16 hex chars>.json`; if absent/malformed it treats the whole `{spec}` as a language tag, so plain `/v1/i18n/catalogs/en-GB` still resolves.
- Router uses `with_state(Arc<MessageBundle>)` so the host server can either mount via `router(bundle)` or merge via `with_i18n_routes(existing, bundle)`.

## Open questions

- (none)
