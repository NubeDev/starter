# Scope — rubix-frontend-wire

## Goal

Wire `rubix/frontend` to `rubix-agent` end-to-end over REST + SSE, with auth, error handling, typed React Query hooks per endpoint family, and a worked SSE-driven panel. Land the React-side primitives as new sibling packages (`@nube/starter-client-react`, `@nube/rubix-client-react`) so every future starter/rubix frontend consumes the same patterns without copy-paste. The SSE primitive itself lands **upstream** in `@nube/starter-client-ts` per R2.

After this job:

- `rubix/frontend` boots, constructs one `RubixClient` (wrapping one `StarterClient`), wraps the app in `QueryClientProvider` + `AuthProvider` + `RubixClientProvider`.
- A `/login` route signs in against `rubix-agent`'s `/api/v1/auth/login`, the session + CSRF cookies persist via `credentials: include`, the rest of the app is gated by the auth guard.
- Existing pages (the boot-intro, dashboard tiles) are real — they call typed hooks like `useDiskUsage()`, `useToolsList()`, `useUserList()` which return `UseQueryResult<T, RubixError>`.
- A new `/extensions` route consumes the `/api/v1/extensions` admin surface (landed by the rubix-extensions-wire job) and renders a live list updated via SSE — proving the streaming primitive round-trips.
- `RubixError` propagates cleanly to a global error boundary that renders the localised Diagnostic message via `react-intl` (already a dep).
- The vite dev server proxies `/api/v1/*` and `/openapi.json` to `127.0.0.1:8088` so dev-mode CORS and cookie scope just work.

This is **wiring + the missing primitives**, not authoring. The clients and the frontend chassis exist; what's missing is: the React provider/hook layer, the SSE primitive, the auth flow, the proxy config, and the typed hooks per endpoint family. Long-term, no shortcuts: SSE lands in the right upstream package, hooks live in dedicated React-side packages.

## In scope

### Phase A — upstream: SSE primitive + `@nube/starter-client-react`

Two upstream pieces in `packages/`. R2 strictly — every future starter consumer benefits.

- **`packages/starter-client-ts/src/client/stream_json.ts`** (~120 lines, verb file) — `pub fn streamJson<T>(starter: StarterClient, path: string, opts?: StreamJsonOptions): AsyncIterable<T>`:
  - Uses native `EventSource` when available (browser) with `withCredentials: true` so the session cookie is sent.
  - Falls back to `fetch` + `ReadableStream` parsing when `EventSource` is missing (node test environment, future React Native).
  - Reconnects with exponential backoff (base 1s, cap 30s, jitter 10%) on connection loss; emits a synthetic `{ event: "reconnecting", attempt }` event on each retry so consumers can render UI state.
  - Honours `AbortController` via `opts.signal`.
  - Parses each `data:` frame as JSON, yields the parsed value typed as `T`. SSE event types other than `message` are surfaced via `opts.onEvent(eventType, data)` (optional).
  - CSRF handling: SSE has no body, so no CSRF token is required by the server; the cookie alone authenticates.
- **`packages/starter-client-ts/src/client/stream_json.test.ts`** — vitest using `EventSource` polyfill + a fetch-mock; asserts parsing, reconnect, abort.
- **`packages/starter-client-ts/`** — `package.json` exports `streamJson` from the barrel.
- **`packages/starter-client-react/`** — new package.
  - `package.json` — name `@nube/starter-client-react`, peer-deps on `react@^19`, `react-dom@^19`, `@tanstack/react-query@^5`, depends on `@nube/starter-client-ts` workspace.
  - `src/index.ts` barrel.
  - `src/provider/starter-client-provider.tsx` (~80 lines) — `StarterClientProvider({ client, children })` and `useStarterClient()`.
  - `src/provider/query-provider.tsx` (~40 lines) — thin wrapper around `QueryClientProvider` with sensible rubix-flavoured defaults (`staleTime: 30s`, `gcTime: 5min`, `retry: (failureCount, error) => !RubixError.is(error, 401, 403) && failureCount < 3`).
  - `src/provider/auth-provider.tsx` (~150 lines) — wraps the children with an auth context built on `useQuery` against `starter.me()`. Exposes `useAuth() -> { user, login, logout, isAuthenticated }`. Login mutates state via `useMutation` calling `starter.login()`, then invalidates the `me` query. Logout calls `starter.logout()` then resets the query cache. The provider renders a configurable fallback (`<unauthenticated />` slot) when `me()` returns 401.
  - `src/hooks/use-event-stream.ts` (~120 lines) — `useEventStream<T>(path, opts)` returns `{ data, error, status, reconnect }` where `status` is `connecting | open | reconnecting | closed | error`. Internally calls `streamJson` and bridges via a React `useSyncExternalStore`. Stable identity for `reconnect`, cleans up on unmount.
  - `tests/auth-provider.test.tsx`, `tests/query-provider.test.tsx`, `tests/use-event-stream.test.tsx` — vitest + react-testing-library. The event-stream test uses MSW or a custom mock EventSource.
- **`packages/starter-client-ts/README.md`** — append a "Streaming" section pointing at `streamJson`. **`packages/starter-client-react/README.md`** — new, present-tense, the four primitives, when to use which.

### Phase B — rubix-side: typed SSE wrappers + `@nube/rubix-client-react`

The rubix client adds typed SSE methods and a new sibling react package mirroring the starter-client-react shape.

- **`packages/rubix-client-ts/src/endpoints/extensions.ts`** (new) — REST + SSE endpoints for the extensions admin surface (landed by the rubix-extensions-wire job, assumed merged before this job starts; if not, raise BLOCKED at stage B.1):
  - `extensionsList()` — `GET /api/v1/extensions`.
  - `extensionsGet(id)` — `GET /api/v1/extensions/<id>`.
  - `extensionsStart(id)`, `extensionsStop(id)`, `extensionsRestart(id)` — POST endpoints with CSRF.
  - `extensionsEnable(id)`, `extensionsDisable(id)` — POST with CSRF.
  - `streamExtensionEvents(opts?): AsyncIterable<ExtensionEvent>` — wraps `streamJson` against `/api/v1/extensions/events`. `ExtensionEvent` is a discriminated union (`lifecycle | log | error`) typed from the codegen output.
  - Sibling `extensions.test.ts` covering each REST method and the stream wrapper.
- **`packages/rubix-client-ts/src/endpoints/audit.ts`** — typed wrapper around the existing `changelog` reads + a `streamAuditTail()` for live changelog updates if rubix-agent exposes a corresponding SSE endpoint (confirm at B.1; if it doesn't, defer the audit stream to a follow-up).
- **`packages/rubix-client-ts/bin/codegen.mjs`** — already wired; running it should pick up the new routes from the post-extensions `rubix/openapi.json` snapshot. If the snapshot is stale, refresh it as part of B.1.
- **`packages/rubix-client-react/`** — new sibling package.
  - `package.json` — name `@nube/rubix-client-react`, peer-deps as above, depends on `@nube/rubix-client-ts` + `@nube/starter-client-react`.
  - `src/index.ts` barrel.
  - `src/provider/rubix-client-provider.tsx` — `RubixClientProvider({ client, children })` and `useRubixClient()`. The provider wraps `StarterClientProvider` internally so a single import covers both.
  - `src/hooks/system.ts` — `useDiskUsage(opts)`, `useDbHealth(opts)`, `useFlowErrors(opts)`. Each returns `UseQueryResult<DiskUsageResponse, RubixError>`. Query keys: `["rubix", "system", "disk"]` etc.
  - `src/hooks/users.ts` — `useUserList()`, `useUserCreate()`, `useUserDisable()`. Mutations call the underlying methods, invalidate `["rubix", "users"]` on success, surface `RubixError` typed.
  - `src/hooks/teams.ts`, `src/hooks/tenants.ts`, `src/hooks/clickhouse.ts`, `src/hooks/flow-ops.ts`, `src/hooks/undo.ts`, `src/hooks/mcp.ts`, `src/hooks/extensions.ts`, `src/hooks/audit.ts` — same pattern per endpoint family.
  - `src/hooks/use-extension-events.ts` — `useExtensionEvents(opts?)` returns `{ events: ExtensionEvent[], status, reconnect }`. Internally calls `useEventStream` against `streamExtensionEvents`.
  - Tests: one `.test.tsx` per hook file covering happy path + error + invalidation.
- **Query-key convention** — documented in the package README. All keys start with `["rubix", <family>, ...]`. The convention enables `queryClient.invalidateQueries(["rubix"])` to nuke everything on logout.

### Phase C — `rubix/frontend` wiring

The frontend integration. Five concrete pieces:

- **`vite.config.ts`** — add `server.proxy` for `/api/v1` and `/openapi.json` to `http://127.0.0.1:8088`. Document the env var `VITE_RUBIX_BASE_URL` for non-default ports.
- **`src/lib/client.ts`** (new, ~30 lines) — `getRubixClient()` constructs the singleton from `VITE_RUBIX_BASE_URL` (default `""` in dev because vite proxies, default the actual URL in prod). Same shape as the per-app pattern documented in `@nube/starter-client-react`'s README.
- **`src/main.tsx`** — wrap the existing TanStack Router app in:
  ```
  <QueryClientProvider>          (from @nube/starter-client-react)
    <RubixClientProvider client={...}>
      <AuthProvider unauthenticatedSlot={<LoginRoute />}>
        <RouterProvider router={router} />
  ```
  No new routing primitives; existing TanStack Router stays.
- **Auth route** — new `src/routes/login.tsx` rendering a login form (email/password), calling `useAuth().login(...)`, redirecting to `/` on success. The route uses the existing `starter-ui-kit` form primitives — no new UI components.
- **Existing pages get real data** — three concrete swaps:
  1. The boot-intro / dashboard tile that today renders mock data → `useDiskUsage()` returning the live percent.
  2. A new `/extensions` route → renders `useExtensionsList()` table + `useExtensionEvents()` live status badges + start/stop buttons calling `useExtensionStart/Stop/Restart`. This is the worked SSE example.
  3. A new `/admin/users` route → `useUserList()` + `useUserCreate()` form + `useUserDisable()` action. Proves the write + undo flow over real endpoints.
- **Error boundary** — `src/components/error-boundary.tsx` catches uncaught `RubixError` from queries/mutations, renders the localised `code` via `react-intl` (the catalogue keys are already in `rubix-spi`'s en/es JSON, exposed to the frontend via the existing `src/i18n/{en,es}.json` — confirm overlap; if some `rubix.*` keys are missing client-side, add them in this stage).
- **Playwright** — extend the existing e2e suite under `e2e/` with three new specs:
  - `auth.spec.ts` — login → dashboard renders → logout → login route shows again.
  - `extensions.spec.ts` — list renders, start button transitions row state via SSE within 5s.
  - `users.spec.ts` — create flow lands, list reflects, undo via the UI button reverts.

### Phase D — closing: docs, tests, PR

- **`packages/starter-client-react/README.md`** + **`packages/rubix-client-react/README.md`** — present-tense, the provider hierarchy, the hook patterns, the query-key convention.
- **`packages/starter-client-ts/README.md`** — append the streaming section.
- **`rubix/docs/design/frontend/README.md`** — new design doc (rubix/docs/design/ already has a `frontend/` placeholder per the earlier inventory): the chassis architecture, where each layer lives (client-ts vs client-react vs frontend), the proxy + env conventions, the SSE worked example.
- **`rubix/docs/sessions/<today>-frontend-wired.md`** — closing session note: per-phase commits, the operator-runnable manual flow (boot agent → boot frontend → login → see live data → start an extension → see the row update via SSE).
- **CI** — extend the existing pnpm + playwright workflows to cover the two new packages and the three new e2e specs. The playwright job needs `mani run demo` (or equivalent) in the background to provide the backend.
- **PR** — one PR off `codeless/rubix-frontend-wire` with phase-by-phase commits.

## Out of scope

- **A second frontend.** The work happens in `rubix/frontend`. `test-ui-5` (which already exists and was touched by the extensions-wire job) is left alone; if adoption is wanted, that's a follow-up.
- **SDUI page rendering.** The frontend gets typed hooks for SDUI when the backend exposes them (Goal 1, still stubbed). Until then, the existing static dashboard tiles stay.
- **WebSocket transport.** SSE is enough for the worked example. WS is a future job if a bidirectional use case shows up.
- **OAuth login.** Email + password only, matching the rubix-agent surface. OAuth lands when the backend exposes it.
- **Theme editor wiring.** `starter-ui-kit/theme-editor` exists and was touched in PR #32's UI follow-up commit; this job doesn't re-wire it. Theme persistence already works via `client.themeGet/Save` in `starter-client-ts`.
- **i18n catalogue authoring.** The frontend already has en/es JSON files; this job adds any `rubix.*` key entries the error boundary needs but doesn't author new domain copy.
- **Multi-tenant tenant switcher.** Out of scope. Single-tenant flow.
- **Live LLM in the frontend.** The chat surface (if any) is not in scope.
- **Storybook / component-library docs.** The hooks have unit tests; storybook stays where it is.
- **No new UI primitives in `@nube/starter-ui-kit`.** If the login form or the extensions table needs a primitive the kit doesn't have, raise an upstream issue and ship a minimal hand-rolled version in `rubix/frontend` with a TODO referencing the upstream issue. Don't grow the kit in this job.

## Constraints

- **R1 — One verb per file.** TS files ≤ 200 lines hard. Each hook is its own file under `src/hooks/`; provider components are their own files under `src/provider/`. The "one hook per file" rule mirrors the "one verb per file" Rust convention.
- **R2 — Upstream-first.** The SSE primitive lands in `starter-client-ts` before `rubix-client-ts` consumes it. `starter-client-react` lands before `rubix-client-react` depends on it. Phases A.1 and A.2 must commit and be reviewable before Phase B begins.
- **R3 — Doc-tier rule.** Source comments link `docs/design/<area>/README.md` only — not SCOPE / sessions / NEW-SESSION / FILE-LAYOUT. Applied to TS source the same way it's applied to Rust source.
- **R4 — Errors are typed.** Every hook returns `UseQueryResult<T, RubixError>` (or `UseMutationResult`). No raw `Error` types leak through. `RubixError` carries the `code` from the Diagnostic.
- **R5 — Catalogue files.** Any `rubix.*` key the error boundary surfaces must exist in both `rubix-spi/catalogues/en.json` and `es.json` (the source of truth, copied into `rubix/frontend/src/i18n/{en,es}.json` via the existing build step — confirm the copy step exists; if not, add one in Phase C).
- **R6 — Tests live with the code.** Each hook gets a sibling `.test.tsx`; each provider gets one. Playwright specs in `e2e/`.
- **Commit messages.** `feat(starter-client-ts):` for the SSE primitive, `feat(starter-client-react):` for the new package, `feat(rubix-client-ts):` for the extensions typed wrappers, `feat(rubix-client-react):` for the hooks, `feat(rubix-frontend):` for the integration, `docs+test:` for the closing.
- **No new `index.ts` barrel-only files.** Every barrel re-exports something. If a directory holds one file, the file replaces the barrel.
- **No `any` types.** TS strict mode + `noImplicitAny`. The codegen-generated types are the source of truth for wire shapes.

## Open questions

1. **Does `rubix-agent` today expose `/api/v1/extensions/events` as SSE?** If yes, this job consumes it as the SSE worked example. If no (because the extensions-wire job either hasn't shipped or didn't include the SSE route), the SSE worked example shifts to a synthetic stream that this job adds to rubix-agent — but that's out of scope here. If the extensions SSE doesn't exist, raise BLOCKED at stage B.1.
2. **What's the rubix MessageKey to `react-intl` mapping shape?** Today `rubix/frontend/src/i18n/{en,es}.json` exists; confirm at C.1 whether the build copies `rubix-spi/catalogues/*.json` into it or whether they drift. If drift, propose a `pnpm sync-catalogues` script.
3. **TanStack Query devtools — include in dev, strip in prod?** Default yes. Add `<ReactQueryDevtools />` behind `import.meta.env.DEV`.
4. **Auth guard — route-level or layout-level?** Layout-level (one wrapping component around the authed routes) is simpler. Default to that. Decide at C.2.
5. **Login redirect target — fixed `/` or `?returnTo=...` honoured?** Default to `?returnTo=...` if present, else `/`. Document in the auth-provider README.
6. **Should `RubixClient` be a hook-returned singleton (`useRubixClient()`) or a context-passed instance?** Default: context-passed via `RubixClientProvider`. The hook returns the context value. Consistent with TanStack Query's pattern.
7. **Service worker / offline?** Out of scope, but call it out in the design doc as a future concern.
8. **Playwright in CI — needs a backend.** Plan: a separate workflow job that boots `mani run demo` in the background, waits for `/healthz`, runs playwright. If the existing workflow already does this for the `auth/extensions/users` specs, reuse the pattern.

## References

- `packages/starter-client-ts/` — the existing fetch client + the SSE primitive's new home.
- `packages/rubix-client-ts/` — the existing typed-wire layer.
- `rubix/frontend/` — the consumer; React 19 + Vite + TanStack Router + TanStack Query already deps.
- `rubix/frontend/package.json` — confirms `@nube/rubix-client-ts` is already a workspace dep.
- `starter-extensions/packages/starter-ext-ui/` — the existing ExtensionHostProvider; the auth provider shape borrows this pattern.
- `rubix/docs/sessions/2026-05-24-smoke-test-pr32.md` — the operator-runnable e2e patterns the closing session note mirrors.
- `rubix/docs/design/agent/README.md` — backend wiring picture this frontend connects to.
- `rubix/SCOPE.md` — R1–R13; R2 (upstream-first) and R5 (catalogues) are the load-bearing rules here.
- `rubix/HOW-TO-CODE.md`, `rubix/FILE-LAYOUT.md`, `rubix/NEW-SESSION.md`.
