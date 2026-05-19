# The AI Agent Capability — Scope

> ⚠ **Read [DOCS/flow/scope/SCOPE.md](../flow/scope/SCOPE.md) first.**
> The flow engine is the runtime substrate for everything in starter,
> including agents. This doc owns:
>
> - The `ai-agent` node kind — what it is, what it guarantees, what
>   features a consumer gets when they drop one into a flow.
> - **R2** — `AiRunner` is the only LLM seam (load-bearing, unchanged).
> - **R4** — Skills are static metadata, content-hash quarantined
>   (load-bearing, unchanged).
> - The `SKILL.md` format and the skill scope rules.
> - The end-to-end example of writing an agent on starter.
>
> Engine mechanics (graph store, propagator, slot writes, three-level
> stop, safe-state, run persistence, extension contribution) live in
> the flow SCOPE. This doc references those rules where they apply
> rather than restating them.

## One-line summary

The **`ai-agent` node kind** is starter's first-class AI agent
capability: a turn-based LLM loop with tool dispatch, session
continuity, skill binding, streaming, and cancellation — runnable
standalone, composable into arbitrary multi-agent workflows via the
flow engine, surfaceable as a Tool on MCP / REST / CLI, and
contributable by extensions.

A consumer composing a starter binary gets, by adding `starter-flow`
plus one of `starter-flow-node-loop` / `starter-flow-node-adk` to
`Cargo.toml`:

- Multi-agent workflows (sequential, parallel, loop, graph) — as flow
  topologies, not hard-coded agent types, so any topology is
  expressible.
- Persistent sessions (multi-turn continuity) surviving process
  restart.
- Claude Code / Copilot / Codex / Anthropic / OpenAI provider routing
  via `starter-ai::AiRunner` — every LLM call goes through one seam.
- Tool registries that every extension feeds into; every tool callable
  by any agent.
- MCP exposure of every agent (every agent is a flow; every flow is a
  Tool via `FlowAsTool`).
- Skill bundles discoverable from a directory, quarantined by default,
  approved by content hash.

No fork of upstream agent frameworks. No bespoke multi-agent runtime.
The flow engine owns topology and orchestration; the `ai-agent` node
kind owns the LLM loop and the agent-specific features around it.

## Why this exists

Three forces converge:

1. **Every starter-based product that "does AI" hits the same wall.**
   It needs an LLM loop, tool dispatch, session continuity, streaming,
   cancellation, and a way to surface itself on MCP / REST / CLI.
   `starter-ai` covers the LLM call; `starter_spi::Tool` + `starter-mcp`
   cover tool dispatch + MCP. The agent loop itself — the turn-by-turn
   "ask model → dispatch tool calls → feed results back → repeat until
   stop" cycle — is the piece that gets re-invented per product.
   Codeless already wrote one (`Runner` trait); rubix-agent doesn't
   need one (its nodes drive themselves); the next product would write
   a third. Starter owns this primitive once, as the `ai-agent` node
   kind.
2. **Multi-agent orchestration is graph topology, not agent
   internals.** Sequential / parallel / loop / graph "agent types" are
   shapes of how agents wire to each other — exactly what the flow
   engine handles. Putting orchestration inside the agent runtime
   would duplicate what flow already does, with worse ergonomics
   (four fixed topologies vs. arbitrary graphs) and a separate
   persistence story. The flow engine owns topology; the agent owns
   the loop.
3. **Skills-as-Markdown is the right packaging unit for prompt +
   tool-allowlist + resources.** Claude Code's `SKILL.md` pattern has
   proven out: an operator drops a directory of MD files into a
   bundle, agents pick a relevant skill by description, the skill
   restricts tool access for that invocation, and updates are reviewed
   in diffs. **moxxy** (a prior internal experiment at NubeIO with
   sandboxed AI extensions) adds the right safety wrinkle: skills are
   *quarantined by default* and approved by content hash, so a skill
   bundle pulled in by a third-party extension is never silently
   trusted. The single most load-bearing safety property in this doc.

## Relationship to existing crates

```
starter-spi                       (Tool, AiRunner, Cancel, Principal,
                                   SecretStore — exists, unchanged)
   ▲
   │
   ├── starter-ai                 (5 providers — exists, unchanged)
   │      Claude CLI, Copilot CLI, Codex CLI, Anthropic REST, OpenAI REST
   │
   ├── starter-mcp                (MCP server — exists, unchanged.
   │                               Surfaces agents-as-tools via
   │                               FlowAsTool — see flow R9)
   │
   ├── starter-flow-spi           (per flow SCOPE — contracts)
   │      SessionStore, RunStore, NodeBehavior, SlotMap, FlowEvent
   │
   ├── starter-flow               (per flow SCOPE — the engine)
   │      Owns run lifecycle, session persistence, skill-selection
   │      hook, MCP/REST/CLI/sub-flow surfaces.
   │
   ├── starter-skills             (NEW — agent SCOPE owns)
   │      SKILL.md parser, SkillRegistry, content-hash + quarantine
   │      approval, ApprovalStore trait + sqlite impl.
   │      Zero engine dependency — usable from any Tool caller too.
   │
   └── starter-flow-node-{loop,adk}   (NEW — agent SCOPE owns)
          Implements the `ai-agent` node kind. Exactly one of these
          is enabled in any given binary; both implement the same
          NodeBehavior surface. See D1.
```

Extensions contribute agents as **flows** rooted at an `ai-agent` node
(per flow R11), not as a separate contribution kind. The
`starter-ext-flow` adapter handles them. There is no
`starter-ext-agents` crate.

## Hard rules (load-bearing)

The rules below are the ones specific to the **agent capability**.
Engine rules (one write chokepoint, engine reads policies, three-level
stop, etc.) live in [flow SCOPE](../flow/scope/SCOPE.md) and apply
transitively.

### R2 — `AiRunner` is the only LLM seam

Every LLM call from any `ai-agent` node — planner, reviewer,
sub-agent body, anything — routes through `starter-ai::AiRunner`. The
node-kind body translates the agent's turn-state into `AiRunner`
input; the registered `AiRunner` impl (Claude CLI / Copilot CLI /
Codex CLI / Anthropic REST / OpenAI REST) dispatches.

This is what makes "must work with the user's Claude Code / Copilot"
load-bearing rather than aspirational: those CLIs are already
`AiRunner` impls. Plug them in once at the binary level; every agent
in every flow uses them, including agents from third-party extensions.

No bypass. An `ai-agent` node body that calls an OpenAI SDK directly
is a bug.

**Enforcement is mechanical, not honour-system.** A CI check on
`starter-flow-node-loop` (and `starter-flow-node-adk` if shipped) runs
`cargo tree -p <crate> --edges normal` and fails if any of
`async-openai`, `anthropic-ai-sdk`, `anthropic-sdk`, `google-genai`,
`aws-sdk-bedrockruntime`, `mistralai`, `ollama-rs`, or any provider
SDK appears in the tree. The only path to a provider is through
`AiRunner`.

### R4 — Skills are static metadata; quarantined by default

`SKILL.md` files are parsed at load time with `deny_unknown_fields`
on the frontmatter. Body and `resources/*` files are read once and
cached. **Never** templated at runtime. Skills are equivalent to
extension manifests in this respect — same anti-prompt-injection
guarantee, same review surface.

Skills loaded from the host's own directory (e.g.
`$XDG_DATA_HOME/<binary>/skills/`) default to `trust: approved`.
Skills contributed by extensions default to `trust: quarantined`:
they load, they appear in `SkillRegistry::list_quarantined()`, but
`SkillRegistry::select(query)` never returns them until an operator
runs the approval flow.

Approval is keyed on the **content hash of the entire skill bundle
directory**. The hash input is defined precisely so it survives
cross-platform git checkouts and editor litter:

1. Enumerate every file in the skill dir, recursively.
2. Exclude paths matching: `.DS_Store`, `Thumbs.db`, `*.swp`,
   `*.swo`, `*~`, anything under `.git/`, anything under `.idea/`,
   anything under `__pycache__/`. (List is in
   `starter-skills::approval::EXCLUDED`; additions go via PR.)
3. Normalise line endings on text files to `\n` (extension-based
   detection: `.md`, `.txt`, `.json`, `.yaml`, `.yml`, `.toml`).
   Binary files (images, archives) hash as-is.
4. Sort by relative path, ASCII byte order.
5. For each file, hash `<relative path>\0<file content>\0` into a
   `blake3` hasher.
6. Output is the hex digest.

A skill is approved by its hash; the operator's approval row in
`ApprovalStore` carries `(skill_id, hash, approved_at, approved_by)`.
On load, the registry recomputes the hash; mismatch → quarantined,
regardless of the prior approval row. An update to any non-excluded
byte re-quarantines.

### R-agent-1 — The `ai-agent` node kind is the agent primitive

There is one agent runtime in starter, and it is the `ai-agent` node
kind. Its body:

- Reads the prompt from its input slot.
- Resolves the active skill (per R4 + the skill scope rules below).
- Calls the LLM via `AiRunner` (R2).
- Dispatches tool calls into the host's `ToolRegistry` (per [agent
  R3 ≡ flow R8](../flow/scope/SCOPE.md): one tool registry, one
  trait, unchanged).
- Maintains per-run turn state (request/response history,
  in-flight tool calls).
- Persists session state via `SessionStore` between turns.
- Streams events as `FlowEvent::NodeEmitted` on its output slot.
- Honours its `cost_cap`, `timeout`, and `session_policy` config
  slots per [flow R3](../flow/scope/SCOPE.md).
- Cancels promptly on `Cancel` token fire (per [flow R13](../flow/scope/SCOPE.md)).

**Multi-agent topology is the flow engine's job, not the agent's.**
"Sequential" / "parallel" / "loop" / "graph" agents from upstream
frameworks correspond to flow topologies:

| Agent-framework name | Equivalent in starter |
|---|---|
| `LlmAgent` | one `ai-agent` node |
| `SequentialAgent` | a linear flow of `ai-agent` nodes |
| `ParallelAgent` | a flow with `fork` → multiple `ai-agent` → `join` |
| `LoopAgent` | a flow with a `loop` node whose body is an `ai-agent` |
| `GraphAgent` | a flow with arbitrary topology (including cycles) |

Starter does not re-export any of these as types — they are *shapes*
in flow YAML, not Rust types. A consumer who wants "a planner →
researcher → reviewer pipeline" writes a sequential flow with three
`ai-agent` nodes, not a `SequentialAgent::new(...)` call.

This is what unblocks Codeless's "stages bound context" pattern
(stages with `session_policy: fresh-per-stage`, `on_failure: gate`)
and Rubix's "long-lived reactive agent" pattern (single `ai-agent`
node with `trigger: event(slot)`, `session_policy: long-lived`) on
the **same** primitive.

### R-agent-2 — Sessions persist through the engine's SessionStore

Multi-turn continuity is the engine's job, not the agent's. An
`ai-agent` node with `session_policy: continue` invokes the engine's
`SessionStore::load(session_id)` at the start of each invocation and
`SessionStore::append(session_id, turn)` at the end of each turn.

Concretely:

- A flow run's input may carry `session_id: Option<SessionId>` —
  `Some` to continue, `None` to start fresh.
- The `session_policy` config slot on each `ai-agent` node
  determines lifecycle:
  - `fresh` — new session per invocation (Codeless's stage discipline).
  - `continue` — pick up where the prior invocation left off (the
    "claude --continue" pattern).
  - `long-lived` — one session shared across many triggers (Rubix's
    long-running assistant pattern).
- The agent's turn-state opaque blob (the model's request/response
  history, partial tool calls) is one of the per-node checkpoint
  blobs the engine persists per [flow R6](../flow/scope/SCOPE.md).
  Format version is owned by the `ai-agent` node-kind crate.

Process restart, crash, and host migration are survivable. Same
guarantees the engine's `RunStore` provides for any node kind.

### R-agent-3 — Agents are first-class Tools and first-class Services

An "agent" in starter is **a flow whose root is an `ai-agent` node**.
Per [flow R9](../flow/scope/SCOPE.md), every flow gets:

- **A `Tool` surface** via `FlowAsTool`. The flow surfaces as a
  callable Tool, automatically MCP-callable (via `starter-mcp`),
  REST-callable (via `starter-server`), CLI-callable (via
  `starter-cli`), and callable as a sub-node from another flow.
- **A `Service` surface** via `FlowAsService`. The flow runs in
  response to events from an `EventSink` (Slack messages, webhook
  hits, scheduled triggers).

Two consequences:

- **Every agent is automatically an MCP tool.** A Claude Desktop
  client connecting to a starter binary sees every host-registered
  agent as a tool. Token-level progress streams as
  `notifications/progress` per [flow R13](../flow/scope/SCOPE.md). No
  extra wiring.
- **Sub-agent calls are sub-flow calls (which are tool calls).** A
  planner agent that calls a researcher agent does so via a
  `tool-call` node wrapping the researcher's `FlowAsTool`. One
  composition mechanism for "agent calls agent", "agent calls tool",
  and "MCP client calls agent" — they all go through the registry.

### R-agent-4 — Extensions contribute agents as flows

Per [flow R11](../flow/scope/SCOPE.md), extensions contribute agents
via `contributes.flows` in `block.yaml`. The flow's root node is an
`ai-agent`. The contribution carries the manifest's `auth:`
declaration; the `starter-ext-flow` adapter applies it at the
boundary so the agent never performs the auth check itself ("adapters
apply auth, not extensions").

```yaml
contributes:
  flows:                          # surfaced by starter-ext-flow
    - id: com.acme.refund.assistant
      flow_file: flows/refund.yaml         # ai-agent + tool-call nodes
      auth: { require_role: Reader }

  skills:                         # surfaced by starter-ext-flow,
                                  # registered in SkillRegistry
                                  # (default trust: quarantined)
    - dir: skills/

  tools: [ ... ]                  # existing; per agent R3 / flow R8
  nodes: [ ... ]                  # existing; per flow R11
```

There is **no `starter-ext-agents` crate**. The `starter-ext-flow`
adapter handles `contributes.flows` and `contributes.skills`
alongside `contributes.nodes`. One adapter, one wire format
(extensions R10's JSON-RPC), no new mechanism.

## The SKILL.md format

```markdown
---
id: com.acme.refund-flow
description: |
  Process a customer refund, including policy check and approval gate
allowed_tools:
  - stripe.refund
  - db.orders.read
  - slack.notify
model_hint: claude-opus-4-7        # optional; overrides default routing
trust: approved                    # approved | quarantined
                                   # default for host-dir skills: approved
                                   # default for extension skills:  quarantined
resources:                         # paths relative to the skill dir
  - refund-policy.md
  - examples.md
---

# Refund flow

When the user asks to refund an order:

1. Read the order with `db.orders.read`.
2. Check it against `refund-policy.md`.
3. If under $100, refund directly via `stripe.refund`.
4. Otherwise, notify `#refunds` via `slack.notify` and pause for
   approval.
```

`SkillRegistry::select(query) -> Option<Skill>` returns at most one
skill. The default `SkillSelector` impl is "LLM picks by description"
(one cheap Haiku call against the registry's descriptions);
alternatives (vector / keyword) are pluggable for consumers that want
deterministic selection.

When a skill is selected for a flow run, every `ai-agent` node in
that run:

- Prepends `skill.body` to its instruction.
- Restricts its tools to the intersection (per skill scope rule 4
  below).
- Mounts `resources/*` as readable files in the node's workspace.
- Tags every span with `skill.id`.

## Skill scope rules

Skill selection interacts with the flow engine through four rules,
specified up-front rather than discovered:

1. **Selection happens once, at outer flow run entry.** When the
   engine starts a flow run, `SkillSelector::select(prompt)` runs
   once; the result threads through the entire run as
   `SkillSelection`. A `loop` node's body does *not* re-select per
   iteration. **Reason:** cheap-Haiku selection cost adds up; a loop
   that re-selects is a loop that thrashes.
2. **Every `ai-agent` node in the run inherits the selection.** Same
   prompt context, same skill, same `allowed_tools` filter, by
   default.
3. **An `ai-agent` node can declare its own skill explicitly via the
   `skill_hint` config slot.** When present, that skill is used for
   the node regardless of outer selection; no selector call. This is
   the escape hatch for workflow steps that are structurally
   different (e.g. a "reviewer" node that always uses the
   `com.acme.review` skill). **`skill_hint` lives on the node's
   config slot, not on a sub-agent manifest.**
4. **Tool allowlist composition is intersection, not union.** The
   effective tool set for an `ai-agent` node is:
   `host_registry ∩ skill.allowed_tools ∩ node.allowed_tools`. A
   skill's allowlist is a *security restriction*, not a recommendation;
   neither a node config nor a sub-flow can widen what the outer
   skill allowed. If the intersection is empty, the node runs with an
   empty tool registry (the flow author's bug; surfaced loudly in
   tracing).

```yaml
# flow.yaml — pipeline with one node overriding the inherited skill
id: com.acme.research.pipeline
trigger: explicit
nodes:
  - id: gather
    kind: ai-agent                # inherits outer skill
  - id: review
    kind: ai-agent
    config:
      skill_hint: com.acme.review # explicit, no selector call
  - id: summarise
    kind: ai-agent                # inherits outer skill
links:
  - { from: gather.out,    to: review.in    }
  - { from: review.out,    to: summarise.in }
```

## End-to-end: writing an agent

A consumer's binary, with an MCP server and an extension that ships
both tools and an agent:

```rust
// main.rs (consumer)
let secrets = FileSecretStore::open("/etc/myapp/secrets.toml")?;
let runner  = starter_ai::registry::Registry::with_defaults()
    .get(&Provider::ClaudeCli).unwrap();              // CLI wrapper

let ext_host = ExtensionHost::load("/var/lib/myapp/extensions")?;

let tools = ToolRegistry::new()
    .extend(ext_host.contributed_tools());

let skills = SkillRegistry::new()
    .load_dir("/var/lib/myapp/skills")                // approved by default
    .extend(ext_host.contributed_skills());           // quarantined by default

let nodes = NodeKindRegistry::with_builtins()         // includes ai-agent
    .extend(ext_host.contributed_node_kinds());

let flows = FlowRegistry::new()
    .load_dir("/var/lib/myapp/flows")
    .extend(ext_host.contributed_flows());            // includes agents

let engine = starter_flow::Engine::builder()
    .with_runner(runner)
    .with_tools(tools)
    .with_skills(skills)
    .with_node_kinds(nodes)
    .with_flows(flows)
    .with_store(starter_store_sqlite::open("data.db")?)
    .build()?;

// Every flow surfaces as a Tool. starter-mcp serves them unchanged.
let mcp_router = starter_mcp::mcp_router::<AppState>(
    Arc::new(engine.as_tool_registry()),              // flows + tools
    McpHttpOptions::new().with_auth(authenticator),
);

ServerBuilder::<AppState>::new(state)
    .merge_router(mcp_router)
    .merge_router(starter_ext_server::admin_routes(&ext_host))
    .build()
    .serve()
    .await?;
```

What an extension author writes to ship an agent:

```yaml
# block.yaml
id: com.acme.refund
version: 0.1.0
runtime: { kind: process, bin: refund-extension }

contributes:
  tools:
    - id: com.acme.refund.stripe
      handler: StripeRefundTool
    - id: com.acme.refund.orders-db
      handler: OrdersReadTool

  skills:
    - dir: skills/

  flows:
    - id: com.acme.refund.assistant
      flow_file: flows/refund.yaml
      auth: { require_role: Reader }
```

```yaml
# flows/refund.yaml
id: com.acme.refund.assistant
trigger: explicit                  # called as a tool / from MCP
nodes:
  - id: assistant
    kind: ai-agent
    config:
      session_policy: continue
      cost_cap: 1.00_usd
      allowed_tools:
        - com.acme.refund.stripe
        - com.acme.refund.orders-db
links: []                          # single-node flow
```

```markdown
<!-- skills/refund-policy/SKILL.md -->
---
id: com.acme.refund-policy
description: Apply the company refund policy to an order
allowed_tools:
  - com.acme.refund.orders-db
  - com.acme.refund.stripe
trust: quarantined                  # extension-shipped; needs operator approval
---

# Refund policy
[ … skill body … ]
```

A Claude Desktop client connecting to the host's MCP endpoint sees
`com.acme.refund.assistant` as a tool. Calling it runs the assistant
flow server-side; the `ai-agent` node selects the
`com.acme.refund-policy` skill (once the operator approves its hash);
the node dispatches `com.acme.refund.orders-db` and
`com.acme.refund.stripe` tool calls into the extension child process
over the supervisor's existing JSON-RPC channel; tokens stream as MCP
`notifications/progress`; the final answer is the tool's return value.
Zero new wire format opened end-to-end.

## What does NOT land

- **No second agent runtime.** The `ai-agent` node kind is the agent
  primitive. No `LlmAgent` re-export, no parallel `Agent` trait.
- **No `SequentialAgent` / `ParallelAgent` / `LoopAgent` /
  `GraphAgent` Rust types.** These are flow topologies, not types.
- **No alternative provider abstraction.** R2; `AiRunner` is the
  seam.
- **No second tool trait.** Agent R3 ≡ flow R8; `starter_spi::Tool`
  is the trait.
- **No "skill marketplace."** Skills live in directories, distributed
  out-of-band (extension bundles, deploy step, git pull). Same
  posture extensions take.
- **No runtime-templated descriptions / instructions.** R4;
  anti-prompt-injection guarantee inherited from extensions R7.
- **No agent-level scheduler / supervisor.** The job-level state
  machine (queued / running / paused / awaiting-review / completed /
  failed / stopped) is a *consumer* concern (codeless-on-starter
  builds it). The flow engine provides per-run lifecycle; the
  multi-run supervisor over many flow runs is domain-specific.
- **No `starter-agent` / `starter-agent-spi` / `starter-agent-surfaces`
  crates.** Subsumed by `starter-flow*`. The `ai-agent` body lives in
  `starter-flow-node-loop` (or `starter-flow-node-adk` per D1); the
  flow surfaces live in `starter-flow-surfaces` per flow SCOPE.

## Decisions made

- **The `ai-agent` node kind is the agent primitive.** R-agent-1.
  Sequential / parallel / loop / graph agents are flow topologies.
- **`AiRunner` is the only LLM seam.** R2. Mechanically enforced by
  CI dep-tree snapshot on `starter-flow-node-loop` (and
  `starter-flow-node-adk` if shipped).
- **Skills inside `starter` workspace, not `starter-extensions`.**
  Skills are useful without extensions (a consumer with no extension
  framework can still drop SKILL.md files in a directory). The
  *adapter* that surfaces extension-contributed skills lives in
  `starter-extensions/starter-ext-flow` where every other adapter
  lives.
- **Agents are flows, not a separate contribution kind.** R-agent-4.
  Avoids inventing a wire shape for "agents" distinct from "flows
  whose root is an ai-agent" — they are the same thing.
- **Sessions through the engine's `SessionStore`, not a separate
  store.** R-agent-2. Re-uses migrations and observability the
  workspace already has via `starter-store-sqlite`'s `flow` feature.
- **Skill quarantine + content-hash approval.** R4. Lifted from
  moxxy. The single most important safety pattern in this design.
- **`skill_hint` on node config slot, not sub-agent manifest.** Skill
  scope rule 3. There is no "sub-agent manifest" under the flow
  model; the per-node config slot is the natural home.

## Open questions

### D1 — `starter-flow-node-loop` or `starter-flow-node-adk`?

The `ai-agent` node kind's body is the LLM loop with tool dispatch.
Two implementations are possible:

- **(a) `starter-flow-node-adk`** — wraps adk-rust's `LlmAgent` for
  just the loop. Pin adk-rust = "x.y.z" exactly; the bridge surface
  is small (Model adapter + Tool adapter + per-turn event mapping)
  since topology and sessions are starter's job, not adk-rust's.
- **(b) `starter-flow-node-loop`** — lifts Codeless's `Runner` shape
  (see `/home/user/code/rust/codeless-workspace/codeless/crates/codeless-runtime/`
  for the working impl to mine — LoC tally is a Phase 4 entry gate,
  not an established fact). No external dep beyond `starter-ai`.

**Recommended default: (b).** Reasons:

- Saves the bridge LoC, the pinned-version dance, and one external
  dep with pre-1.0 churn.
- Loses adk-rust's planner heuristics — neither codeless nor rubix
  uses them, but that's a real feature we'd be skipping.
- (a) remains shippable later as a sibling crate if a consumer wants
  it; the two are mutually-exclusive cargo features on the same
  node-kind id, so a flow that uses `ai-agent` works with either.

Decision deferred to Phase 4 entry gate (see phasing below). The
audit at the gate is: count the actual bridge LoC in (a), measure the
loop LoC in (b), pick on evidence.

### D2 — Where the host skills directory lives by default

`$XDG_DATA_HOME/<binary>/skills/` matches the extensions + flows
default convention. Belongs in `starter-config`'s defaults, not here.

### D3 — Hot-reload of skills

A skill file is edited; does the registry pick it up without a
restart? Probably yes for host-dir skills (file-watch + re-hash;
approval status preserved iff hash unchanged), but the extensions
framework doesn't hot-reload either, so consistency argues against.
Defer.

### D4 — Per-skill rate limit / cost cap

The skill frontmatter could carry `max_tokens_per_invocation` or
`max_cost_usd` per skill, with enforcement at the `AiRunner` boundary.
Useful for publicly-exposed agents; not blocking v1. Note that
node-level `cost_cap` already exists ([flow R3](../flow/scope/SCOPE.md))
— this is the per-skill orthogonal control.

## Smoke tests (before merging)

In addition to the flow SCOPE's smoke tests and the workspace-level
ones:

### "AiRunner is the only LLM seam" test (R2)

Two parts, both required:

**Part A** — A binary composes `provider-claude` (`AiRunner` impl
wrapping the `claude-wrapper` CLI) and a `ToolRegistry` with one
trivial tool. Five flows exercise five topologies — single-node,
sequential, parallel, loop, graph — each built around `ai-agent`
nodes. Every LLM call observed in tracing routes through the agent
runner's `AiRunner` call. If any path bypasses it, R2 has slipped.

**Part B** — `cargo tree -p starter-flow-node-loop --edges normal`
(and `-p starter-flow-node-adk` if shipped) snapshot test in CI fails
if any of `async-openai`, `anthropic-ai-sdk`, `anthropic-sdk`,
`google-genai`, `aws-sdk-bedrockruntime`, `mistralai`, or `ollama-rs`
appear in the transitive dep tree. The only path to a provider is
through `AiRunner`.

### "Skill quarantine survives bundle update" test (R4)

An extension ships a skill bundle. Operator approves it; the
`SkillRegistry` returns it from `select()`. Operator updates one byte
in the skill body. Next host start: the skill is re-quarantined;
`select()` no longer returns it; the previous approval row is still
present in storage but does not apply because the hash changed. If a
content edit silently inherits trust, R4 has slipped.

### "Skill selection happens once per outer flow run" test

A flow with a `loop` node containing an `ai-agent` body runs 10
iterations. `SkillSelector::select(...)` is called exactly once. The
selected skill threads through all 10 iterations. If selection fires
per iteration, skill scope rule 1 has slipped.

### "Tool allowlist composition is intersection" test

A skill declares `allowed_tools: [a, b]`. A node's config declares
`allowed_tools: [a, c]`. The node's effective tool registry contains
exactly `[a]`. If `c` leaks through, skill scope rule 4 has slipped
(security regression).

### "Multi-agent composition without re-export" test

A flow with three `ai-agent` nodes wired sequentially produces the
same end-to-end behaviour a "SequentialAgent of three LlmAgents"
would in upstream frameworks: gather → review → summarise, each with
its own optional `skill_hint`, intermediate values passed through
slots, all routed through one `AiRunner`. **No `SequentialAgent`
Rust type appears anywhere in the workspace.** If the test depends
on importing a topology type from adk-rust or any other framework,
R-agent-1 has slipped.

### "Extension contributes an agent over MCP" test

A process-flavour extension contributes
`contributes.flows: [{ id: com.acme.summariser, flow_file: ... }]`
whose root node is an `ai-agent`. The host loads it. A Claude Desktop
client connects to the host's MCP endpoint, lists tools, sees
`com.acme.summariser`, and calls it. The flow runs server-side; the
`ai-agent` node executes in-host (the LLM call goes through
`AiRunner`); any tool calls dispatched into extension-contributed
tools route over the supervisor's existing JSON-RPC channel; tokens
stream as MCP `notifications/progress`. No new wire format is opened.

## Phasing

The agent capability ships as part of the flow engine's phasing (see
[flow SCOPE phasing](../flow/scope/SCOPE.md#phasing)). The
agent-specific phases:

### Phase A — `starter-skills` (parallel with flow Phase 1)

- `SKILL.md` parser with `deny_unknown_fields` frontmatter.
- `SkillRegistry::load_dir`, content-hash, `select()` with default
  LLM-picks selector.
- Quarantine state + `ApprovalStore` trait; in-memory + sqlite impls.
- Smoke: drop two SKILL.md files in a dir, call `select("refund a
  customer")`, observe the right one returned; smoke test "skill
  quarantine survives bundle update" passes.

Skills work today without agents. A consumer with a plain `Tool`
caller (no flow engine) can already adopt them via
`SkillRegistry::select` + a tool-set filter.

### Phase B — `starter-flow-node-loop` (flow Phase 4)

Recommended default for D1.

- Lift Codeless's `Runner` shape; route every LLM call through
  `AiRunner`; integrate with the engine's `SessionStore` for
  multi-turn continuity.
- Skill-selection hook bound at flow-run entry; per-node `skill_hint`
  override.
- Cost-cap enforcement at the `AiRunner` boundary.
- Smoke: "AiRunner is the only LLM seam" Part A passes for
  `ai-agent` in single-node, sequential, parallel, loop, and graph
  flows; "skill selection happens once per outer flow run" passes;
  "tool allowlist composition is intersection" passes.

### Phase C — `starter-flow-node-adk` (optional, deferred)

If a consumer needs adk-rust's planner heuristics or upstream-tracked
agent features, this crate ships as a sibling implementation of the
same `ai-agent` node-kind id, mutually-exclusive with
`starter-flow-node-loop` via cargo features. Adds an Appendix-A-style
bridge LoC gate at merge — if the bridge exceeds the recommended
ceiling, revisit.

### Phase D — Extension-contributed agents

Falls out of `starter-ext-flow` (flow Phase 6). Once the adapter
handles `contributes.flows` + `contributes.skills`, extension-shipped
agents work end-to-end. Smoke: "extension contributes an agent over
MCP" passes.

## Bottom line

**The AI agent capability is first-class in starter, with all the
features a consumer needs to ship a product: multi-agent workflows
(via flow topology), persistent sessions, provider routing through
`AiRunner`, MCP exposure of every agent, skill bundles with
content-hash quarantine, extension-contributed agents — all through
the seams that already exist in the workspace.** The runtime
substrate is the flow engine
([DOCS/flow/scope/SCOPE.md](../flow/scope/SCOPE.md)); the
agent-specific surface is the `ai-agent` node kind, R2 (AiRunner-only
LLM seam), R4 (skills static + content-hash quarantine), and the
skill scope rules. No second runtime, no fork, no parallel agent
universe. One engine, one tool trait, one LLM seam, one skill format,
one path from contribution to MCP — the agent is a citizen of starter,
not an island.
