## Done

- created `packages/starter-ui-kit-native/` with the 13 primitives (button, card, input, tabs, badge, switch, slider, select, sheet, dialog, spinner, skeleton, tooltip), one verb per file, each ≤200 lines
- prop API mirrors `@nube/starter-ui-kit` 1:1; only intentional diff is RN's `onPress` vs web's `onClick`
- every primitive ships `accessibilityRole` + a resolvable `accessibilityLabel`/`accessibilityHint` path on the underlying RN element
- `src/theme.ts` exposes `useTheme()` reading `@nube/starter-theme-tokens` + the `useLayoutPreferences` zustand store from `@nube/starter-ui-core/theme-editor` (named imports only; no DOM-bound siblings dragged in)
- `example/app.tsx` story-style harness renders every primitive against `{light/platform-default, dark/platform-default, light/modern-minimal, dark/violet-bloom}`
- vitest unit tests per primitive: snapshot + a11y prop assertion (27 tests, all passing) backed by host-element mocks for `react-native`, `react-native-svg`, `moti`, `react-native-reanimated` (aliased via `vitest.config.ts`)
- type shims under `src/types/peers.d.ts` keep `tsc --noEmit` green without installing the native toolchain
- `src/no-web-imports.test.ts` structurally enforces "MUST NOT import `@nube/starter-ui-kit`"
- committed as `d108e62`

## Next

- (none) — stage 7 (`@nube/starter-ui-sdui-native`) picked up by the next session per scope plan

## What you need to know

- this package's `react`/`react-dom` devDeps are pinned to ^18 (not 19) to match the React copy that `@nube/starter-ui-core` resolves via zustand — mixing 18+19 produced a "useCallback on null" crash; do not bump in isolation
- the vitest mocks live at `src/__mocks__/*` and are wired only via `vitest.config.ts` `resolve.alias` — they are not exported; consumers see the real RN peers
- palette values are oklch() strings; RN does not parse oklch natively. That's a known mobile gap to address in a later polish stage; primitives forward the string as-is to `backgroundColor`/`color` so snapshots are stable but actual paint on-device will need an oklch→rgb converter wired into `useTheme.color()`
- `Slider` uses `PanResponder`; under jsdom the gesture mock returns empty `panHandlers`, so tests only validate mount + a11y (not drag math)
- `Tooltip` triggers on long-press (no hover on RN); the trigger's `accessibilityHint` defaults to "Long-press to see tooltip"
- pre-existing failures in `packages/starter-ui-core`'s `src/auth/auth.test.tsx` (2 tests, fetchJson 404) are NOT introduced by this stage — confirmed by re-running with the new package stashed
- `pnpm-workspace.yaml` already includes `packages/*`, so no edit was needed there (contrast with stage 4's tokens scaffold)

## Open questions

- (none)
