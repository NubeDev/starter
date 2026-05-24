## Done

- Read packages/starter-client-ts/src/ end-to-end (client.ts, error/starter-error.ts + test, endpoints/{auth,theme,health,openapi}.ts, auth.test.ts, package.json).
- Cross-checked rubix/SCOPE.md and DOCS/agent/SCOPE.md for any rubix-client-ts phase definitions; none exist — the job goal is the authoritative phase spec.
- Confirmed the job goal's four named Phase A primitives (CSRF helper, fetch helpers, StarterError.is, generalised codegen script) cover every common primitive rubix-client-ts will need for typed request/response/error verbs. No Phase-A expansion required.
- No files written, no commit (stage is analysis-only).

## Next

- Stage 2 (phase A.2): implement the Phase A primitives at these exact paths:
- NEW `packages/starter-client-ts/src/util/cookies.ts` — `export function readCookie(name: string): string | undefined` + `export const CSRF_COOKIE = "starter_csrf"`. Refactor `endpoints/auth.ts` to import instead of redefining.
- NEW `packages/starter-client-ts/src/client/request.ts` — `requestJson<T>(client, path, init, opts)`, `requestVoid(client, path, init, opts)`, `uploadBinary(client, path, file, opts)`. `opts` includes `errorFor: (res: Response) => Promise<Error>` so rubix can throw RubixError instead of StarterError. Default `errorFor` = `StarterError.fromResponse`. Helpers handle base-URL join, header merge (incl. CSRF injection for mutating verbs when `csrf: true`), `credentials: "include"`, and !ok → throw mapped error. Refactor auth.ts/theme.ts/health.ts/openapi.ts to use them.
- EDIT `packages/starter-client-ts/src/error/starter-error.ts` — add `static is(err: unknown): err is StarterError`. Add a test in the existing `starter-error.test.ts`.
- Stage 3 (phase A.3): NEW `scripts/codegen-openapi.mjs` (repo root) accepting `--input <path> --output <path>`. Update `packages/starter-client-ts/package.json` `codegen` script to call it. Verify `pnpm --filter @nube/starter-client-ts codegen` still produces an identical `src/generated/index.ts` (no diff).

## What you need to know

- `endpoints/auth.ts` currently has `const CSRF_COOKIE = "starter_csrf"` and a private `readCookie()` — both move into `util/cookies.ts` and become the single source.
- Error-mapping pluggability is the load-bearing design choice for `request.ts`: rubix wraps (not extends) StarterClient and throws `RubixError`. Helpers must accept an `errorFor` callback rather than hardcoding `StarterError.fromResponse`. This keeps Phase C from re-duplicating fetch boilerplate.
- `endpoints/health.ts` and `endpoints/openapi.ts` currently don't call `StarterError.fromResponse` on !ok — refactoring through `requestJson` will silently fix that. Note in the A.2 commit message.
- The job goal lists Phase A as upstream-only (lands in starter-client-ts), Phase C as rubix-client-ts construction. SCOPE.md does NOT currently document these phases; updating SCOPE.md is a later stage's concern (likely the final-doc stage), not A.x.
- Generalised codegen script: per `package.json` script style across the repo, repo-root `scripts/codegen-openapi.mjs` is the natural home — keep it dependency-free (just spawn `openapi-typescript`).

## Open questions

- Does Phase C need an SSE / EventSource primitive for rubix-agent's typed event streams? Job goal says "every wired verb with typed request/response/error" — "verb" reads as request/response only, so SSE is excluded from this build, but worth confirming before Phase C designs the agent surface. If yes, a 5th Phase A primitive (`src/client/stream.ts`) must be added before A.2 starts.
- Should `errorFor` default to `StarterError.fromResponse` (convenient for starter callers) or be required (forces explicit choice; safer in shared helpers)? Recommend defaulting, but call this out in A.2.
- Codegen script location: repo-root `scripts/codegen-openapi.mjs` vs a dedicated `@nube/dev-tools` workspace package. Recommend the simpler script for now; can promote to a package if more codegen flows appear.
