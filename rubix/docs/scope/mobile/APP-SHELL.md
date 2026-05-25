# Mobile — app shell

The `rubix/mobile/` app is an Expo (SDK 54, locked) TypeScript app
that mirrors the layered chassis of
[`docs/design/frontend/README.md`](../../design/frontend/README.md).

```
rubix/mobile                    app shell, screens, brand chrome
    │
    ▼
@nube/starter-ui-sdui-native    side-effect: registers RN renderers
    │
    ▼
@nube/starter-ui-kit-native     RN primitives, mirrors ui-kit
    │
    ▼
@nube/starter-theme-tokens      token values, shared with web
    │
    ▼
@nube/rubix-client-react        typed hooks per endpoint family
    │
    ▼
@nube/rubix-client-ts           typed REST + SSE
    │
    ▼
@nube/starter-client-react      providers + useEventStream
    │
    ▼
@nube/starter-client-ts         StarterClient, fetchJson, streamJson
```

Same arrow as the web app; only the top three boxes differ.

## Location

```
rubix/
  mobile/
    app.json                ← Expo config
    metro.config.js         ← workspace symlink resolution
    package.json
    src/
      App.tsx               ← provider stack only
      lib/client.ts         ← StarterClient + RubixClient factories (per connection)
      auth/strategy.ts      ← token-based AuthStrategy
      local-db/             ← SQLite multi-instance store — see LOCAL-DB.md
      connection/
        provider.tsx        ← derives clients from active connection
      theme/provider.tsx    ← RN ThemeProvider reading layout-prefs store
      i18n/provider.tsx     ← thin wrapper around react-intl
      navigation/           ← Expo Router file-based routes
        _layout.tsx
        index.tsx           ← redirects to active connection's last page, or /connections/new
        login.tsx           ← per-connection login
        connections/
          index.tsx         ← list, tap to activate, swipe to delete
          new.tsx           ← add a server (URL + label), probe /healthz, push to login
          [id].tsx          ← edit metadata, force re-login
        dashboards/
          [pageId].tsx      ← <SduiPage pageRef={...} />
          index.tsx         ← list of available dashboards on the active connection
        settings.tsx
```

Add `rubix/mobile` to the root [`pnpm-workspace.yaml`](../../../../pnpm-workspace.yaml).

## Provider stack

`App.tsx` mounts the same providers as
[`rubix/frontend/src/main.tsx`](../../../frontend/src/main.tsx)
minus the web-only pieces. Each provider is one import; no logic
in `App.tsx` beyond composition.

Order (outermost first):

1. `QueryClientProvider` (`@tanstack/react-query`).
2. `LocalDbProvider` (`src/local-db/provider.tsx`) — opens the
   SQLite database, runs migrations, exposes the handle. See
   [LOCAL-DB.md](./LOCAL-DB.md).
3. `ConnectionProvider` (`src/connection/provider.tsx`) — reads
   the active connection from the local DB and constructs
   `StarterClient` + `RubixClient` for it. Cache isolation is via
   **key namespacing**, not `queryClient.clear()` (clear aborts
   in-flight queries and flickers). See
   [Active-connection publication](#active-connection-publication)
   below for the mechanism that makes `starterQueryKey` see the
   live id. Stale per-connection entries are GC'd by React-Query's
   normal `gcTime`.
4. `StarterClientProvider` with the **token** `AuthStrategy`
   (`@nube/starter-ui-core/auth` + `src/auth/strategy.ts`),
   fed by `ConnectionProvider`.
5. `RubixClientProvider` (wraps it again — that's the existing
   convention, see [`docs/design/frontend/README.md`](../../design/frontend/README.md#chassis-architecture)).
6. `ThemeProvider` (`src/theme/provider.tsx`) — subscribes to the
   layout-preferences store from `starter-ui-core/theme-editor`
   and exposes resolved tokens via context.
7. `I18nProvider` (`src/i18n/provider.tsx`) — `react-intl` with the
   same `en.json` / `es.json` the web app uses.
8. `SduiProvider` from `@nube/starter-ui-sdui-react/headless` with
   `createHttpSduiTransport({ client: rubixClient })`. The
   `/headless` subpath is a precondition on the package — see
   [REUSE.md](./REUSE.md#reused--sdui-after-a-package-split-blocker)
   and [NEW-PACKAGES.md](./NEW-PACKAGES.md#precondition--sdui-react-package-split).

If there is **no active connection** (fresh install), the
navigation root sends the user to `/connections/new` and the
client-using providers render a no-op placeholder until one is
selected.

Total ≤ 40 lines. If it grows, the providers are doing too much.

## Auth strategy

Cookies don't exist in RN's `fetch`; mobile uses bearer tokens.
The app is multi-instance — one token per saved connection.

### Backend prerequisite — LANDED

> **Promotion note:** this whole subsection describes work that
> *has happened* before mobile code starts; on promotion to
> `docs/design/mobile/app-shell.md` it collapses to a one-line
> pointer at `docs/design/auth/token-issuance.md`.

Mobile needs a credentials→bearer issuance route. `POST
/api/v1/auth/login` is cookie-shaped (returns `{ csrf_token }`,
sets `starter_session`) and unusable from RN's cookie-less
`fetch`. The credentials→bearer counterpart is now live:
`POST /api/v1/auth/token` in `starter-auth-users`, accepting
`{ email, password, tenant_id? }` and returning `{ token,
expires_at, token_type: "Bearer" }`. Full payload + error
contract (incl. parity with login's `password_not_set` envelope
and the multi-membership `tenant_required` envelope) is recorded
in [`docs/design/auth/token-issuance.md`](../../design/auth/token-issuance.md).
Unblocks
[THIN-SLICE Block 4](./THIN-SLICE.md#block-4--rubixmobile-scaffold--login--provider-stack);
recorded as a consequence in
[ADR 0004](../../adr/0004-react-native-mobile-app.md#consequences).

### Strategy

`src/auth/strategy.ts` exposes a thin `useLogin()` hook on top of
the `tokenStrategy` from `@nube/starter-ui-core/auth`. The
upstream `tokenStrategy.login(client, { kind: 'token', token })`
**does not exchange credentials** — it installs an already-issued
bearer string on the client. The credentials POST is the app's
job. So mobile's login flow is two steps:

1. **Issue.** POST `{ email, password }` to
   `<base_url>/api/v1/auth/token` (the route added in the
   Backend prerequisite above) via a plain `fetch` against the
   active connection's `base_url`. On 200, write the returned
   `{ token, expires_at }` to `expo-secure-store` under the key
   `rubix.token.<connectionId>`.
2. **Install.** Call
   `tokenStrategy.login(client, { kind: 'token', token })` so the
   in-memory `StarterClient` has `Authorization: Bearer <token>`
   for the rest of the session.

Other strategy operations:

- `currentToken()` — read from `expo-secure-store` on cold start;
  if present, install on the client via step 2 only.
- `logout()` — POST `/api/v1/auth/logout`, delete the secure-store
  entry, call `tokenStrategy.logout(client)`.
- On any **401** mid-session: evict the token, preserve
  `connection_state.last_opened_page_ref`, route to per-connection
  login. See [401 mid-session](#401-mid-session) below for the
  re-resume contract. Refresh tokens are a follow-up — see
  [NON-GOALS.md](./NON-GOALS.md#technical).

> `tokenStrategy.login` is verified in
> [`packages/starter-ui-core/src/auth/strategy.ts`](../../../../packages/starter-ui-core/src/auth/strategy.ts)
> to throw on `{ kind: 'credentials' }`. The two-step shape above
> is therefore not a stylistic choice — it is what the existing
> contract requires.

### Active-connection publication

`starterQueryKey` from `@nube/starter-ui-core/query` is a **pure
helper** that prefixes a key array with an active-connection
scope. It does **not** read from React context on its own. Mobile
publishes the active id with this shape, in order of preference:

1. `ConnectionProvider` writes the id into a tiny
   `useActiveConnectionId()` zustand store (one module-level
   atom, no React context). All sites that build query keys call
   `starterQueryKey(useActiveConnectionId(), [...])`.
2. A thin `useStarterQuery` wrapper around `useQuery` reads the
   active id from the same store and prepends it, so call sites
   don't have to remember.

The store is module-level mutable but written by exactly one
place (`ConnectionProvider.setActiveId`), so two readers can
never disagree. This is the only place in the app where the
multi-instance guarantee is actually made; if you change it,
update this section.

### Server unreachable

The active connection's agent can go away at any moment (laptop
sleep, VPN drop, router reboot). Dashboard routes render in
these phases:

1. **Loading** — React-Query `isPending`. Skeleton.
2. **Cached-empty** — no cached page yet, no response yet after
   `queryTimeoutMs` (default 8s). Same skeleton, plus a non-modal
   "reaching `<label>`…" hint.
3. **Unreachable** — network error or 5xx after retry. Render an
   "agent unreachable" panel with the connection label and
   `base_url`, a Retry button, and a "Switch connection" link.
4. **Cached-stale** — a previously-loaded page is in the
   React-Query cache but the latest fetch failed. Render the
   cached payload with a small stale badge; never block the UI
   on a failed refetch when there is data to show.

No phase silently swallows the error; the panel and the badge
are both `accessibilityRole="alert"`.

### 401 mid-session

On any 401:

1. Evict the token from memory + secure-store.
2. Preserve `connection_state.last_opened_page_ref` and the
   current route in `state/pending-route.ts`.
3. Route to `/login` for the active connection.
4. After successful re-login, restore the pending route. If the
   server now returns 404 for that `pageRef` (page was deleted /
   renamed agent-side), fall back to `/dashboards` index and
   show a one-shot toast naming the missing page.

### Token expiry

`expires_at` from the issuance response is treated as **advisory
only** in v1. The app does **not** proactively refresh — there
is no refresh-token (deferred,
[NON-GOALS.md](./NON-GOALS.md#technical)). The app reacts to 401
as above. The expiry value is stored alongside the token in
secure-store so a future refresh implementation has the field
without a migration.

## Storage

| Concern | Mechanism |
|---|---|
| Saved connections to remote agents | **SQLite via `expo-sqlite`** — see [LOCAL-DB.md](./LOCAL-DB.md). The app is multi-instance: one phone, many agents. |
| Auth token (per connection) | **`expo-secure-store`** (platform Keychain on iOS, Keystore on Android), one entry per connection id. Plain bearer string — no JS-side encryption layer. Rationale and rejected AES-GCM-in-SQLite alternative in [LOCAL-DB.md](./LOCAL-DB.md#secret-handling). |
| Theme + layout preferences | zustand `persist` middleware backed by `@react-native-async-storage/async-storage`. |
| i18n locale | Same. |
| React-Query cache | In-memory. Per-connection cache isolation via key namespacing through `starterQueryKey` (see provider stack item 3). Persist later via `@tanstack/query-async-storage-persister`. |

## Environment

Expo public env vars only — `EXPO_PUBLIC_*` is the only prefix
shipped to the runtime bundle.

```
EXPO_PUBLIC_RUBIX_DEFAULT_BASE_URL=http://10.0.2.2:8088   # optional dev convenience
```

In production, **the app has no default backend URL**. Connections
live in the local SQLite store and the user adds them through
`/connections/new`. The env var above is a dev affordance only:
if set, the first launch seeds a single connection with that URL
so simulators don't require manual entry.

## Metro

`metro.config.js` must:

- `watchFolders` → repo root, so workspace packages reload on edit.
- `resolver.nodeModulesPaths` → both `rubix/mobile/node_modules`
  and the root `node_modules`, so symlinked deps resolve.
- `resolver.disableHierarchicalLookup` → `true`, to avoid
  resolving the same React twice.

This is the standard Expo + pnpm-workspace recipe; document the
exact config in the package PR.

## Required RN runtime deps

Installed in `rubix/mobile/package.json` (Expo doesn't ship these
by default):

- `@tanstack/react-query`
- `react-intl` — Expo SDK 54 / Hermes ships full Intl; no polyfill
  needed. Add `@formatjs/intl-pluralrules` only if a lower target is
  ever required.
- `zustand`
- `@react-native-async-storage/async-storage`
- `expo-sqlite` — multi-instance connection store. See
  [LOCAL-DB.md](./LOCAL-DB.md).
- `expo-secure-store` — per-connection bearer token.
- `react-native-sse` — provides `EventSource`. The starter SSE
  hook reads `(globalThis as { EventSource? }).EventSource`, so
  `App.tsx` must do
  `globalThis.EventSource = require('react-native-sse').default`
  at boot (or pass it explicitly to `streamJson`'s
  `eventSourceCtor`). It is not a drop-in polyfill.
- `react-native-svg` — dashboard widgets.
- `react-native-reanimated`, `moti` — animation in widgets and
  the kit. Reanimated 3 currently requires
  `'react-native-reanimated/plugin'` as the **last** entry in
  `babel.config.js`. As of Expo SDK 54 / Reanimated 4 the worklets-
  plugin migration has landed: the live config uses
  `react-native-worklets/plugin` as the last entry (see
  `rubix/mobile/babel.config.js` and the `react-native-worklets`
  dep in `rubix/mobile/package.json`).

## Theme + dark mode

`ThemeProvider` (provider stack item 6) subscribes to the
layout-preferences store from `starter-ui-core/theme-editor` and
resolves tokens against `starter-theme-tokens`. The web reader
reads `window.matchMedia('(prefers-color-scheme: dark)')`; on
RN the same store is initialised from
`Appearance.getColorScheme()` and updated via
`Appearance.addChangeListener`. Result: `useColorScheme` parity
with web, same store, no fork.

## Network reality

RN's `fetch` ignores CORS (it isn't a browser), but operators
sometimes terminate rubix behind a reverse proxy with a WAF that
allow-lists known User-Agents. The mobile app sends a fixed UA
(`rubix-mobile/<version> (<platform>)`); if a deployment proxy
rejects it, the proxy needs to allow-list that UA. There is no
in-app override.

## Import lint

A lint rule in `rubix/mobile` forbids importing:

- `@nube/starter-ui-kit`
- `@nube/starter-ui-flow`
- `@nube/starter-ui-export`
- `@nube/starter-ui-authz`
- `@nube/starter-ui-ai-builder`
- `@nube/starter-sdui-react`
- `@nube/starter-ui-dashboard`
- `@nube/starter-ui-core/layout`
- `@nube/starter-ui-core/theme-editor/utils/apply-theme`
- `@nube/starter-ui-core/theme-editor/utils/apply-preferences`
- `@nube/starter-ui-core/theme-editor/utils/generate-css`
- `@nube/starter-ui-core/theme-editor/utils/tailwind-css`
- `@nube/starter-ui-core/theme-editor/utils/parse-css-input`
- `@nube/starter-ui-core/theme-editor/transport`
- `@nube/starter-ui-sdui-react` (root) — must use `/headless`
  (see [REUSE.md](./REUSE.md#reused--sdui-after-a-package-split-blocker)).

The choice of lint tool (`eslint-plugin-import` `no-restricted-imports`
vs alternatives) is an implementation detail recorded in
[ADR 0004](../../adr/0004-react-native-mobile-app.md#consequences).
CI runs the lint on every PR.

**Named-export restriction caveat:** `no-restricted-imports`
restricts **paths**, not named exports. The REUSE.md guidance
that e.g. `@nube/starter-ui-core/auth` may only import
`tokenStrategy` (not `sessionStrategy`, not `externalStrategy`)
cannot be path-enforced as long as `starter-ui-core/auth` is a
single subpath. Two options, picked at Block 4:

1. **Code-review enforcement only**, with a CODEOWNERS rule that
   routes any mobile change touching imports to a reviewer who
   knows the matrix.
2. **Custom AST rule** via `no-restricted-syntax` matching
   `ImportDeclaration` source + `ImportSpecifier` name pairs.
   Heavier; only worth it if (1) bleeds.

Until starter-ui-core ships finer subpaths (e.g.
`/auth/token`), neither REUSE.md nor this section can claim full
automated enforcement of the named-export discipline.

The path-level rule is what enforces the rest of the
[reuse matrix](./REUSE.md); without it the boundary erodes
silently.

### Block-4 essential imports (quick reference)

A full allow/forbid list is in [REUSE.md](./REUSE.md). The
minimum a fresh implementer of Block 4 needs:

- **Allowed:** `@nube/starter-client-ts`, `@nube/starter-client-react`,
  `@nube/starter-ui-ir`, `@nube/rubix-client-ts`,
  `@nube/rubix-client-react`,
  `@nube/starter-ui-core/{auth, query, i18n}` (named exports only —
  see REUSE.md), the workspace `starter-ui-kit-native` +
  `starter-ui-sdui-native` + `starter-theme-tokens` packages.
- **Forbidden:** everything in the path list above plus everything
  in [REUSE.md §Explicitly NOT reused](./REUSE.md#explicitly-not-reused).
