# ADR 0004 — React Native mobile app reuses the chassis at the kit seam

**Status:** accepted, 2026-05-25. **Supersedes** [ADR 0002](./0002-backend-only.md).
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
  doesn't share cookies with `WebView`, so the same `AuthStrategy`
  abstraction must support both. It already does.

## Consequences

- The web `starter-ui-kit` is refactored to read tokens from the
  new `starter-theme-tokens` package. This is a non-behaviour
  change on web, validated by visual snapshot diff in CI.
- `starter-ui-sdui-react/renderer/` becomes one of two consumers
  of the registry; the registry surface (`registerRenderer`,
  `lookupRenderer`, `listRenderers`) is now load-bearing for two
  packages and must not be folded back into the web renderers.
- The mobile app enforces the reuse boundary via an ESLint
  `no-restricted-imports` rule listed in
  [docs/scope/mobile/APP-SHELL.md](../scope/mobile/APP-SHELL.md#import-lint).
- The app is **multi-instance**: one phone talks to many
  rubix-agent servers, with an on-device SQLite store as the
  source of truth for saved connections and per-connection
  bearer tokens. See
  [docs/scope/mobile/LOCAL-DB.md](../scope/mobile/LOCAL-DB.md).
  Backend must accept bearer tokens on routes that today require
  the session cookie. If a route requires cookie-only auth, it
  is a backend bug surfaced by the mobile app, not a mobile
  workaround.

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

Rejected for the first surface. Expo SDK 52+ supports the
runtime deps we need (`react-native-svg`, `react-native-reanimated`,
SSE polyfill, AsyncStorage), gives us EAS for builds, and removes
two weeks of platform plumbing. If we hit an Expo wall we eject;
the codebase is structured so ejecting changes only `rubix/mobile/`,
not the workspace packages.

### E. Ship dashboards as a Tauri mobile build

Rejected. Tauri Mobile is alpha-grade for both iOS and Android
and would force every operator to install a custom binary instead
of using the store. The mobile audience is operators on
unmanaged phones — store install is non-negotiable.
