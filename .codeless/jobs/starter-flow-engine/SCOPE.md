# Scope — starter-flow-engine

> Source of truth: [`DOCS/flow/scope/SCOPE.md`](../../../DOCS/flow/scope/SCOPE.md)
> §"Phase 2 — `starter-flow` engine (in-memory stores)" and the
> upstream Hard-rules block. This file is the per-job brief the
> runner reads at the top of every stage; intentionally short. When
> this file disagrees with the source-of-truth SCOPE, that doc wins —
> open an issue and update this file. Sibling: the completed
> [`../starter-flow-scaffold/`](../starter-flow-scaffold/) job (Phase 1
> contracts crate + three empty skeletons) is merged on master and
> this job builds on its outputs.

## Goal

Land **Phase 2 in full**: the `starter-flow` engine in memory,
plus the bodies of the two built-in node kinds Phase 2 names —
`transform` and `tool-call`. Persistence (Phase 3) and the
`ai-agent` body (Phase 4) are explicitly out. After this job, the
two Phase 2 exit smoke tests from the flow SCOPE pass green: **"One
write chokepoint"** and **"Engine is reader of policies"**, and the
workspace-builds-without-adk-rust dep-tree gate still holds.

## In scope

- **D1 entry gate.** Lock D1 (adk-rust vs lifted Codeless `Runner`
  for the future `ai-agent` body) at stage 1. Default-bias from the
  flow SCOPE: option (b) — `starter-flow-node-loop`, lift Codeless's
  `Runner` shape. Decision lives in this job's SCOPE.md "Decisions"
  with rule + revisit-trigger; the body itself stays Phase 4.
- **`InMemoryGraphStore`** in `crates/starter-flow/src/graph.rs`:
  the `starter_flow_spi::GraphStore` impl that owns the single
  `write_slot` chokepoint per **R2**. `tokio::sync::RwLock` over a
  `BTreeMap<NodeId, Node>` for storage; `tokio::sync::broadcast`
  for the change stream. Replay semantics are wired from day one —
  `WriteSlotOpts { replay: true }` suppresses `SlotChanged` to
  subscribers; idempotent-write short-circuit is on by default,
  defeated by `WriteSlotOpts { force: true }`.
- **Slot propagator** in `crates/starter-flow/src/propagator.rs`:
  synchronous tokio loop subscribed to `SlotChanged`, fanning
  values along outbound `Link`s, owning the per-run propagation
  budget (R1's cycle-budget: `max_propagation_hops = 1000` default,
  per-run overridable). Never bypasses `write_slot`. Honours the
  per-run `Cancel` token within bounded latency.
- **`NodeKindRegistry` + `FlowRegistry`** in
  `crates/starter-flow/src/registry.rs`. Reserved
  `starter.flow.*` namespace is registry-enforced per R10.
  `tokio::sync::RwLock`-protected; refuses duplicate registrations.
- **Engine state machine** in
  `crates/starter-flow/src/engine.rs` per R12:
  `Starting → Running → Pausing → Paused → Resuming → Stopping →
  Stopped`. Type-state enum; `tokio::sync::watch` for observable
  state. `Engine::stop` walks `IsWritable` nodes and drives their
  `safe_state` per R12.
- **Run lifecycle + engine-typed `RunState`** in
  `crates/starter-flow/src/run.rs` and
  `crates/starter-flow/src/state.rs` per R6. `FlowRunner::start`
  runs `SkillSelector::select` once per outer run, threads
  `Arc<SkillSelection>` through every `ai-agent` invocation later
  (R7's outer-run binding seam is wired now even though the
  `ai-agent` body is Phase 4). `RunStore` is a trait seam plus an
  in-memory `Vec<RunState>` for tests; sqlite impl is Phase 3.
- **`transform` node-kind body** in
  `crates/starter-flow-nodes/src/transform.rs` behind the existing
  `transform` feature. Pure-fn body; `&self` per R5; writes its
  output through `GraphStore::write_slot` (never bypassing R2).
- **`tool-call` node-kind body** in
  `crates/starter-flow-nodes/src/tool_call.rs` behind the existing
  `tool-call` feature. Looks up the `Tool` in the host-provided
  `starter_spi::ToolRegistry` per R8; invokes with the run's
  `Principal` + `Cancel` + `EventSink`; auth is applied at the
  adapter, not here, per extensions R13.
- **Phase 2 SCOPE smoke tests** in `crates/smoke-tests/`: "One
  write chokepoint" and "Engine is reader of policies", word-for-
  word with the flow SCOPE §"Smoke tests" block.
- **Dep-tree gates** stay green: the Phase 1 `starter-flow-spi`
  baseline at `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`
  matches byte-for-byte (this job does not modify `starter-flow-spi`);
  `cargo tree -p starter-flow --edges normal` and
  `cargo tree -p starter-flow-nodes --edges normal` contain no
  `adk-rust` entry (D1 locked toward (b)).

## Out of scope

- **Persistence** — `FlowStore` / `RunStore` / `SessionStore` impls
  in `starter-store-sqlite` behind the `flow` feature. Phase 3.
- **`FlowAsTool` / `FlowAsService` bodies.** The structs are empty
  placeholders from Phase 1; trait impls land in Phase 3 once the
  engine exists. (This job exists *so they can*, but it does not
  fill them in.)
- **`ai-agent` node-kind body.** Phase 4 + D1 resolution. The kind
  module exists from Phase 1 as a `KIND_ID` constant; this job
  leaves it alone except for ensuring the outer-run skill seam
  exists in the engine.
- **Remaining built-in node kinds.** `branch`, `merge`, `gate`,
  `subflow`, `trigger.{explicit, event, schedule, webhook}`,
  `http-out`, `log`, `sleep`. Phase 5.
- **`starter-ext-flow` adapter.** Lives in `starter-extensions/`;
  Phase 6 of the flow SCOPE; not in this workspace.
- **Three-level stop persistence + SIGTERM bin wiring.** The
  engine exposes the `pause` / `resume` / `stop` API and walks
  safe-state on stop; the bin-level SIGTERM hook is Phase 7.
- **Modifications to `starter-flow-spi`.** Phase 1 froze the
  contracts. If a real gap is found, surface it as an issue and a
  separate small PR — do not enlarge `-spi` mid-Phase-2.
- **Workflow agents** (`SequentialAgent`, etc.). Per R7 those are
  flow topologies, not types.

## Hard rules (load-bearing — inherited from flow SCOPE)

Restated so the runner re-reads them every stage even though the
SCOPE doc itself owns the text:

- **R1 — Everything is a Node.** No second executable-unit
  abstraction added here. Engine code that runs anything goes
  through `NodeBehavior::invoke`. The cycle-budget is the cap.
- **R2 — Slots are the only I/O surface; one write chokepoint.**
  Every write — from any source — enters `GraphStore::write_slot`.
  The propagator is a *reader* + a *scheduler*; it never writes
  slots directly. Replay (`WriteSlotOpts.replay`) skips emitting
  `SlotChanged`. Safe-state writes (R12) DO emit `SlotChanged`.
  Both flags appear in the per-write tracing span.
- **R3 — Engine is a reader of policies, never an owner.** No
  hardcoded match arms on policy slot names anywhere in
  `starter-flow`. Policies are read from the graph at the moment
  they apply. The smoke test in stage 11 grep-asserts this.
- **R5 — Node behaviours are stateless.** `transform::invoke` and
  `tool_call::invoke` take `&self`. State lives in slots, in the
  run/session store, or in secrets.
- **R6 — Checkpoints are engine-typed.** `RunState` lives in
  `starter-flow`, not in `starter-flow-spi`'s opaque blob seam.
  Phase 2 ships `RunState`; sqlite serialisation is Phase 3.
- **R7 — AI agent is a node kind, not a runtime.** This job does
  NOT add the body; it does wire the outer-run skill-selection
  seam so Phase 4 cannot retro-fit a different shape.
- **R8 — Nodes are not Tools; Tools are one node kind.** The
  `tool-call` body wraps `starter_spi::Tool` lookups from the
  host-provided `ToolRegistry`. The two traits stay separate.
- **R10 — Reverse-DNS ids; namespace ownership enforced.**
  Registry refuses any non-host attempt to register a
  `starter.flow.*` kind. `transform` and `tool-call` keep their
  Phase 1 `KIND_ID` constants verbatim.
- **R12 — Three-level stop + safe-state on every writable
  output.** Engine state machine ships here; the SIGTERM bin hook
  is Phase 7. `Engine::stop` walks `IsWritable` nodes and applies
  `safe_state` policy per R3.
- **R13 — Streaming + cancellation + observability reuse
  existing seams.** `FlowEvent` is the streaming surface
  (`broadcast` per run). `Cancel` is `starter_spi::ai::Cancel`,
  re-exported through `starter-flow-spi`. Every `node.invoke`
  opens a span; every `flow.run` opens a span; tool-call spans
  are children.

## Constraints

- **`starter-flow` deps stay minimal.** Only `starter-flow-spi`
  (`default-features = []`) and `tokio` (`rt-multi-thread`). No
  new top-level deps without a SCOPE amendment.
- **No `adk-rust` anywhere.** `cargo tree -p starter-flow
  --edges normal` and `cargo tree -p starter-flow-nodes
  --edges normal` must contain zero `adk-rust` entries through
  this job's life. CI gate in stage 12.
- **`default-features = []` posture stays.** No new node-kind
  features default-on. The `transform` and `tool-call` bodies
  ship behind the existing features locked in
  [`../starter-flow-scaffold/SCOPE.md`](../starter-flow-scaffold/SCOPE.md)
  D-S1.2.
- **MSRV 1.78** (workspace). `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo fmt --check` non-
  negotiable per the workspace CLAUDE.md.
- **Tests live with the code.** Stages 3–7, 9, 10 each commit
  unit tests in the same commit as the body. Stage 11 lands
  cross-crate smokes in `crates/smoke-tests/`.
- **No engine knowledge of policy slot names.** Stage 11's R3
  grep-test enforces this. If a stage finds itself wanting a
  `match policy.session_policy { … }` arm, the design is wrong;
  read the slot, treat the value as data.
- **One logical batch per stage.** A stage that grows past one
  module-shape commit is two stages.

## Decisions

Locked in stage 1. Each lists the rule it derives from and the
**revisit trigger** — the event that should reopen the question.
Anything else is noise.

### D-S2.1 — D1 — `ai-agent` body: `starter-flow-node-loop` (option b)

The future `ai-agent` node-kind body (Phase 4) is **option (b)
`starter-flow-node-loop`** — lift Codeless's `Runner` shape; route
every LLM call through `starter_ai::AiRunner`; integrate with the
engine's `SessionStore` for multi-turn continuity. **Option (a)
`starter-flow-node-adk` is rejected for v1.**

- **Why.** Flow SCOPE R7: "the AI agent is a node kind, not a
  runtime." Both codeless and rubix's shipping LLM loops already
  fit (b)'s shape; neither uses adk-rust's planner heuristics.
  Picking (b) eliminates the bridge-LoC tally, the pinned-version
  dance, and one external dep. The
  "workspace-builds-without-adk-rust" smoke (flow SCOPE Smoke-tests
  block) becomes a load-bearing CI gate, not a documentation
  aspiration.
- **Implication for Phase 2.** None of the engine code in this job
  imports adk-rust. The future Phase-4 crate is named
  `starter-flow-node-loop`; the placeholder Phase-1 `ai-agent`
  feature in `starter-flow-nodes` remains a `KIND_ID` constant
  with no body until Phase 4.
- **Revisit when.** A workspace consumer surfaces a concrete need
  for an adk-rust feature (planner heuristics, upstream-tracked
  agent capabilities, multi-modal LlmAgent) that codeless/rubix do
  not provide and that cannot be cheaply duplicated in the lifted
  Runner. At that point: open option (c) — `starter-flow-node-adk`
  ships as a sibling implementation of the same `ai-agent` kind id,
  mutually-exclusive with `-node-loop` via cargo features (per the
  agent SCOPE Phase C posture). Do not flip the default for less.

### D-S2.2 — Cycle-budget defaults

`max_propagation_hops = 1000` per run, overridable at
`FlowRunner::start`. Idempotent-write short-circuit on by default,
defeated per-write via `WriteSlotOpts { force: true }`. Both are
public API surface from this job forward.

- **Why.** Flow SCOPE R1 cycle-bounding block names these two
  mechanisms verbatim. The 1000-hop default is "high enough for
  legitimate convergent flows, low enough that a runaway terminates
  within seconds at human-scale rates." Idempotent-write is the
  cheap short-circuit; `force = true` exists for the few cases
  (heartbeats, externally-observed re-publishes) where re-emitting
  is the desired semantic.
- **Implication for stage 4.** Propagator's per-run counter
  increments on every scheduled hop; exceeding the cap marks the
  run `Failed { reason: cycle-budget-exhausted }` and emits the
  same span shape any other run failure does.
- **Revisit when.** A real-world flow trips 1000 hops in normal
  operation (not a bug). At that point a per-flow override at
  flow-manifest level lands as a small follow-up; the default
  stays where it is until evidence says otherwise.

### D-S2.3 — In-memory store substrate

Storage in `InMemoryGraphStore`:
`tokio::sync::RwLock<BTreeMap<NodeId, Node>>` for node + slot
state; `tokio::sync::broadcast::Sender<GraphEvent>` for the
change stream (channel size 1024, configurable at construction).
`SubscriptionStream` is `tokio_stream::wrappers::BroadcastStream`.

- **Why.** Phase 2 is in-memory only; the goal is "smallest thing
  that exposes the GraphStore trait honestly so the propagator
  has something to drive." `BTreeMap` over `HashMap` for
  deterministic iteration in tests; `broadcast` over `mpsc`
  because the subscribe contract is multi-consumer per R13.
  Channel size 1024 is the rubix `live_wire` default; configurable
  so a high-throughput consumer can grow it.
- **Implication for Phase 3.** The sqlite impls are NOT
  pre-decided by this — the `GraphStore` trait contract is what
  they implement; this is one valid implementation. A Phase-3
  sqlite-backed `GraphStore` can use a different in-memory cache
  + `SqlitePool` writeback without changing the trait.
- **Revisit when.** Lock contention shows up in a real benchmark.
  At that point the `RwLock<BTreeMap>` is the first thing to
  replace with a sharded or lock-free structure; the trait does
  not change.

### D-S2.4 — `FlowEvent` stream cardinality

One `broadcast::Sender<FlowEvent>` per `FlowRun`, attached to the
`RunHandle`. Multi-consumer per-subscriber semantics (REST SSE,
CLI NDJSON, MCP `notifications/progress` all attach independently
per R13). Channel size 256 per run; lag-error on slow consumer
surfaces as a typed warning, not silent drops.

- **Why.** Flow SCOPE R13 says streaming reuses existing seams;
  the existing pattern in `starter-ai`'s `OnEvent` and
  `starter_spi`'s event channels is `broadcast` per session/run.
  Per-run cardinality (not engine-global) is what makes
  cancellation crisp — when the run ends, its sender drops, every
  subscriber gets a `Closed` signal in bounded time.
- **Implication for stage 7.** `RunState` carries the
  `broadcast::Sender`; `RunHandle` exposes
  `RunHandle::subscribe() -> impl Stream<Item = FlowEvent>` which
  every adapter consumes through.
- **Revisit when.** A consumer surfaces a use case for
  cross-run event aggregation (e.g. an audit pipeline subscribed
  to *every* run on the engine). At that point an engine-level
  fan-in is a small addition; the per-run shape stays the
  primary surface.

## Cross-cutting checks the runner must keep honest

- **R2 chokepoint** — `grep -rn 'fn write_' crates/starter-flow/src`
  shows exactly one `write_slot` definition. No other write
  function on `GraphStore` exists; if a stage adds one, R2 has
  slipped.
- **R3 policy-name discipline** — `grep -rn
  'session_policy\|on_failure\|cost_cap\|safe_state\|trigger\|auth\|timeout'
  crates/starter-flow/src` returns hits ONLY inside R3-citing doc
  comments. Match arms on these strings fail the smoke.
- **No `adk-rust`** — `cargo tree -p starter-flow --edges
  normal | grep adk-rust` returns empty. Same for
  `starter-flow-nodes`. Stage 12 CI gate.
- **`-spi` baseline unchanged** — `cargo tree -p starter-flow-spi
  --edges normal | diff - DOCS/flow/scope/starter-flow-spi-deps.
  baseline.txt` is empty. Phase 2 must not regress Phase 1's
  baseline.
- **R5 `&self` discipline** — every `NodeBehavior::invoke` body
  in `starter-flow-nodes` uses `&self`, never `&mut self`.
- **Outer-run skill-selection seam present** — `RunState` carries
  `Arc<SkillSelection>`; `FlowRunner::start` calls the
  `SkillSelector` exactly once. Phase 4 reads this seam; Phase 2
  ships it empty (a default selector returns `SkillSelection::default()`).

## Deliverables

- `crates/starter-flow/src/{graph,propagator,registry,engine,run,state}.rs`
  populated; module-level doc comments still point at the SCOPE
  sections that own each module.
- `crates/starter-flow-nodes/src/transform.rs` and
  `crates/starter-flow-nodes/src/tool_call.rs` populated; the
  `KIND_ID` constants stay verbatim.
- Unit tests inside each `starter-flow` and `starter-flow-nodes`
  module + the cross-crate Phase 2 smoke tests in
  `crates/smoke-tests/`.
- `cargo build --workspace --all-features` green.
- `cargo clippy --workspace --all-targets -- -D warnings` green.
- `cargo fmt --check` green.
- `cargo tree -p starter-flow-spi --edges normal` matches
  `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`
  byte-for-byte.
- `cargo tree -p starter-flow --edges normal` and
  `cargo tree -p starter-flow-nodes --edges normal` contain no
  `adk-rust`.
- Two Phase 2 SCOPE Smoke tests pass: "One write chokepoint",
  "Engine is reader of policies".

## Open questions (resolve in stage 1)

1. **`transform` body language.** Bias: a registered Rust
   closure stored at kind-registration time, indexed by a
   `transform.fn_id` config slot. Reason: `rhai` is heavy and
   Phase 5's `transform` semantics may want richer scripting
   anyway (`rhai`, `starlark`, embedded `wasm`). Ship the Rust-
   closure path first; the richer transform language is a
   Phase-5 decision, not Phase 2's.
2. **`ToolRegistry` injection shape for `tool-call`.** Bias:
   the engine is constructed with `Arc<dyn ToolRegistry>` and
   threads it through to every `tool-call` invocation. Matches
   the existing `starter-mcp` registry-injection pattern;
   single source of truth per R8.
3. **`SkillSelector` default for Phase 2.** Bias: a no-op
   selector that returns `SkillSelection::default()`. The real
   selector lives in `starter-skills` (agent SCOPE Phase A)
   which is parallel-track; Phase 2 ships the seam, not the
   selector.
4. **`SubscriptionStream` channel size.** Bias: 1024 default,
   configurable at `InMemoryGraphStore::with_capacity`. Matches
   rubix `live_wire`.

D1 is locked at stage 1 per D-S2.1 above; the four above are
sub-decisions whose outcome is recorded with the same shape under
"Decisions" once stage 1 finishes.
