## Done

- Added `@nube/starter-ui-flow` + `@xyflow/react` deps to rubix/frontend.
- Created `rubix/frontend/src/lib/flow-registry.ts` building a `NodeKindRegistry` at app boot from starter-ui-flow's `BUILTIN_NODE_KINDS`, with the `ai-agent` entry replaced by the rubix override. Exposes `buildFlowRegistry()` + `getFlowRegistry()` singleton.
- Created `rubix/frontend/src/lib/flow-nodes/ai-agent-node.tsx` — wraps `BaseNode` and renders `data.skill_hint` as a label line + `data.allowed_tools.length` as a pill badge.
- `pnpm --filter @nube/rubix-frontend typecheck` green.
- Committed as `82a15d2`.

## Next

- Stage 6 picks up; consume `getFlowRegistry()` from the flow browser route (whatever surface mounts FlowCanvas).

## What you need to know

- `@nube/starter-ui-flow` was not previously a rubix dep — added together with `@xyflow/react ^12.3.0` (NodeProps type isn't re-exported by starter-ui-flow, so the rubix override imports it directly).
- The registry deliberately filters BUILTIN_NODE_KINDS through an allowlist (`BUILTIN_KINDS_KEPT`) so any new upstream kind requires an explicit opt-in here rather than silently appearing.
- starter-ui-flow itself was NOT modified, per scope.

## Open questions

- (none)
