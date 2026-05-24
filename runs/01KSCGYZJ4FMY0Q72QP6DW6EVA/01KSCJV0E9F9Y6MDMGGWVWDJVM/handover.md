## Done

- Added src/endpoints/clickhouse.ts (ruleWrite, martCreate, retentionSet), flow_ops.ts (flowDeploy, flowLint, flowList, flowDuplicate), undo.ts (undoLast) to @nube/rubix-client-ts, each dispatching POST /api/v1/tools/{tool_id} with readCsrfHeader() threaded into headers
- Sibling .test.ts files per module asserting wire shape, X-CSRF-Token header, and a 403 problem+json StarterError branch
- Updated src/endpoints/index.ts barrel to re-export the three new modules
- pnpm --filter @nube/rubix-client-ts test → 8 files / 28 tests passing; typecheck clean
- Committed as b94d966 with title "stage 12: phase C.4 — clickhouse + flow_ops + undo endpoints (Goals 4 + 3)"

## Next

- Stage 13 — next phase per WORKFLOW (likely C.5 covering remaining goals: mcp, dashboard-stub, weekly-report-stub, or analytics/tags/clipboard) — a fresh session should pick it up

## What you need to know

- Tool IDs use dot-segmented form on the wire: rubix.clickhouse.rule.write, rubix.clickhouse.mart.create, rubix.clickhouse.retention.set, rubix.flow_ops.deploy/lint/list/duplicate, rubix.undo.last (taken from rubix-tools/* tool definitions)
- undoLast returns { group_id } — undo's last.rs in rubix-tools returns a plain JSON object, NOT a Diagnostic-wrapped response, unlike the other verbs. The TS shape matches the actual server response, not the placeholder DTO in rubix-spi/src/dto/undo/last.rs
- All eight test files (existing + new three) share the same fetch-recording harness pattern that originated in user.test.ts
- flow_ops/validate exists in rubix-spi DTO + rubix-client placeholder but was explicitly excluded from this stage's scope; it's not on the TS surface yet
- Diagnostic type is reused from ./system.js (the canonical local type alias)

## Open questions

- (none)
