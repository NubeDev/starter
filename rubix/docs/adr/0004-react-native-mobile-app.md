# ADR 0004 — React Native mobile app reuses the chassis at the kit seam

**Status:** accepted, 2026-05-25. **Partially supersedes**
[ADR 0002](./0002-backend-only.md): the `rubix/frontend/` SPA
already shipped without a superseding ADR; this ADR closes that
policy gap retroactively and extends it to mobile. A separate ADR
for the web frontend's own justification (ADR 0005?) is still
outstanding.
**Cites:** [SCOPE R6](../../SCOPE.md#r6),
[HOW-TO-CODE §0a](../../HOW-TO-CODE.md#0a--doc-tiers-and-what-code-may-reference),
[docs/design/frontend/README.md](../design/frontend/README.md),
[docs/scope/mobile/](../scope/mobile/README.md)

## Decision

The Rubix mobile app (iOS + Android) is an **Expo** React Native
app at `rubix/mobile/`. It reuses every TypeScript chassis layer
beneath `@nube/starter-ui-kit` unchanged, and adds four new
workspace packages that replace the two web-only layers:

```
NEW  @nube/starter-theme-tokens         ← also adopted by web kit
NEW  @nube/starter-ui-kit-native        ← mirrors starter-ui-kit API
NEW  @nube/starter-ui-sdui-native       ← registers into sdui-react registry
NEW  @nube/starter-ui-dashboard-native  ← mirrors starter-ui-dashboard API
```

Authentication uses bearer tokens (not cookies). Navigation uses
Expo Router. Storage uses `@react-native-async-storage/async-storage`.

The full plan lives in [docs/scope/mobile/](../scope/mobile/README.md).

## Context

- Rule R6 already isolates the DOM at two seams: `starter-ui-kit`
  (primitives) and the renderer registry inside
  `starter-ui-sdui-react`. Everything else (`starter-client-ts`,
  `starter-client-react`, `starter-ui-ir`, the DOM-free subpaths
  of `starter-ui-core`, the `SduiProvider` + hooks + transport)
  is portable to React Native today.
- The web app is the lighthouse consumer of those layers; a
  second consumer is the cheapest test of whether the seam holds.
- Operators want dashboards on a phone. They do not want the
  flow editor or admin surfaces on a phone — those stay web-only.
- The token-vs-cookie split is not a preference: RN's `fetch`
  doesn't share cookies with a `WebView`, so mobile needs a
  bearer-flavoured auth. The `AuthStrategy` abstraction in
  `@nube/starter-ui-core/auth` happens to already ship a
  `tokenStrategy` alongside `sessionStrategy` (cookie/CSRF,
  web-only by semantics) and `externalStrategy`
  (`window.location.assign`, web-only by implementation), so
  mobile gets to reuse the abstraction. `tokenStrategy.login`
  installs an already-issued bearer; the credentials→token
  exchange itself is the app's job, not the strategy's — mobile
  performs the exchange in `src/auth/strategy.ts` against the
  active connection's `base_url`. See
  [docs/scope/mobile/APP-SHELL.md §Strategy](../scope/mobile/APP-SHELL.md#strategy).

## Consequences

- The web `starter-ui-kit` is refactored to read tokens from the
  new `starter-theme-tokens` package. This is a non-behaviour
  change on web, validated by visual diff (snapshot harness or
  documented manual review — see
  [docs/scope/mobile/NEW-PACKAGES.md](../scope/mobile/NEW-PACKAGES.md#starter-theme-tokens)).
- `@nube/starter-ui-sdui-react` gains a `./headless` subpath
  (registry + provider + hooks + transport, no renderers). This
  is a precondition on the whole plan — see
  [docs/scope/mobile/REUSE.md](../scope/mobile/REUSE.md#reused--sdui-after-a-package-split-blocker).
  As part of the same refactor, `sdui-page.tsx` is decoupled from
  the renderer barrel (`./renderer/index.js`) and depends only on
  the registry under `./headless`; without that decoupling,
  importing `/headless` still pulls every web renderer
  transitively. See
  [docs/scope/mobile/NEW-PACKAGES.md §Precondition](../scope/mobile/NEW-PACKAGES.md#precondition--sdui-react-package-split).
- `@nube/starter-ui-core` exposes the portable subset cleanly
  (`tokenStrategy` from `/auth`; types/store/formatters from
  `/preferences` separately from the `PreferencesProvider` React
  component, which writes to `document`).
- **Backend gains a bearer-issuing route** (e.g.
  `POST /api/v1/auth/token` returning `{ token, expires_at }`)
  as a precondition to mobile login. This ADR does not justify
  that route on its own merits; mobile blocks on it. Specified
  in [docs/design/auth/](../design/auth/).
- The mobile app enforces the reuse boundary via an ESLint
  `no-restricted-imports` rule (chosen for ubiquity, no
  alternative seriously evaluated) listed in
  [docs/scope/mobile/APP-SHELL.md](../scope/mobile/APP-SHELL.md#import-lint).
- The app is **multi-instance**: one phone talks to many
  rubix-agent servers, with an on-device SQLite store as the
  source of truth for saved connection metadata and
  `expo-secure-store` as the home for per-connection bearer
  tokens. See [docs/scope/mobile/LOCAL-DB.md](../scope/mobile/LOCAL-DB.md).
- Per-connection cache isolation uses key namespacing via
  `starterQueryKey`, not `queryClient.clear()` on swap (which
  aborts in-flight queries).

## Alternatives considered

### A. Fork `starter-ui-kit` into a divergent mobile copy

Rejected. Two kits diverge within a release. The token-source
discipline (one `starter-theme-tokens` package) is the cheaper
way to keep the visual language identical.

### B. Use React Native for Web and share `starter-ui-kit` directly

Rejected. The existing kit is built on Radix + Tailwind; porting
it to RNW is a larger and riskier change than building a parallel
kit with the same API. The web app would gain nothing and the
mobile app would inherit Radix's web-only surface area
(focus management, popper, portals) without using any of it.

### C. Build native (Swift + Kotlin) apps

Rejected for the first surface. The portable layers we already
own cover ~70% of the mobile work; throwing them away to ship
two native codebases costs more than the four new packages.
Native is on the table later if Expo cannot meet a specific
need (e.g., a deep platform integration with no Expo module),
but that decision is its own ADR.

### D. Bare React Native (no Expo)

Rejected. **Expo (SDK 54, locked) is the runtime for `rubix/mobile`.**
Expo supplies every runtime dep we need (`react-native-svg`,
`react-native-reanimated`, the SSE polyfill, AsyncStorage,
`expo-sqlite`, `expo-secure-store`), gives us EAS for signed
iOS + Android builds without owning a Mac fleet, and removes
roughly two weeks of platform plumbing per surface. The workspace
packages (`starter-theme-tokens`, `starter-ui-kit-native`,
`starter-ui-sdui-native`, `starter-ui-dashboard-native`) are
Expo-agnostic by construction — they consume only React Native
APIs, not Expo modules — so the lock-in is confined to
`rubix/mobile/` (the app shell) and the two `expo-*` storage
choices. Revisiting Expo is a new ADR, not a Block-N deviation.

### E. Ship dashboards as a Tauri mobile build

Rejected. Tauri Mobile is alpha-grade for both iOS and Android
and would force every operator to install a custom binary instead
of using the store. The mobile audience is operators on
unmanaged phones — store install is non-negotiable.

### F. Navigation: React Navigation vs Expo Router

Chose **Expo Router**. File-based routing mirrors the Vite +
TanStack-Router pattern of `rubix/frontend`, reduces hand-wired
stack code, and is the Expo-recommended default. React Navigation
remains underneath; switching the surface API is reversible.

### G. State management: zustand vs Redux / Jotai / Context-only

Chose **zustand**. Already used in `@nube/starter-ui-core` for
the theme + i18n stores; mobile reuses those stores verbatim
and adopting any other library would mean two state stacks.
Redux is heavier without payoff for this surface; Jotai is
plausible but adds a second paradigm.

### H. Token storage: `expo-secure-store` vs SQLite + AES-GCM vs SQLCipher

Chose **`expo-secure-store`** for tokens (SQLite for everything
else). The earlier AES-GCM-in-SQLite scheme was rejected: RN has
no `crypto.subtle`, `expo-crypto` has no AES, and a third-party
crypto module would contradict the Expo-managed promise of
alternative D. The threat model (offline SQLite read by another
app) is covered by storing tokens directly in the platform
keychain. SQLCipher remains an escape hatch behind a future
security-review ADR.

### I. E2E framework: Maestro vs Detox

Chose **Maestro**. YAML flows, no native build step, runs against
the Expo-built binary as-is, and works on the same emulators the
dev loop already uses. Detox requires native build configuration
and is heavier to maintain in an Expo-managed project; the only
reason to revisit is if Maestro's CI runner becomes a bottleneck
or an interaction we need (deep gestures, native dialogs) is
better expressed in JS. Carried forward from
[THIN-SLICE Block 5](../scope/mobile/THIN-SLICE.md#block-5--dashboardspageidtsx--the-slice-itself).
