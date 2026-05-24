# `rubix/frontend` — design

The Rubix Agent Console is a React + Vite SPA that talks to
`rubix-agent` over REST (mutations + reads) and SSE (live status). It
is the lighthouse consumer of every TypeScript chassis package in the
workspace and the reference implementation operators eyeball before
building their own.

## Chassis architecture

Four layers, bottom-up, each only depending on the one below it:

```
rubix/frontend                 app shell, routes, branded chrome
    │
    ▼
@nube/rubix-client-react       typed hooks per endpoint family
    │
    ▼
@nube/rubix-client-ts          typed REST + SSE methods, RubixError
    │
    ▼
@nube/starter-client-react     StarterClientProvider, QueryProvider,
                               AuthProvider, useEventStream
    │
    ▼
@nube/starter-client-ts        StarterClient, fetchJson, streamJson,
                               readCsrfHeader, StarterError
```

Why this many layers:

- `starter-client-ts` is transport-only. Non-React consumers (CLI
  scripts, server-side codegen, SSR experiments) import it directly.
- `starter-client-react` is the React adapter. It owns *no* endpoint
  knowledge — just the providers + the `useEventStream` bridge. Every
  future starter-based frontend reuses it.
- `rubix-client-ts` is the typed HTTP/SSE client over `rubix-agent`.
  Generated types live in `src/generated/`; endpoint files are hand
  curated and ~5 lines each because the upstream `fetchJson`
  helpers already cover URL build / credentials / error throw.
- `rubix-client-react` mounts a `RubixClientProvider` (which wraps
  `StarterClientProvider` internally) and exports one hook per
  endpoint family. All keys live under `['rubix', <family>, …]`.
- `rubix/frontend` is route code, brand chrome, and the SDUI surface.
  It constructs **one** `RubixClient` in `src/lib/client.ts`, mounts
  the three providers in `main.tsx`, and never reaches around them.

The split exists so the next starter-based frontend (think a tenant
console, an alerts inbox, a separate flow-programmer app) can stand
up by re-running the same provider stack and writing nothing more
than route code. None of the layers below `rubix/frontend` know
anything brand-specific — they are reusable across every rubix-like
agent we may run.

## Client vs client-react vs frontend

| Concern                   | Lives in              |
|---------------------------|-----------------------|
| Base URL, fetch override  | `starter-client-ts`   |
| Cookie auth + CSRF echo   | `starter-client-ts`   |
| SSE primitive (`streamJson`) | `starter-client-ts` |
| TanStack Query defaults   | `starter-client-react` |
| `useAuth`, `useEventStream` | `starter-client-react` |
| Typed REST verbs          | `rubix-client-ts`     |
| Typed SSE wrappers        | `rubix-client-ts`     |
| `use<Family>` hooks       | `rubix-client-react`  |
| Routes, branding, copy    | `rubix/frontend`      |

A rule of thumb: if a change would help **every** rubix-like
frontend, it lives in one of the lower layers. If it only helps the
Rubix Console, it lives in `rubix/frontend`.

## Vite proxy + env conventions

The dev server proxies all backend traffic so the browser and the
fetched API share an origin — no CORS, no third-party cookies, the
session cookie just works:

```ts
// rubix/frontend/vite.config.ts
server: {
  proxy: {
    "/api/v1":      { target: "http://127.0.0.1:8088", changeOrigin: true },
    "/openapi.json":{ target: "http://127.0.0.1:8088", changeOrigin: true },
  },
},
```

`getRubixClient()` reads `VITE_RUBIX_BASE_URL`:

- **Dev** — leave it empty. Vite proxies on the same origin.
- **Prod** — set to the agent's public origin
  (`https://agent.example.com`). The client switches to absolute URLs
  and the session cookie travels because `credentials: "include"` is
  baked into `fetchJson`.
- **Tests / Storybook** — set to a mock server origin and the same
  code paths exercise.

i18n catalogues from `rubix-spi` are copied into `src/i18n/` by
`pnpm sync-catalogues` so missing-key warnings surface at build time,
not first paint.

## SSE worked example — `/extensions`

The extensions admin route is the worked example for the whole
chassis. End-to-end it threads through every layer:

```tsx
// rubix/frontend/src/routes/extensions.tsx
import {
  useExtensionsList,
  useExtensionEvents,
  useExtensionStart,
} from "@nube/rubix-client-react";

export function ExtensionsRoute() {
  const { data: rows } = useExtensionsList();
  const { events, status, reconnect } = useExtensionEvents();
  const start = useExtensionStart();

  // Merge the snapshot from REST with live transitions from SSE.
  const merged = mergeWithEvents(rows ?? [], events);

  return (
    <div>
      <ConnectionBadge status={status} onReconnect={reconnect} />
      <ExtensionTable
        rows={merged}
        onStart={(mount) => start.mutate({ mount })}
      />
    </div>
  );
}
```

What each layer contributes:

1. `streamJson<ExtensionEvent>(starter, "/api/v1/extensions/events")`
   in `starter-client-ts` opens an `EventSource` against the agent
   (the request goes through the Vite proxy → 127.0.0.1:8088) and
   parses each `data:` payload as `ExtensionEvent`.
2. `streamExtensionEvents()` in `rubix-client-ts` is a one-line
   wrapper that pins the path and the union type, so the iterator is
   `AsyncIterable<ExtensionEvent>` (a discriminated union of
   `lifecycle | log | error`).
3. `useEventStream` in `starter-client-react` bridges the iterable
   into React via `useSyncExternalStore`, exposes a stable
   `reconnect` identity, and tears down on unmount through an
   `AbortController`.
4. `useExtensionEvents` in `rubix-client-react` calls
   `useEventStream` with the factory above and returns
   `{ events, status, reconnect }`. The query key is
   `['rubix', 'extensions', 'events']`.
5. `useExtensionStart` is a TanStack mutation that calls
   `extensionsStart` (which echoes the `starter_csrf` cookie via
   `readCsrfHeader`) and invalidates `['rubix', 'extensions']` on
   success. The next SSE `lifecycle` event reconciles the row state.

Operator-runnable manual flow lives in
[`docs/sessions/2026-05-24-frontend-wired.md`](../../sessions/2026-05-24-frontend-wired.md).
