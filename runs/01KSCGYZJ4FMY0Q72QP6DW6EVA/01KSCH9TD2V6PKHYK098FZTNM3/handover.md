## Done

- Added `StarterError.is(err, status?)` static type guard in `packages/starter-client-ts/src/error/starter-error.ts` with two new tests in `starter-error.test.ts` (narrowing + status match).
- Extracted codegen to `packages/starter-client-ts/bin/codegen.mjs` (accepts `--input`/`--output`, defaults to `../../openapi.json` and `./src/generated/index.ts`).
- Updated `package.json`: `scripts.codegen` -> `node ./bin/codegen.mjs`; added `bin` to `files`.
- Ran `pnpm --filter @nube/starter-client-ts codegen` — no diff in generated output. Typecheck + tests (10/10) pass.
- Committed as `feat(starter-client-ts): StarterError.is + codegen generalisation`.

## Next

- Stage 4 (next session) picks up the next phase of the rubix-client-ts plan.

## What you need to know

- `bin/codegen.mjs` shells out via `npx --no-install openapi-typescript` so it inherits the package's devDependency; reusable from rubix-client-ts by passing different `--input`/`--output`.
- Test for `.is` calls it with `unknown`, `Error`, `undefined`, string, plain object — covers the non-instance branch.

## Open questions

- (none)
