## Done

- Added `packages/starter-client-react/src/provider/auth-provider.tsx` — `AuthProvider` + `useAuth` + `ME_QUERY_KEY`; login mutation invalidates `me`, logout clears the cache; `unauthenticatedSlot` rendered on `StarterError` 401, optional `loadingSlot` while pending.
- Added `packages/starter-client-react/src/hooks/use-event-stream.ts` — bridges `streamJson` via `useSyncExternalStore`; `{ data, error, status, reconnect }`; status set `connecting|open|reconnecting|closed|error`; stable `reconnect` (generation counter); cleans up on unmount via AbortController.
- Exported new symbols from `src/index.ts`.
- Tests (jsdom + @testing-library/react) for query-provider retry policy, AuthProvider unauth/auth branches, and useEventStream open/reconnect/unmount via a mock `EventSource`. Vitest setup file adds `afterEach(cleanup)`.
- `pnpm --filter @nube/starter-client-react typecheck` and `test` green (3 files / 9 tests pass).
- Committed as `phase A.3 — starter-client-react auth + useEventStream`.

## Next

- Stage 4 per the job plan (next sibling package / hook layer). A fresh session picks it up.

## What you need to know

- vitest config now uses `environment: "jsdom"` and a `src/test-setup.ts` setup file; new devDeps `@testing-library/react` and `jsdom` added.
- Tests use a real `StarterClient` with a stubbed `fetch` (and a `MockEventSource`) rather than MSW — keeps the dep surface minimal and matches the pattern already used in `starter-client-ts`. The scope mentioned MSW *or* custom mock EventSource; I chose the latter.
- The auth-provider test only verifies anon/auth render branches and that the login mutation pipeline is reachable; clicking the harness `login` button isn't possible while the anon slot is shown (children are hidden), which is the correct production behaviour.

## Open questions

- (none)
