# Scope — rubix-client-ts

## Goal

Ship `@nube/rubix-client-ts`, a hand-curated TypeScript HTTP client over rubix-agent's REST surface, codegen'd from a freshly-emitted `rubix/openapi.json`, that mirrors the shape of `@nube/starter-client-ts` and the Rust `rubix-client` crate. It wraps a `StarterClient` instance for the auth + transport plumbing (no duplicated auth code) and adds per-area endpoint modules for every rubix verb wired by the end of the goals-2-4-3 job (user-admin, clickhouse-ruler, flow-programmer) plus the already-wired Goal 5 surface (system-check, alert) plus the MCP introspection surface (`tools/list`, `tools/call`) plus the rubix-specific reads (audit, undo cursor, flow definitions list).

While in there, the job lands the necessary uplifts in `@nube/starter-client-ts` that are common-to-starter (any client primitive rubix needs that genuinely belongs upstream, per R2). These uplifts land **first**, in their own commits, with the same upstream-first discipline used in PRs #28 → #31. The client work itself follows.

The success bar is: a fresh TypeScript consumer (e.g. `test-ui-5` or any future rubix UI) imports `@nube/rubix-client-ts`, constructs a `RubixClient(starter)`, and calls every wired verb with a typed request → typed response → typed `RubixError`. The bundled codegen step (`pnpm --filter @nube/rubix-client-ts run codegen`) reads `rubix/openapi.json` and writes `src/generated/index.ts`. CI catches drift via a new `rubix-openapi-drift` job mirroring the existing `openapi-drift` one.

## In scope

### Phase A — upstream uplifts in `@nube/starter-client-ts`

The rubix client should not reinvent primitives. Anything common to starter + rubix lands upstream first per R2. Specifically:

- **`StarterClientCore` exposed for composition.** The current `StarterClient` is a concrete class. Extract the constructor surface (`baseUrl` normalisation, `fetch` injection, `headers` defaults) into a re-exported base or a public `StarterClient.options` getter so `RubixClient` can construct itself from a `StarterClient` instance without reaching into private fields. Pick the minimal move — if `StarterClient` already exposes everything `RubixClient` needs publicly (`baseUrl`, `fetch`, `headers` are `readonly` on the class), this stage is documentation-only.
- **CSRF helper extracted.** The cookie-read pattern in `endpoints/auth.ts` (`readCookie('starter_csrf')` → `X-CSRF-Token` header) is identical for every CSRF-protected mutating endpoint in starter + rubix. Extract to `src/client/csrf.ts` as `readCsrfHeader(cookieName?): Record<string,string>`. Updates `auth.logout`, `theme.themeSave`, `theme.themeUploadLogo`, and every future mutating endpoint to call it. `RubixClient` reuses the same helper.
- **`fetchJson` / `fetchVoid` / `fetchBytes` private helpers.** Every endpoint file in starter-client-ts repeats the same shape: build URL, call `fetch` with `credentials: include`, branch on `res.ok`, throw `StarterError.fromResponse`, return `res.json() as T`. Extract those four lines into `src/client/fetch_helpers.ts` (one verb per file: `fetchJson.ts`, `fetchVoid.ts`, `fetchBytes.ts`) so endpoint files become a one-liner per method. `RubixClient` reuses the helpers (passing its own `baseUrl` + headers).
- **`StarterError.is(error, status?)` type guard.** TS consumers often want `if (StarterError.is(err, 401))` to branch on auth failure. Add the static type guard. `RubixError` extends `StarterError` and inherits it.
- **OpenAPI codegen script generalised.** The current `codegen` script hardcodes `../../openapi.json` and `./src/generated/index.ts`. Generalise to a `bin/codegen.mjs` that takes `--input` and `--output` so the same npm script can be reused from `rubix-client-ts/package.json` pointing at `rubix/openapi.json`. Keep the existing `pnpm codegen` working with the same defaults.
- **`README.md` reshuffle** — add a "Building a domain client on top of StarterClient" section that points at `@nube/rubix-client-ts` as the worked example. Same edit pattern as the rubix-changes ledger upstream pattern.

These upstream changes do **not** break the existing `endpoints/auth.ts`, `endpoints/theme.ts`, `endpoints/openapi.ts`, `endpoints/health.ts` files. They land as refactors that those files adopt in the same commit. The public API of `StarterClient` is additive: existing consumers (`test-ui-5`, any in-tree UI package importing the client today) keep working without code changes.

### Phase B — rubix-agent emits `rubix/openapi.json`

The codegen seam. Today rubix-agent has utoipa `ToSchema` on the DTOs in `rubix-spi` but no `OpenApi` document assembled and no `/openapi.json` route. This phase closes that gap. The shape follows starter-server's `routes/openapi_doc.rs` exactly.

- **`rubix-agent::openapi` module** — verb file `rubix/crates/rubix-agent/src/openapi.rs` exposing `pub fn rubix_openapi() -> utoipa::openapi::OpenApi` that assembles every rubix route + DTO into one document. Mirrors `starter-server::openapi`. Includes:
  - The `info` block (title `rubix-agent`, version from `Cargo.toml`, description pointing at `docs/design/agent/README.md`).
  - Servers entry (`http://127.0.0.1:8088` for the dev binary, configurable via the `RUBIX_BIND` env var if surfaced).
  - Tags per goal (`auth`, `system`, `user-admin`, `clickhouse-ruler`, `flow-programmer`, `mcp`, `undo`, `dashboard` stub, `weekly-report` stub) — one per goal area so the codegen surfaces clean per-tag namespaces in TS.
  - Every route emitted by rubix-agent's REST router today + the routes the goals-2-4-3 job adds. Pulled via utoipa's `#[utoipa::path]` macros on each handler.
- **`/openapi.json` route** — verb file `rubix/crates/rubix-agent/src/routes/openapi_doc.rs` mirroring starter-server's. Wired into the agent's `Router` at boot in `main.rs`. Available unauthenticated (matches starter-server's behaviour).
- **`utoipa::path` attributes on every handler** — every Axum handler in `rubix-agent::routes` (today and post-#31) grows a `#[utoipa::path(...)]` annotation. New routes added by the goals-2-4-3 job (covered separately in that job — this scope just relies on the result). For this job's purposes: at submit time, run `gh pr view 32` (or whichever PR closes the goals-2-4-3 job) to confirm what landed; if any handler lacks `utoipa::path`, add the attribute in this job's Phase B with a one-line commit per crate touched.
- **`rubix/openapi.json` snapshot** — committed snapshot at `rubix/openapi.json` (matching the workspace-root `openapi.json` pattern). Regenerated by a new script `rubix/scripts/snapshot-openapi.sh` that boots the agent in a child process with `RUBIX_BIND=127.0.0.1:0`, curls `/openapi.json`, writes to the snapshot path, kills the child. The CI drift job (Phase D) re-runs this and fails on diff.
- **Integration test** — `rubix/crates/rubix-agent/tests/openapi_test.rs` asserts the served document parses, has the expected tag count, and includes the canary path `/api/v1/tools/rubix.system.disk` (and post-goals-2-4-3 the other goal paths). Same pattern as `starter-server` tests.

### Phase C — `@nube/rubix-client-ts` package

The client itself. Layout mirrors `starter-client-ts` exactly:

```
packages/rubix-client-ts/
├── package.json          name: @nube/rubix-client-ts, depends on @nube/starter-client-ts ^workspace
├── tsconfig.json         extends starter-client-ts's
├── vitest.config.ts      same
├── README.md             mirrors starter-client-ts's; "extends StarterClient" worked example
├── src/
│   ├── index.ts          exports { RubixClient, RubixError } and re-exports * from endpoints
│   ├── client/
│   │   └── client.ts     `class RubixClient { constructor(starter: StarterClient) }`; exposes
│   │                     `.starter` field plus convenience pass-throughs to the underlying
│   │                     fetch helpers from starter-client-ts.
│   ├── error/
│   │   └── rubix-error.ts  `class RubixError extends StarterError`; adds the rubix-specific
│   │                       `code` field (`rubix.user.created`, `rubix.flow.deployed`, …) parsed
│   │                       from the response Diagnostic payload.
│   ├── endpoints/
│   │   ├── index.ts      barrel of every endpoint family below
│   │   ├── system.ts     `disk()`, `db()`, `flowErrors()`
│   │   ├── alert.ts      `send(request)`
│   │   ├── user.ts       `userCreate()`, `userDisable()`, `userList()`
│   │   ├── team.ts       `teamCreate()`, `teamAssign()`
│   │   ├── tenant.ts     `tenantList()`
│   │   ├── clickhouse.ts `ruleWrite()`, `martCreate()`, `retentionSet()`
│   │   ├── flow_ops.ts   `flowDeploy()`, `flowLint()`, `flowList()`, `flowDuplicate()`
│   │   ├── undo.ts       `undoLast()`
│   │   ├── mcp.ts        `mcpToolsList()`, `mcpToolsCall(name, args, opts?)`
│   │   └── audit.ts      `auditList(filters?)` (reads from starter-changelog via the rubix route)
│   └── generated/
│       └── index.ts      codegen output from rubix/openapi.json; never hand-edited
└── tests/
    ├── client.test.ts                construct, assert .starter is the passed instance
    ├── endpoints/                    one .test.ts per endpoint family using msw
    │   ├── system.test.ts
    │   ├── user.test.ts
    │   ├── flow_ops.test.ts
    │   ├── clickhouse.test.ts
    │   ├── undo.test.ts
    │   ├── mcp.test.ts
    │   └── audit.test.ts
    └── round-trip.test.ts            end-to-end against a recorded fixture
```

Each endpoint file follows the auth.ts / theme.ts pattern exactly: import wire types from `../generated/index.js` via `components["schemas"]["..."]`, declare `declare module "../client/client.js"` augmenting `RubixClient`, implement each method as `RubixClient.prototype.<verb> = async function ...`. Mutating endpoints call the upstream `readCsrfHeader()` helper extracted in Phase A. Error path: every non-2xx throws `RubixError.fromResponse(res)` which reads the Diagnostic `code` field and surfaces it on the thrown error.

**`mcpToolsCall` deserves a callout.** MCP is JSON-RPC over HTTP; the rubix-agent route is `POST /api/v1/mcp` with a JSON-RPC envelope. `mcpToolsCall(name, args, opts?)` builds the envelope, sends it, parses the `result.structuredContent`, and returns a typed result. `opts` carries `acceptLanguage` (mapped to the `_meta.acceptLanguage` field per the existing rubix MCP contract). Same shape as the existing Rust rubix-client's MCP wrapper if one exists; if not (confirm at Phase C.1), implement the wrapper here and consider whether it deserves promotion to the upstream Rust `rubix-client::mcp` later.

**Testing discipline:**
- `vitest run` green for all endpoint .test.ts files; each test uses `msw` (or the same fetch-mock pattern the existing `auth.test.ts` uses — confirm at Phase C.1 and reuse) to stub the response.
- `round-trip.test.ts` records a real session by curling against a running agent (or, if running an agent in CI is impractical, against a fixture-replay variant of the agent — `RUBIX_AI_FIXTURE` is already wired per PR #30) and asserts every method returns the expected shape.
- `pnpm --filter @nube/rubix-client-ts typecheck` clean.

### Phase D — codegen drift CI + docs + PR

- **`rubix-openapi-drift` CI job** — new GitHub Actions job (mirrors the existing `openapi-drift`) that runs `rubix/scripts/snapshot-openapi.sh`, diffs against the committed `rubix/openapi.json`, fails on diff. The existing `pnpm codegen` step also needs to be runnable in CI without a live agent — pick whether to (a) boot the agent transiently in CI or (b) rely solely on the committed snapshot for codegen and gate snapshot regeneration as a manual step. Default to (b) — simpler — and document the regen flow.
- **Workspace integration** — add `rubix-client-ts` to `pnpm-workspace.yaml`. Add it to the root `package.json` scripts where appropriate. Ensure `pnpm install` from the workspace root resolves the workspace dependency.
- **Design doc** — `rubix/docs/design/client-ts/README.md` present-tense: what the client exposes, the relationship to starter-client-ts, the codegen flow, the error type.
- **Session note** — `rubix/docs/sessions/<today>-rubix-client-ts-landed.md` documenting the new package, the upstream uplifts in starter-client-ts, the verification evidence (typecheck count, vitest counts, drift CI green).
- **PR** — one PR off `codeless/rubix-client-ts` with the four-phase commit history.

## Out of scope

- **No React, no hooks, no UI components.** The client is fetch-level only; UI packages (`starter-ui-*`, future `rubix-ui-*`) consume it.
- **No WebSocket / SSE wrapper for live flow runs.** The Rust `rubix-client` doesn't have one yet either; if needed, this lands as a follow-up job. The `mcpToolsCall` method is plain JSON-RPC over HTTP.
- **No retry / backoff / circuit breaker primitives.** The upstream `StarterClient` doesn't have them; if they're wanted, they belong upstream first, in a separate job.
- **No authentication helpers beyond what `StarterClient` already provides.** `RubixClient` delegates auth entirely — no rubix-specific session model, no token vault, no OAuth flow.
- **No code in `rubix/openapi.json` that comes from a hand-written spec.** The snapshot is regenerated from the live agent's `/openapi.json`; the agent is the source of truth.
- **No changes to the Rust `rubix-client` crate** beyond what surfaces during the work as an upstream-first need (e.g. if the TS client needs a route shape that the Rust client also wants, raise it). Default position: this is a TS-only job.
- **No multi-tenant scoping in the client constructor.** `RubixClient(starter)` uses the same tenant the session cookie carries. Per-tenant clients are a future concern.
- **No graduating the existing `test-ui-5` to consume this client in the same job.** That's a follow-up — adoption typically rides its own PR for review surface clarity.

## Constraints

- **R1 — One verb per file (TS too).** ≤ 200 lines hard for `.ts` files (lower than Rust because JS verbosity hurts readability faster). Each endpoint method's file matches the Rust verb file's shape: one verb, the DTO type imports, the augmentation block.
- **R2 — Upstream-first.** Anything common to starter + rubix lands in `@nube/starter-client-ts` first, in its own commit, before the rubix consumer. Phase A captures the known set; if Phase C surfaces another candidate, pause and raise it.
- **R3 — Doc-tier rule.** Code comments link to `docs/design/<area>/README.md` only. Never `SCOPE.md`, `HOW-TO-CODE.md`, `NEW-SESSION.md`, `docs/scope/`, or `docs/sessions/`. `./rubix/scripts/lint-doc-refs.sh` enforces it on Rust; the same convention applies to TS comments by hand-review (no lint exists for TS today).
- **R4 — Wire types come from codegen.** Hand-edits to `src/generated/index.ts` are forbidden. Hand-rolled aliases (like the `ThemeStyles` re-exports in `endpoints/theme.ts`) are allowed in endpoint files only.
- **R5 — Catalogue files unaffected.** The TS client does not need its own i18n catalogue; the rubix server already i18n-resolves message keys when `Accept-Language` is set. The TS client just plumbs the header through.
- **R6 — Tests live with the code.** Each endpoint file has a sibling `.test.ts` in the same commit.
- **Commit messages.** `feat(starter-client-ts):` for Phase A, `feat(rubix-agent):` for Phase B, `feat(rubix-client-ts):` for Phase C, `chore(ci+docs):` for Phase D.
- **Package name.** `@nube/rubix-client-ts`. Same scope as `@nube/starter-client-ts`. Workspace dependency.
- **Node version + TS version.** Match starter-client-ts's `tsconfig.json` exactly so the two packages co-typecheck without conflict.

## Open questions

1. **Does `rubix-agent` already serve any route under a stable prefix like `/api/v1/`, or are routes top-level?** Phase B.1 must confirm before laying out the OpenAPI document. Default assumption: `/api/v1/...` for stateful endpoints (matches starter-server), `/openapi.json`, `/healthz` at root.
2. **Is the MCP HTTP endpoint at `/mcp` or `/api/v1/mcp`?** Affects `endpoints/mcp.ts`. Phase C.1 confirms by grep.
3. **`audit` endpoint surface.** Does rubix expose audit reads under its own route, or does it surface `starter-changelog` directly via a starter-server-mounted route? If the latter, the audit endpoint belongs in `starter-client-ts`, not `rubix-client-ts`. Phase C.1 grep + decide; if it belongs upstream, this becomes a Phase A item and the rubix `audit.ts` is dropped from Phase C.
4. **Diagnostic shape on errors.** The current MessageKey-driven Diagnostic carries `code`, `params`, and rendered text per locale. `RubixError.code` field: parse from `body.code` directly, or from `body.summary.code` (which is what `rubix.system.disk` REST returns per the smoke session note)? Default to the latter; document the shape in `rubix-error.ts`.
5. **Do we want a `RubixClient.dispose()` for AbortController cleanup?** `StarterClient` doesn't have one. If we add it here, we add it upstream first per R2. Default: defer until a real consumer needs it.
6. **Snapshot regen story for `rubix/openapi.json` in CI.** Stage D.1 picks (a) boot agent in CI for codegen vs (b) committed snapshot only. Default (b); the drift job is what catches staleness. Confirm at REVIEW.

## References

- `packages/starter-client-ts/` — the exemplar this work mirrors.
- `rubix/crates/rubix-client/src/client.rs` — the Rust analogue; same `inner` wrapping shape.
- `rubix/crates/rubix-spi/src/dto/` — every rubix DTO with utoipa `ToSchema` already attached.
- `crates/starter-server/src/openapi.rs` and `routes/openapi_doc.rs` — the OpenAPI emission pattern to mirror in rubix-agent.
- `crates/starter-server/src/handlers/` — utoipa::path examples to mirror in rubix-agent handlers (if any are missing post-goals-2-4-3).
- `rubix/docs/design/agent/README.md` — runtime wiring picture.
- `rubix/docs/sessions/2026-05-24-smoke-test-pr30.md` — current verified state of the REST surface.
- `rubix/SCOPE.md` — R1–R13.
- `packages/starter-ui-kit/` — a future consumer of `rubix-client-ts` (theme-editor work landed in commit 24140c9).
