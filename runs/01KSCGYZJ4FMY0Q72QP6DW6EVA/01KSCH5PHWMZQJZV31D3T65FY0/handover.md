## Done

- Added `client/csrf.ts` (readCookie + readCsrfHeader, default cookie `starter_csrf`)
- Added `client/fetch_json.ts`, `fetch_void.ts`, `fetch_bytes.ts` helpers (URL build, credentials:include, StarterError throw on !ok, typed return)
- Refactored `endpoints/auth.ts`, `theme.ts`, `openapi.ts`, `health.ts` to use the new helpers; each method ≈ ~5 lines
- `pnpm --filter @nube/starter-client-ts typecheck` and `test` both green (8/8 tests pass)
- Committed as `9fe6465` with message starting "phase A.2 — starter-client-ts fetch helpers + CSRF helper"

## Next

- Stage 3 of 15 picks up next

## What you need to know

- `fetchVoid` returns the `Response` (not `void`) so callers like `themeSave` can branch on `res.status === 204` while still benefiting from the shared !ok throw
- Helpers force `credentials: "include"` unconditionally — appropriate for all current endpoints; if a future endpoint needs to opt out, change the helper signature
- `readCsrfHeader()` returns `{}` when cookie/document is absent, so it can be spread unconditionally into headers
- `pnpm install` was required (node_modules was missing); pnpm-lock.yaml diff was discarded before commit so only source changes landed

## Open questions

- (none)
