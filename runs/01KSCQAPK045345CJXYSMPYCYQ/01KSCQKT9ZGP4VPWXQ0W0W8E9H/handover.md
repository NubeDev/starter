## Done

- Scaffolded `packages/starter-client-react/` with package.json (peer-deps react@^19, react-dom@^19, @tanstack/react-query@^5; depends on workspace `@nube/starter-client-ts`), tsconfig extending starter-client-ts, vitest.config.ts.
- Implemented `src/provider/starter-client-provider.tsx` (context + `StarterClientProvider` + `useStarterClient` that throws if unmounted) and `src/provider/query-provider.tsx` (wraps `QueryClientProvider` with staleTime 30s / gcTime 5min / retry skipping on StarterError 401/403).
- Barrel `src/index.ts` re-exports both providers and `useStarterClient`.
- `pnpm install` + `pnpm --filter @nube/starter-client-react typecheck` green.
- Committed as `phase A.2 — starter-client-react package scaffold`.

## Next

- (none) — next session picks up Stage 3.

## What you need to know

- `pnpm-workspace.yaml` already includes `packages/*`, so no edit was needed.
- `StarterError` is exported from `@nube/starter-client-ts` index; QueryProvider's retry uses `instanceof StarterError` to gate on `.status`.
- No tests added in this stage (scope said scaffold + providers); environment is node-only in vitest.config — DOM-needing tests will need jsdom later.

## Open questions

- (none)
