## Done

- Wired `GET /api/flows/{id}/events` into `FlowEditor.tsx` via `useSse`; reduced `RunEvent` frames into a `RunOverlay` (per-node `running`/`ok`/`error`/etc. plus `activeEdges`) and passed it to `<FlowCanvas overlay={overlay} />`.
- Added a "Run" button next to Save that POSTs `/api/flows/{id}/fire`. Disabled while dirty or while a run is in flight. Errors surface as a non-destructive Alert.
- Held the terminal frame for 1s on `RunFinished` before clearing the overlay (`TERMINAL_CLEAR_MS = 1000`).
- Added a `RecentRunsPanel` below the canvas: last 10 runs from `GET /api/flows/{id}/runs` with status `<Badge>` + timestamp, polling every 5s and force-refetched on `run-started`/`run-finished`.
- `pnpm typecheck` green (run inside `examples/flow-agent/frontend`).
- Committed as `72945d5` on `codeless/flow-agent-example`.

## Next

- Stage 4 (per `SCOPE.md`) — fresh session will pick it up.

## What you need to know

- The reducer filters node-status/edge-active events by `activeRunId` so a stale run can't bleed into a new one. `activeRunId` is seeded both by the `run-started` SSE and by `fire.mutate`'s `onSuccess` (whichever arrives first).
- `RunEventDto` is a local type mirroring `src/sse.rs::RunEvent` (kebab-case discriminants). Keep them aligned if the Rust enum changes.
- `statusToNodeRunState` accepts `running`/`ok`/`success`/`completed`/`error`/`failed`/`cancelled`/`skipped`/`ready`; anything else maps to `idle`. The engine emits `running`/`ok`/`error` today — extra cases are forward-compat.
- The Run button stays disabled while `dirty` so users save first; the tooltip explains why. Active run blocks re-fire to avoid overlapping overlays.
- `useSse` reconnects automatically (EventSource default) — no manual retry logic added.

## Open questions

- (none)
