## Done

- Reviewed diff from 83e48e8 → bb8a2c0 against rulebook Layer-1 invariants (R1/R2/R4/R5) and the SCOPE.md surface for flow-agent.
- Confirmed wire formats are unchanged: chat SSE frames (`text` / `tool-call` / `error` / `[DONE]`) and run-event SSE (`run-started` / `node-status` / `edge-active` / `run-finished`) both match SCOPE §"SSE endpoints" and the `createSseAdapter` default parser.
- Verified run-event overlay uses `useSse` whose cleanup closes the EventSource on URL change/unmount — no subscriber leak across navigations; the terminal-clear `setTimeout` is also cleared on unmount.
- Verified chat stream is POST+fetch with AbortSignal owned by `<Chat>` and the adapter `useMemo` is keyed on agent id, so re-navigation tears down the prior reader cleanly.
- `cargo check -p flow-agent` green.
- Wrote `examples/flow-agent/STAGE-5-REVIEW.md` capturing the review notes; committed as `stage 5 — Layer-1 invariant gate before agent-as-tool bridge` (cf3f25b).

## Next

- Stage 6: implement the agent-as-tool bridge (register each flow as an `AiTool`; agent tool calls fire the flow over the same domain path the UI uses; runs emit on the same SSE channel).

## What you need to know

- PASS: R1/R2/R4/R5 hold, wire formats untouched, no subscriber leak across navigations, no scope drift in stages 1–4.
- Known limitation (NOT a regression): chat SSE has no mid-stream reconnect because POST-based SSE has no resumable protocol; this is the documented contract of `@nube/starter-ui-chat`'s `createSseAdapter`. EventSource on `/api/flows/{id}/events` does auto-reconnect.
- Backgrounded tab: modern browsers keep the EventSource open; the server-side broadcast channel drops on lag rather than buffering per client — acceptable for this demo.
- The only `auth`-string match under `examples/flow-agent/` is a user-facing hint about `claude auth login` for the CLI runner; no `Authenticator`, cookie, bearer, or JWT wiring exists.

## Open questions

- (none)

PASS: Layer-1 invariants (R1 crate-dep direction, R2 single SSE+REST transport, R4/R5 trust boundary) hold across stages 1–4, wire formats match SCOPE.md, run-event subscribers clean up on navigation, and no scope drift was introduced.
