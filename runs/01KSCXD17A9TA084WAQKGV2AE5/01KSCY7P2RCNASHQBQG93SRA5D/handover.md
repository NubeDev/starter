## Done

- Investigated stage B.1 prerequisite per SCOPE OQ-2: confirmed `rubix.flow_ops.list` returns metadata only (`flow_id`, `revision_id`) — see `rubix/packages/rubix-client-ts/src/endpoints/flow_ops.ts:40-49` and Rust DTO `rubix/crates/rubix-spi/src/dto/flow_ops/list.rs:20-38`.
- Confirmed no `rubix.flow_ops.get` verb exists (list.rs:53 doc-comment refers to it as "a future" verb).
- Confirmed no alternative HTTP route exposes flow bodies — `body_yaml` is only present in the internal `FlowDefStore` (`rubix/crates/rubix-tools/src/flow_ops/store.rs:44`) consumed by `deploy.rs` / `duplicate.rs`; no axum handler or tool surfaces it.

## Next

- BLOCKED. Next session must NOT add `useFlowDefinition` against a non-existent endpoint. Either (a) wait for a `rubix.flow_ops.get` tool to be implemented in `rubix-tools` + `rubix-spi` (returning `{flow_id, revision_id, body_yaml}` for a given `flow_id`) and exposed in `rubix-client-ts` as `flowGet`, or (b) extend `FlowListResponse` to include `body_yaml` per item (simpler but heavier on the list payload). Once either lands, this stage can be unblocked: add `useFlowDefinition(flowId)` to `rubix/packages/rubix-client-react/src/hooks/flow-ops.ts` that calls the new endpoint, parses `body_yaml` via the `yaml` npm package into `FlowGraph`, and add a sibling `.test.tsx`.

## What you need to know

- Missing endpoints (one paragraph): the rubix-agent surface has no read-by-id verb for flow definitions. `rubix.flow_ops.list` returns `Vec<{flow_id, revision_id}>` with no body; `rubix.flow_ops.lint` accepts a `body_yaml` but does not return one; `rubix.flow_ops.deploy` writes a body but doesn't read; `rubix.flow_ops.duplicate` reads a source body server-side but does not return it to the caller. There is no `/api/v1/flows-definitions/<id>` HTTP route — flow definitions are only addressable via the internal `pg://flows_definitions/{id}` SPI path used by `flow_notify.rs` and `flows_seed.rs`. Implementing this stage therefore requires a new Rust-side tool (`rubix.flow_ops.get`) or extending `FlowListResponse` with `body_yaml`.
- No code committed this stage (BLOCKED halt; no files modified).
- Worktree: `/home/user/.codeless/worktrees/job-01KSCXD17A9TA084WAQKGV2AE5`.

## Open questions

- Which path does the platform team prefer to unblock B.1: add `rubix.flow_ops.get` (clean, paged-friendly) or inline `body_yaml` into `FlowListResponse` (smaller diff, larger list payload)?
