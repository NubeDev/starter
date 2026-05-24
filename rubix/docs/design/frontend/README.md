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

## Route map (after the frontend-surfaces branch)

The console surfaces after `codeless/rubix-frontend-surfaces` lands:

| Path                  | Mounts                                               | Source |
|-----------------------|------------------------------------------------------|--------|
| `/` (dashboard)       | feature tiles + boot intro                           | `src/routes/dashboard.tsx` |
| `/login`              | starter-ui-auth login                                | `src/routes/login.tsx` |
| `/flows`              | flow list (id, latest revision, deployed_at, supersession_count) | `src/routes/flows/index.tsx` |
| `/flows/$flowId`      | `<FlowCanvas registry={flowRegistry} readOnly />`    | `src/routes/flows/$flowId.tsx` |
| `/extensions`         | extensions table + SSE worked example                | `src/routes/extensions.tsx` |
| `/admin/access`       | `<AuthzAdmin>` (8 tabs)                              | `src/routes/admin/access.tsx` |
| `/admin/users`        | user admin panel + undo                              | `src/routes/admin/users.tsx` |
| `/admin/warehouse`    | `<WarehouseAdmin>` (rules · marts · retention · insights) | `src/routes/admin/warehouse.tsx` |
| `/settings`           | per-user prefs (locale, units, theme)                | `src/routes/settings.tsx` |

Left-nav groups the entries as **Home · Flows · Extensions · Admin · Settings**;
the admin section expands to access/users/warehouse. Every route is gated by
the cookie session — unauthenticated visits redirect to `/login`.

## Rubix-flavoured `NodeKindRegistry` boot wiring

`@nube/starter-ui-flow` ships a built-in registry covering the four kinds
shared across every starter-flow consumer (`ai-agent`, `tool-call`,
`trigger`, `branch`). Rubix builds its own registry on top of that at boot
in [`src/lib/flow-registry.ts`](../../../frontend/src/lib/flow-registry.ts):

1. Pull the starter built-ins via `builtinNodeKinds()`.
2. Replace the `ai-agent` entry with the rubix-specific renderer at
   [`src/lib/flow-nodes/ai-agent-node.tsx`](../../../frontend/src/lib/flow-nodes/ai-agent-node.tsx)
   so the canvas shows `skill_hint` as a label and `allowed_tools.length`
   as a badge — fields that only exist in the rubix flow schema.
3. Export the assembled registry as a singleton; `/flows/$flowId` passes
   it to `<FlowCanvas registry={flowRegistry} readOnly />`.

The override is **rubix-side only** — `@nube/starter-ui-flow` is never
patched. The same pattern lets the next consumer (a tenant console, a
flow-programmer app) layer its own node kinds without forking the package.

## Warehouse admin surface design

`/admin/warehouse` is the rubix-side equivalent of `<AuthzAdmin>` — a
tabbed shell with four panels, each a verb file ≤ 200 LOC under
[`src/components/admin/warehouse/`](../../../frontend/src/components/admin/warehouse/):

| Tab        | Panel                | Hooks |
|------------|----------------------|-------|
| Rules      | `rules-panel.tsx`    | `useClickhouseRulesList`, `useClickhouseRuleWrite` |
| Marts      | `marts-panel.tsx`    | `useClickhouseMartsList`, `useClickhouseMartCreate`, `useClickhouseMartDrop` |
| Retention  | `retention-panel.tsx`| `useClickhouseTablesList`, `useClickhouseRetentionSet` |
| Insights   | `insights-panel.tsx` | `useInsightsRulesList`, `useInsightsRuleCreate`, `useInsightsRuleEnable`, `useInsightsRuleDisable` |

The shell ([`warehouse-admin.tsx`](../../../frontend/src/components/admin/warehouse/warehouse-admin.tsx))
mirrors `<AuthzAdmin>`: starter-ui-kit `<Tabs>`, one panel per tab,
`<ErrorBoundary>` at the route level, `<Skeleton>` while `isLoading`,
`<EmptyState>` when a list returns zero rows. Every mutation flows
through a rubix-agent verb that writes a `undo_snapshots` row, so the
operator can roll back via `rubix.undo.last` or the panel's undo button.

i18n keys for this surface live under `admin.warehouse.*` in
`src/i18n/{en,es}.json`, copied from the `rubix-spi` catalogue at
build time by `pnpm sync-catalogues`.

## Toast error listener pattern

Uncaught `RubixError`s thrown by hooks need to surface as a localised
toast, not a blank screen. The pattern (target wiring lives at
`src/components/toast-error-listener.tsx`, mounted once in `main.tsx`
alongside `<ErrorBoundary>`):

```tsx
// rubix/frontend/src/components/toast-error-listener.tsx
import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { RubixError } from "@nube/rubix-client-ts";
import { useToast } from "@nube/starter-ui-kit"; // ← OQ-6: see status note

export function ToastErrorListener() {
  const qc = useQueryClient();
  const { toast } = useToast();
  useEffect(() => {
    const cache = qc.getQueryCache();
    const unsub = cache.subscribe((evt) => {
      if (evt.type === "updated" && evt.action.type === "error") {
        const err = evt.action.error;
        if (err instanceof RubixError) {
          toast({ kind: "error", titleKey: err.messageKey, params: err.params });
        }
      }
    });
    return unsub;
  }, [qc, toast]);
  return null;
}
```

Why the listener pattern and not per-hook handlers:

- One place to translate `RubixError.messageKey` through the i18n
  catalogue, so every uncaught query/mutation error reads the same.
- TanStack's `QueryCache.subscribe` covers both `useQuery` and
  `useMutation` failures without each consumer wiring `onError`.
- The component renders nothing — mounting it once is the whole API.

**Status:** SCOPE OQ-6 — `@nube/starter-ui-kit` does not yet export a
`Toast` primitive (see session note for the frontend-surfaces branch).
Until the upstream lands, only the `<EmptyState>` + `<Skeleton>` half of
Phase D.2 ships; the listener wires up the same day the primitive does.
