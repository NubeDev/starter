## Done

- Added `packages/rubix-client-ts/src/endpoints/user.ts` (userCreate, userDisable, userList), `team.ts` (teamCreate, teamAssign), `tenant.ts` (tenantList). Each mutating method (and tool POSTs in general) threads `readCsrfHeader()` from `@nube/starter-client-ts` into request headers.
- Added sibling `*.test.ts` files (vitest, node env) that stub `globalThis.document.cookie = "starter_csrf=csrf-test-token"` in `beforeEach`, capture the outgoing Request via fake fetch, and assert method / URL / JSON body / content-type / `X-CSRF-Token` header, plus a problem+json error path.
- Updated `src/endpoints/index.ts` barrel to re-export the three new modules.
- `pnpm --filter @nube/rubix-client-ts test` → 5 files, 16 tests passing. `typecheck` clean.
- Committed as `510b520` with title starting `stage 11: phase C.3 — user + team + tenant endpoints (Goal 2)`.

## Next

- Stage 12 in the 15-stage plan (next phase C verb family, per SCOPE).

## What you need to know

- DTO shapes were mirrored from `rubix/crates/rubix-spi/src/dto/{user,team,tenant}/*.rs` (e.g. `UserCreateRequest { email, role, password_hash? }`, `UserDisableResponse { …, was_already_disabled, disabled_at_ms }`, `TeamAssignResponse { …, already_member, assigned_at_ms }`, `TenantListItem { tenant_id, name, locale }`). `Diagnostic` is reused from `endpoints/system.ts`.
- Wire path is the generic tool route `POST /api/v1/tools/{tool_id}` — same as existing `system.ts` / `alert.ts`. Read-only `userList` / `tenantList` still POST with the CSRF header for symmetry with the server CSRF middleware on `/api/v1/tools/*`.
- Tests run under `environment: "node"`, so `document` does not exist — the suite assigns a minimal `{ cookie }` stub and deletes it in `afterEach`. `readCsrfHeader()` returns `{ "X-CSRF-Token": value }` (capitalised key) — assertions use that exact case.
- Existing `system.ts` / `alert.ts` do NOT thread CSRF; stage 11 intentionally only adds CSRF to the new files. Whether to retrofit the older files is a separate decision.

## Open questions

- (none)
