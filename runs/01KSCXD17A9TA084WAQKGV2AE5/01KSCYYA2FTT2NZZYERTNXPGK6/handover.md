## Done

- Reviewed Phase B diff: B.1 6eb258c, B.2 82a15d2, B.3 40e7191, B.4 5cf4298.
- Confirmed `readOnly={true}` on `<FlowCanvas>` at rubix/frontend/src/routes/flows/$flowId.tsx:80, alongside showMiniMap / showControls / showBackground.
- Verified Layer-1 invariants hold: no crate-graph changes (R1), no new transport — still `/api/v1/tools/*` POST (R2), no trust-boundary code touched (R4/R5), and `FlowListRequest`/`FlowListResponse` wire types unchanged.
- Confirmed `@nube/starter-ui-flow` is untouched — ai-agent override lives rubix-side in `rubix/frontend/src/lib/flow-nodes/ai-agent-node.tsx` + `flow-registry.ts`.
- Recorded SCOPE OQ-2 evidence: no `rubix.flow_ops.get` verb and no `/api/v1/flows-definitions/<id>` route exist; `useFlowDefinition` synthesises a one-node placeholder FlowGraph + the `/flows/$flowId` route surfaces a `flows.detail.placeholder` banner. Hook signature `UseQueryResult<FlowGraphResult>` is stable for the real yaml.parse swap when the backend body endpoint lands.
- Emitted gate sentinel and committed empty marker commit `320749f`.

## Next

- Phase C (a fresh session should pick up): rubix-side ClickHouse + insights admin surface per SCOPE.

## What you need to know

- PASS: four B commits present (6eb258c B.1 BLOCKED note, 82a15d2 B.2 registry+override, 40e7191 B.3 routes+useFlowDefinition, 5cf4298 B.4 e2e); FlowCanvas mounted readOnly; OQ-2 resolved by client-side placeholder behind a stable hook signature; no wire formats, crate graph, transport, or trust-boundary code touched.
- Operator-runnable manual flow: `make start` → http://localhost:5173 → login → "Flows" in fleet nav → `/flows` table renders ≥6 rows → click `com.rubix.scheduled-system-check` → `/flows/$flowId` mounts FlowCanvas readOnly with the rubix ai-agent override visible and a placeholder banner above the canvas.
- Caveat (non-gating, already documented in B.1/B.3 commits): canvas shows a stub node, not the deployed YAML, until rubix-agent exposes a flow-body endpoint. Swap-in is a one-call change in `useFlowDefinition`.
- `yaml` npm dep was intentionally NOT added in B.1 (deferred until the backend endpoint lands) — keep that in mind if a later phase looks for it.

## Open questions

- (none) — OQ-2 explicitly resolved this stage; downstream phases can proceed.
