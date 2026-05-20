## Done

- Stage 5 landed in commit c1d548e on branch codeless/notes-prefs-i18n.
- Rust manifest schema (`starter-ext-spi`) now accepts `requires:` as bare strings (singleton ids) or typed `{id, version}`, and parses `contributes.i18n.catalogs: { <lang>: <path> }`. Two new unit tests in `manifest.rs`.
- New Rust route `GET /extensions/{id}/i18n/{lang}` (`starter-ext-server/src/i18n.rs`), same ETag/safe-join/unauthed posture as the UI bundle route; module wired through `lib.rs` and `router.rs`.
- `examples/notes/extensions/com.nube.hello/block.yaml` declares the two singleton requires and `contributes.i18n.catalogs`; sibling `i18n/{en,es}.json` ship `greeting` + ICU-plural `unread`.
- `packages/starter-ui-core/src/i18n/extension-messages.ts` provides `registerExtensionMessages` (D-NP.3 namespacing, drop+warn collision), `subscribeExtensionMessages`, `getExtensionMessages`, `setExtensionMessageTelemetry`, and a `_reset…ForTesting` helper. `IntlProvider` now merges the registry into its react-intl messages via `useSyncExternalStore`.
- Notes host: `extension-host.ts` retains each extension's `contributes.i18n.catalogs` in a module-level map and exposes `_listExtensionCatalogsForTesting`; new `extension-catalog-loader.tsx` (mounted in `app.tsx` inside `IntlProvider`) lazy-fetches the active language's catalog and de-dupes per (extension, language).
- `pnpm -w run check:i18n` runs `scripts/check-i18n.mjs` (zero-dep Node walker) asserting key-set parity vs `en.json`, non-empty values, and matching ICU placeholder identifiers. Currently passes for 3 catalog dirs.
- `examples/notes/frontend/src/extension-catalog-merge.test.tsx` — 3 cases (namespacing, lazy fetch counts across en→es→en flip, collision drop). All workspace vitest suites I touched still pass; `cargo build -p starter-ext-server` + `cargo test -p starter-ext-spi --lib manifest` green.

## Next

- Stage 6 — Playwright e2e per `examples/notes/user-pref.md` § Stage 6: au-bbq-operator fixture, panel renders Spanish greeting + °F after a Settings language flip without reload, html lang flips, BroadcastChannel propagates to a second tab.

## What you need to know

- `registerExtensionMessages` lives in `@nube/starter-ui-core/i18n` (not in the SDK) because the registry must share module state with `IntlProvider`. The SDK barrel only documents the location (see comment in `starter-ext-sdk-ts/src/index.ts`) — it deliberately does not re-export to keep the existing SDK→ui-core dep ban intact.
- Collision telemetry computes the intruded namespace as `key.slice(0, key.lastIndexOf("."))` — reverse-DNS depth is variable; the last dot is the only stable key/namespace boundary.
- The catalog endpoint accepts `/i18n/<lang>` with or without `.json`; the handler strips the suffix before looking up the manifest entry. The manifest key is the bare language tag (`en`, `es`).
- `check:i18n` regex extracts placeholders only at ICU positions (`{ident,` or `{ident}`); plural-arm prose like `{No unread notes}` is intentionally ignored.
- `<ExtensionCatalogLoader>` keeps a module-level `FETCHED` set so flipping language back-and-forth issues no redundant fetches; the production-grade `BroadcastChannel` propagation Stage 7 calls for is *not* wired here.

## Open questions

- The Stage 5 scope mentions `extension.catalog_key_collision` surfacing on "the registry detail page" — there is no such UI yet; current telemetry sink defaults to `console.warn`. Stage 7 (or a follow-up) may want to wire `setExtensionMessageTelemetry` from the notes host into the same telemetry bus as the singleton events.
