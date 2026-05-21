## Done

- Added a `Decisions` section to `examples/flow-agent/SCOPE.md` recording D1–D4 for the Page Builder slice (sidebar live-update via `useSyncExternalStore` + `storage`-event, buffered-patch fixture timeline `t=0/50/60/80…780 ms`, `<SduiHost>` as a thin `SduiProvider` wrapper with a module-level no-op dispatcher, and `frontend/package.json` confirmed untouched).
- Committed as `Stage 1: resolve Page Builder open design points` on branch `codeless/flow-agent-page-builder` (commit `fef70fd`).

## Next

- Stage 2 of 5: implement the foundation files implied by the decisions — `lib/pages-store.ts` (with the `usePages` `useSyncExternalStore` hook + `storage` listener), `lib/sdui-shim.tsx` (`<SduiHost>`), and the routes wiring in `src/app.tsx` + sidebar entries in `src/layout/Shell.tsx`. Do not start; a fresh session picks it up per the staged plan.

## What you need to know

- Verified all `@nube/*` deps (`starter-sdui-react`, `starter-ui-ai-builder`, `starter-ui-skills`, `starter-ui-chat`, `starter-ui-kit`) are already `workspace:*` deps in `examples/flow-agent/frontend/package.json` — D4 reflects that, and PAGE-BUILDER.md's `✎ +3 workspace deps` annotation is stale (called out in the decision).
- Verified `SduiProvider`, `useBuilder`, and `createInMemorySkillsAdapter` are exported from their respective packages (grep hits in `packages/starter-sdui-react/src/index.ts`, `packages/starter-ui-ai-builder/src/hooks/index.ts`, `packages/starter-ui-skills/src/adapters/index.ts`).
- The `SduiAction` type name is used illustratively in the D3 code sketch; the implementation stage must read the real `SduiProvider` props (likely a `dispatcher`/`dispatch` shape) from `@nube/starter-sdui-react` and adjust if the field names differ — this is flagged inline in SCOPE.md.
- `useBuilder`'s default buffer window is referenced as 1 s; if the implementation finds a different default in `packages/starter-ui-ai-builder/src/hooks/use-builder.ts`, the D2 timings are still safe (parent arrives 30 ms after first patch and 20 ms after the second).
- LLM-free; everything stays on fixtures + `localStorage` per the slice contract.

## Open questions

- (none)
