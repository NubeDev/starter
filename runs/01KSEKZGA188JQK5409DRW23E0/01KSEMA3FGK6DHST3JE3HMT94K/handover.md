## Done

- Carved `@nube/starter-ui-sdui-react/headless` subpath: moved `registry.ts`, `render.tsx`, `sdui-provider.tsx`, `page-state.tsx`, `sdui-page.tsx`, all hooks, and the transport into `src/headless/`.
- Rewired every `render-*.tsx` and `test-utils.tsx` to import the registry/render/provider/hooks/page-state from `../headless/...`.
- `src/sdui-page.tsx` (now at `src/headless/sdui-page.tsx`) imports `Render` from `./render.js` and `listRenderers` from `./registry.js` — no longer triggers the web renderer barrel.
- `src/renderer/index.ts` is now a side-effect-only barrel registering web renderers + exporting `Render<X>` components.
- `src/index.ts` re-exports `./headless/index.js` and `./renderer/index.js` (so the web SPA still gets renderer registration on import).
- Added `"./headless"` export to `package.json`.
- Added `src/headless/headless-graph.test.ts` that statically walks the import graph rooted at `src/headless/index.ts` and asserts no `@nube/starter-ui-kit` bare specifier appears.
- Committed as `phase 0 (slice 0): @nube/starter-ui-sdui-react ./headless subpath split` (`6c3dab6`).

## Next

- (none) — next stage starts in a fresh session.

## What you need to know

- `pnpm typecheck` is green in both `packages/starter-ui-sdui-react` and `rubix/frontend`.
- `pnpm test` in `starter-ui-sdui-react`: 14 passed / 1 failed. The single failure is the pre-existing `render-chart` "3 series" assertion (confirmed by stashing and rerunning on `b988d46`) — unrelated to this slice.
- The headless subpath is now self-contained: importing `@nube/starter-ui-sdui-react/headless` pulls in only `react`, `react-dom` (transitively via headless hooks), `@tanstack/react-query`, `@nube/starter-ui-ir`, `@nube/starter-client-ts` — no `@nube/starter-ui-kit`.
- `src/headless/sdui-page.tsx` deviates slightly from the literal plan wording (`./headless/render.js`) — moved the file into `headless/` so the subpath ships `<SduiPage>` directly; relative imports inside use `./render.js`/`./registry.js`. Plan intent (decoupled from renderer barrel, kit-free graph) is preserved and verified by the graph test.

## Open questions

- (none)
