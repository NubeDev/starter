## Done

- Added `packages/test-ui-5` (Vite + React + vitest) with `src/app/extensions/page.tsx` wiring `ExtensionHostProvider` from `@nube/starter-ext-ui` against the rubix-agent base URL (`VITE_RUBIX_AGENT_BASE_URL`, defaults to `http://localhost:8080`) and rendering `<ExtensionSlot id="main"/>` inside a visible region.
- Added vitest smoke (`page.test.tsx`) that pre-registers a fake remote contributing into slot `main` and asserts the panel mounts; covers the empty-slot case too.
- Added `rubix/extensions/com.rubix.example/ui/main.tsx` exporting a minimal React component that reads `useHostTheme()` and renders `hello-from-com.rubix.example`.
- Updated `rubix/extensions/com.rubix.example/block.yaml` with `contributes.ui` (entry `ui/remoteEntry.js`, exposes `{name: Main, module: ./main, slot: main}`).
- `pnpm --filter @nube/test-ui-5 typecheck`, `test`, and `lint` all green (lint is aliased to `tsc --noEmit` since the workspace has no eslint config; matches the existing zero-lint convention across other packages).
- Committed as `stage 12: phase D.2 — test-ui-5 ExtensionHostProvider page` on `codeless/rubix-extensions-wire`.

## Next

- Phase D.3 / next stage in the plan (not started this session).

## What you need to know

- `tsconfig.json` `include` is `["src"]` only — `vitest.config.ts` was excluded because vitest@2 transitively pulls in vite@5 types that clash with the workspace's vite@6 types when typechecking config files. Tests still run fine.
- The `ui/main.tsx` placeholder is **not** wired into a build pipeline yet; `block.yaml` references the bundled artefact path `ui/remoteEntry.js` which Phase E (per scope notes) is expected to emit. For now the test-ui-5 visual surface is fully exercised via the in-process vitest fake remote; running `pnpm --filter @nube/test-ui-5 dev` against a live rubix-agent will show an empty slot until the federation bundle is built.
- Dev server port is 5180 to avoid collisions with test-ui-2/3 (5175) and the rubix frontend (5185).

## Open questions

- The frontend host has no router yet (single-route shell). When a real router lands, the page should move to whatever path convention (`/extensions`) downstream consumers prefer — currently it's mounted at `/`.
