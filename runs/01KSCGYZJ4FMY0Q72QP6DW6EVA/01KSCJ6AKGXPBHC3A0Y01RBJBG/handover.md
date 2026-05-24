## Done

- Scaffolded `packages/rubix-client-ts/` with `package.json` (workspace dep on `@nube/starter-client-ts`; `codegen` script invokes the shared `starter-client-ts/bin/codegen.mjs --input ../../rubix/openapi.json --output ./src/generated/index.ts`), `tsconfig.json` extending starter's, `vitest.config.ts`.
- `src/client/client.ts`: `class RubixClient` constructor takes a `StarterClient`, exposes it as readonly `.starter`.
- `src/error/rubix-error.ts`: `class RubixError extends StarterError`; static async `fromResponse` parses `.code` from `body.summary.code` (preferred) with `body.code` as fallback (per SCOPE OQ-4).
- `src/index.ts` barrel exports `{ RubixClient, RubixError }`.
- Ran `pnpm install`, then `pnpm --filter @nube/rubix-client-ts codegen` → wrote `src/generated/index.ts` (committed per convention). `pnpm --filter @nube/rubix-client-ts typecheck` green; starter typecheck still green.
- Committed as `e72e211` "stage 9: phase C.1 — rubix-client-ts package scaffold".

## Next

- Stage 10: phase C.2 — port endpoint families (auth, system, user-admin, clickhouse-ruler, flow-programmer, mcp, undo, dashboard-stub, weekly-report-stub) mirroring the per-tag layout in `rubix/openapi.json`; each endpoint augments `RubixClient` via TS declaration merging and throws `RubixError.fromResponse(res)` on non-2xx.

## What you need to know

- `pnpm-workspace.yaml` already globs `packages/*` — no edit was needed.
- Had to add `export type { Problem }` to `packages/starter-client-ts/src/index.ts` so `RubixError` can type the parent's `.problem` without reaching into starter internals. Public surface only grew; no regression.
- `rubix/openapi.json` currently has `"components": {}` (no shared schemas emitted), so the generated `src/generated/index.ts` is paths-only. Endpoint port (stage 10) will need to define request/response types inline or land utoipa schemas upstream first.
- Codegen output is committed (per the existing starter-client-ts convention noted in SCOPE/HOW-TO).

## Open questions

- (none)
