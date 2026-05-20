# Stage 5 — Layer-1 invariant review (gate before agent-as-tool bridge)

Reviewed diff from `83e48e8` (Phase 1 starter) through `bb8a2c0` (stage 4
agents chat surface).

## Verdict

**PASS** — Layer-1 invariants hold; no scope drift from `SCOPE.md`.

## R1 — crate dependency direction

`examples/flow-agent/Cargo.toml` consumes workspace crates downstream
only: `starter-spi`, `starter-config`, `starter-observability`,
`starter-server`, `starter-store-sqlite`, `starter-flow`,
`starter-flow-nodes`, `starter-flow-spi`, `starter-ai`. No starter
crate gains a reverse dependency on the example. No new starter crate
was added.

## R2 — single transport

Only SSE + REST as required by F3. `grep` finds no `websocket`,
`grpc`, `tonic`, or `mcp::transport` references under
`examples/flow-agent/`. The chat stream and run-event channel both
use `axum::response::sse::Sse` with the 15 s keep-alive helper
documented in SCOPE.

## R4 / R5 — trust boundary

No `Authenticator`, no bearer/cookie/JWT handling, no auth crate
dependency. F4 (no login, bind to `127.0.0.1`) is preserved. The only
`auth` substring match is a user-facing hint string in
`ai_runtime.rs` about `claude auth login` for the Claude CLI runner.

## Wire formats

* Chat SSE frames in `ai_runtime.rs` emit
  `{"type":"text","text":…}`, `{"type":"tool-call","toolCall":…}`,
  `{"type":"error","error":…}`, and the literal `[DONE]` sentinel —
  exactly the contract `createSseAdapter`'s default parser in
  `@nube/starter-ui-chat` consumes.
* Run-event SSE in `rest.rs::flow_events` serializes the `RunEvent`
  enum (`run-started` / `node-status` / `edge-active` /
  `run-finished`) as JSON; the frontend `RunEventDto` in
  `FlowEditor.tsx` matches each variant 1:1.

No wire-format mutation in stages 1–4.

## Disconnect robustness (specific gate items)

* **Run-event overlay (EventSource via `useSse`).** `useSse` opens
  `new EventSource(url)` inside a `useEffect`, returns
  `() => es.close()` for cleanup, and is keyed on `url` (`null`
  disables). EventSource handles reconnect natively on network
  drops; cleanup on URL change / unmount prevents subscriber leaks
  when navigating between flows. Tab-backgrounding leaves the
  EventSource open in modern browsers; the broadcast channel on the
  server side is bounded and drops on lag (no per-client buffering).
* **Chat stream (POST + fetch + AbortSignal).** `createSseAdapter`
  reads `res.body` and yields deltas until `signal.aborted` or
  `[DONE]`. The `<Chat>` host owns the controller and aborts on
  unmount; navigating between agents re-mounts `<AgentChat>` (the
  `useMemo` for `adapter` is keyed on `id`) so no stale reader
  survives. Mid-stream reconnect is not implemented — POST-based SSE
  has no resumable protocol — but this is the documented contract of
  `createSseAdapter`, not a regression.
* **Subscriber leaks across navigation.** Confirmed by inspection:
  `useSse` is the only run-event subscriber, lives at component
  scope, and its cleanup closes the underlying EventSource. The
  terminal-clear `setTimeout` is also cleared on unmount.

## Scope drift

Stages 1–4 land exactly the surfaces SCOPE phases 2–4 describe:
visual editor against `/api/flows/{id}` (phase 2), `/fire` + SSE
overlay + runs panel (phase 3), agents chat + providers list (phase
4). No new REST routes, SSE channels, or crate dependencies beyond
what `SCOPE.md` enumerates.
