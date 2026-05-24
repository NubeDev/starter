# Workflow — rubix-frontend-wire

## Sequencing

15 stages across four phases. Strict order: A (upstream SSE + starter-client-react) → B (rubix-side typed wrappers + rubix-client-react) → C (frontend wiring + routes + e2e) → D (closing). Three REVIEW gates.

This job is **upstream-heavy** for the React-side primitives — same R2 discipline as every prior rubix job. The two new sibling React packages (`@nube/starter-client-react`, `@nube/rubix-client-react`) are the long-term home for every future React consumer's primitives.

## Per-stage discipline

### Phase A — upstream React-side primitives

Three commits. A.1 lands SSE. A.2 lands the starter-react scaffold + Query + StarterClient providers. A.3 lands Auth + useEventStream.

1. **`streamJson` is a verb file.** Single export, ≤ 200 lines TS. EventSource browser path + fetch/ReadableStream node-path. Don't over-abstract; one reconnect strategy, one auth pattern.
2. **`@nube/starter-client-react` peer-deps React 19 and TanStack Query 5.** Already what `rubix/frontend` uses. Pin majors; floats on minors.
3. **AuthProvider is the most opinionated piece.** It owns the `me` query, the login/logout mutations, and the unauthenticated-slot rendering. The unauthenticated slot is configurable so consumers can render their own login route — don't ship a default login form in the upstream package.
4. **`useEventStream` uses `useSyncExternalStore`.** Not `useState` + `useEffect`. The subscribe/getSnapshot pattern is what React expects for external mutable sources; using `useState` invites stale closures and lost events.
5. **Tests use MSW or a custom mock EventSource.** Don't rely on a live SSE server in vitest. The polyfill exists; use it.
6. `pnpm --filter @nube/starter-client-ts typecheck && test` and `pnpm --filter @nube/starter-client-react typecheck && test` green per stage.

### Phase B — rubix-side typed wrappers + react hooks

Five commits. B.1 confirms the extensions-wire dependency. B.2 adds the extensions endpoint family + SSE wrapper. B.3 scaffolds rubix-client-react. B.4 + B.5 land all the hooks.

1. **B.1 is gated on rubix-extensions-wire merging.** If that PR isn't on master, BLOCKED. The extensions admin REST surface and the SSE event-stream endpoint are dependencies. Working around this would mean adding the routes in this job — out of scope.
2. **Regenerate `rubix/openapi.json` BEFORE writing the extensions endpoint.** The codegen output must match the actual route surface. If running `snapshot-openapi.sh` produces a diff, commit it first (or as part of B.1 if non-trivial; otherwise skip-no-diff).
3. **Don't duplicate JSON-RPC envelope logic.** The existing `mcp.ts` already has the envelope builder; lift it into a shared helper if Phase B's extensions endpoint needs it. If extensions admin is plain REST (it should be — admin endpoints don't speak JSON-RPC), don't touch.
4. **Each hook file is one family.** `src/hooks/users.ts` has `useUserList`, `useUserCreate`, `useUserDisable`. Don't fan them out further per-verb; one file per family is the long-term-sane unit.
5. **Query keys follow `["rubix", family, ...]`.** Documented in the README. `queryClient.invalidateQueries({queryKey: ["rubix"]})` nukes everything; `queryClient.invalidateQueries({queryKey: ["rubix", "users"]})` nukes one family. Tests assert the invalidation behaviour.
6. **Mutations invalidate, not refetch directly.** TanStack Query best practice — invalidate the affected query key, let consumers refetch when they remount. The test asserts invalidation, not refetch order.
7. `pnpm --filter @nube/rubix-client-ts typecheck && test` and `pnpm --filter @nube/rubix-client-react typecheck && test` green per stage.

### Phase C — frontend integration

Four commits. C.1 lays the foundation (proxy + singleton + i18n sync). C.2 wires providers + auth. C.3 builds the user-visible routes. C.4 adds e2e.

1. **Vite proxy is the only dev-mode wiring needed.** Production builds rely on the deployment putting the backend at the same origin or behind a reverse proxy. Don't add CORS handling in the frontend.
2. **The client singleton lives in `src/lib/client.ts`, not as a React-context dependency.** Components access it via `useRubixClient()` which reads from the provider; the singleton is just for the provider to construct from.
3. **i18n sync is a build-time copy, not runtime fetch.** The catalogue files are static. If the build step isn't there, add a `pnpm sync-catalogues` script and call it from `predev`/`prebuild` hooks. Don't fetch at runtime — locale switching at boot must be deterministic.
4. **Auth guard is layout-level.** A single `<AuthGuard>` wrapper around the authed route subtree. Per-route guards multiply by route count; this job ships 3-4 routes and a single guard is correct.
5. **The dashboard real-data swap is small.** One mock removed, one `useDiskUsage()` added, one loading state, one error state. Don't redesign the dashboard.
6. **The extensions route is the worked SSE example.** Document that in the route file's header comment. Future contributors learn the SSE pattern from this file.
7. **Error boundary uses `react-intl` for the message render.** The `code` is the message key; `react-intl` resolves it against the locale at render. If a key is missing, `react-intl` renders the key itself (don't fall back to a generic message — the missing-key surface is the signal to add it to the catalogue).
8. **Playwright specs assume a running backend.** Each spec's header documents the prerequisite. Don't try to spawn the backend from playwright — that's CI's job, not the spec's.

### Phase D — closing

One stage. Three artifacts (READMEs, design doc, session note) + CI extension + PR. Treat this stage as "make the work findable in a year." Stale READMEs are how teams forget which package contains what.

1. **The four READMEs (starter-client-ts streaming section, starter-client-react, rubix-client-react, frontend design doc) link to each other.** A future contributor reading `rubix/docs/design/frontend/README.md` should be one click from each package's README.
2. **Session note follows the goals-2-4-3 shape.** Per-phase summary, operator-runnable manual flow, test counts.
3. **CI changes are minimal.** Add the two new packages to the existing pnpm typecheck/test matrix; add a new playwright job (or extend the existing one) that boots the backend via `mani run demo` in the background. Don't reinvent CI.

## Anti-patterns specific to this job

- **Don't put React hooks in `@nube/rubix-client-ts`.** That package stays React-free. Hooks belong in `@nube/rubix-client-react`.
- **Don't add WebSocket support.** SSE is enough. Future bidirectional needs are a separate job.
- **Don't wire test-ui-5 in this job.** `rubix/frontend` is the consumer. test-ui-5 adoption is a follow-up.
- **Don't add OAuth login.** Email + password only.
- **Don't grow `@nube/starter-ui-kit` in this job.** If the kit lacks a primitive, hand-roll a minimal version in `rubix/frontend` with an upstream TODO.
- **Don't fetch i18n catalogues at runtime.** Build-time copy only.
- **Don't add `any` types.** Strict TS throughout.
- **Don't write a generic "fetch" hook that hides the underlying RubixClient method.** Each hook should be one method's wrapper; the abstraction layer is the hook, not a meta-fetcher.
- **Don't list paths with brace expansion in handovers.** Trips diff-verify.
- **Don't list a path under Done that the stage didn't modify.** Same trap.
- **Don't `--no-verify`, don't `--force`.**

## REVIEW gate behaviour

Three gates: A↔B, B↔C, C↔D. Each commits and pushes the stages that led to it; the gate itself commits nothing.

At each gate, the handover must include:

- One-line title per commit in the phase.
- `pnpm typecheck` + `pnpm test` counts per package.
- For A↔B: confirmation that the two new React packages typecheck against React 19 + TanStack Query 5 without warnings.
- For B↔C: confirmation that `rubix-client-react`'s hooks compile against a live `rubix-agent` (one curl-and-paste evidence line proving the extensions SSE wrapper works).
- For C↔D: one operator-runnable manual flow demonstrating the full chain.
- Any deviation from SCOPE.
- Open Questions evidence where the stage answered one.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in order. Do **not** rename or reorder them.

1. `checks` — run the stage's verify list. Every step must pass.
2. `docs` — update `handover.md` for the next stage and the active session doc.
3. `git` — stage the changes, commit with the message `stage N: <one-line title from template.yaml>`, push to `codeless/rubix-frontend-wire`.

REVIEW gate stages mark `git` as `skipped — gate-only`. B.1 marks `git` as `skipped — no diff` if the snapshot refresh produced no changes. Never `--force`, never `--no-verify`.

## Hard rules (repeated)

- One verb per file. TS ≤ 200 lines hard, ~80 typical. One hook per file under `src/hooks/`.
- Source comments link `docs/design/<area>/README.md` only.
- No phasing markers in code.
- Upstream-first (R2). starter-client-ts SSE primitive lands before rubix-client-ts consumes; starter-client-react lands before rubix-client-react depends on it.
- Errors are typed. `UseQueryResult<T, RubixError>` everywhere. No raw `Error`.
- Catalogue files are the source of truth for MessageKeys. New keys land in both en.json + es.json same commit.
- Tests live with the code in the same commit.
- No `any` types. Strict TS.
- Comments explain *why*, not *what*. No emojis.

## References

- `packages/starter-client-ts/` — the existing client + SSE primitive's new home.
- `packages/rubix-client-ts/` — the existing typed-wire layer.
- `rubix/frontend/` — the consumer.
- `rubix/frontend/package.json` — confirms `@nube/rubix-client-ts` is a workspace dep already.
- `starter-extensions/packages/starter-ext-ui/` — existing provider pattern to mirror.
- `rubix/docs/sessions/2026-05-24-smoke-test-pr32.md` — e2e pattern.
- `rubix/docs/design/agent/README.md` — backend wiring this frontend connects to.
- `rubix/SCOPE.md`, `rubix/HOW-TO-CODE.md`, `rubix/FILE-LAYOUT.md`, `rubix/NEW-SESSION.md`.
