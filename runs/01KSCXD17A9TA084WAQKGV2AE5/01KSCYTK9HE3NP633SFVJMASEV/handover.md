## Done

- Added rubix/frontend/e2e/flows.spec.ts: logs in, navigates /flows, asserts ≥6 table rows, clicks com.rubix.scheduled-system-check, asserts a `.sf-node__kind` span containing `ai-agent` mounts in the FlowCanvas.
- Committed as 5cf4298 on codeless/rubix-frontend-surfaces.

## Next

- Stage 8 (next session) per the WORKFLOW.

## What you need to know

- The test mirrors authz-admin.spec.ts login flow (op@example.com / rubix-dev-passwd).
- Selector for the ai-agent node uses `.sf-node__kind` — BaseNode renders `<span className="sf-node__kind">{spec.kind}</span>`, which is the literal `ai-agent`. The rubix override `RubixAiAgentNode` still wraps BaseNode, so the selector is valid whether the body endpoint is live or stubbed.
- `useFlowDefinition` currently synthesises a placeholder graph with one `ai-agent` node per flow (rubix-agent still has no flow-body endpoint — see stage 3 BLOCKED note), so the canvas assertion is satisfied by the placeholder.
- Could not actually run `pnpm --filter @nube/rubix-frontend e2e` here — no running rubix-agent backend in this worktree. Operator must run `mani run demo` then `pnpm --filter @nube/rubix-frontend e2e` to verify green.

## Open questions

- (none)
