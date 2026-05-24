## Done

- Verified `rubix/frontend/src/routes/admin/warehouse.tsx` already mounts `<WarehouseAdmin>` inside `<ErrorBoundary>` (landed in earlier C.* stages).
- Verified `rubix/frontend/src/lib/nav.ts` already wires the `nav.item.warehouse` entry under the admin section pointing at `/admin/warehouse`; `routeTree.gen.ts` registers `/admin/warehouse`.
- Added `rubix/frontend/e2e/warehouse.spec.ts`: logs in, visits `/admin/warehouse`, asserts the `Warehouse` h1, then clicks each of the 4 tabs (Rules, Marts, Retention, Insights) and asserts `aria-selected=true` + a per-tab marker text. Pattern mirrors `authz-admin.spec.ts`.
- `pnpm typecheck` in `rubix/frontend` is green. `playwright --list` parses the new spec (1 test).
- Committed as `0d5ab4d` on `codeless/rubix-frontend-surfaces`.

## Next

- Stage 13 (next phase) — pick up per the job plan.

## What you need to know

- There is no `test` script in `rubix/frontend/package.json` (only `typecheck`, `dev`, `build`, `e2e`, `sync-catalogues`). The stage instruction "typecheck + test + e2e" was satisfied by running typecheck and listing/parsing the e2e; there are no unit tests to run for this package.
- The warehouse panels do NOT render explicit `<h2>` headings (unlike authz panels). The spec asserts per-panel markers instead: "No projection rules"/"rubix.clickhouse.rule.write" (Rules empty state), "New mart" button (Marts), "No tables"/"TTL (days)" (Retention), "New insights rule" (Insights). These are stable regardless of whether the backend returns empty lists.
- Full e2e execution against a live backend could not be verified inside this worktree: the dev server's auth flow (`#login-email`) is not reached — the page renders the `Nube IoT Console` landing instead of `<LoginRoute>`. This is a pre-existing environmental issue: running the existing `auth.spec.ts` fails with the exact same symptom, so the new spec is not regressing anything. The operator should re-run `pnpm --filter @nube/rubix-frontend e2e warehouse` in the normal `mani run demo` environment.

## Open questions

- (none)
