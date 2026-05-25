## Done

- reviewed Phase E diff (9fe334d..HEAD) against Layer-1 invariants R1/R2/R4/R5
- PASS: R1 deps stay one-way (rubix-spi/-tools/-client-*/frontend only consume starter-flow-spi); R2 single SSE route /api/v1/flows/{id}/events + JSON-RPC tools unchanged; R4/R5 trust boundary intact — server still validates via DefinitionManager::publish on deploy; wire-format additions (FlowListItem.body_yaml, flow_ops.kinds verb, NodeSlotValue DTO) are purely additive

## Next

- Phase F (a fresh session will pick it up)

## What you need to know

- four E commits: 7e18919 (E.1 list.body_yaml + flow_ops.kinds), e09d320 (E.2 useFlowEvents hook), d100b4a (E.3 live overlay + settings sidebar), cb934fa (E.4 playwright spec)
- e2e count: 1 new playwright spec file (rubix/frontend/e2e/flow-live-tick.spec.ts) covering live-tick + hot-edit + restart-persistence scenarios
- operator-runnable manual flow: `make start` → open http://127.0.0.1:5185/flows/com.rubix.tick-counter → watch count climb every 5s → click counter node → change step to 10 → Save → next tick jumps +10 → on the trigger node change cron to `*/2 * * * * *` → Save → ticks accelerate to every 2s → refresh browser → count preserved → `make restart` → count restored from sqlite and resumes climbing

## Open questions

- (none)
