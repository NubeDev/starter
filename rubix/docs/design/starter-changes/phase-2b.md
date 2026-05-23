# Starter changes — Phase 2b gates

`starter-mcp` + `starter-flow-surfaces` gaps discovered while
scoping rubix thin-slice PR 3 (MCP exposure). PR 3 could not land
without hacks until the three items below landed upstream. Size
order (smallest first); per the Phase 2a one-PR-per-piece
discipline, each shipped as its own upstream PR.

**All three items are now merged in-tree.** PR 3 (MCP exposure) is
unblocked.

See [README.md](./README.md) for the index and per-item format.
Adjacent: [phase-2a.md](./phase-2a.md) (auth Postgres impls,
complete) and [phase-2c.md](./phase-2c.md) (gRPC/CLI rough edges +
the latent `starter-i18n` interpolate feature-gate bug).

## U1 — `starter-mcp` Accept-Language plumbing

`starter-mcp` extend · blocks rubix PR 3 · **merged (commit `4c15dcb`)**, ~150 LOC (smallest).

*Gap.* `starter-mcp::http::handle()` read only the body; no task-local for session locale (cf. `principal_local`). PR 3's exit signal ("Spanish on `Accept-Language: es-AR`") could not be met — MCP-dispatched tools could not see caller language.

*Shape.* Extract `Accept-Language` in `mcp_router`'s HTTP handler and at the stdio `initialize` handshake; bind as task-local mirror of `with_principal` so tools read via `current_locale()`; plumb through `dispatch` so `tools/call` runs inside the locale scope.

*No workaround.* Reading the header in `rubix-agent` and stuffing it into tool args sideways forks the MCP transport contract for one consumer — violates R2 and the no-hacks discipline locked in -5.

*Test.* `starter-mcp` round-trip: `tools/call` with `Accept-Language: es-AR`; tool reads `current_locale()` → `es-AR`. **Landed; 32 tests in `starter-mcp`.**

## U2 — `starter-mcp` `InMemoryTransport` real implementation

`starter-mcp` extend · blocks rubix PR 3 · **merged (commit `9ab273d`)**, medium PR.

*Gap.* `InMemoryTransport { _private: () }` was a stub. PR 3 names "round-trip via the MCP testing harness" as the test surface; without one, no end-to-end MCP assertion was possible.

*Shape.* Bidirectional in-memory channel pair implementing the transport trait; constructor returns `(client, server)`; async `send`/`recv` over unbounded channels. Standard MCP-SDK shape.

*No workaround.* Calling dispatch directly skipped serialisation, framing, and `initialize` — the very surface PR 3 must prove. Textbook hack.

*Test.* Round-trip on the new transport: client `tools/list` then `tools/call`; assert framed JSON-RPC. **Landed.**

## U3 — `starter-flow-surfaces` `FlowRegistry::resolve` + `FlowAsTool::from_registry`

`starter-flow-surfaces` extend · blocks rubix PR 3 · **merged (commit `7216d78`)**, largest.

*Gap.* Wrapping `com.rubix.scheduled-system-check` as a `FlowAsTool` required hand-rolled per-flow plumbing: `Engine` + topology + terminal slots + seed/output adapters + explicit schemas (D-F3.4 forbids derivation). `FlowAsTool` docstring (~L76-79) noted auto-resolve "lands later." ~50 lines of glue per flow.

*Shape.* `FlowRegistry::resolve(flow_id, revision) → (Topology, Vec<TerminalSlot>)`; YAML loader registering `flows/*.yaml` into a `FlowRegistry` at boot; `FlowAsTool::from_registry(flow_id, revision, engine)` constructor that wires default seed/output adapters.

*No workaround.* Per-flow hand-rolled glue is the duplication R2 forbids; a rubix-local helper would fork the registry contract.

*Test.* `from_registry("com.example.foo", "v1", engine)` returns a `FlowAsTool` whose `dispatch` runs end-to-end and produces a `Diagnostic` matching the topology's terminal slot. **Landed; 7 new tests in `starter-flow-surfaces` (plus existing stage7/stage8 tests).**

## Summary

PR 3 wiring in rubix now reduces to a single line:

```rust
let tool = FlowAsTool::from_registry(&registry, &flow_id, &rev, engine).await?;
```

plus loading the YAML and assigning adapter slot names. See the
active session handoff
[2026-05-23-next-steps-7.md](../../sessions/2026-05-23-next-steps-7.md)
for the next-session target.
