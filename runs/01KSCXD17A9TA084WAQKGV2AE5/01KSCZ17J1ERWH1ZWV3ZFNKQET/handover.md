## Done

- Stage 9 analysis only — confirmed rubix-agent routes module exposes only `auth`, `openapi_doc`, and `tools` (the generic `POST /api/v1/tools/{tool_id}` dispatcher).
- Enumerated all `rubix.clickhouse.*` and `rubix.insights.*` tool ids registered across `rubix/crates`: only three write verbs exist (`rubix.clickhouse.rule.write`, `rubix.clickhouse.mart.create`, `rubix.clickhouse.retention.set`); zero `rubix.insights.*` tool ids exist.
- Surveyed `rubix/packages/rubix-client-react/src/hooks/`: present hook files are `audit`, `clickhouse`, `extensions`, `flow-ops`, `mcp`, `system`, `teams`, `tenants`, `undo`, `use-extension-events`, `users`. The `clickhouse.ts` hook exports only `useRuleWrite`, `useMartCreate`, `useRetentionSet`, `CLICKHOUSE_KEY`. No `insights.ts` hook file exists. None of the eight hooks named in the stage description currently exist.
- No commits made (analysis-only stage).

## Next

- **BLOCKED** — a separate rubix-agent job must add the eight missing tool ids / endpoints (see "What you need to know") before phase C.1 of this job can resume.
- After that job lands, the next session for THIS job should re-run Stage 9's verification (re-grep `rubix/crates/rubix-agent/src/routes/` + `rubix/crates/rubix-spi/src/dto/`) and, if green, proceed to the next stage in SCOPE.md.

## What you need to know

- Eight missing rubix-agent endpoints / tool ids (each dispatchable via `POST /api/v1/tools/{tool_id}` or a dedicated route):
- The agent uses a single generic dispatcher `POST /api/v1/tools/{tool_id}` (see `rubix/crates/rubix-agent/src/routes/tools.rs`); no per-verb REST route file is needed — only tool registration in `rubix-tools` + DTOs in `rubix-spi/src/dto/`.
- Existing precedent for tool-id naming + DTO layout: `rubix/crates/rubix-spi/src/dto/clickhouse/{rule_write,mart_create,retention_set}.rs`.
- Hook coverage in `rubix-client-react` for already-shipped endpoints: `clickhouse.ts` covers the three write verbs; `audit.ts`, `extensions.ts`, `flow-ops.ts`, `mcp.ts`, `system.ts`, `teams.ts`, `tenants.ts`, `undo.ts`, `users.ts` cover their respective surfaces. None of the eight new hooks exist yet — they will need to be authored alongside (or after) the new endpoints; whether that happens in the rubix-agent job, in `rubix-client-react`, or in this frontend job is an open question (see below).
- No files were modified in this stage; `git status` is clean relative to job start.

## Open questions

- Which job/team owns authoring the eight new `rubix-client-react` hooks once the endpoints land — the rubix-agent endpoint job, a follow-up `rubix-client-react` job, or a later stage of this frontend job? SCOPE language ("a separate rubix-agent job lands them") implies endpoints only; hook authorship is unspecified.
- Whether `rubix.clickhouse.tables.list` should expose all warehouse tables or only the rubix-managed subset (marts + ruler-written) — a product decision the endpoint job will need.
- Whether insights rules need a `delete` verb in addition to enable/disable — not in the stage's hook list, so assumed out of scope, but worth confirming with the agent job's spec author.
