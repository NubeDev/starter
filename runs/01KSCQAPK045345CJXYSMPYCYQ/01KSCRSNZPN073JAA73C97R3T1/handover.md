## Done

- Added six hook files under rubix/packages/rubix-client-react/src/hooks/: teams.ts, tenants.ts, clickhouse.ts, flow-ops.ts, undo.ts, audit.ts — each with a sibling .test.tsx.
- Hooks mirror the rubix-client-ts endpoint family shape and dispatch through the typed RubixClient methods (audit calls fetchJson against `/v1/audit` on the wrapped StarterClient because no typed audit surface exists yet).
- Each family exports a `*_KEY` constant (`TEAMS_KEY`, `TENANTS_KEY`, `CLICKHOUSE_KEY`, `FLOW_OPS_KEY`, `UNDO_KEY`, `AUDIT_KEY`). Mutations invalidate their family prefix; `useUndoLast` invalidates `['rubix']` root.
- README.md added documenting the `['rubix', <family>, ...]` query-key convention with examples and the audit OQ-3 note.
- src/index.ts barrel updated to re-export all six new hook modules.
- `pnpm --filter @nube/rubix-client-react typecheck` clean; `pnpm --filter @nube/rubix-client-react test` green — 12 files, 51 tests.
- Committed as `phase B.5 — rubix-client-react remaining hooks (teams/tenants/clickhouse/flow-ops/undo/audit)` (feat(rubix-client-react) remaining hook families).

## Next

- (none) — next session picks up Stage 10 of 16.

## What you need to know

- `useFlowLint` is modelled as a mutation, not a query, because callers supply fresh YAML per invocation; it deliberately does NOT invalidate the flow_ops prefix.
- The hook module file is `flow-ops.ts` (kebab-case, matching `use-extension-events.ts`) but the family segment in the query key is `'flow_ops'` (snake_case, matching the Rust tool id `rubix.flow_ops.*`). README calls this out.
- `audit.ts` uses loose types (`AuditFilter` / `AuditPage` with `unknown[]` for `changes`) because starter-client-ts has no typed audit surface yet. Per the rubix-client-ts endpoints/index.ts comment this is SCOPE OQ-3 — when the typed surface lands, replace the `fetchJson` call with `client.starter.audit*`.
- Stale duplicate path `rubix/packages/rubix-client-ts/rubix/packages/rubix-client-react/` exists on disk but is untracked; ignored.

## Open questions

- Whether `useUndoLast` invalidating the whole `['rubix']` root is too aggressive; left as-is since undo is an admin-grade rare affordance, but a future stage could narrow it once we know which families undo actually touches.
