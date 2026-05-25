# starter-ui-kit-native — story-style harness

Renders every primitive in isolation against light, dark, and two
named palettes ("modern-minimal", "violet-bloom"). Drop
[`./app.tsx`](./app.tsx) as the root of a fresh Expo project; the
harness pulls its theme from `useLayoutPreferences()` (the same
zustand store the web theme editor writes to), so toggling
mode / palette from a Settings screen — or a REPL — re-themes
every primitive live.

## What a reviewer should look at

- One row per primitive, labelled.
- Toggle the mode via the global preferences store; the harness
  re-themes without remounting.
- Cycle through `PALETTE_CYCLE` (exported alongside `ExampleApp`)
  to eyeball the four required palette/mode combinations:
  `{light, platform-default}`, `{dark, platform-default}`,
  `{light, modern-minimal}`, `{dark, violet-bloom}`.

## What it deliberately does not do

- No state persistence beyond what `useLayoutPreferences`
  already provides (localStorage on web, AsyncStorage adapter on
  RN — wire that in the Expo app, not here).
- No Navigation/Router — the harness is a single `ScrollView`.
- No `rubix/mobile/` glue — this package ships independently of
  the app job.
