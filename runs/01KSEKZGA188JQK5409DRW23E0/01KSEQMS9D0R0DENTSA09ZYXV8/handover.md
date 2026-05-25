## Done

- Scaffolded `packages/starter-ui-dashboard-native/` with `package.json` (peerDeps: react, react-native, react-native-svg, moti; optional peer lucide-react for `ActivityItem.icon` type), `tsconfig.json`, `vitest.config.ts`, `README.md`.
- RN ports of all four widgets with identical prop APIs to `@nube/starter-ui-dashboard`: `MetricCard`, `RadialProgress`, `ActivityFeed`, `PerformanceChart`. Each widget imports only `@nube/starter-ui-kit-native` + `react-native-svg` + `moti` (+ `react`) — no direct `react-native` primitives.
- Barrel `src/index.ts` re-exports widgets and their `Props`/`ActivityItem` types.
- Host-element mocks for the three runtime peers under `src/__mocks__/`; ambient module shims under `src/types/peers.d.ts`.
- Vitest unit tests per widget: 12 tests, all passing under jsdom (`pnpm -C packages/starter-ui-dashboard-native test` → 4 files, 12 passed).
- Workspace pick-up confirmed via `pnpm install` (auto-added via existing `packages/*` glob — no `pnpm-workspace.yaml` edit needed).
- Committed as `phase 5 (slice 5) — @nube/starter-ui-dashboard-native new package`.

## Next

- Stage 9 (final REVIEW gate): verify all four new packages build green together — `pnpm -w build` + per-package `pnpm test` for the four packages; capture transcripts; then the rubix mobile job is unblocked to start scaffolding `rubix/mobile/`.

## What you need to know

- Per-package `pnpm typecheck` for `starter-ui-dashboard-native` reports errors **only** inside `../starter-ui-kit-native/src/` — identical to the pre-existing failure already present in stage 7's `starter-ui-sdui-native` typecheck (real `react-native` types from the installed peer conflict with the kit's `peers.d.ts` shims once it's pulled across a workspace edge). `starter-ui-kit-native`'s OWN typecheck still passes in isolation because its `peers.d.ts` wins. Tests are green everywhere. The fix belongs in `starter-ui-kit-native` (e.g. a `paths`-based or `typesVersions` indirection so the kit doesn't pull real `react-native` types for its consumers) and was deliberately not done in this stage — out of scope.
- `ActivityItem.icon` is typed as `LucideIcon` from `lucide-react` for prop-API parity with the web widget; consumers on mobile may pass the structurally-compatible `lucide-react-native` equivalent (both accept `size`/`color`). `lucide-react` is declared as an optional peer.
- `className?: string` is kept on every `Props` purely for type-parity with the web component; the RN widgets ignore it (RN has no className). Documented inline.
- Mock kit exposes only the symbols the widgets touch (`Card`, `Box`, `Row`, `Column`, `Text`, `Badge`, `Button`, `useTheme`); reaching for anything else from a widget will fail at import time during tests — that's the depend-only-on-kit acceptance check.

## Open questions

- (none)
