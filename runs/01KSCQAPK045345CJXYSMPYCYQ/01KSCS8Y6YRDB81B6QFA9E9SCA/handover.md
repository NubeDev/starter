## Done

- vite.config.ts: added server.proxy for `/api/v1` and `/openapi.json` → `http://127.0.0.1:8088`
- src/lib/client.ts: `getRubixClient()` singleton constructed from `VITE_RUBIX_BASE_URL` (empty by default)
- Added `@nube/starter-client-ts` workspace dep to rubix/frontend so StarterClient can be imported directly
- scripts/sync-catalogues.mjs + `pnpm sync-catalogues` script: copies `rubix.*` keys from `rubix/crates/rubix-spi/catalogues/{en,es}.json` into `src/i18n/{en,es}.json` (idempotent, preserves shell keys); ran it once, 39 keys synced per locale
- Added `pnpm typecheck` script; `pnpm --filter @nube/rubix-frontend typecheck` green; `pnpm -w run check:i18n` green
- README documents `VITE_RUBIX_BASE_URL`, the dev proxy, and the new scripts
- Committed as `phase C.1 — frontend vite proxy + client singleton + i18n sync` on `codeless/rubix-frontend-wire`

## Next

- (none) — handed back to the runtime for the next stage

## What you need to know

- `VITE_RUBIX_BASE_URL` is empty by default; same-origin requests rely on the Vite proxy in dev and a reverse proxy in prod. Override with a fully-qualified URL only for non-default ports or production builds.
- The catalogue sync is manual (`pnpm sync-catalogues`), not wired into build/CI. Workspace `check:i18n` only enforces parity per app, not SPI↔frontend parity, so re-run sync whenever `rubix-spi/catalogues` changes. The script writes keys in `(non-rubix existing keys) + (sorted rubix.* keys)` order — diffs after re-running should be empty.
- `RubixClient` is wrapping a `StarterClient`; both are exported from their own packages — `getRubixClient()` is the only place that constructs either.

## Open questions

- SCOPE OQ-2 is closed by the on-demand sync script; if reviewers want the sync to happen automatically on `pnpm build`, prepend it to the `build` script in a follow-up.
