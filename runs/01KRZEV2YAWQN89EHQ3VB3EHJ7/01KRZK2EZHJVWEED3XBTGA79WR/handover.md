## Done

- reviewed stage 6 (supervisor) + stage 7 (server admin slice) diffs against Layer-1 invariants R1/R2/R4/R5 + wire-format stability
- confirmed crate dependency arrow holds: ext-spi → starter-spi only; supervisor → ext-spi/ext-host/jsonrpc-stdio; server → ext-spi/ext-host/ext-supervisor/starter-spi/starter-server
- confirmed JSON-RPC framing is shared via the newly-extracted starter-jsonrpc-stdio (one impl, consumed by both starter-mcp and the supervisor — no drift)
- confirmed CapabilityGate enforces at the wire boundary per R8 with the violation counter wired through to GET /extensions/<id>

## Next

- (none — next session picks up the next stage)

## What you need to know

- PASS sentinel emitted below
- gate exit-criteria smoke tests named in the stage description (extension-survives-host-restart, crash-loop-is-bounded, capability-violation-rejected-logged-counted end-to-end, in_memory_transport() state-machine, admin SSE event tail end-to-end) are NOT present in the diff. Supervisor only ships the hello-process integration test + per-module unit tests; admin routes test does not exercise the SSE path. These are outside the narrow Layer-1 rubric this review stage is gated on, but should be raised in a later ramp step
- additional non-blocking quality bugs spotted: handle_frame emits StateTransition{Running} per response (ring spam); send_sigterm on unix is documented to actually start_kill (SIGKILL) immediately so the shutdown_grace_ms window never applies in v0.1; do_handshake will misclassify any frame the child emits before the init result as a malformed response

## Open questions

- should the missing in_memory_transport / SSE-tail / restart-survives smoke tests be promoted into the rulebook as Layer-1 checks, or stay as stage-exit checklist items the review is not required to enforce?
