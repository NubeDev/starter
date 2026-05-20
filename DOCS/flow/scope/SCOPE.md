# `starter-flow` — Scope

## One-line summary

A node-based, event-driven runtime graph at the centre of starter.
**Everything is a node.** Tools are nodes. AI agents are nodes. Triggers
are nodes. Extensions contribute node kinds. Flows are graphs of nodes
wired by typed slots; the engine reacts to slot changes through a
single write chokepoint and propagates downstream. The AI agent
foundation ([`DOCS/agent/SCOPE.md`](../../agent/SCOPE.md)) folds into
this design as the `ai-agent` node kind; tools
([`DOCS/tools/scope/SCOPE.md`](../../tools/scope/SCOPE.md)) remain
unchanged as the request/response primitive and surface as the
`tool-call` node kind; extensions
([`DOCS/extensions/scope/SCOPE.md`](../../extensions/scope/SCOPE.md))
contribute node kinds via a new `contributes.nodes` block alongside the
existing `contributes.tools` / `agents` / `skills` / `cli` / `rest` /
`workers` / `ui`.

One engine. One graph. One event bus. One write path. One node trait.
Tools, agents, services, skills, MCP — all compose through it without
inventing a new wire shape.

## Why this exists

Three forces converge:

1. **Every starter-based product reaches for the same shape.** Codeless
   needs a staged job state machine (`Repo → Job → Stage → Task` with
   verify-gates, cost caps, fresh-session-per-stage). Rubix needs a
   reactive long-lived slot-propagation graph (Everything-As-Node,
   safe-state on outputs, cycles allowed, months of uptime). They look
   different on the surface — one is "DAG-of-stages", the other is
   "reactive node graph" — but they share the **same primitive**: a
   graph of nodes with typed I/O, lifecycle policies on the node, and an
   engine that walks the graph in response to events. The differences
   are configuration on nodes (`session_policy`, `on_failure`,
   `cost_cap`, `safe_state`, `trigger`), not different engines.

2. **The AI agent's workflow primitives are graph topology, not agent
   internals.** The agent SCOPE today re-exports `SequentialAgent`,
   `ParallelAgent`, `LoopAgent`, `GraphAgent` from `adk-rust` (R1).
   These *are* graph topologies. Once starter owns a flow engine, those
   topologies belong here — not as adk-rust types behind a re-export.
   The only adk-rust primitive that remains load-bearing is the
   turn-based LLM-loop-with-tool-dispatch. A working implementation of
   that loop exists in Codeless's `Runner` trait
   (`/home/user/code/rust/codeless-workspace/codeless/crates/codeless-runtime/`)
   and is available to lift; LoC tally is a Phase 4 entry gate, not an
   established fact. This collapses the agent SCOPE's R1 from "adk-rust
   owns workflow types and the loop" down to "the `ai-agent` node kind
   owns the loop; topology is the flow engine's job."

3. **Node-RED's model is the right authoring shape for AI + integration
   workflows in 2026.** Visual or YAML wiring beats hand-rolled state
   machines for the kind of work products built on starter actually do:
   ingest a webhook, run an agent, branch on its output, write to a
   database, post to Slack, loop if a condition holds. Crossflow
   (Bevy-coupled, v0.0.6) validates the *shape* is good even if its
   substrate is wrong for us. Rubix has already shipped a Tokio
   propagator over a unified node graph and proven the model under load.
   The third implementation is the right time to extract.

Starter ships this as the **core** rather than a side crate because the
parts only justify their cost together: the flow engine is more valuable
when extensions can contribute node kinds; node-kind extensions are more
valuable when tools and agents are themselves node kinds; agents and
tools are more valuable when a flow can compose them with branching,
loops, and persistence — all of which the engine handles, not the agent.

## What this supersedes / modifies in other SCOPEs

Read these alongside this doc; the rules below override where they
conflict:

- **[Agent SCOPE](../../agent/SCOPE.md) R1** — "adk-rust is THE agent
  runtime; not optional, not swappable." **Superseded.** Workflow agents
  (`SequentialAgent`, `ParallelAgent`, `LoopAgent`, `GraphAgent`) are
  flow topologies, owned by `starter-flow`, not adk-rust. The only
  agent-runtime primitive that remains is the `ai-agent` node kind —
  one turn-based LLM loop with tool dispatch. See R7 below for what
  replaces R1.
- **[Agent SCOPE](../../agent/SCOPE.md) R2** — "`AiRunner` is the only
  LLM seam." **Unchanged.** The `ai-agent` node kind's body routes every
  LLM call through `starter-ai::AiRunner`. Same CI dep-tree snapshot
  test applies to whichever crate ships the node-kind body.
- **[Agent SCOPE](../../agent/SCOPE.md) R3** — "One tool registry, one
  trait." **Unchanged.** `starter_spi::tool::Tool` remains the tool
  trait. A `tool-call` node kind wraps any registered `Tool`; a flow
  surfaces as a `Tool` via `FlowAsTool` (analogue of `AgentAsTool`).
  See R8 below for the Node↔Tool relationship.
- **[Agent SCOPE](../../agent/SCOPE.md) R4** — "Skills are static
  metadata; quarantined by default." **Unchanged**, and now first-class:
  skills bind to the `ai-agent` node kind's *invocation*, not to the
  topology. See R7 below.
- **[Agent SCOPE](../../agent/SCOPE.md) R5** — "Sessions persist;
  behaviours are stateless; checkpoints are versioned." **Refined.**
  Stateless behaviours and persistent sessions stay. The
  "adk-rust-internal checkpoint blob" concern dissolves because flow
  topology is no longer an adk-rust type — checkpoint state is the
  engine's own typed `RunState` over `starter-store-sqlite`. See R6
  below.
- **[Agent SCOPE](../../agent/SCOPE.md) R6** — "Agents are first-class
  Tools and first-class Services." **Generalised.** *Flows* are
  first-class Tools and first-class Services; an "agent" is a flow whose
  root node is an `ai-agent`. See R8 and R9 below.
- **[Agent SCOPE](../../agent/SCOPE.md) R7** — "Extensions can
  contribute agents and skills." **Extended.** Extensions also
  contribute node kinds and complete flows. See R11 below.
- **[Agent SCOPE](../../agent/SCOPE.md) R8** — "Streaming, cancellation,
  observability reuse existing seams." **Unchanged and inherited
  verbatim.** Flow events are `Stream<FlowEvent>`, same shape as
  `Stream<AgentEvent>`. Same `Cancel`, same observability spans.
- **[Tools SCOPE](../../tools/scope/SCOPE.md)** — entire doc stands.
  `Tool` and `Service` traits unchanged; per-integration crate model
  unchanged; `SecretString`-based config unchanged. The flow engine
  consumes `Tool` and `Service` impls from the registries those crates
  build; it does not require them to be reshaped.
- **[Extensions SCOPE](../../extensions/scope/SCOPE.md)** — entire doc
  stands. The flow engine adds **one new contribution kind**
  (`contributes.nodes`) and **one new adapter crate**
  (`starter-ext-flow`) following R13's "contribute once, surface
  everywhere" pattern. No other changes to the extension framework.

## Relationship to existing crates

```
starter-spi                       (Tool, Service, AiRunner, Cancel,
                                   Principal, SecretStore — exists)
   ▲
   │
   ├── starter-ai                 (5 providers — exists)
   │
   ├── starter-mcp                (MCP server — exists; gains
   │                               FlowAsTool surfacing per R8)
   │
   ├── starter-flow-spi           (NEW — contracts crate)
   │      Node, NodeKind, NodeId, SlotRef, SlotValue, Link,
   │      Trigger, FlowId, FlowRevisionId, RunId, FlowEvent,
   │      RunState, FlowStore, RunStore, GraphStore traits.
   │      Depends only on starter-spi. Zero runtime logic.
   │
   ├── starter-flow               (NEW — THE engine)
   │      GraphStore impl (single write chokepoint),
   │      slot propagator (synchronous tokio),
   │      NodeKindRegistry, FlowRegistry,
   │      engine state machine + three-level stop,
   │      run lifecycle + checkpointing,
   │      skill-selection hook for ai-agent nodes,
   │      observability spans on every node invocation.
   │
   ├── starter-flow-nodes         (NEW — built-in node kinds)
   │      ai-agent, tool-call, transform (rhai), branch, merge,
   │      gate (review/verify/approval), subflow,
   │      trigger.{explicit, event, schedule, webhook},
   │      http-out, log, sleep.
   │      Each behind its own cargo feature.
   │
   ├── starter-flow-surfaces      (NEW — Flow ↔ Tool/Service wrappers)
   │      FlowAsTool    — Flow → starter_spi::Tool
   │      FlowAsService — Flow → starter_spi::Service
   │      (mirrors the agent SCOPE's starter-agent-surfaces)
   │
   ├── starter-store-sqlite       (exists; gains ONE feature = "flow"
   │                               with FlowStore + RunStore + SessionStore
   │                               impls. Sessions are an engine concern
   │                               under R6; no separate "session" feature.)
   │
   └── starter-skills             (per agent SCOPE — exists / planned;
                                   bound to ai-agent node invocations)
```

Extensions integration lives in a **sibling crate inside
`starter-extensions/`** per extensions R13:

```
starter-extensions/crates/
   └── starter-ext-flow           (NEW adapter crate)
          Reads contributes.nodes and contributes.flows from
          block.yaml. Registers extension-shipped node kinds into
          the host's NodeKindRegistry and flow definitions into
          FlowRegistry. Same two-phase loader, namespace ownership,
          capability discipline as every other extensions adapter.
```

Default-features stay empty per workspace policy: a consumer who
doesn't want flows pays nothing. No engine spawned, no flow directory
scanned.

## Hard rules (load-bearing)

These are the rules whose violation collapses the design. Each is
written down so future contributors have to argue *against* the rule
rather than around it.

### R1 — Everything is a Node

Every executable unit in the system is a node implementing
`starter_flow_spi::Node`:

- An AI agent invocation is an `ai-agent` node.
- A tool call is a `tool-call` node wrapping a `starter_spi::Tool`.
- A transform (pure function) is a `transform` node.
- A branch / merge / loop-condition is a node.
- A trigger (explicit, event-driven, scheduled, webhook) is a node.
- A subflow is a node (composition is free).
- An extension-contributed integration is a node kind.

A node has:

- A stable **id** (namespace-scoped — see R10).
- A **kind** (declared in metadata; identifies which `NodeBehavior`
  implementation runs the node).
- Typed **input slots** and **output slots** (the I/O surface).
- A **lifecycle** (`Created → Active → Paused → Stopped → Removed`).
- An **event stream** (anyone can subscribe to slot changes or
  lifecycle transitions on this node).
- Optional **config slots** carrying policies the engine reads (see
  R3): `session_policy`, `on_failure`, `cost_cap`, `safe_state`,
  `trigger`, `auth`, `timeout`.

This commits to the unified-graph model proven by rubix-agent (see
[rubix EVERYTHING-AS-NODE](https://github.com/NubeIO/rubix-agent/blob/main/docs/design/flow/EVERYTHING-AS-NODE.md)).
The cost is real: get the node model wrong and everything downstream is
wrong. The benefit is also real: one event bus, one subscription
mechanism, one permission model, one audit pipeline, one UI shape — the
"extensibility for free" property that makes the whole stack hang
together.

**The corollary:** there is no second "executable unit" abstraction in
starter. If something runs in response to an event or a request, it
runs as a node. Tools, services, agents, scheduled workers — all
either are nodes or are wrapped by a node kind.

**Cycles are bounded by a per-run propagation budget.** A flow run
carries (a) a monotonic epoch counter on every propagation hop and (b)
a `max_propagation_hops` cap per run (default `1000`, overridable per
flow). Each `SlotChanged` event the propagator schedules increments the
counter; exceeding the cap marks the run `Failed` with
`cycle-budget-exhausted` and the engine emits the same span shape it
emits for any other run failure. Additional short-circuits: if a slot's
new value equals its prior value, the propagator does not enqueue
downstream invocations (idempotent writes do not loop). These are the
mechanisms that let R1 say "cycles allowed" without "cycles can hang
the engine."

### R2 — Slots are the only I/O surface; one write chokepoint

All data flowing between nodes flows through **typed slots**. A slot is
a named, typed property on a node; some are read-only (the node
publishes), some are writable (downstream nodes write into them, the
node reacts).

**Every write — from any source — enters through one function:
`GraphStore::write_slot`.** REST endpoints, CLI commands, internal node
ticks, propagation from upstream slots, replay from a checkpoint —
they all funnel through the same call. The propagator subscribes to
`GraphEvent::SlotChanged` from that single chokepoint and fans values
downstream along outbound `Link`s.

Reason: with one write path, every invariant the engine cares about
(authorisation, audit, type checking, safe-state, observability, replay)
is enforced in one place. The rubix runtime proved this scales; the
alternative (multiple write paths) is how every "PLC-style" system
accretes inconsistencies until it can't be reasoned about. This is the
single most load-bearing implementation rule in the engine.

Slot wire shape is **Node-RED compatible**: a message envelope with
`payload`, `topic`, metadata, and arbitrary custom fields. Settings on a
node can be overridden per-message via declared `msg_overrides` (e.g.
`msg.url` → HTTP client URL). This is the format consumers will
recognise from Node-RED, n8n, and Zapier — adopting it is cheap and
unlocks the entire mental model.

**Replay does not re-fire subscribers.** Replay (run resume from
checkpoint, audit replay) reconstructs `GraphStore` state by writing
slots through `write_slot` with `WriteSlotOpts { replay: true }`. The
propagator skips emitting `SlotChanged` events for replay writes, so
downstream nodes are not re-invoked from side-effecting tools / external
APIs. The replay path is the only writer that bypasses subscribers; the
engine's safe-state drive (R12) *does* publish `SlotChanged` (operators
need safe-state in audit). Two flags, two semantics, both visible in
tracing on every write.

### R3 — The engine is a reader of policies, never an owner

Lifecycle policies live as **config-role slots on the node they govern**,
not as state owned by the engine. The engine walks the graph and reads
these slots at the moment the policy applies:

- `session_policy` — `fresh | continue | long-lived`. Determines
  whether an `ai-agent` node starts a fresh session per invocation or
  continues an existing one. Codeless's "stages bound context" is
  `session_policy: fresh` on its stage nodes; Rubix's long-lived
  reactive nodes are `session_policy: long-lived`. Both are the same
  primitive with different policy values.
- `on_failure` — `gate | hold-last | fail-safe | release | abort`.
  Codeless's "verify-gated; halt visibly" is `on_failure: gate`;
  Rubix's "safe-state on output slot" is `on_failure: hold-last` or
  `fail-safe`. Same primitive, different policy.
- `cost_cap` — optional `Usd` budget per invocation (LLM costs,
  external API costs). Codeless's per-stage cost cap.
- `safe_state` — the value an output slot drives to on stop /
  shutdown / disconnect. Three concrete policies (lifted from rubix
  RUNTIME): `hold-last`, `fail-safe(value)`, `release` (let
  downstream protocol default win). Applied by the engine at
  three-level-stop (R12).
- `trigger` — `explicit | event(slot_ref) | schedule(cron) |
  webhook(path)`. What causes this node (and the flow rooted at it) to
  fire.
- `auth` — `Principal` required to invoke this node. Applied at the
  boundary by adapters (per extensions R13 "adapters apply auth, not
  extensions").
- `timeout` — wall-clock cap; engine cancels via `Cancel` token on
  expiry (per R13).

The engine never owns these policies; it reads them, applies them, and
emits an event when it does. This is the rubix doctrine — *"the engine
is a reader of policies, not an owner"* — generalised across both the
Rubix shape and the Codeless shape. The Studio, audit log, RBAC, and
subscription fabric all treat policy slots exactly like any other slot,
with no special case.

### R4 — Node-kind metadata is static; declared, never runtime-templated

A node kind is declared in `block.yaml` (for extension-contributed
kinds) or in a `node_kind.yaml` colocated with the Rust source (for
built-in kinds). The declaration includes:

- `id` — the node-kind id (namespace-scoped per R10).
- `display_name`, `description_file` (path to a static markdown file).
- `input_slots` and `output_slots` (each with name, type, default).
- `config_slots` (the policy slots from R3 plus kind-specific config).
- `facets` (e.g. `IsWritable`, `IsTrigger`, `IsCompositional`).
- `requires` — capabilities the kind needs (per extensions R6).

`deny_unknown_fields` on the parser; static files cached at load time;
**never** templated at runtime. Same anti-prompt-injection guarantee as
[extensions R7](../../extensions/scope/SCOPE.md) and [agent
R4](../../agent/SCOPE.md). LLM-facing descriptions are byte-identical
at load and call time — the same R7 anti-prompt-injection guarantee the
extensions framework already enforces.

### R5 — Node behaviours are stateless

`NodeBehavior` impls take `&self`, never `&mut self`. Per-instance state
lives in:

- **Slots** (graph-visible state — input/output/config values).
- The host-provided **session/run store** (multi-turn LLM continuity,
  partial-progress checkpoints, run history).
- The host's **secret store** (credentials per R5 of tools SCOPE).

Same discipline as [extensions R5](../../extensions/scope/SCOPE.md),
[agent R5](../../agent/SCOPE.md), [tools R2](../../tools/scope/SCOPE.md).
This is what keeps node kinds interchangeable across builtin / process /
WASM flavours (extensions R1) and what makes per-run state survive
process restart.

### R6 — Sessions persist; runs persist; checkpoints are engine-typed

A **session** is multi-turn continuity for an `ai-agent` node (or any
node kind that opts into sessions). A **run** is a single invocation of
a flow — start to terminal state — with all per-run state (current
node, in-flight slot writes, partial outputs). Both persist through
`SessionStore` and `RunStore` traits in `starter-flow-spi`, with default
impls in `starter-store-sqlite` behind the `flow` feature.

**Checkpoints are typed by the engine, not by an external dep.** A run
checkpoint records (a) the engine's own `RunState` (which nodes have
fired, current message envelope, propagation queue) and (b) per-node
opaque blobs the node kind chooses to persist (e.g. an `ai-agent`'s
turn history). Format version is a `u32` column tied to the engine's
own SemVer.

This is the simplification the agent SCOPE's R5 enabled once topology
moved out of adk-rust. The old "checkpoint contains adk-rust-internal
types, abandon mid-run on upgrade" dance dissolves: the engine owns its
own state shape, so an upgrade only requires a migration on `RunState`,
which is starter's own type.

### R7 — The AI agent is a node kind, not a runtime

The agent foundation collapses to **one node kind**: `ai-agent`. Its
body is a turn-based LLM loop with tool dispatch:

- Reads the prompt from its input slot.
- Resolves the active skill (per [agent SCOPE
  R4](../../agent/SCOPE.md) — quarantine + content-hash approval still
  applies). Skill selection happens once per outer flow run, threads
  through the run as `SkillSelection`, restricts `tools` to the
  intersection of `host_registry.filter(&skill.allowed_tools)` and the
  node-kind's declared `allowed_tools` (per agent SCOPE R4 skill scope
  rule 4: composition is intersection, not union).
- Calls the LLM via `starter_ai::AiRunner` (agent SCOPE R2: the only
  LLM seam, unchanged).
- Dispatches tool calls into the `ToolRegistry` the engine was built
  with (agent SCOPE R3: one tool registry, unchanged).
- Streams events as `FlowEvent::NodeEmitted` on its output slot.
- Honours its `cost_cap`, `timeout`, and `session_policy` config slots
  per R3.

Workflow topology (`SequentialAgent`, `ParallelAgent`, `LoopAgent`,
`GraphAgent`) is **not** in this node kind. Those are flow shapes:

- "Sequential" → a linear flow of `ai-agent` nodes connected by slots.
- "Parallel" → a flow with a `fork` node fanning to multiple
  `ai-agent` nodes that merge at a `join`.
- "Loop" → a flow with a `loop` node whose body is invoked until a
  branch condition fires.
- "Graph" → a flow with arbitrary topology, including cycles.

This is what dissolves the agent SCOPE's R1 conflict with Codeless.
Codeless's `Runner` shape (the LLM-loop primitive) IS the `ai-agent`
node kind's body. Codeless adopts starter by writing a flow whose nodes
have `session_policy: fresh-per-stage`, `on_failure: gate`, and a
`cost_cap` — no separate engine.

**`adk-rust` is optional and limited to this one node kind.** If kept
at all, it ships as `starter-flow-node-adk` with the `ai-agent` body
implemented in terms of adk-rust's `LlmAgent`. The leaner alternative —
lifting Codeless's `Runner` trait into `starter-flow-node-loop` — is
the recommended default for v1. **Decision deferred to D1 below;
either way, the workflow agents do not ship.**

### R8 — Nodes are not Tools; Tools are one node kind

`starter_spi::Tool` and `starter_flow_spi::Node` are different traits
with different shapes:

- A **Tool** is request/response. Input → Output. Stateless from the
  caller's view. MCP-callable. JSON-schema-described. Lives in
  `starter-spi` for any consumer that doesn't need a flow engine.
- A **Node** is event-driven. Has typed slots. Has lifecycle. Has
  config policies the engine reads. Has an event stream. Can be
  long-lived.

They map both directions:

- **`tool-call` node kind** wraps a `Tool`. Its input slot accepts
  the tool's input JSON; it calls the tool; the result lands on its
  output slot. Every registered `Tool` is automatically callable as a
  `tool-call` node — no extension author writes glue.
- **`FlowAsTool`** wraps a flow as a `Tool`. The flow's entry trigger
  receives the tool call's input; the flow's terminal output node's
  value becomes the tool's return value. This is the analogue of
  [agent SCOPE](../../agent/SCOPE.md)'s `AgentAsTool` — every flow is
  automatically a Tool, which means every flow is automatically
  MCP-callable, REST-callable, CLI-callable, and callable from another
  flow as a `tool-call` node.

This preserves [agent SCOPE R3](../../agent/SCOPE.md) (one tool
registry, one trait), keeps `starter-mcp` and the existing REST/CLI
adapters working unmodified, and avoids inventing a third contribution
kind on the wire. **Tool stays as the MCP-callable primitive; Node is
the engine-internal primitive. Neither subsumes the other.**

### R9 — Flows are first-class Tools and first-class Services

Per [agent SCOPE R6](../../agent/SCOPE.md) generalised, every flow gets:

- **A `Tool` surface** via `FlowAsTool`. Callable from MCP, REST, CLI,
  and from another flow as a `tool-call` node. The flow runs to
  completion server-side; output streams as `notifications/progress`
  on MCP or SSE on REST.
- **A `Service` surface** via `FlowAsService`. Reads from an
  `EventSink` (per [tools SCOPE R4](../../tools/scope/SCOPE.md));
  invokes the flow per event. Inherits every property of the tools
  SCOPE's `Service` trait, including the registry-owned shutdown watch
  ([tools SCOPE D3](../../tools/scope/SCOPE.md)).

A flow with a `trigger.webhook` entry node and an `ai-agent` body and
a `tool-call` output is, simultaneously: a webhook endpoint (via REST
adapter), an MCP tool (via FlowAsTool), and an event-driven service
(via FlowAsService). The author wrote one flow.

### R10 — Reverse-DNS ids; namespace ownership enforced

Lifted from [extensions R4](../../extensions/scope/SCOPE.md) and
applied to every flow identifier: node-kind ids, flow ids, slot ids
within a flow. An extension contributing a node kind owns
`<extension-id>.<kind-name>` and nothing else. Reserved prefixes
(`sys.*`, `starter.*`, `flow.*`) belong to the host and cannot be
claimed. Same rationale: kills the entire class of "extension A breaks
because extension B shadowed its id" bugs.

### R11 — Extensions contribute node kinds and flows; same discipline

New contribution kinds in `block.yaml`:

```yaml
contributes:
  nodes:                          # surfaced by starter-ext-flow
    - id: com.acme.weather.current
      handler: WeatherCurrentNode
      kind_file: nodes/weather-current.yaml   # static node-kind metadata
      requires: [http_out: ["api.weather.gov"]]

  flows:                          # surfaced by starter-ext-flow
    - id: com.acme.refund.flow
      flow_file: flows/refund.yaml            # static flow definition
      auth: { require_role: Reader }
```

The `starter-ext-flow` adapter:

- Enforces namespace ownership per [extensions R4](../../extensions/scope/SCOPE.md)
  and R10 above — every node-kind id and flow id must be the extension
  id or a dotted descendant.
- Two-phase commits per [extensions R3](../../extensions/scope/SCOPE.md)
  — one bad node kind never poisons the registry.
- Applies the manifest's `auth:` declaration at the boundary so the
  flow never performs the auth check itself ("adapters apply auth, not
  extensions").
- Reuses the existing JSON-RPC stdio channel ([extensions R10](../../extensions/scope/SCOPE.md))
  for process-flavour and WASM-flavour extensions. A node kind backed
  by a process extension runs in the child; the engine invokes it over
  JSON-RPC exactly as for any other contribution. **No new wire
  format.**

### R12 — Three-level stop; safe-state on every writable output

Lifted from rubix RUNTIME and made universal:

| Level | What stops | Who triggers | When to use |
|---|---|---|---|
| **Flow** | One flow pauses; others keep running | Operator with flow-edit role | Editing a flow, debugging, commissioning |
| **Engine** | All flows pause; engine reports `Paused`; process stays up | Site admin | Maintenance windows |
| **Process** | Entire process exits | OS / systemd | OS patching, hardware work |

Engine state machine (lifted verbatim from rubix RUNTIME):
`Starting → Running → Pausing → Paused → Resuming → Stopping → Stopped`.
Observable via API and via traces. Every transition is logged.

On every stop, the engine walks the graph for nodes with
`facets == IsWritable` and applies the `safe_state` policy from R3.
Three policies: `hold-last`, `fail-safe(value)`, `release` (downstream
default). This is what makes the engine usable for AI flows that hit
external systems (the same node-RED `msg.complete` semantics generalised
to safe-state policies).

**`release` semantics are kind-validated, not engine-enforced.** Some
protocols natively support "relinquish to downstream default" (BACnet
priority arrays, configuration toggles with declared defaults); others
do not (a plain HTTP POST has no "unset"). The engine does not paper
over the difference. At node-kind load time, kinds declare whether they
support `release`; a flow that sets `safe_state: release` on an output
slot whose kind does not support it fails validation with a clear
error. For kinds that do support it, the node-kind body is responsible
for the protocol-level handshake (and for handling the inherent race
between the release write and a downstream reconnect — same problem
rubix RUNTIME handles per-protocol). The engine guarantees the
*invocation*, not the *protocol outcome*.

Graceful shutdown protocol (lifted from rubix RUNTIME): SIGTERM triggers
(1) stop accepting new triggers, (2) finish in-flight runs with a short
timeout, (3) drive writable outputs to safe state, (4) flush the
`RunStore` to disk, (5) exit cleanly. SIGKILL only after grace.

### R13 — Streaming, cancellation, observability reuse existing seams

Same as [agent R8](../../agent/SCOPE.md), inherited verbatim:

- **Streaming.** `FlowEvent` is `Stream<Item = FlowEvent>`. Same
  shape `starter-ai`'s `OnEvent` and `starter_spi`'s event channels
  already use. Adapters render natively per transport (SSE on REST,
  NDJSON on CLI, `notifications/progress` on MCP, server-streaming on
  gRPC) per [extensions R13](../../extensions/scope/SCOPE.md) streaming
  convention.
- **Cancellation.** Reuses `starter_spi::ai::Cancel`. A REST client
  disconnect → adapter cancel notification → flow run's `Cancel` token
  fires → propagator stops scheduling new nodes → in-flight `ai-agent`
  cancels its LLM call. Same path the existing AI runners use.
- **Observability.** Every flow run spans on `flow.run`; every node
  invocation spans on `node.invoke` (child of `flow.run`); tool calls
  span on `tool.call` (child of `node.invoke`, already true). Metrics
  include flow latency, node latency, slot-write throughput, restart
  counts, cancellation counts.

## Skills bind to the `ai-agent` node kind

Per [agent SCOPE R4](../../agent/SCOPE.md), skills are static
SKILL.md bundles with content-hash approval and quarantine discipline.
Under the flow engine, skills are **selected per outer flow run**, not
per node:

1. The engine's `FlowRunner::start(flow, input)` runs
   `SkillSelector::select(input)` once. Result is a `SkillSelection`.
2. The selection threads through the run. Every `ai-agent` node in the
   flow sees the same `SkillSelection` in its invocation context.
3. The `ai-agent` node's `tools` allowlist is the *intersection* of:
   - the host's `ToolRegistry`,
   - the skill's `allowed_tools` (if a skill is selected),
   - the node's own declared `allowed_tools` in its config slot.
4. An `ai-agent` node can declare `skill_hint:` in its config slot to
   override selection — same escape hatch as [agent SCOPE R4 skill
   scope rule 3](../../agent/SCOPE.md), now applied per node rather
   than per sub-agent.

This means a flow with three `ai-agent` nodes — gather, review,
summarise — defaults to one skill across all three (the LoopAgent
re-selection problem from the agent SCOPE never arises because the
engine controls invocation, not the agent). Each node can override
with `skill_hint:` if it has a structurally different role.

Skill resources (`resources/*.md`) are mounted as readable files in
the `ai-agent` node's workspace, exactly as in the agent SCOPE. The
content-hash approval invariant is preserved end-to-end.

## MCP exposes flows the same way it exposes tools

`starter-mcp` is unchanged. The flow engine surfaces every flow as a
`Tool` via `FlowAsTool` and registers it in the same `ToolRegistry`
`starter-mcp` already serves. A Claude Desktop client connects, lists
tools, and sees:

- Every host-registered tool.
- Every extension-contributed tool.
- Every flow (host or extension), as a tool. Calling it runs the flow
  server-side; node-level progress streams as MCP
  `notifications/progress` events; the final terminal-node output is
  the tool's return value.

No new MCP transport. No new wire format. This is the same property
[agent SCOPE R6](../../agent/SCOPE.md) promised for agents, now true
for any flow.

## What lands in `starter-flow-spi`

Contracts crate. Zero runtime, zero I/O. Depends only on
`starter-spi`.

```rust
pub use spi::Principal;
pub use spi::secrets::SecretString;
pub use spi::ai::Cancel;

pub mod node {
    pub trait NodeBehavior: Send + Sync + 'static {
        fn kind_id(&self) -> &KindId;
        async fn invoke(&self, ctx: NodeCtx<'_>, input: SlotMap)
            -> SpiResult<SlotMap, NodeError>;
        async fn on_lifecycle(&self, ctx: NodeCtx<'_>, ev: LifecycleEvent)
            -> SpiResult<(), NodeError> { Ok(()) }
    }

    pub struct NodeId(/* reverse-DNS string newtype */);
    pub struct KindId(/* reverse-DNS string newtype */);
    pub struct SlotRef { pub node: NodeId, pub slot: String }

    pub enum SlotValue {
        Null, Bool(bool), Int(i64), Float(f64), String(String),
        Bytes(Vec<u8>), Json(serde_json::Value),
    }
    pub type SlotMap = std::collections::BTreeMap<String, SlotValue>;
}

pub mod graph {
    pub trait GraphStore: Send + Sync + 'static {
        async fn write_slot(&self, slot: &SlotRef, value: SlotValue,
                            opts: WriteSlotOpts) -> SpiResult<(), GraphError>;
        async fn read_slot(&self, slot: &SlotRef) -> SpiResult<SlotValue, GraphError>;
        fn subscribe(&self, opts: SubscribeOpts) -> SubscriptionStream;
    }
}

pub mod flow {
    pub struct FlowId(/* reverse-DNS string newtype */);
    pub struct FlowRevisionId(/* UUID newtype */);
    pub struct RunId(/* UUID newtype */);

    pub enum FlowEvent {
        RunStarted { run: RunId, flow: FlowId },
        NodeStarted { run: RunId, node: NodeId },
        NodeEmitted { run: RunId, node: NodeId, slot: String, value: SlotValue },
        NodeFailed { run: RunId, node: NodeId, error: NodeError },
        RunCompleted { run: RunId, output: SlotMap },
        RunFailed { run: RunId, error: FlowError },
        RunCancelled { run: RunId },
    }

    pub trait FlowStore: Send + Sync + 'static { /* CRUD + revisions */ }
    pub trait RunStore: Send + Sync + 'static { /* CRUD + checkpoints */ }
}

pub mod skill {
    pub use starter_skills::SkillSelection;   // re-export
}
```

`#[non_exhaustive]` on every public enum and config struct. SemVer
discipline mirrors `starter-spi`.

## What does NOT land

- **A visual canvas / palette in starter.** UI lives in a separate
  package (`starter-ui-flow` if and when shipped), or in the consumer.
  The engine is headless.
- **A second event bus.** The flow engine uses `GraphStore::subscribe`
  for slot changes; service-emitted events use the existing
  `EventSink` from [tools SCOPE](../../tools/scope/SCOPE.md).
- **A second IPC.** Cross-process node-kind invocations use the
  existing extensions JSON-RPC stdio channel
  ([extensions R10](../../extensions/scope/SCOPE.md)). No gRPC, no
  Unix sockets, no shared memory in v1.
- **A durable scheduler.** The `trigger.schedule` node kind uses
  in-process cron; durable scheduling (cross-restart guaranteed firing)
  is a separate concern that gets its own crate when a real consumer
  needs it. (Same posture as [tools SCOPE](../../tools/scope/SCOPE.md).)
- **Safety-rated or hard real-time guarantees.** Same disclaimer as
  rubix RUNTIME. Sub-second reactivity is the design target; sub-10ms
  control loops are not.
- **A flow marketplace.** Flows are distributed out-of-band (extension
  bundles, deploy step, git pull). Same posture as extensions and
  skills.
- **A multi-tenant flow isolation model in v1.** WASM-flavour node
  kinds inherit `starter-ext-wasm`'s capability surface; process-flavour
  inherit `starter-ext-supervisor`'s model. Anything beyond that is
  consumer policy.
- **Workflow agents from adk-rust.** Per R7. `SequentialAgent`,
  `ParallelAgent`, `LoopAgent`, `GraphAgent` are flow topologies, not
  dep-imported types.
- **Bevy ECS.** Per the crossflow evaluation. Tokio is the runtime; the
  rubix `live_wire.rs` model is the substrate.

## Decisions made

- **One engine across both Codeless and Rubix shapes.** Codeless's
  `Job → Stage → Task` becomes a flow with `session_policy: fresh-per-stage`
  + `on_failure: gate` + `cost_cap` on stage nodes. Rubix's reactive
  graph becomes a flow with `trigger: event(slot)` + long-lived nodes.
  Same engine, different policy values. This is the decision that makes
  the flow engine load-bearing for the workspace.
- **Lift the rubix `graph` substrate, do not reinvent.** The
  `live_wire.rs` Tokio propagator + single `write_slot` chokepoint
  pattern is the v1 implementation. Generalise where rubix's API is
  too domain-specific (e.g. station-tree path naming becomes generic
  node-id naming).
- **Node-RED wire shape.** Message envelope with `payload`, `topic`,
  custom fields, declared `msg_overrides`. Adopted because the
  ecosystem of authoring tools and the operator mental model already
  exist; cost of adoption is one struct definition.
- **Authoring as YAML, storage as JSON, persistence in
  `starter-store-sqlite`.** Same posture as rubix. Operators
  hand-author YAML; the engine normalises to JSON at the boundary; the
  store is the source of truth for runs. Revisions are immutable;
  `head_seq` pointer per flow tracks the current revision.
- **Skills bind per outer flow run.** Per the agent SCOPE R4 skill
  scope rule 1, generalised: selection happens once, threads through
  the run, intersects with each node's declared `allowed_tools`.
  Avoids the LoopAgent re-selection thrash.
- **Tool stays as the MCP-callable primitive; Node is engine-internal.**
  R8 above. Two traits, not one.
- **D1 resolved — `starter-flow-node-loop` wins; adk-rust stays out of
  the workspace dep tree.** The `ai-agent` node kind body lifts
  Codeless's `Runner` shape and routes every LLM call through
  `starter_ai::AiRunner`. Derives from **R7** (the AI agent is a node
  kind, not a runtime — workflow topology belongs to the engine, leaving
  only the turn-based LLM-loop-with-tool-dispatch in the node body, which
  Codeless's `Runner` already implements). Costs us adk-rust's planner
  heuristics, which neither Codeless nor Rubix consumes today. **Revisit
  trigger:** a consumer surfaces a hard, documented need for adk-rust
  planner heuristics that neither Codeless's `Runner` shape nor Rubix's
  reactive propagator provides — at which point Phase 4 may add
  `starter-flow-node-adk` as a *second*, opt-in body (per R7's "if kept
  at all, it ships as `starter-flow-node-adk`"), not as a replacement.
  Phase 2 builds against this decision so that the `workspace builds
  without adk-rust` smoke gate stays green.
- **D1a — Cycle-bound budget defaults (Phase 2 entry-gate lock).**
  Per R1, every flow run carries a per-run propagation budget. Defaults
  locked at: `max_propagation_hops = 1000`; idempotent-write
  short-circuit `on` (a `write_slot` whose new value equals the prior
  value does not enqueue downstream invocations). Both are overridable
  at `FlowRunner::start` via `RunOpts { max_propagation_hops,
  idempotent_short_circuit }`; neither default is sealed inside the
  propagator. Derives from **R1** ("cycles allowed" only holds if
  cycles can't hang the engine). A run that exhausts the cap is marked
  `Failed { error: cycle-budget-exhausted }` and emits the same span
  shape as any other run failure.
- **D1b — In-memory store shape for Phase 2.** The Phase 2 `GraphStore`
  impl is `std::collections::BTreeMap<SlotRef, SlotValue>` guarded by
  `tokio::sync::RwLock`, with `SubscriptionStream` backed by
  `tokio::sync::broadcast`. Single writer goes through `write_slot`
  (R2); subscribers receive `GraphEvent::SlotChanged` from one
  per-store `broadcast::Sender`. This is **Phase 2 only**; the SQLite
  `FlowStore` / `RunStore` / `SessionStore` impls in
  `starter-store-sqlite` land in **Phase 3** and do not pre-decide
  their on-disk shape here beyond the `GraphStore` trait contract
  already specified. Anything the in-memory impl exposes beyond that
  trait is an internal detail of `starter-flow` and may change in
  Phase 3 without an SPI bump.
- **D1c — `FlowEvent` stream cardinality.** One stream per `FlowRun`.
  Each `FlowRun` handle owns a `tokio::sync::broadcast::Sender<FlowEvent>`;
  every subscriber obtains its own `Receiver` via `FlowRun::subscribe()`
  and sees the full sequence from the point of subscription forward
  (standard `tokio::sync::broadcast` per-subscriber semantics, lagged
  receivers observe a `Lagged` error rather than silently dropping
  data). Multi-consumer is supported (REST adapter, CLI adapter, audit
  span exporter can all subscribe to the same run concurrently). This
  is the cardinality R13 ("streaming reuses existing seams")
  presupposes; locking it here so Phase 2 doesn't accidentally ship a
  single-consumer mpsc that R13 would have to walk back.
- **D1d — Phase 2 catch-up: smoke-test home crate.** The two Phase 2
  Smoke tests ("One write chokepoint", "Engine is reader of policies")
  and the R3 grep-contract test live as integration tests under
  `crates/starter-flow/tests/`, not under `crates/smoke-tests/`. The
  workspace `crates/smoke-tests/` crate is owned by the tools SCOPE
  Stage 9 work and cross-crate end-to-end smokes; placing engine-
  internal smokes there would collide with that ownership and pull
  `starter-flow` into a crate whose dep graph is meant to stay
  consumer-shaped. Engine-internal behaviour proofs ride with the
  engine crate they prove. **Revisit trigger:** a smoke needs node
  kinds or surfaces from a crate `starter-flow` cannot depend on
  without a cycle — then that smoke (and only that smoke) moves to
  `crates/smoke-tests/` under the tools SCOPE ownership.
- **D1e — `transform` body substrate.** The Phase 2 `transform`
  node-kind body in `crates/starter-flow-nodes/src/transform.rs`
  uses a **registered Rust closure** indexed by a `config.fn_id`
  slot value — closures are registered against the `transform` kind
  at kind-registration time and looked up per invocation by the
  `fn_id` the node config carries. No `rhai`, no `starlark`, no
  embedded `wasm` dep is added to `starter-flow-nodes` in Phase 2.
  Inherits the sub-decision locked in the merged starter-flow-engine
  job Stage 1. **Revisit trigger:** a consumer surfaces a need for
  in-flow scripting that the Rust-closure path cannot serve — at
  which point Phase 5's richer `transform` language (`rhai` /
  `starlark` / `wasm`) is reopened as a *second*, opt-in body, not
  as a replacement.
- **D1f — `tool-call` body `ToolRegistry` injection shape.** The
  Phase 2 `tool-call` node-kind body in
  `crates/starter-flow-nodes/src/tool_call.rs` looks up its `Tool`
  via an `Arc<dyn starter_spi::ToolRegistry>` **threaded through the
  run** (engine is constructed with the registry; the propagator
  passes it into each `tool-call` invocation via the per-run
  context). No global static, no `OnceCell`, no thread-local. Same
  substrate locked in the merged starter-flow-engine job Stage 1,
  re-affirmed here so the catch-up bodies don't drift. **Revisit
  trigger:** a node kind genuinely cannot accept the registry by
  argument (e.g. a pre-Phase-2 callsite outside the engine's call
  graph) — at which point we add a registry accessor on the run
  context, not a global.
- **D1g — R3 grep-contract test placement and shape.** The R3 grep-
  contract test lives at `crates/starter-flow/tests/
  r3_no_policy_match_arms.rs` and asserts on the crate's own `src/`
  tree that no code path performs a literal `match` arm on any of
  the seven policy slot names — `session_policy`, `on_failure`,
  `cost_cap`, `safe_state`, `trigger`, `auth`, `timeout`. Hits in
  doc comments (`///`, `//!`, `/* … */`) and string literals are
  fine; hits inside a `match` expression's arms (a literal pattern
  matching one of those names, or a string-equality compare against
  one of those names inside a match arm) are a **stage-fail**.
  Test reads `crates/starter-flow/src/**/*.rs` directly (no
  `cargo expand`, no syn — a line-oriented scan is sufficient and
  keeps the test dep-free). This is the executable form of R3
  ("engine is reader of policies, not a switch over policy names").
  **Revisit trigger:** the engine legitimately needs to dispatch on
  policy *slot* names (not policy *values*) — at which point R3
  itself is revisited, not the test.
- **D1h — Phase 2 dep-tree gates landed as an automated test.** The
  workspace dep-tree gates promised by the original starter-flow-
  engine job (no `adk-rust` under `starter-flow` or
  `starter-flow-nodes`, the `starter-flow-spi` Phase 1 dep baseline
  in `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` holds, and
  no flow crate depends on the Phase 3 surface crates `starter-mcp`
  / `starter-server` / `starter-cli`) live as an executable
  integration test at
  `crates/starter-flow/tests/workspace_dep_tree_gates.rs`. The test
  shells out to `cargo tree --edges normal` for each flow crate from
  the workspace root resolved via `CARGO_MANIFEST_DIR`; absolute
  worktree paths are normalised to `<WORKTREE>/…` before the SPI
  baseline diff so the test is stable across worktree relocations.
  **Revisit trigger:** the SPI crate legitimately gains a new
  dependency — at which point the baseline file is re-generated in
  the same commit that adds the dep, and the change must be
  reviewed.
- **D-F4.1 — `ai-agent` body lives in
  `crates/starter-flow-nodes/src/ai_agent.rs` behind a new
  default-off `ai-agent` cargo feature on `starter-flow-nodes`.**
  Mirrors the Phase 3 stage-4 posture (`flow` feature on
  `starter-store-sqlite`): the dep-tree gate forbidding `adk-rust`
  stays trivially green for consumers who don't enable the feature,
  and the headless-appliance posture (SCOPE 735) is preserved.
  **Revisit trigger:** the body grows large enough that a
  dedicated `starter-flow-node-ai-agent` crate becomes cheaper to
  maintain than a feature flag on the polyglot nodes crate.
- **D-F4.2 — D1 re-confirmed: `starter-flow-node-loop` shape
  wins; adk-rust stays out.** Phase 4 lifts Codeless's `Runner`
  (turn-based LLM-loop with tool-dispatch) and routes every model
  call through `starter_spi::ai::AiRunner`. **Revisit trigger:**
  verbatim D1 — a consumer surfaces a hard, documented need for
  adk-rust planner heuristics that neither Codeless's `Runner`
  shape nor Rubix's reactive propagator provides. At that point a
  second opt-in body ships as `starter-flow-node-adk` *alongside*,
  not as a replacement.
- **D-F4.3 — `AiRunnerRegistry` lives in `starter-flow-spi`,
  hosted via `Engine::with_ai_runner_registry`.** Trait shape
  mirrors `ToolRegistry` (`fn lookup(&self, provider_id: &KindId)
  -> Option<Arc<dyn AiRunner>>`). Concrete `StaticAiRunnerRegistry`
  ships in `crates/starter-flow-nodes/src/ai_agent.rs` next to the
  body. **Revisit trigger:** a host needs dynamic provider
  registration after engine construction — at which point the
  trait gains a `register` method as an additive default-impl
  extension (per the established `#[non_exhaustive]` posture).
- **D-F4.4 — `SkillSelector` trait + `NullSkillSelector`
  default; per-node `skill_hint` is the only override.** The full
  `starter-skills` crate is **not** in Phase 4 scope. Phase 4 ships
  the trait, the `SkillSelection` enum (`None` |
  `Selected { skill_id, allowed_tools, resources, content_hash }`),
  the `Engine::with_skill_selector(...)` builder hook, and a
  `NullSkillSelector` returning `SkillSelection::None`. **Revisit
  trigger:** the `starter-skills` crate lands as a workspace member
  — at which point the placeholder re-export in
  `starter-flow-spi/src/skill.rs` swaps for the real type and the
  `cfg(any())` gate drops.
- **D-F4.5 — Tools allowlist for any ai-agent invocation is the
  intersection of (host `ToolRegistry` keyset ∩ skill
  `allowed_tools` if `Selected` ∩ node `config.allowed_tools` if
  declared); empty intersection is a hard error.** Body surfaces
  `NodeError::Domain { code: "no_tools_visible" }` rather than
  silently running with zero tools. The intersection is computed
  once at invocation entry and frozen for the duration of the LLM
  loop; mid-loop skill bundle updates do not change the allowlist
  within an invocation. **Revisit trigger:** a consumer surfaces a
  legitimate "ai-agent runs with zero tools" case (pure-text
  workflow) — at which point an explicit
  `allow_zero_tools: bool` config slot lands and the default
  stays the hard error.
- **D-F4.6 — Session mode is per-node config; default
  `FreshPerInvocation`.** Enum `SessionMode { FreshPerInvocation,
  ReuseAcrossRun, ReuseAcrossFlow }` (`#[non_exhaustive]`).
  `ReuseAcrossRun` keys on `(node_id, run_id)`;
  `ReuseAcrossFlow` keys on
  `(node_id, flow_id, blake3(principal_id))`. `SessionStore`
  from Phase 3 holds transcripts; new
  `SessionId::for_ai_agent_node(...)` constructor is deterministic
  in its inputs. **Revisit trigger:** a real production workload
  surfaces a cross-flow session-sharing case — at which point a
  fourth `ReuseAcrossPrincipal` variant lands additively.
- **D-F4.7 — Cancellation: `NodeCtx::cancel` is wrapped at the
  call site as `starter_spi::ai::Cancel`; cancel-to-exit ≤ 200 ms.**
  Same budget as `tool-call`. The LLM loop `select!`s against
  `cancel.cancelled().await` between turns and inside tool
  dispatch. **Revisit trigger:** a real provider surfaces a
  cancel-during-stream-decode path that exceeds the 200 ms budget
  — at which point the budget moves to a config slot per the
  Phase 3 D-F3.11 retry-backoff precedent.
- **D-F4.8 — `provider_id` is mandatory; no implicit default.**
  Node config slot `provider_id` is a non-empty reverse-DNS
  `KindId`. Missing or unregistered surfaces
  `NodeError::Domain { code: "provider_id_required" }` or
  `{ code: "provider_not_registered" }`. Explicit beats implicit
  (matches R10 + the headless-appliance posture). **Revisit
  trigger:** a developer-tooling case surfaces a real need for a
  "pick whatever is registered" fallback — at which point an
  explicit `provider_id: "any"` sentinel lands, not an implicit
  default.
- **D-F4.9 — Tool-call dispatch within the LLM loop uses the
  same `ToolRegistry::lookup` chokepoint as the `tool-call` body
  (R8 verbatim).** When the model emits a tool-call, the ai-agent
  body resolves it via the host's registered `Tool`, awaits the
  result, appends it as a tool-result message, and continues the
  loop. No second tool-dispatch path. **Revisit trigger:** none
  expected; this is a core R8 invariant.
- **D-F4.10 — Phase 4 SCOPE smokes live under
  `crates/smoke-tests/tests/` per D-F3.6 precedent.** Two files,
  one commit per file:
  `ai_agent_is_just_a_node_kind.rs` (R1 + R2 + R12) and
  `skill_quarantine_survives_bundle_update_through_a_flow.rs`
  (per-run skill freeze + tools-intersection enforcement). Each
  smoke depends on `starter-ai` (the `RecordingAiRunner`
  testkit) and `starter-flow-nodes` (the body), neither of which
  `starter-flow` can depend on without a cycle — the D-F3.6
  revisit-trigger applies verbatim. **Revisit trigger:** none
  expected; this matches the established Phase 3 smoke location.
- **D-F4.11 — `RecordingAiRunner` testkit lives in
  `crates/starter-ai/src/testing.rs` behind the existing
  `testing` cargo feature mirror.** Records every call with
  `(input, tools_visible, session_id_or_none,
  principal_id_hash, turn_number)` and replays a configurable
  script of `(text, tool_calls)` per turn. Keeps the harness
  close to the real provider impls; the two Phase 4 smokes
  import `starter-ai` with `--features testing` per the
  established pattern. **Revisit trigger:** another consumer
  needs a different testkit shape (e.g. a streaming-token
  fake) — at which point the testkit module gains a sibling
  type, not a replacement.
- **D-F4.12 — Sixth dep-tree gate covers the opt-in
  `--features ai-agent` path.** Add
  `starter_flow_nodes_with_ai_agent_feature_does_not_pull_adk_rust`
  to `crates/starter-flow/tests/workspace_dep_tree_gates.rs`.
  Test shape mirrors the existing four `*_contains_no_adk_rust`
  tests verbatim — same `Command::new("cargo")` + same grep +
  same assert; only the `--features ai-agent` arg differs. The
  D1 invariant holds whether or not consumers enable the
  feature. **Revisit trigger:** the body grows enough sub-features
  that per-feature gates become unwieldy — at which point the
  gate is parameterised over the feature set.
- **D-F5.1 — `trigger.explicit` body lives in
  `crates/starter-flow-nodes/src/trigger_explicit.rs` behind a
  new default-off `trigger-explicit` cargo feature on
  `starter-flow-nodes`.** Mirrors the D-F4.1 posture verbatim:
  feature-off path stays adk-rust-free trivially, headless
  appliance preserved. The body exposes a host-side
  `fire(payload: SlotMap)` handle via a per-channel
  `tokio::sync::mpsc` pair; the registry trait
  `TriggerChannelRegistry` is local to the body file (no SPI
  edit — the registry is opaque to the engine, only the body
  resolves channels). Mandatory config slot `channel_id`
  (reverse-DNS `KindId`) lets a host distinguish multiple
  explicit triggers by binding each to a separate Sender. No
  axum/cron/event-bus substrate; that's `trigger.{webhook,
  schedule, event}` territory for a follow-up job. **Revisit
  trigger:** a consumer needs cross-process fire (e.g. a CLI
  fires a flow running in a long-lived daemon) — at which point
  the channel registry gains a transport-agnostic
  `RemoteTriggerChannel` impl alongside the in-process one.
- **D-F5.2 — `log` body lives in
  `crates/starter-flow-nodes/src/log.rs` behind a new
  default-off `log` cargo feature on `starter-flow-nodes`.**
  Body emits its mandatory `value` input slot as a
  structured `tracing::event!` at a configurable level (`level`
  config slot, optional, default `Level::INFO`, validated to
  enum `{"trace","debug","info","warn","error"}`). Event target
  is hardcoded `"starter.flow.log"` and the event payload
  includes `(node_id, run_id, principal_id_hash, value)`. No
  file/stdout sink — R13 mandates the existing tracing seam
  routes to whatever subscriber the host attached. Output slot
  `emitted` carries a passthrough copy of the input so the
  slot can chain into a downstream node if desired. **Revisit
  trigger:** a consumer surfaces a need for non-tracing sinks
  (file, syslog, OTEL log signal direct) — at which point a
  `sink` config slot lands enumerating supported targets, not
  a parallel sink trait.
- **D-F5.3 — Phase 5 demo wiring lives in `examples/notes/`
  against the existing Claude runner from `starter-ai`.** The
  demo flow at `examples/notes/flows/codeless-demo.yaml`
  chains `trigger.explicit → ai-agent → log`. Node-kind
  registration + `AiRunnerRegistry` + `TriggerChannelRegistry`
  construction happens at notes-app boot (verified at stage 4
  entry — likely `examples/notes/src/server.rs`). The fire
  endpoint is `POST /api/flows/codeless-demo/fire` returning a
  run id; subsequent log events stream through the existing
  notes UI event channel. CLI parity is out of scope for this
  demo. **Revisit trigger:** a second example host adopts the
  flow stack — at which point common boot-time wiring lifts
  into a `starter-flow-host` helper crate, not duplicated
  between hosts.
- **D-F5.4 — End-to-end smoke
  `codeless_shape_on_one_engine.rs` lives under
  `crates/smoke-tests/tests/` per D-F4.10 precedent.** Driven
  by `RecordingAiRunner` from `starter-ai`'s `testing` feature
  so CI never hits the Anthropic API; the notes host runs the
  real Claude runner only when a user fires the button locally.
  One `#[tokio::test]` asserts the trigger-payload→ai-agent
  input plumbing and the ai-agent-output→log event plumbing
  end-to-end on one engine. The Rubix half of Phase 5's
  "Codeless and Rubix shape on one engine" smoke is **not** in
  this job — it needs `branch` + `merge` + `http-out` and
  lands in the follow-up job that picks up the remaining eight
  node kinds. **Revisit trigger:** none expected; this matches
  the D-F4.10 / D-F3.6 smoke-location precedent.
- **D-F5.5 — Two new dep-tree gates cover the opt-in
  `--features trigger-explicit` and `--features log` paths.**
  Add `starter_flow_nodes_with_trigger_explicit_feature_does_not_pull_adk_rust`
  and `starter_flow_nodes_with_log_feature_does_not_pull_adk_rust`
  to `crates/starter-flow/tests/workspace_dep_tree_gates.rs`.
  Test shape mirrors the six existing `*_contains_no_adk_rust`
  gates verbatim — same `Command::new("cargo")` + same grep +
  same assert; only the `--features <name>` arg differs.
  **Revisit trigger:** verbatim D-F4.12 — the body grows
  enough sub-features that per-feature gates become unwieldy,
  at which point the gates parameterise over the feature set.
- **D-F5.6 — `ai-agent` body supports both `RunnerInput::Rest`
  and `RunnerInput::Cli` via the `input_kind` config slot.**
  Default `"rest"` preserves Phase 4 behaviour (LLM-loop with
  host `ToolRegistry` tool dispatch); `"cli"` drives a
  CLI-shape runner (e.g. `ClaudeRunner`) exactly once and
  surfaces `RunResult::text` as the body's output. The CLI
  path skips the host tool-dispatch path because the CLI
  binary owns its own tool-call loop (MCP servers configured
  via `CliCfg::mcp_*`); `turn_count` is always `1` on this
  path and `SessionStore` reads/writes are no-ops (CLI resume
  lives on `CliCfg::resume_id`, threaded by future work, not
  by `SessionStore`). A new `AiAgent::with_input_kind(...)`
  builder mirrors the `with_provider_id` Phase-4 workaround
  so hosts can pin the kind at construction time until the
  propagator can route arbitrary config slots. **Revisit
  trigger:** a third input kind appears (e.g. `Stream`), at
  which point the slot becomes an open enum and the body
  factors out a `RunnerDispatch` trait.

## Open questions

- **D2 — Where the host flow directory lives by default.**
  `$XDG_DATA_HOME/<binary>/flows/` matches the extensions + skills
  default convention. Belongs in `starter-config`'s defaults, not here.
- **D3 — Hot-reload of flows.** A flow file is edited; does the
  registry pick it up without a restart? Probably yes for host-dir
  flows (file-watch + re-parse + new revision; running invocations
  finish on the old revision), but the extensions framework doesn't
  hot-reload either. Defer.
- **D4 — Per-flow rate limit / cost cap.** The flow manifest could
  carry `max_concurrent_runs`, `max_cost_per_run_usd`. Useful for
  publicly-exposed flows; not blocking v1.
- **D5 — Visual canvas package.** A `starter-ui-flow` package (React +
  the rubix node-canvas pattern) would close the loop for authoring.
  Out of scope for the engine itself; lands when a UI consumer asks.
- **D6 — Subflow invocation perf path.** Direct in-process call for
  builtin↔builtin nodes; JSON-RPC over the supervisor's stdio channel
  for any cross-flavour or cross-extension hop. Only the perf knob is
  open — the *semantics* are fixed: both paths emit the same span
  shape (`node.invoke` with `parent_run_id` linkage), both honour the
  same `Cancel` token, both surface the same `FlowEvent` stream.

## Smoke tests (before merging)

In addition to the workspace-level smoke tests in
[SCOPE.md](../../../SCOPE.md):

### "One write chokepoint" test

Three writers — a REST handler, a CLI command, an internal propagator
tick — each write to the same slot. All three are observed (in tracing)
to enter `GraphStore::write_slot` and emit a single `SlotChanged`
event. If any writer bypasses the chokepoint, R2 has slipped.

### "Engine is reader of policies" test

A flow has a writable output node with `safe_state: fail-safe(0)` and
another with `safe_state: hold-last`. The engine is stopped via
`Engine::stop()`. Both outputs receive the engine-driven write to their
declared safe state. No code in `starter-flow` knows about either node
specifically — the engine walked the graph and read the slots. If the
engine has a hardcoded policy registry, R3 has slipped.

### "Codeless and Rubix shape on one engine" test

Two flows in the same engine:
- **Codeless-shape**: a sequential flow of three `ai-agent` nodes,
  each with `session_policy: fresh-per-stage`, `on_failure: gate`,
  `cost_cap: 0.50_usd`. Trigger: `explicit`.
- **Rubix-shape**: a flow with a `trigger: event(slot:src.temperature)`
  on entry, an `ai-agent` node with `session_policy: long-lived`, and a
  writable output with `safe_state: hold-last`.

Both run end-to-end. The Codeless flow halts at the middle gate as
expected; the Rubix flow keeps running and reacts to a second
slot-change event. If either shape requires a special-case engine
path, R1/R3 have slipped.

### "AI agent is just a node kind" test

A flow with a single `ai-agent` node, a `tool-call` node, and a
`branch` node. The `ai-agent` node calls the LLM, dispatches a tool
call into the tool registry, and emits a result. The result is written
to the `branch` node's input slot; the branch routes to one of two
output flows. Every LLM call is observed (in tracing) to route through
`AiRunner`. No `adk-rust` workflow-agent types appear in any span. If
the test depends on adk-rust's `LlmAgent` being the *only* way to do
this, R7 has slipped — the agent runtime is no longer "the runtime",
it's a node-kind body.

### "Flow surfaces as MCP tool" test

A flow `com.example.summarise` is registered. A Claude Desktop client
connects to the host's MCP endpoint, lists tools, and sees
`com.example.summarise`. Calling it runs the flow server-side; each
node's progress streams as MCP `notifications/progress`; the terminal
node's output is the tool's return value. **No code in `starter-mcp`
changed**; the flow registered itself as a Tool via `FlowAsTool` and
landed in the same `ToolRegistry` `starter-mcp` already serves. If
`starter-mcp` needed a patch to surface flows, R9 has slipped.

### "Extension contributes a node kind" test

A process-flavour extension contributes
`contributes.nodes: [{ id: com.acme.weather.current, … }]`. The host
loads it. A flow uses the new node kind. The flow runs end-to-end; the
node's body executes in the extension child over the supervisor's
existing JSON-RPC channel; capability boundaries on the extension are
enforced by the supervisor. No new wire format opens.

### "Skill quarantine survives bundle update through a flow" test

A flow contains an `ai-agent` node. An extension ships a skill bundle.
Operator approves it; the flow's runs pick it up via
`SkillSelector::select`. Operator updates one byte in the skill body.
The next flow run does **not** see the skill (it's re-quarantined per
content-hash); the `ai-agent` node falls back to the default skill or
no skill. If a content edit silently inherits trust through the flow
engine, [agent SCOPE R4](../../agent/SCOPE.md) has slipped.

### "Cancellation propagates through a flow" test

A REST client hits an SSE endpoint that drives a flow with three
`ai-agent` nodes in a loop. The client disconnects after the first
node emits. Within a few hundred milliseconds: REST adapter cancels →
flow run's `Cancel` token fires → propagator stops scheduling new
nodes → the in-flight `ai-agent` cancels its `AiRunner` call → no
further events emit. Same `Cancel` path the existing AI runners use.

### "Three-level stop drives safe state" test

A flow with a writable output node holding value `42`,
`safe_state: fail-safe(0)`. Engine-level `disable` is triggered. The
output is observed (via subscription) to receive a write of `0`. The
engine state machine transitions to `Paused`. A subsequent
`Engine::enable()` returns to `Running`; the output's next computed
value is written normally. If safe-state didn't fire, R12 has slipped.

### "Workspace builds without adk-rust" test

`cargo tree -p starter-flow --edges normal` and `cargo tree -p
starter-flow-nodes --edges normal` snapshots fail the build if
`adk-rust` appears in either tree. If D1 lands on the "leaner LLM-loop"
side (recommended), this snapshot is the gate that keeps it that way.
If D1 lands on "adk-rust under the ai-agent node kind", this test
becomes a `cargo tree -p starter-flow-node-adk` snapshot ensuring
adk-rust is confined to that one crate.

## Phasing (each phase independently mergeable)

### Phase 1 — `starter-flow-spi`

- `Node`, `NodeBehavior`, `KindId`, `NodeId`, `SlotRef`, `SlotValue`,
  `SlotMap`.
- `GraphStore` trait, `WriteSlotOpts`, `SubscribeOpts`.
- `FlowId`, `FlowRevisionId`, `RunId`, `FlowEvent`.
- `FlowStore`, `RunStore` traits.
- Re-exports from `starter-spi` (`Cancel`, `Principal`, `SecretString`).
- No engine. No I/O. No async runtime in deps beyond what `starter-spi`
  already pulls.

Workspace builds; nothing executes yet.

### Phase 2 — `starter-flow` engine (in-memory stores)

- In-memory `GraphStore` impl with single-chokepoint `write_slot`.
- Slot propagator (synchronous tokio loop, subscribes to
  `GraphEvent::SlotChanged`).
- `NodeKindRegistry`, `FlowRegistry`.
- Engine state machine (`Starting → Running → Pausing → Paused →
  Resuming → Stopping → Stopped`).
- Per-flow `Cancel` plumbing; `FlowEvent` stream.
- Two built-in node kinds: `transform` (pure-fn) and `tool-call`.
- Smoke: "one write chokepoint" passes; "engine is reader of policies"
  passes for a trivial `safe_state` test.

### Phase 3 — Persistence + surface wrappers

- `FlowStore` + `RunStore` impls in `starter-store-sqlite` behind a
  `flow` feature.
- Run checkpointing on slot writes; resume from checkpoint after a
  process restart.
- `starter-flow-surfaces`: `FlowAsTool`, `FlowAsService`.
- Smoke: flow invoked from MCP via `starter-mcp` unchanged; flow runs
  as a `Service` driven by a `tokio` test channel; "same-source
  streams over four transports" smoke from the workspace SCOPE
  extended with a `FlowEvent` source.

### Phase 4 — `ai-agent` node kind (D1 resolution)

- One of:
  - `starter-flow-node-loop` — Codeless's `Runner` shape, lifted.
  - `starter-flow-node-adk` — adk-rust's `LlmAgent`, wrapped.
- Routes every LLM call through `AiRunner` (smoke from [agent SCOPE R2](../../agent/SCOPE.md)
  Part B applies).
- Skill selection hook bound to flow-run entry; per-node `skill_hint`
  override.
- Smoke: "AI agent is just a node kind" passes; "skill quarantine
  survives bundle update through a flow" passes.

### Phase 5 — Remaining built-in node kinds

- `branch`, `merge`, `subflow`, `gate`, `trigger.{explicit, event,
  schedule, webhook}`, `http-out`, `log`, `sleep`.
- Each behind its own cargo feature.
- Smoke: "Codeless and Rubix shape on one engine" passes.

### Phase 6 — `starter-ext-flow` adapter

- Inside `starter-extensions/`, parallel to `starter-ext-mcp`.
- Parses `contributes.nodes` and `contributes.flows`.
- Two-phase commit, namespace ownership, capability discipline.
- Smoke: "extension contributes a node kind" passes for builtin,
  process, and wasm flavours.

### Phase 7 — Three-level stop + safe-state

- Engine `stop`, `pause`, `resume` APIs.
- Per-flow `pause` / `resume`.
- `safe_state` policy walked on engine stop and on per-flow stop.
- Smoke: "three-level stop drives safe state" passes.

### Phase 8 — Optional: visual canvas (`starter-ui-flow`)

- Out of scope for the engine itself.
- Lands when a UI consumer asks and a peer-review of the
  rubix/codeless canvas patterns has identified what's portable.

## Bottom line

**`starter-flow` is the engine at the centre of starter from this
SCOPE forward.** Everything is a node. Tools are nodes (via `tool-call`
wrapping `starter_spi::Tool`). The AI agent is a node (`ai-agent`,
routing through `AiRunner`, binding skills per outer flow run).
Workflow topology is the engine's job, not adk-rust's. Extensions
contribute node kinds via `contributes.nodes` and flows via
`contributes.flows`; same namespace ownership, two-phase commit, and
capability discipline as every other extensions contribution. Flows
surface as Tools (MCP, REST, CLI, sub-node) and as Services (event-driven
listeners) through the wrappers `starter-flow-surfaces` ships. The
engine reads policies — `session_policy`, `on_failure`, `cost_cap`,
`safe_state`, `trigger`, `auth`, `timeout` — from config slots on the
nodes themselves; the engine never owns them. One graph, one write
chokepoint, one event bus, one node trait, one tool trait, one runtime.
Codeless's staged-job shape and Rubix's reactive-graph shape are the
same primitive with different policy values.

This SCOPE supersedes [agent SCOPE](../../agent/SCOPE.md) R1 (adk-rust
is no longer "THE runtime"; topology lives here) and refines R5 (engine
owns its own checkpoint shape). All other rules from the agent, tools,
and extensions SCOPEs stand unchanged.
