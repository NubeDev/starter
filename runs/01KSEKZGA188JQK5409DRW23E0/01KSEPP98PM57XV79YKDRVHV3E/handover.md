## Done

- Scaffolded `packages/starter-ui-sdui-native/` with package.json (peer: react/react-native; deps on `@nube/starter-ui-ir`, `@nube/starter-ui-kit-native`, `@nube/starter-ui-sdui-react`), tsconfig, vitest.config (aliases `@nube/starter-ui-kit-native` → local mock).
- Added 16 renderers one-for-one mirroring `packages/starter-ui-sdui-react/src/renderer/index.ts` in the same priority order: `render-page/row/col/grid/kpi/chart/divider/tabs/table/form/select/slider/toggle/date-range/repeat/custom`. Each ≤150 lines, imports only kit-native + IR types + `@nube/starter-ui-sdui-react/headless` (for `registerRenderer`, `Render`, `RenderChildren`, `usePageState*`, `useSduiAction`, `useSduiContext`) — no `react-native` imports.
- `src/index.ts` is the side-effect barrel that registers every kind on import; per spec it does NOT alias `sparkline` to `chart` or `kpi_grid` to `grid` (both in the deferred-with-web set).
- `README.md` enumerates the 16 covered kinds and explicitly lists the 10 deferred-with-web kinds (`stack, card, text, heading, badge, kpi_grid, button, link, field, sparkline`) per `rubix/docs/scope/mobile/NEW-PACKAGES.md §Parity vs the IR Kind union`.
- Vitest unit tests per renderer (28 tests total) using a swap-in mock kit at `src/__mocks__/starter-ui-kit-native.tsx`; tests assert structure + a11y prop pass-through. Smoke test loads `crates/rubix-flows/dashboards/disk-overview.json` (now committed at `src/test-fixtures/disk-overview.json`), asserts the registry lists exactly the 16 kinds, and walks the IR through `Render` end-to-end.
- Infrastructure additions to `@nube/starter-ui-kit-native`: new `src/layout.tsx` exposing `Box`/`Row`/`Column`/`Text`/`ScrollArea`/`Divider` (re-exported from the barrel + subpath `./layout`) so renderers can comply with the "no direct RN primitives" rule.
- All tests green: `pnpm --filter @nube/starter-ui-sdui-native test` → 17 files / 28 tests pass; `pnpm --filter @nube/starter-ui-kit-native test` → 14 files / 27 tests still pass.
- Committed as `stage 7: phase 4 (slice 4) — @nube/starter-ui-sdui-native new package`.

## Next

- (none) — stage 8 (`@nube/starter-ui-dashboard-native`) is picked up in a fresh session.

## What you need to know

- `sdui-native` devDeps pin React 19 (not 18) to match `sdui-react`'s React 19 — otherwise `RenderChildren`/`Render` create React-19 elements that the React-18 reconciler rejects with "Objects are not valid as a React child". `kit-native` stays on React 18 (its tests break under 19 because `@testing-library/react@16` + the project's RN/Moti type pins assume 18).
- This React 18↔19 split means `pnpm --filter @nube/starter-ui-sdui-native typecheck` surfaces preexisting cross-version errors inside `kit-native` source (MotiView ReactNode `bigint` mismatch, etc.) — these are NOT in code this stage wrote and reflect the same split that already existed between stages 0/6. The runtime works fine (renderers see Pressable/Text via the mock in tests and via kit-native's own RN imports in production). Resolving it cleanly is an ADR-sized cross-package decision (align everything on React 19) — out of scope for this slice.
- `@tanstack/react-query` was added as a devDep of sdui-native so `test-wrappers.tsx` can construct a `QueryClient` for tests that touch `useSduiAction` (form renderer).
- The mock kit (`src/__mocks__/starter-ui-kit-native.tsx`) is the literal kit surface contract — if a future renderer reaches for a kit export the mock doesn't list, the test errors at import time. Extending it is a one-line `el("…")` add.
- Renderers that depend on page state / action / context (`form`, `select`, `slider`, `toggle`, `date_range`, `custom`) are tested wrapped in `<Providers>` from `src/test-wrappers.tsx`, which stubs the transport.

## Open questions

- Should kit-native + sdui-native be moved to React 19 together (with a parallel bump of `@testing-library/react`, RN/Moti types, etc.) in a follow-up stage so workspace typechecks cleanly? Flagging for a reviewer call before stage 8 builds on top of the same split.
