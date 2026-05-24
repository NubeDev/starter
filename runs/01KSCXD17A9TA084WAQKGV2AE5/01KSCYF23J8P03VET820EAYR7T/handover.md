## Done

- Added `useFlowsList` (alias of `useFlowList`) + `useFlowDefinition(flowId)` to `rubix/packages/rubix-client-react/src/hooks/flow-ops.ts`. `useFlowDefinition` synthesises a placeholder `FlowGraph` from list metadata because rubix-agent still has no body endpoint (stage 3 BLOCKED). `FlowGraph` is mirrored inline as a structural type to avoid coupling the transport package to `@nube/starter-ui-flow`.
- Added `rubix/frontend/src/routes/flows/index.tsx` — table with the four spec columns (`flow_id`, `latest_revision_id`, `last_deployed_at`, `supersession_count`), `<Skeleton>` loading, `<Empty>` empty state, row click → `/flows/$flowId`.
- Added `rubix/frontend/src/routes/flows/$flowId.tsx` — `<FlowCanvas registry graph readOnly showMiniMap showControls showBackground>` using `buildFlowRegistry()`; placeholder banner shown when the synthesised graph is in use.
- `main.tsx`: added `@xyflow/react/dist/style.css` + `@nube/starter-ui-flow/styles.css` imports before `./styles/theme.css`.
- Nav: re-pointed the existing fleet `Flows` entry from `#flows` to `/flows`.
- i18n: added `flows.*` keys to `en.json` (es.json not extended this stage — fallbacks present via `defaultMessage`).
- Regenerated `routeTree.gen.ts` via `@tanstack/router-cli generate`.
- `pnpm --filter @nube/rubix-frontend typecheck` clean; `pnpm --filter @nube/rubix-client-react test` 51/51 green.
- Committed as `40e7191`.

## Next

- Stage 7 picks up the next phase per WORKFLOW.md. Manual smoke: `pnpm --filter @nube/rubix-frontend dev` → `/flows` (table) → row click → `/flows/$flowId` (canvas with placeholder banner).

## What you need to know

- `last_deployed_at` and `supersession_count` columns render em-dash placeholders — `rubix.flow_ops.list` only returns `{flow_id, revision_id}`. Extending the DTO (or adding `rubix.flow_ops.get`) will let those cells + the real FlowCanvas graph light up; the hook signature is intentionally stable.
- `useFlowDefinition` calls `client.flowList({})` and filters client-side — fine for the small flow counts seen today, but should switch to a dedicated get-by-id once it lands (no extra round-trips, cacheable per `flowId`).
- No es.json additions this stage; the route components pass `defaultMessage` so missing locale keys do not throw — extend `es.json` when locales sweep happens.
- `@nube/starter-ui-flow` is a *peer dependency in spirit* of `useFlowDefinition` (its return type is assignable to its `FlowGraph`) but is not a package-level dependency of `rubix-client-react`; the mirror interface keeps the layering clean.

## Open questions

- Which path unblocks B.1 properly: add `rubix.flow_ops.get` (preferred — paged-friendly, keeps `list` light) or inline `body_yaml` into `FlowListResponse`? Stage 3 raised this and stage 6 still cannot answer it.
