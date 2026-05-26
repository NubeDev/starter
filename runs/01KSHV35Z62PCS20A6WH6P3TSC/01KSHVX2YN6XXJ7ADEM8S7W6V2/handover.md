## Done

- Replaced hardcoded `'system'` tenant in `rubix/frontend/src/routes/dashboards/$pageId_.edit.tsx` and `rubix/packages/rubix-client-react/src/hooks/dashboard.ts` with the active tenant from `useTenantList()` (`tenants[0].tenant_id`); fetch + save transport gated on tenant resolution, with explicit error/empty/loading states.
- `useDashboardGet` signature is now `(tenantId, pageId, options?)` — query key includes `tenantId`, enable flag requires both.
- Added `onDiscardRequested` callback prop on `PuckBuilder`; route bumps `reloadKey` synchronously. Dropped `window.__rubixPuckDiscardRequested` from `builder.tsx` and the 250ms polling effect from the edit route.
- Fixed `packages/starter-ui-sdui-react/src/renderer/render-chart.test.tsx` — replaced stale `"3 series"` assertion with `data-sdui-chart-series-count="0"` (empty series objects collapse to placeholder).
- Trimmed three resolved items from README §Next tasks in `packages/starter-ui-sdui-puck/README.md`.
- `pnpm --filter @nube/starter-ui-sdui-puck test` (19 tests) + `typecheck` green; `pnpm --filter @nube/starter-ui-sdui-react test` (24 tests) green; `pnpm --filter @nube/rubix-frontend typecheck` clean.
- Committed as `5b038eb`.

## Next

- Stage 3+: §B3 analytics-template list verb wiring (still throws a free-text-fallback error in the edit route), §B6 runtime schema-hash banner, Scope 11 SSE banner + revalidate-on-resume, expanded placeholder coverage.

## What you need to know

- `MeResponse` has no tenant field. The session-active-tenant convention used here matches `rubix/frontend/src/components/top-header.tsx` (the 1-tenant / 2+-tenant branches both treat `tenants[0]` as active). If a session-tenant API ships later, swap the source in `$pageId_.edit.tsx` and `useDashboardGet` callers.
- `useDashboardGet` is currently only declared in `dashboard.ts` (no production callers found via grep), so the signature change is safe — any future caller must pass `tenantId` explicitly.
- `makeRubixSaveTransport(client, activeTenantId)` is now constructed per-render but is only handed to `<PuckBuilder>` once tenant + page have resolved; the builder's `onSave` capture is stable from there.

## Open questions

- (none)
