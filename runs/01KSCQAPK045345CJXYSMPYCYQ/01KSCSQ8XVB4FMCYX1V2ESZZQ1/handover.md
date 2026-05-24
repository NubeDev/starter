## Done

- Dashboard `RadialProgress` now reads `useDiskUsage()` live percent (with `ErrorBoundary` wrap on the route)
- New `/extensions` route: `useExtensionsList` table + per-row Start/Stop/Restart mutations + `useExtensionEvents` SSE overlaying live lifecycle state on the badge column
- New `/admin/users` route: `useUserList` table + `useUserCreate` form + per-row `useUserDisable` + header `useUndoLast` button
- New `src/components/error-boundary.tsx` class component catching `RubixError`, rendering `intl.formatMessage({ id: error.code })` and a generic fallback for non-rubix errors
- Added EN+ES i18n keys: `dashboard.diskUsage*`, `errors.*`, `extensions.*`, `users.*`, `common.loading|yes|no`
- Manually extended `src/routeTree.gen.ts` to register `/extensions` and `/admin/users` (router-plugin only regenerates during vite dev/build; the file is `@ts-nocheck` so this is safe until next vite run)
- `pnpm --filter @nube/rubix-frontend typecheck` green; `test` script is undeclared so `pnpm run test` exits 0 (no vitest is wired into rubix/frontend)
- Committed as `8beddb1` with message starting "phase C.3 — dashboard real data + extensions route + users admin route + error boundary"

## Next

- Stage 14 (next session): per WORKFLOW.md — likely C.4 nav/sidebar links to the new routes + E2E playwright pass

## What you need to know

- `RubixError.problem` is typed as `Problem`; the error boundary casts it to a plain record to spread as react-intl params. If a future stage tightens that type the cast becomes the lever to revisit.
- `routeTree.gen.ts` is auto-generated; the next `pnpm dev`/`pnpm build` will overwrite my manual edits — that's fine because the vite plugin will rediscover both new route files from `src/routes/{extensions.tsx, admin/users.tsx}`.
- ErrorBoundary only catches thrown render errors; queries/mutations need `throwOnError: true` to surface. Hooks default to not throwing, so today the boundary mainly catches RubixError thrown from synchronous render-time conditions or future `throwOnError`-marked hooks. Worth raising upstream in a follow-up stage if behavior should be lift-by-default.
- No `nav.item.users` / `nav.item.adminUsers` key was added — nav linking is deferred to a follow-up stage; users reach the new routes by URL for now.

## Open questions

- (none)
