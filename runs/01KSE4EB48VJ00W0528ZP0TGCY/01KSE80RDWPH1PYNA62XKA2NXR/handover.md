## Done

- Added `rubix/packages/rubix-client-react/src/hooks/flow-events.ts` exposing `useFlowEvents(flowId, opts?)` over `/api/v1/flows/{flowId}/events` with `{ events, latest, runOverlay, status, error, reconnect }`. `runOverlay` aggregates each NodeEmitted frame into `nodes[node]="ok"` and `slotValues[node][slot]=value`, structurally matching `@nube/starter-ui-flow`'s `RunOverlay`.
- Added sibling `flow-events.test.tsx` using the existing `makeHarness` + a `MockEventSource` to assert per-flow SSE path, frame accumulation, bufferSize cap, overlay aggregation, and reconnect-clears-state.
- Re-exported the new hook from `src/index.ts`.
- `pnpm --filter @nube/rubix-client-react typecheck` and `... test` both pass (14 files / 66 tests).
- Committed as `phase E.2 — useFlowEvents hook` on branch `codeless/rubix-flow-live-tick-demo`.

## Next

- (none) — next stage will be picked up by a fresh session.

## What you need to know

- The rubix-agent SSE route currently only emits `NodeEmitted` as default `data:` frames (see `rubix/crates/rubix-agent/src/routes/flow_events.rs`); other variants are server-side filtered. The hook therefore promotes every received frame to `"ok"`. The `NodeRunState` union and `runOverlay` shape are forward-compat for `running`/`error` once the server adds named SSE event types.
- Followed the `useExtensionEvents` pattern (ring buffer via `useEffect` on `stream.data`, mock EventSource in tests). The package has no `@nube/starter-ui-flow` dep — types are structural mirrors, matching `flow-ops.ts`.
- `flowEventsPath` is exported and `encodeURIComponent`s the flow id.

## Open questions

- (none)
