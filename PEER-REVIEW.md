# Peer Review — `starter` workspace

**Date:** 2026-05-20
**Reviewer:** Claude (Opus 4.7)
**Branch:** `codeless/starter-flow-phase5-demo`
**Scope:** Verify the workspace delivers on four stated goals:

1. Reusable library that can be extended **without forking** the core
2. Generic server framework with **backend + frontend** extensions
3. Generic framework usable as a **flow-based workflow AI engine**
4. **gRPC / CLI / SSE / REST** all usable and extendable by extensions

---

## Verdict at a glance

| # | Goal | Verdict |
|---|------|---------|
| 1 | Reusable library, extensible without forking | **Yes** (with caveats) |
| 2 | Generic server framework — backend + frontend extensions | **Partial** |
| 3 | Generic flow-based AI workflow engine | **Partial** |
| 4 | Transports gRPC / CLI / SSE / REST usable and extendable | **Partial** |

The architecture is the strongest part of this repo. The seams, the SPI discipline, the manifest-driven extension model, and the frontend Module Federation story are all real and well thought out. The shortfall is *unfinished implementation* — several runtime paths are still `NotWired` stubs — not design that needs rework. Closing the top items below moves all three "Partial" verdicts to "Yes".

---

## Goal 1 — Reusable library, fork-free extension

**Verdict: Yes, with caveats.**

### Design — sound

The `R2` rule pins [crates/starter-spi/src/lib.rs](crates/starter-spi/src/lib.rs) as the zero-dep contracts crate. Every workspace `Cargo.toml` honors `default-features = []`. The dependency arrow in [SCOPE.md:281-313](SCOPE.md#L281-L313) is mechanically enforced. The sibling `starter-extensions/` workspace is explicitly excluded at [Cargo.toml:32](Cargo.toml#L32), preserving the "small libraries, not a framework" character.

### Strengths

- All public traits live in spi: `Authenticator`, `SecretStore`, `AiRunner`, `Tool`, `Service`, `Cancel`. Two-impl proofs ride them (`TokenAuthenticator` vs auth-server `Authenticator`, `RecordingAiRunner` vs `ClaudeRunner` in [crates/starter-ai/src/runners/](crates/starter-ai/src/runners)).
- `starter-server`'s seam is "hand me an axum Router", not "implement my Route trait" ([crates/starter-server/src/builder/](crates/starter-server/src/builder)). Right call.
- `examples/notes` (a real consumer) wires nine starter crates without modifying any of them ([examples/notes/src/server.rs](examples/notes/src/server.rs)).

### Caveats

- The `BoxedAuthenticator` newtype at [examples/notes/src/server.rs:230-237](examples/notes/src/server.rs#L230-L237) is a workaround for `Arc<dyn Authenticator>` not satisfying the generic bounds in `with_principal` / `McpHttpOptions::with_auth`. Every consumer using dyn auth will write the same boilerplate — the seam is not as polished as advertised.
- [TODO.md:19-22](TODO.md#L19-L22) admits the workspace `Cargo.toml` was partially reconstructed; phases 7-10 (docs, full example, theme) are open. The "ship without fork" smoke test from [SCOPE.md:710-717](SCOPE.md#L710-L717) is not a green CI signal today.

---

## Goal 2 — Server framework with backend + frontend extensions

**Verdict: Partial.**

### Design — sound and unusually well thought out

Manifest is the source of truth ([DOCS/extensions/scope/SCOPE.md R3](DOCS/extensions/scope/SCOPE.md)), one trait + three flavours (R1), reverse-DNS namespace ownership enforced ([starter-extensions/crates/starter-ext-host/src/validate.rs](starter-extensions/crates/starter-ext-host/src/validate.rs)). Backend transports each get a separate adapter crate (R13). Frontend extensions go through Module Federation with negotiated singletons ([starter-extensions/packages/starter-ext-ui/src/host-manager.ts](starter-extensions/packages/starter-ext-ui/src/host-manager.ts)).

### Backend strengths

- Two-phase commit loader ([starter-extensions/crates/starter-ext-host/src/loader.rs](starter-extensions/crates/starter-ext-host/src/loader.rs)) — a bad manifest never half-loads the registry.
- `BuiltinRestDispatcher`, `BuiltinCliDispatcher`, MCP `register_tools` all wire end-to-end.
- Admin slice (`GET /extensions`, `POST /extensions/<id>/enable`) is real and `Role::Admin`-gated ([starter-extensions/crates/starter-ext-server/src/admin.rs](starter-extensions/crates/starter-ext-server/src/admin.rs)).
- Supervisor is fleshed out (725 LOC at [starter-extensions/crates/starter-ext-supervisor/src/supervisor.rs](starter-extensions/crates/starter-ext-supervisor/src/supervisor.rs)) — spawn, restart policy, stderr forwarding, EventRing, JSON-RPC framing all present.

### Frontend strengths — real, not aspirational

- Real `ExtensionHostManager` with singleton negotiation ([starter-extensions/packages/starter-ext-ui/src/host-manager.ts](starter-extensions/packages/starter-ext-ui/src/host-manager.ts)).
- Notes example mounts an actual `<ExtensionSlot id="sidebar"/>` at [examples/notes/frontend/src/app.tsx:297](examples/notes/frontend/src/app.tsx#L297).
- MF runtime does dynamic `import(remoteEntry.js)` ([examples/notes/frontend/src/extension-host.ts:220](examples/notes/frontend/src/extension-host.ts#L220)).
- `com.nube.hello` example extension declares UI, tool, REST, i18n catalogs in one manifest ([examples/notes/extensions/com.nube.hello/block.yaml](examples/notes/extensions/com.nube.hello/block.yaml)).

### Critical gap — process/wasm dispatch is `NotWired`

Every adapter ships a `ProcessXxxDispatcher` / `WasmXxxDispatcher` that returns `DispatchError::NotWired`:

- [starter-extensions/crates/starter-ext-cli/src/dispatcher.rs:543-586](starter-extensions/crates/starter-ext-cli/src/dispatcher.rs#L543-L586)
- [starter-extensions/crates/starter-ext-server/src/rest/dispatcher.rs:337-366](starter-extensions/crates/starter-ext-server/src/rest/dispatcher.rs#L337-L366)
- [starter-extensions/crates/starter-ext-grpc/src/dispatcher.rs](starter-extensions/crates/starter-ext-grpc/src/dispatcher.rs)
- `starter-ext-mcp` only handles builtin per its own docs ([starter-extensions/crates/starter-ext-mcp/src/lib.rs:24-27](starter-extensions/crates/starter-ext-mcp/src/lib.rs#L24-L27))

The supervisor is built; the wire between supervisor and dispatcher isn't. Today a process-flavour extension cannot serve a tool call — it boots, dies if it crashes, and reports state, but its handlers are unreachable. **This is the single biggest gap between SCOPE and reality.**

Also: `examples/notes/src/server.rs` builds an `ExtensionGrpcService` but wires `NotWiredGrpcDispatcher` ([examples/notes/src/server.rs:215](examples/notes/src/server.rs#L215)). The notes app's gRPC backplane is decorative. `starter-ext-wasm` exists (492 LOC of host) but is feature-gated and not wired through any dispatcher.

---

## Goal 3 — Flow-based AI workflow engine

**Verdict: Partial.**

### Design — excellent

"Everything is a node" ([DOCS/flow/scope/SCOPE.md R1](DOCS/flow/scope/SCOPE.md)), single write chokepoint, stateless behaviours ([crates/starter-flow-spi/src/node.rs:25-42](crates/starter-flow-spi/src/node.rs#L25-L42)), AI agent is *just* a node kind that uses `starter-ai::AiRunner`. `FlowAsTool` / `FlowAsService` ([crates/starter-flow-surfaces/src/lib.rs](crates/starter-flow-surfaces/src/lib.rs)) makes every flow MCP/REST/CLI-callable for free.

### Strengths

- Working three-node demo (`trigger.explicit → ai-agent → log`) running against a real Claude CLI runner ([examples/notes/src/flow_demo.rs:81-187](examples/notes/src/flow_demo.rs#L81-L187)).
- Engine has real bodies: ~1.1k LOC propagator ([crates/starter-flow/src/propagator.rs](crates/starter-flow/src/propagator.rs)), 1.3k LOC run/RunHandle ([crates/starter-flow/src/run.rs](crates/starter-flow/src/run.rs)), 689 LOC engine ([crates/starter-flow/src/engine.rs](crates/starter-flow/src/engine.rs)).
- `ai_agent.rs` is 1551 LOC — real implementation, not a sketch ([crates/starter-flow-nodes/src/ai_agent.rs](crates/starter-flow-nodes/src/ai_agent.rs)).
- Smoke tests cover crash-and-resume, four-transport event streaming, skill quarantine, FlowAsService ([crates/smoke-tests/tests/](crates/smoke-tests/tests)).

### Weaknesses

- **Most built-in node kinds are unimplemented.** ~13 LOC stubs each: `branch.rs`, `merge.rs`, `gate.rs`, `http_out.rs`, `sleep.rs`, `subflow.rs`, `trigger_event.rs`, `trigger_schedule.rs`, `trigger_webhook.rs` ([crates/starter-flow-nodes/src/branch.rs](crates/starter-flow-nodes/src/branch.rs)). Only `ai_agent`, `tool_call`, `log`, `transform`, `trigger_explicit` have real bodies. The motivating "ingest webhook → agent → branch → DB → loop" workflow from [DOCS/flow/scope/SCOPE.md:54-62](DOCS/flow/scope/SCOPE.md#L54-L62) is not yet expressible.
- **`contributes.nodes` and the `starter-ext-flow` adapter don't exist.** Flow SCOPE promises extensions contribute node kinds; there is no adapter crate ([crates/starter-flow/src/registry.rs:48](crates/starter-flow/src/registry.rs#L48) marks this Phase 6). Extensions can ship REST handlers and tools, but cannot ship flow nodes.
- **Propagator quiescence bug.** The demo's `quiescence = 60s` at [examples/notes/src/flow_demo.rs:167-170](examples/notes/src/flow_demo.rs#L167-L170) — and its admission that "the propagator emits no SlotChanged events while a node body is mid-invoke" — is a real leak. The handler short-circuits on `NodeEmitted` at [examples/notes/src/flow_demo.rs:330-336](examples/notes/src/flow_demo.rs#L330-L336) to avoid waiting it out.

---

## Goal 4 — gRPC / CLI / SSE / REST usable + extendable

**Verdict: Partial.**

### Design — clean

Each transport is a thin adapter around `Tool` / `Authenticator` / `Router` seams in spi. Every transport has a sibling extension adapter (`starter-ext-server/rest`, `starter-ext-cli`, `starter-ext-mcp`, `starter-ext-grpc`). New transports = new adapter crate ([DOCS/extensions/scope/SCOPE.md R13:320-359](DOCS/extensions/scope/SCOPE.md#L320-L359)). Per-run `broadcast::Receiver<FlowEvent>` rides the same shape over each transport ([crates/smoke-tests/tests/flow_event_stream_over_four_transports.rs](crates/smoke-tests/tests/flow_event_stream_over_four_transports.rs)).

### Strengths

- All four transports present and tested at host level: `starter-server` (REST + SSE helpers at [crates/starter-server/src/sse/](crates/starter-server/src/sse)), `starter-grpc` ([crates/starter-grpc/src/service.rs](crates/starter-grpc/src/service.rs)), `starter-mcp` (HTTP behind `feature="http"`), `starter-cli` ([crates/starter-cli/src/registry.rs](crates/starter-cli/src/registry.rs)).
- Auth is uniform via `Authenticator::verify(&str)` — same trait across HTTP, MCP, gRPC ([SCOPE.md:816-821](SCOPE.md#L816-L821)).

### Inconsistencies

- **SSE for extensions is one-sided.** Host SSE helpers exist ([crates/starter-server/src/sse/](crates/starter-server/src/sse)) but extensions don't have a `contributes.sse` block — SSE from extensions piggybacks on `streaming: stdout` REST entries via NDJSON.
- **gRPC for the host is tool-only.** `starter-grpc` ships exactly one service: `starter.tools.v1.Tools/{ListTools,CallTool}` ([crates/starter-grpc/src/lib.rs:6-7](crates/starter-grpc/src/lib.rs#L6-L7)). Consumer-defined typed gRPC services are explicitly out of scope ([crates/starter-grpc/src/lib.rs:13-18](crates/starter-grpc/src/lib.rs#L13-L18)).
- **gRPC for extensions is wired but not dispatched.** `NotWiredGrpcDispatcher` is the only path the notes example takes ([examples/notes/src/server.rs:213-217](examples/notes/src/server.rs#L213-L217)).
- **MCP-SSE deferred to v0.2** ([TODO.md:496-498](TODO.md#L496-L498)). MCP-over-HTTP is single-shot, not streaming.
- **`CallTool` is unary only** ([crates/starter-grpc/src/lib.rs:38-44](crates/starter-grpc/src/lib.rs#L38-L44)). No gRPC server-streaming `Tool` until a `StreamingTool` trait lands in spi.

Streaming surface area: SSE + MCP-SSE + gRPC-streaming + CLI-streaming form a 2×2 with three quadrants empty.

---

## Cross-cutting findings

### Where extensions could be forced to fork

None at the SPI / engine level. Risk is in features absent today: an extension author who needs a custom node kind, an SSE-streaming endpoint, or a typed gRPC service has nowhere to ship it without modifying starter or starter-extensions. Today this is "wait for the adapter to land", not "fork", but consumers asked to ship now will fork rather than wait.

### Code smells

- Stringly-typed slot keys (`format!("{node_id}.{slot}")` round-tripped through `BTreeMap<String, SlotValue>` at [examples/notes/src/flow_demo.rs:296](examples/notes/src/flow_demo.rs#L296)) — should be a `SlotRef`-keyed map.
- `BoxedAuthenticator` newtype noise — `Arc<dyn Trait>` ergonomics that generic-bound APIs don't accept directly.
- `match slot_to_json(...)` fallback at [examples/notes/src/flow_demo.rs:352](examples/notes/src/flow_demo.rs#L352) uses `format!("{other:?}")` — silent lossy serialization on unanticipated `SlotValue` variants.
- The 60s quiescence padding ([examples/notes/src/flow_demo.rs:167-170](examples/notes/src/flow_demo.rs#L167-L170)) leaks engine internals into consumer code.

### Test coverage of extension points

- Builtin path well-tested ([starter-extensions/crates/starter-ext-server/tests/rest_routes.rs](starter-extensions/crates/starter-ext-server/tests/rest_routes.rs), `host-manager.test.ts`).
- Process flavour covered at the supervisor level but **the dispatch path has no e2e test** because the dispatch path doesn't exist.
- No test asserts "extension contributes to all four transports from one manifest" (R13). `com.nube.hello` contributes tool + REST + UI but not gRPC / CLI together.

---

## Top 5 prioritized recommendations

1. **Land process-flavour synchronous JSON-RPC dispatch** across REST, CLI, gRPC, MCP. The supervisor speaks JSON-RPC; build the request/response demultiplexer on top and remove `NotWired` from those three crates' `ProcessXxxDispatcher` types. Without this, R13's "one trait, three flavours" is not delivered.

2. **Implement the missing flow node kinds** (`branch`, `merge`, `gate`, `sleep`, `http_out`, `trigger.webhook`, `trigger.schedule`, `trigger.event`, `subflow`) at [crates/starter-flow-nodes/src/](crates/starter-flow-nodes/src). They are ~13 LOC stubs today. Without them, the engine cannot express the workflow its own SCOPE motivates with.

3. **Ship `starter-ext-flow`** (the missing adapter crate) so extensions can contribute node kinds via `contributes.nodes`. This is the seam that converts the flow engine from "consumer-owned graph" to "extension-extensible workflow engine" — Goal 3's full claim.

4. **Fix `Arc<dyn Authenticator>` ergonomics** at `with_principal` / `McpHttpOptions::with_auth` / `router_with_auth` so consumers don't write `BoxedAuthenticator` newtypes ([examples/notes/src/server.rs:230-237](examples/notes/src/server.rs#L230-L237)). One trait-bound widening fixes it.

5. **Add a "contribute everywhere" smoke test** mirroring the four-transport flow event smoke. A single extension manifest with `contributes.tools + cli + rest + grpc + ui` that asserts the host wires the contribution into all five surfaces. Today the wiring is there but no test gates regressions on it; the next architectural drift will break one transport silently.

---

## Bottom line

The architecture is the strongest part of this repo. The SPI discipline, manifest-driven extension model, supervisor design, and frontend MF story are all real. The shortfall is unfinished implementation, not design that needs rework. Closing recommendations 1-3 above moves all three "Partial" verdicts to "Yes".

---

## Second-reviewer sign-off

**Date:** 2026-05-20
**Reviewer:** Claude (Opus 4.7, 1M context) — independent pass

Re-verified every concrete claim against the current tree:

- Stub node sizes match (11–14 LOC across `branch`, `merge`, `gate`, `sleep`, `http_out`, `subflow`, `trigger_event`, `trigger_schedule`, `trigger_webhook`); `ai_agent` (1551), `tool_call` (601), `transform` (506), `trigger_explicit` (490), `log` (422) are the only real bodies in [crates/starter-flow-nodes/src/](crates/starter-flow-nodes/src).
- `NotWired` paths confirmed in [starter-ext-grpc/src/dispatcher.rs](starter-extensions/crates/starter-ext-grpc/src/dispatcher.rs), [starter-ext-server/src/rest/dispatcher.rs](starter-extensions/crates/starter-ext-server/src/rest/dispatcher.rs), and [starter-ext-cli/src/dispatcher.rs](starter-extensions/crates/starter-ext-cli/src/dispatcher.rs). The `Process*Dispatcher` variants all return `DispatchError::NotWired` today.
- `examples/notes/src/server.rs` imports `NotWiredGrpcDispatcher` at line 29 and instantiates it at line 215 — gRPC for extensions in the demo is non-functional as stated.
- `BoxedAuthenticator` newtype is used three times (lines 102, 150, 215) — the ergonomic gap is not a one-site nit.
- `starter-ext-flow` is absent from [starter-extensions/crates/](starter-extensions/crates) and explicitly Phase-6 in [DOCS/flow/scope/SCOPE.md:1280](DOCS/flow/scope/SCOPE.md#L1280) and [DOCS/agent/SCOPE.md:310](DOCS/agent/SCOPE.md#L310).
- The 60s quiescence padding and `NodeEmitted` short-circuit at [examples/notes/src/flow_demo.rs:167-170](examples/notes/src/flow_demo.rs#L167-L170) and [examples/notes/src/flow_demo.rs:330-336](examples/notes/src/flow_demo.rs#L330-L336) read exactly as the review describes — a real engine-internal leak, not a demo cosmetic.

**Concur with verdicts and prioritization.** The "Partial" labels for Goals 2/3/4 are honest, not pessimistic — design is delivered, dispatch wiring is not. Recommendations 1 (process dispatch) and 3 (`starter-ext-flow`) are the load-bearing items; everything else is downstream of them.

One small addition worth tracking alongside Rec 5: a smoke test that asserts a single manifest's `contributes.nodes` becomes a callable node kind after host load. Without it, `starter-ext-flow` can ship and silently regress the same way the four-transport surface can today.
