# Mobile — app shell

The `rubix/mobile/` app is an Expo (SDK 52+) TypeScript app that
mirrors the layered chassis of
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
   `StarterClient` + `RubixClient` for it. Swapping the active
   connection rebuilds both clients and clears the query cache.
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
8. `SduiProvider` from `@nube/starter-ui-sdui-react` with
   `createHttpSduiTransport({ client: rubixClient })`.

If there is **no active connection** (fresh install), the
navigation root sends the user to `/connections/new` and the
client-using providers render a no-op placeholder until one is
selected.

Total ≤ 40 lines. If it grows, the providers are doing too much.

## Auth strategy

Cookies don't exist in RN's `fetch`; mobile uses bearer tokens.

`src/auth/strategy.ts` implements the `AuthStrategy` contract from
`@nube/starter-ui-core/auth`:

- `login(email, password)` → POST `/api/v1/auth/login`, persist the
  returned token in `@react-native-async-storage/async-storage`.
- `currentToken()` → read from AsyncStorage on cold start.
- `authHeader()` → `Authorization: Bearer <token>`.
- `logout()` → POST `/api/v1/auth/logout`, clear AsyncStorage.

**Backend prerequisite:** the agent must accept bearer tokens on the
same routes that today accept cookies. Confirm against
[`docs/design/auth/`](../../design/auth/) before scaffolding;
if it doesn't, that work blocks mobile and lands first as a
backend PR.

## Storage

| Concern | Mechanism |
|---|---|
| Saved connections to remote agents | **SQLite via `expo-sqlite`** — see [LOCAL-DB.md](./LOCAL-DB.md). The app is multi-instance: one phone, many agents. |
| Auth token (per connection) | SQLite `auth_token` table, AES-GCM-encrypted with a key from `expo-secure-store`. See [LOCAL-DB.md](./LOCAL-DB.md#secret-handling). |
| Theme + layout preferences | zustand `persist` middleware backed by `@react-native-async-storage/async-storage`. |
| i18n locale | Same. |
| React-Query cache | In-memory only. Reset on active-connection switch so server A's data never bleeds into server B (see [LOCAL-DB.md](./LOCAL-DB.md#how-the-rest-of-the-app-sees-it)). Persist later via `@tanstack/query-async-storage-persister`. |

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
- `react-intl`
- `zustand`
- `@react-native-async-storage/async-storage`
- `react-native-sse` — polyfill for `useEventStream`.
- `react-native-svg` — dashboard widgets.
- `react-native-reanimated`, `moti` — animation in widgets and
  the kit.

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
- `@nube/starter-ui-sdui-react/renderer`

Implementation: `eslint-plugin-import` `no-restricted-imports`
rule in `rubix/mobile/.eslintrc`. CI runs the lint on every PR.

The rule is what enforces the "reuse matrix" in
[REUSE.md](./REUSE.md); without it the boundary erodes silently.
