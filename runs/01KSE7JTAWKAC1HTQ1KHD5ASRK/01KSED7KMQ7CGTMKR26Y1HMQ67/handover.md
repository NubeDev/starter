## Done

- Added `@nube/starter-ui-ir` (TS mirror of the `starter-ui-ir` Rust crate, per scope OQ-7) under `packages/starter-ui-ir/` with `package.json`, `tsconfig`, `src/index.ts` (IR_VERSION + UiComponent / UiResolve* / UiActionResponse / Subscription* / Request types), README.
- Added `@nube/starter-ui-sdui-react` under `packages/starter-ui-sdui-react/`:
- `package.json` (peer-deps React 19 + @tanstack/react-query 5 + @nube/starter-ui-ir; deps @nube/starter-client-ts + @nube/starter-ui-kit), `tsconfig.json`, `vitest.config.ts`, `README.md`.
- `src/index.ts` barrel; `src/sdui-page.tsx` consuming `useSduiResolve` + `useSduiSubscriptions` (and surfacing dry-run / error states); `src/page-state.tsx` (PageStateProvider + `usePageStateKey`).
- `src/transport/index.ts` — `SduiTransport` interface + `createHttpSduiTransport` over `StarterClient` (mirrors starter-ui-ai-builder/http.ts shape).
- `src/provider/sdui-provider.tsx` with `SduiProvider` context (transport + optional custom renderer registry) and `useSduiTransport`.
- 14 verb files under `src/renderer/` (page, grid, kpi, chart, table, form, tabs, select, slider, toggle, date-range, divider, custom, repeat) each ≤ 150 LOC, all self-register via `registerRenderer`; `render.tsx` walker with dangling fallback; barrel side-effect imports them.
- 14 sibling `.test.tsx` smokes using `react-dom/server::renderToStaticMarkup` + a small `test-utils.tsx` harness with a `nullTransport()`. All 14 pass.
- pnpm-workspace already includes `packages/*` so the new packages are picked up — `pnpm install` linked them and `pnpm -F @nube/starter-ui-sdui-react typecheck` + `test` are both green.
- Committed as `feat(starter-ui-sdui-react) new package — SduiPage + per-variant renderers + transport seam`.

## Next

- (none) — fresh session picks up Stage 14.

## What you need to know

- The IR TS mirror is hand-maintained; document warns to mirror Rust changes. Long term it should be codegen'd from `crates/starter-ui-ir/schema/starter-ui-ir.schema.json`.
- The default `transport.subscribe()` is a polling stub (15 s tick); rubix's frontend transport adapter (Phase D.2) is expected to override `subscribe()` with SSE.
- `<SduiPage>` builds `ClientCapabilities` from `listRenderers()` at mount, sending `ir_versions: [IR_VERSION]` + the 14 built-in variants + any host custom renderer ids registered ahead of mount (host custom renderers are injected via `<SduiProvider customRenderers={{...}}>` — they're looked up in `RenderCustom` but currently aren't auto-folded into `custom_renderers`; if scope needs that, the capability builder should read `customRenderers` from context instead of just `listRenderers()`).
- Renderer fallback: unknown `node.type` → `data-sdui-dangling` placeholder; missing custom renderer id → `data-sdui-custom-missing` placeholder. No throws.
- The `useSduiSubscriptions` queryKey shape is a best-effort echo of the resolve hook's key; if Stage 14 needs deterministic invalidation, refactor `use-resolve.ts` to export the key builder.

## Open questions

- Should the capability payload also include `customRenderers` registered via `<SduiProvider>` (and not just the built-ins from `listRenderers()`)? Left as built-ins-only for v1.
- The `repeat` renderer currently re-renders the same template per item without scoping `$row` into page-state; scope notes say the resolver inlines repeats server-side, so the client path is a thin fallback. Confirm with Stage 14 whether that's enough for the bundled disk-overview demo.
