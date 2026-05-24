## Done

- Extended `rubix/packages/rubix-client-react/src/hooks/clickhouse.ts` with `useClickhouseRulesList`, `useClickhouseMartsList`, `useClickhouseMartDrop`, `useClickhouseTablesList` and added matching cases to `clickhouse.test.tsx`
- Added `rubix/packages/rubix-client-react/src/hooks/insights.ts` with `useInsightsRulesList`, `useInsightsRuleCreate`, `useInsightsRuleEnable`, `useInsightsRuleDisable` plus sibling `insights.test.tsx`
- `pnpm --filter @nube/rubix-client-react typecheck` clean; `test` 13 files / 60 tests green (+13 new across the two suites)
- Committed as `stage 10: phase C.2 — rubix-client-react hooks for clickhouse + insights — feat(rubix-client-react) clickhouse list/drop + insights hooks` (`9f98927`)

## Next

- (none) — next session picks up the following stage

## What you need to know

- All eight new hooks dispatch directly via `fetchJson` against the generic `POST /api/v1/tools/{tool_id}` route (mirroring the inlined-transport pattern `useAudit` established) because the backing tool ids do not yet exist in `@nube/rubix-client-ts` / `rubix-agent` — the stage 9 handover flagged this as BLOCKED but stage 10 was still scheduled. The hook signatures stay stable so swapping to typed `client.*` methods is a one-liner per hook once the agent job lands the endpoints.
- Mutation hooks thread `X-CSRF-Token` and invalidate `['rubix','clickhouse']` / `['rubix','insights']` on success. Read hooks key on `[…, 'rules' | 'marts' | 'tables']`.
- Inline DTOs (`ClickhouseRuleSummary`, `InsightsRuleSummary`, etc.) are intentionally loose mirrors of the eventual `rubix-spi` shapes — tighten when the typed client lands.
- Tool ids used (need agent-side registration): `rubix.clickhouse.{rule.list,mart.list,mart.drop,tables.list}`, `rubix.insights.rule.{list,create,enable,disable}`.

## Open questions

- The agent-side tool ids are still missing (see stage 9 BLOCKED handover); UI work that consumes these hooks will hit 404s against a live backend until the rubix-agent job lands the endpoints. Tests are mock-based so they pass regardless.
