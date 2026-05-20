## Done

- packages/starter-ui-core/src/preferences/{types.ts,units.ts,formatters.ts,provider.tsx,index.ts} new module with PreferencesProvider (react-query + context, no zustand), usePreferences hook, pure formatDate/formatTime/formatNumber/formatCurrency/formatQuantity formatters, and static unit map mirroring starter-spi::units::StaticRegistry
- packages/starter-ui-core/src/preferences/__fixtures__/units.json snapshot of expected GET /v1/units used as the cross-check fixture
- packages/starter-ui-core/src/preferences/{units.test.ts,formatters.test.ts,provider.test.tsx} cover convertUnit (matches Rust integration numbers), every formatter, a (locale, prefs) snapshot matrix, and provider mount/PATCH/cache-invalidation flow
- package.json#exports gains "./preferences" subpath; src/index.ts barrels the new module
- pnpm test green (69 tests across 7 files); pnpm typecheck clean
- committed as "stage 17 — Phase 4 @nube/starter-ui-core: PreferencesProvider + formatters module."

## Next

- Stage 18 (next session) — Phase 4 IntlProvider wiring via react-intl bound to the loaded catalog + the Settings page wired into starter-auth-users' account page

## What you need to know

- State-management lock for Phase 4: react-query + React context (no zustand) — documented in provider.tsx
- The TS unit map and the JSON fixture must move together when the Rust StaticRegistry changes; units.test.ts will fail loudly on drift
- formatNumber uses a locale-chain fallback (forced "en-US"/"de-DE"/"fr-FR" prepended) when the user's locale default disagrees with their explicit number_format
- Preferences endpoints are NOT in @nube/starter-client-ts's codegen yet; the provider fetches via client.fetch + client.baseUrl directly (mirrored types in types.ts) — when codegen catches up, types.ts should re-export from starter-client-ts
- Workspace sentinel `@starter/default` matches the Rust resolver default; exposed as DEFAULT_WORKSPACE

## Open questions

- (none)
