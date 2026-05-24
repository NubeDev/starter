## Done

- Scaffolded `rubix/packages/rubix-client-react` (`@nube/rubix-client-react`) with package.json, tsconfig, vitest.config, test-setup, src/index.ts barrel
- Implemented `RubixClientProvider` + `useRubixClient` in `src/provider/rubix-client-provider.tsx`; provider mounts a nested `StarterClientProvider` using `client.starter` so starter-side hooks resolve against the same transport
- Added sibling `rubix-client-provider.test.tsx` (3 tests passing) covering context propagation, the nested starter provider, and the loud throw outside a provider
- Fixed an unrelated broken `extends` path in `rubix/packages/rubix-client-ts/tsconfig.json` (`../starter-client-ts/tsconfig.json` → `../../../packages/starter-client-ts/tsconfig.json`) — was breaking vite's tsconfig-chain resolution as soon as anything imported from rubix-client-ts in jsdom tests
- `pnpm --filter @nube/rubix-client-react typecheck` and `test` both green
- Committed as `phase B.3 — rubix-client-react package scaffold + provider`

## Next

- (none — next session picks up the next stage)

## What you need to know

- `pnpm-workspace.yaml` already globs `rubix/packages/*`, so no edit there was needed
- Peer-deps match `@nube/starter-client-react` (react@^19, react-dom@^19, @tanstack/react-query@^5). React Query is listed as a peer even though this stage doesn't use it yet — sibling typed-hook stages will
- `@nube/starter-client-ts` is in devDependencies for the test (we construct a `StarterClient` directly in a test). Endpoint hooks added in later stages should consume it via the rubix client's `.starter`, not as a direct dep

## Open questions

- (none)
