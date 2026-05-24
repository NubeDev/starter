# 2026-05-24 — `rubix/frontend` wired end-to-end

Session note for the `codeless/rubix-frontend-wire` branch. The job
landed the React-side chassis (`@nube/starter-client-react`,
`@nube/rubix-client-react`), the upstream SSE primitive
(`streamJson`), and wired `rubix/frontend` to `rubix-agent` over REST
+ SSE with cookie auth and a worked SSE-driven panel.

## Per-phase summary

### Phase A — SSE primitive + React chassis (3 commits)

- `feat(starter-client-ts) streamJson SSE primitive` — added
  `client/stream_json.ts` exposing `streamJson<T>(starter, path,
  opts?): AsyncIterable<T>`. EventSource when available with
  `withCredentials: true`, fetch + ReadableStream fallback,
  exponential backoff (1s base, 30s cap, 10% jitter), synthetic
  `reconnecting` envelope, honours `AbortController`.
- `feat(starter-client-react) scaffold + StarterClientProvider +
  QueryProvider` — new package at `packages/starter-client-react/`.
  Peer-deps react@^19 / react-dom@^19 / @tanstack/react-query@^5.
  Provider stack and the QueryClient defaults (30s stale, 5min gc,
  retry skips 401/403).
- `feat(starter-client-react) AuthProvider + useEventStream` —
  `AuthProvider` wraps a `me` query with a configurable
  `unauthenticatedSlot`; `useEventStream` bridges `streamJson` via
  `useSyncExternalStore` with stable `reconnect` identity.

### Phase B — typed React client + endpoint hooks (5 commits)

- `chore(rubix-client-ts) refresh openapi.json snapshot post-extensions`
  — regenerated `rubix/openapi.json` and the codegen output now the
  extensions admin routes are on master.
- `feat(rubix-client-ts) extensions REST + SSE endpoints` — added
  `endpoints/extensions.ts` (list/get/start/stop/restart/enable/disable
  all threading `readCsrfHeader`) and `streamExtensionEvents()`
  wrapping `streamJson` with `ExtensionEvent` as a discriminated
  union.
- `feat(rubix-client-react) scaffold + provider` — new package at
  `rubix/packages/rubix-client-react/`. `RubixClientProvider` mounts
  `StarterClientProvider` internally so a host only mounts one
  provider per layer.
- `feat(rubix-client-react) read hooks for system/users/mcp/extensions`
  — `useDiskUsage`, `useDbHealth`, `useFlowErrors`, `useUserList`,
  `useUserCreate`, `useUserDisable`, `useToolsList`,
  `useExtensionsList`, the five extension lifecycle mutations, and
  `useExtensionEvents` (the SSE bridge for the worked example).
- `feat(rubix-client-react) remaining hook families` — teams,
  tenants, clickhouse, flow-ops, undo, audit. Query-key convention
  documented in the package README: `['rubix', <family>,
  ...<discriminator>]`.

### Phase C — frontend wiring (4 commits)

- `feat(rubix-frontend) vite proxy + client singleton + i18n sync` —
  `vite.config.ts` proxies `/api/v1` + `/openapi.json` to
  `http://127.0.0.1:8088`; `src/lib/client.ts` exposes
  `getRubixClient()` from `VITE_RUBIX_BASE_URL`; `pnpm
  sync-catalogues` copies the `rubix.*` keys out of
  `rubix-spi/catalogues/`.
- `feat(rubix-frontend) providers wired + login route + auth guard`
  — `main.tsx` mounts
  `QueryProvider → RubixClientProvider → AuthProvider →
  RouterProvider`. `routes/login.tsx` is the layout-level auth guard
  (SCOPE OQ-4). `<ReactQueryDevtools/>` shows under
  `import.meta.env.DEV` (SCOPE OQ-3).
- `feat(rubix-frontend) real dashboard + extensions + users routes +
  error boundary` — the dashboard reads live `useDiskUsage()`;
  `/extensions` is the SSE worked example; `/admin/users` exercises
  create + disable + undo. `components/error-boundary.tsx` localises
  `RubixError.code` via react-intl using the synced catalogue keys.
- `test(rubix-frontend) playwright specs for auth + extensions +
  users` — three new specs at `e2e/auth.spec.ts`, `e2e/extensions.spec.ts`,
  `e2e/users.spec.ts`. Each documents the `mani run demo`
  prerequisite at the top of the file.

### Phase D — closing docs + CI + PR (this commit)

- `packages/starter-client-react/README.md` — present-tense docs on
  provider hierarchy, hook patterns, and query-key convention.
- `packages/starter-client-ts/README.md` — appended a Streaming
  section pointing at `streamJson`.
- `rubix/docs/design/frontend/README.md` — chassis architecture,
  client-ts vs client-react vs frontend layering, proxy + env
  conventions, SSE worked example.
- `.github/workflows/ci.yml` — the existing `pnpm` job already runs
  `pnpm -r run typecheck` and `pnpm -r run test`, which covers the
  two new packages automatically (they're in the workspace). The
  new `rubix-frontend-e2e` workflow runs the three Playwright specs
  against a backgrounded `rubix-agent`.

## Test counts (final)

| Package                       | Vitest               | Notes                                |
|-------------------------------|----------------------|--------------------------------------|
| `@nube/starter-client-ts`     | typecheck + test green | `stream_json.test.ts` added in A.1 |
| `@nube/starter-client-react`  | typecheck + test green | provider + auth + event-stream     |
| `@nube/rubix-client-ts`       | typecheck + test green | + `extensions.test.ts`             |
| `@nube/rubix-client-react`    | typecheck + test green | one `.test.tsx` per hook file      |
| `@nube/rubix-frontend`        | typecheck green        | 3 new Playwright specs             |

Test totals at the final HEAD of `codeless/rubix-frontend-wire` are
reproducible by running `pnpm -r run test` from the repo root.

## Operator-runnable manual flow

```
# Terminal 1 — bring the agent up (Postgres + ClickHouse + seed + agent)
cd rubix
mani run demo

# Terminal 2 — start the dev server
cd ..
pnpm --filter @nube/rubix-frontend dev

# Browser
open http://localhost:5173
#   1. The unauthenticated slot renders /login.
#   2. Sign in as op@example.com / rubix-dev-passwd.
#   3. The dashboard tile shows a *live* disk percentage from
#      useDiskUsage() — confirm by watching the number change after
#      `dd if=/dev/zero of=/tmp/x bs=1M count=100`.
#   4. Navigate to /extensions. Click "Start" on a row — the SSE
#      lifecycle event transitions the badge to "running" within ~1s
#      (look for status: 'open' on the connection badge).
#   5. Navigate to /admin/users. Create a user, then click Undo. The
#      table shrinks back via the useUndoLast() invalidation of
#      ['rubix'].
#   6. Sign out — the unauthenticated slot reappears.
```

If the SSE badge sticks on `reconnecting`, the agent is probably down
— check Terminal 1.
