## Done

- Added `packages/rubix-client-ts/src/endpoints/system.ts` (disk, db, flowErrors) and `alert.ts` (send), each dispatching to `POST /api/v1/tools/{tool_id}` via the starter `fetchJson` helper through the wrapped `.starter`.
- Added `src/endpoints/index.ts` barrel and wired it into the package root `src/index.ts`.
- Added sibling `system.test.ts` (5 tests) and `alert.test.ts` (2 tests) mirroring the fetch-mock pattern in `packages/starter-client-ts/src/endpoints/auth.test.ts`. All 7 new tests pass; starter-client-ts still 10/10.
- Re-exported `fetchJson` / `fetchVoid` / `fetchBytes` / `readCsrfHeader` from `@nube/starter-client-ts`'s barrel so rubix endpoints can import them.
- Committed as `feat(rubix-client-ts) system + alert + audit endpoints` (55f4493).

## Next

- (none) — next session picks up Stage 11.

## What you need to know

- SCOPE OQ-3 resolved: audit reads do NOT belong in rubix-client-ts. The `/v1/audit` list route is mounted by `crates/starter-audit/src/routes.rs` on starter-server, not rubix-agent. The endpoints barrel comment records this; `audit.ts` was intentionally dropped from this stage. Should be raised back to starter-client-ts as a follow-up Phase A item if not already tracked.
- Route prefix is `/api/v1/tools/{tool_id}` per `rubix-agent/src/routes/tools.rs` (the Rust `rubix-client` crate still uses an outdated `/v1/tools/...` prefix — TS client follows the live agent, not the stale Rust client).
- DTO shapes are inlined as TS interfaces in `system.ts` / `alert.ts` because `rubix/openapi.json` has empty `components` (the dispatch route advertises a free-form body). Shapes mirror `rubix-spi/src/dto/system/{disk,db,flow_errors,alert_send}.rs`.
- Errors thrown today are `StarterError` (from `fetchJson`), not `RubixError`. Wiring `RubixError.fromResponse` into a rubix-flavored fetch helper is a future refinement — out of scope for C.2.

## Open questions

- (none)
