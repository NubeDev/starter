# `ai-builder` — Scope

> ⚠ **Read these first.** This doc layers on top of four SCOPEs that
> own the load-bearing primitives. If anything below contradicts them,
> they win.
>
> - [DOCS/frontend/sdui/SCOPE.md](../sdui/SCOPE.md) — **the wire
>   format**. ai-builder is the AI authoring mode for SDUI. The IR
>   variants, binding grammar, action protocol, capability handshake,
>   and DoS limits all come from SDUI; ai-builder does not invent a
>   second structured-UI format.
> - [DOCS/flow/scope/SCOPE.md](../../flow/scope/SCOPE.md) — the runtime
>   substrate. An ai-builder session is a flow run. Push channels are
>   slot writes. Surfaces (MCP / REST / sub-flow) come from
>   `FlowAsTool` / `FlowAsService` for free.
> - [DOCS/agent/SCOPE.md](../../agent/SCOPE.md) — `AiRunner` is the
>   only LLM seam (R2). Skills are `SKILL.md` with content-hash
>   quarantine (R4). The agent primitive is the `ai-agent` node kind
>   (R-agent-1).
> - [DOCS/agent/SKILLS.md](../../agent/SKILLS.md) — the `starter-skills`
>   crate that implements the `SKILL.md` parser, `SkillRegistry`,
>   content-hash + quarantine state machine, `ApprovalStore`, and the
>   default LLM-by-description selector. Phase 5 below (reference
>   skills) depends on this crate landing.
> - [DOCS/frontend/theme/README.md](../theme/README.md) — the theme
>   editor's token model (38 tokens + `ShellConfig`) and
>   `ThemeTransport` seam. The AI theme-gen slice below targets that
>   exact shape; it does not introduce a parallel token model.

## One-line summary

`ai-builder` is starter's **AI authoring mode for SDUI**: a host
agent emits `ComponentTree` deltas (and `TokenPatch` deltas for the
theme slice) into a flow's output slot; the React surface renders
them as they stream using the same SDUI `<Renderer>` hand-authored
pages render through. Two slices share one node kind —

- **page-builder** — agent emits SDUI `ComponentTree`s rendered
  against `@nube/starter-ui-kit` shadcn primitives via
  `@nube/starter-sdui-react`.
- **theme-builder** — agent emits the 38 theme tokens + `ShellConfig`
  the theme editor already owns; the same editor consumes the result.

Both slices are flows rooted at an `ai-agent` node. The LLM call goes
through `AiRunner` (agent R2). The "push from backend without a chat
round-trip" path is just another node writing to the same slot (flow
R8). React surfaces consume an SSE endpoint that `FlowAsService`
mounts automatically and feeds events into the SDUI renderer's
existing patch / full-render pipeline.

## Why this exists

Three forces converge:

1. **Two products want AI-driven UI.** A scope-preview / dashboard
   surface where the user types a request and structured UI streams
   back, plus a theme editor that takes prompts ("make this warmer",
   "match `#0a7c4d`"). The plumbing is the same; only the output
   shape differs (component tree vs token map).
2. **SDUI already solves "AI emits structured UI" — it just needs an
   authoring mode wired up.** Rubix's SDUI work (now ported into
   [`DOCS/frontend/sdui/SCOPE.md`](../sdui/SCOPE.md)) defines the IR,
   the binding grammar that lets one page render against N targets,
   the action protocol, and the live-subscription mechanism.
   Hand-authored pages and Rust-DSL pages already work end-to-end.
   AI authoring is the third path onto the same renderer; ai-builder
   is the crate that wires it up.
3. **A prior internal prototype validated the end-to-end shape.**
   `ai-ui` (`/home/user/code/rust/ai-ui/SCOPE.md`) built this against
   [OpenUI](https://github.com/thesysdev/openui) with its own
   provider trait, push channel, skill format, and wire format —
   inventing all four because starter didn't have them yet. Starter
   now has them (`AiRunner`, slot writes, `SKILL.md`, SDUI IR);
   ai-builder is the same idea rebuilt on starter's seams.

This is the **third implementation of "AI emits UI"** in this
codebase family, which is the right time to extract.

## Relationship to existing crates

```
starter-spi                       (Tool, AiRunner, Service — exists)
   ▲
   │
   ├── starter-ai                 (5 providers — exists, unchanged)
   ├── starter-skills             (SKILL.md + quarantine — per
   │                               SKILLS.md, in flight)
   ├── starter-flow               (engine — per flow SCOPE, exists)
   ├── starter-flow-node-loop     (ai-agent node body — per agent
   │                               SCOPE, exists)
   │
   ├── starter-ui-ir              (per sdui SCOPE — typed component
   │                               IR. ai-builder emits ComponentTree
   │                               deltas in this format.)
   ├── starter-ui-bindings        (per sdui SCOPE — binding grammar.
   │                               AI-emitted trees use `{{$target/...}}`
   │                               just like hand-authored ones.)
   ├── starter-ui-builder         (per sdui SCOPE — typed Rust
   │                               constructors. ai-builder's prompt
   │                               crate uses its component
   │                               vocabulary as the prompt schema.)
   │
   ├── starter-ai-builder-prompt  (NEW — assembles the system prompt:
   │                               SDUI IR JSON Schema + theme token
   │                               schema + binding-grammar primer
   │                               + selected SKILL.md bodies.)
   ├── starter-flow-node-ai-builder
   │                              (NEW — node kind whose body wraps
   │                               the ai-agent loop, constrains the
   │                               output format to SDUI IR
   │                               (or TokenPatch for the theme slice),
   │                               and emits BuilderEvent stream items
   │                               on the output slot.)
   │
   ├── starter-ui-kit             (exists — shadcn primitives + theme
   │                               editor)
   ├── starter-ui-core            (exists — react-query/zustand hooks
   │                               + theme-editor state)
   ├── @nube/starter-sdui-react   (per sdui SCOPE — the renderer.
   │                               ai-builder reuses <Renderer> and
   │                               the action dispatcher unchanged.)
   │
   ├── @nube/starter-ai-builder-react       (NEW — <AiBuilder> wrapper
   │                                         that opens the flow's SSE,
   │                                         feeds BuilderEvents into
   │                                         the SDUI <Renderer>'s
   │                                         applyPatch helpers, owns
   │                                         the prompt input chrome.)
   └── @nube/starter-ai-builder-react-theme (NEW — <AiThemeBuilder>;
                                             applies TokenPatch events
                                             through the existing
                                             ThemeTransport.)
```

There is no `starter-ai-builder-lang` crate. The wire format is SDUI
(`starter-ui-ir`); inventing a parallel format inside one workspace
was the wrong call. The only ai-builder-specific Rust crates are
`starter-ai-builder-prompt` (prompt assembly) and
`starter-flow-node-ai-builder` (the node body).

There is no `starter-ai-builder-core` Rust crate either. The
"engine" for ai-builder is the flow engine + the SDUI renderer; both
already exist.

## Hard rules

These are specific to ai-builder. SDUI, flow, and agent rules apply
transitively. **In particular, SDUI R2 (versioned IR), R3 (no domain
strings), R4 (server-side bindings), R5 (action protocol), R7
(`custom` escape hatch), R8 (DoS limits), and R9 (no client logic)
apply to AI-emitted trees identically to hand-authored ones.**

### R1 — AI output is SDUI IR or a delta on SDUI IR

Every page-builder emission is one of:

- `BuilderEvent::FullRender { tree: ComponentTree }` — replace the
  whole pane.
- `BuilderEvent::Patch { target_component_id, tree: ComponentTree }`
  — replace one subtree (same shape as SDUI `/ui/action`'s `patch`
  response per [sdui R5](../sdui/SCOPE.md#r5--one-action-endpoint-discriminated-response)).
- `BuilderEvent::TokenPatch { mode: "light"|"dark", keys: { ... } }` —
  theme slice only; replaces N theme token values.
- `BuilderEvent::ShellPatch { ... }` — theme slice only; updates the
  `ShellConfig` sidecar.
- `BuilderEvent::Status { phase: "thinking"|"writing"|"done"|"error",
  message? }` — UX-only status frames.

**There is no AI-specific component type.** A component the AI emits
goes through the same JSON Schema validator hand-authored pages go
through. A component the IR doesn't have is a `custom` variant per
sdui R7 — same escape hatch.

The `BuilderEvent` enum itself lives in `starter-flow-node-ai-builder`
(it's a stream-control envelope, not a UI primitive). The payloads
inside it are `starter-ui-ir` types verbatim. The wire JSON shape
is the contract; consumers' TS code generates its types from a
`BuilderEvent.schema.json` the Rust crate emits at build time.

**Patch ordering on the client.** SDUI's `applyPatch` assumes the
target component already exists in the rendered tree (it's a
"find by id, replace subtree" operation). During AI streaming,
that assumption can be violated if a `Patch` arrives before the
`FullRender` that introduces its `target_component_id` — possible
on reconnect, on slow renders, or if the agent gets clever about
ordering. The contract:

- The client's event loop **buffers `Patch` events whose
  `target_component_id` is not present in the current tree** for up
  to 2 seconds, replaying them once a matching `FullRender` (or a
  prior `Patch` introducing the parent) lands.
- After 2 seconds the buffered `Patch` is dropped and a structured
  warning is logged. The tree continues rendering whatever did
  arrive.
- The server **does not need to guarantee `FullRender`-before-`Patch`
  ordering**, only that every `Patch` is *eventually* preceded by a
  resolvable parent within the buffer window. This is the stream
  invariant ai-builder adds on top of SDUI's `applyPatch`.

### R2 — Every LLM call routes through `AiRunner`

Inherited from agent R2. `starter-flow-node-ai-builder`'s body
constructs `AiRunner` input from the user message + flow input slots
+ the assembled system prompt, then parses the runner's token stream
into `BuilderEvent`s. **No provider SDK appears in the dep tree.**
Same CI check that protects `starter-flow-node-loop` applies:
`cargo tree -p starter-flow-node-ai-builder --edges normal` must not
contain `async-openai`, `anthropic-ai-sdk`, `anthropic-sdk`,
`google-genai`, `aws-sdk-bedrockruntime`, `mistralai`, or `ollama-rs`.

### R3 — Push without a chat round-trip is a slot write, not a new channel

The "backend emits a card without the user typing" path the prior
`ai-ui` prototype solved with a bespoke `tokio::sync::broadcast` is
satisfied here by **another node writing to the ai-builder node's
output slot**. The flow engine already propagates slot writes to
downstream subscribers (flow R8 — one write chokepoint). The SSE
endpoint a React surface subscribes to is whatever `FlowAsService`
exposes for that flow.

A new push primitive in `ai-builder` is a bug. Adding a second event
bus in this layer breaks the engine's single-write-chokepoint
guarantee.

### R4 — The prompt is the IR vocabulary + the binding grammar + skills

The system prompt is assembled from five inputs:

```
[base SDUI authoring guide: when to emit what variant]
[SDUI IR JSON Schema: every component variant + props + version]
[binding-grammar primer: $target / $stack / $user / $page / $self,
                        with the / child-walk and . slot-read rules]
[entity-graph summary: the kinds the agent can target, with slot lists]
[theme token schema: 38 keys + valid CSS value shapes — theme slice only]
[selected SKILL.md bodies, alphabetical by id]
```

**No runtime templating** in any of the five inputs (per agent R4 —
anti-prompt-injection).

**Cache invalidation triggers, normative.** The prompt is content-
hashed; a cache hit only occurs if every input below is unchanged.
The triggers are:

| Input | Re-assembled on |
|---|---|
| Base authoring guide | Crate version bump (compiled in). |
| IR JSON Schema | `starter-ui-ir` version bump (compiled in). |
| Binding-grammar primer | `starter-ui-bindings` version bump (compiled in). |
| Entity-graph summary | **Schema change to a kind: re-assembled. Instance add/remove/slot-value change: NOT re-assembled.** The summary is *kind metadata*, not entity inventory. |
| Theme token schema | `starter-ui-kit` token schema bump (compiled in). |
| Skill bodies | Any `SKILL.md` content-hash change in the active set (per [SKILLS.md R-skills-2](../../agent/SKILLS.md)). |

The entity-summary rule is the load-bearing one and the row most
likely to be misread. The summary teaches the model "kind
`com.acme.sensor` has slots `value`, `units`, `last_seen`." It does
**not** teach the model "there are 1,742 sensors right now." The
former is stable across hours-to-days; the latter changes
constantly and shouldn't move the prompt.

A kind-schema change (slot added, kind registered, kind deprecated)
triggers re-assembly via a hook on the `EntityGraph` trait that the
host exposes. The hook fires on schema mutations only, not on
instance mutations. Without this distinction, "prompt is content-
hashed" would hold only within an unchanged-entity window — i.e.,
never in a live system. (Same class of bug as the SKILLS.md reload
race; same shape of fix.)

The IR JSON Schema is emitted by `starter-ui-ir`'s build step (the
same one SDUI's capability endpoint uses). The entity-graph summary
is generated by the host at flow load (and on the schema-change
hook) via the `EntityGraph` trait
([sdui S-D1](../sdui/SCOPE.md#s-d1--where-the-entity-graph-trait-lives)).

**Single source of truth for the component vocabulary.** When SDUI
adds a `chart` variant, AI knows about it on the next prompt
assembly. When a consumer adds a `custom` renderer-id, the prompt
includes that id and its props schema. No second catalogue to keep
in sync.

### R5 — Theme-builder writes through the existing `ThemeTransport`

The theme-builder slice does not invent its own persistence path. A
`TokenPatch` event is applied to a preview pane immediately; on
"save", the React shell calls the same `ThemeTransport.save(...)`
the human editor uses. The agent never touches storage directly.

Consequence: the **REST contract, the validator, the asset upload
limits, and the auth/admin gating from the theme editor all apply
unchanged** to AI-generated themes. A `url(...)` or `@import`
injected into a token value is rejected by the existing `400`
validator, not by a parallel ai-builder filter.

### R6 — One opinion per React package

`@nube/starter-ai-builder-react` ships the page-builder surface;
`@nube/starter-ai-builder-react-theme` ships the theme-builder
surface. Both depend on `@nube/starter-ui-kit` (shadcn);
page-builder additionally depends on `@nube/starter-sdui-react` (it
reuses the `<Renderer>`); theme-builder additionally depends on
`@nube/starter-ui-core/theme-editor` (it reuses the Zustand store).
A consumer picks one, the other, or both.

### R7 — Hardcoded entity ids in AI output are rejected at the parser

The "AI binds against `$target`" property is enforceable, not just
observable. After `starter-flow-node-ai-builder` parses a
`BuilderEvent::FullRender` or `BuilderEvent::Patch` payload, a
post-parse pass walks every string-valued field in the tree and
checks: if a value matches the host's `EntityId` regex (exposed by
the `EntityGraph` trait) **and** is not wrapped in `{{ ... }}`
binding syntax, the event is rejected with a
`BuilderEventError::HardcodedEntityId { path, value }` and not
written to the output slot.

Concretely:

- `{ "value": "{{$target/temp.value}}" }` — allowed (binding).
- `{ "value": "Building 1" }` — allowed (not an entity id).
- `{ "value": "node-7f8a2c-..." }` where that matches `EntityId`'s
  regex — **rejected**. The agent must rewrite as
  `{{$target.id}}` or `{{$stack.site.id}}` and resubmit.

The error surfaces back into the chat as a structured tool error,
giving the agent one turn to correct. Three consecutive rejections
in one turn fail the run (cost-cap protection).

**Why this rule lives in ai-builder, not SDUI.** Hand-authored and
DSL-authored pages can legitimately reference entity ids — a
consumer pinning a specific dashboard to a specific building is
fine. The "must bind, not hardcode" property is specific to AI
authoring, where hardcoding is the failure mode of "AI generated
50 pages instead of one reusable page."

The host's `EntityId` regex is exposed via the `EntityGraph` trait;
hosts that don't have a stable id format opt out by returning
`None`, in which case this check is skipped and the smoke degrades
to fixture observation.

### R8 — No state, no DB, no auth in ai-builder

State (chat history beyond one turn, generated-page revisions, saved
themes) is the consumer's problem **or** the engine's
(`SessionStore` per agent R-agent-2; `ThemeTransport` per the theme
editor). Auth gating happens at the flow's `FlowAsTool` /
`FlowAsService` boundary via `starter-ext-flow` (per agent
R-agent-4) or the surrounding axum router — never inside the
ai-builder node.

## Authoring flow (page-builder)

```
┌───────────────┐    user prompt    ┌─────────────────────┐
│ React surface │ ────────────────▶ │  flow input slot    │
│ <AiBuilder>   │                   └─────────────────────┘
└───────────────┘                              │
        ▲                                       ▼
        │                            ┌─────────────────────┐
        │  BuilderEvent stream       │ ai-builder node     │
        │  (FullRender | Patch       │  - assemble prompt  │
        │   | Status)                │  - call AiRunner    │
        │                            │  - parse tokens     │
        │                            │  - emit on out slot │
        │                            └─────────────────────┘
        │                                       │
        │                                       ▼
┌───────────────┐                    ┌─────────────────────┐
│ SDUI Renderer │ ◀───── SSE ─────── │ FlowAsService SSE   │
│ <Renderer>    │  (BuilderEvents    │ endpoint            │
│  + applyPatch │   over the wire)   └─────────────────────┘
└───────────────┘
```

The `<Renderer>` doesn't care that the tree came from the AI; it's
the same renderer hand-authored pages go through. `BuilderEvent::Patch`
maps onto SDUI's existing `applyPatch` helper unchanged.
`BuilderEvent::FullRender` maps onto a full re-render. `Status`
frames update the prompt chrome (spinner, "writing..." indicator)
and don't touch the tree.

## Surfaces

### Page-builder — Rust

```rust
let engine = starter_flow::Engine::builder()
    .with_runner(runner)                   // AiRunner of choice
    .with_skills(skills)                   // SkillRegistry
    .with_node_kinds(
        NodeKindRegistry::with_builtins()
            .register(starter_flow_node_ai_builder::node_kind()),
    )
    .with_flows(flows)
    .with_store(starter_store_sqlite::open("data.db")?)
    .build()?;
```

A minimal page-builder flow:

```yaml
# flows/page-builder.yaml
id: com.acme.page-builder
trigger: explicit
nodes:
  - id: builder
    kind: ai-builder
    config:
      session_policy: continue
      slice: page                          # page | theme
      ir_version: 5                        # SDUI IR version to target
      cost_cap_usd: 0.50          # plain YAML number; unit is the key
links: []
```

The flow is automatically exposed as `POST /flows/com.acme.page-builder`
by `starter-server`, as a tool over MCP by `starter-mcp`, and as a
sub-node callable from another flow — all via `FlowAsTool` (flow R9).

### Page-builder — React

```tsx
import { AiBuilder } from "@nube/starter-ai-builder-react";
import { StarterClient } from "@nube/starter-client-ts";

const client = new StarterClient({ baseUrl: "/" });

export function ScopePreview() {
  return (
    <AiBuilder
      flowId="com.acme.page-builder"
      client={client}
      onSave={(tree) => savePageRevision(tree)}
    />
  );
}
```

`<AiBuilder>` owns the SSE connection, the in-flight `ComponentTree`,
and the prompt input chrome. Rendering delegates to
`@nube/starter-sdui-react`'s `<Renderer>` unchanged — the same one
hand-authored pages render through. Component dispatch, action
round-trips, live subscriptions, optimistic patches, and capability
handshake all work without ai-builder touching them.

### Theme-builder — Rust

```yaml
# flows/theme-builder.yaml
id: com.acme.theme-builder
trigger: explicit
nodes:
  - id: builder
    kind: ai-builder
    config:
      session_policy: continue
      slice: theme
      cost_cap_usd: 0.20
links: []
```

The node emits `BuilderEvent::TokenPatch { mode, keys }` and
`BuilderEvent::ShellPatch { ... }` instead of `ComponentTree`
mutations.

### Theme-builder — React

```tsx
import { AiThemeBuilder } from "@nube/starter-ai-builder-react-theme";
import { httpThemeTransport } from "@nube/starter-ui-core/theme-editor";

export function AiThemeRoute() {
  const transport = httpThemeTransport({ client });
  return (
    <AiThemeBuilder
      flowId="com.acme.theme-builder"
      client={client}
      transport={transport}
    />
  );
}
```

`<AiThemeBuilder>` is `<ThemeEditorPage>` plus a prompt input. Token
patches stream into the same Zustand store the human editor mutates,
so undo/redo, dirty flag, contrast badges, and live preview all
work unchanged. "Save" calls `transport.save(...)`.

## Skills for ai-builder

Two reference skills ship in the workspace under
`skills/starter.ai-builder.dashboards/` and
`skills/starter.ai-builder.themes/`. The directory name **must
equal** the `id:` in the frontmatter (per
[SKILLS.md](../../agent/SKILLS.md)'s "Skill bundle layout on disk"
rule); the `starter-skills` loader fails `load_dir` otherwise.
Both follow the `SKILL.md` format from agent SCOPE Part B verbatim.

```markdown
---
id: starter.ai-builder.dashboards
description: |
  Build an IoT / ops dashboard from device telemetry: KPI tiles at top,
  time-series charts in the middle, alert table at the bottom.
allowed_tools: []
trust: approved
---

When the user asks for a dashboard, emit an SDUI ComponentTree:

- Root is `{ "type": "page" }` with a title bound to `{{$target.name}}`.
- Put up to four `kpi` components in a `kpi_grid` at the top.
- Use `chart` (line) for any value over time; `chart` (bar) for grouped
  aggregates. Always source from `{{$target/<child>.<slot>}}` — never
  hardcode a node id.
- Show an `alert` component above the grid when an alarm condition
  binding evaluates truthy.
- Default chart range: last 24h via `$page.chart_range` unless the user
  specifies otherwise.
- Prefer one page that binds against `$target` over many hardcoded
  pages — same renderer, every device.
```

```markdown
---
id: starter.ai-builder.themes
description: |
  Edit shadcn theme tokens. Respect WCAG AA on foreground/background
  pairs. Output OKLCH for all colours.
allowed_tools: []
trust: approved
---

When the user asks to change the theme, emit TokenPatch events:

- Token values are `oklch(L C H)`. Never `hex` or `rgb()`.
- Keep `--background` / `--foreground` contrast ≥ 4.5:1 (WCAG AA body).
- When the user names a brand colour, anchor `--primary` to it and
  derive `--accent`, `--ring`, and `--primary-foreground` from it.
- Never emit `url(...)`, `@import`, or `expression(...)` inside a
  token value — the server validator will 400 the save.
- Emit one TokenPatch per coherent change-set; the user's undo
  granularity is per-event.
```

A consumer that adds their own components ships skills the same
way: drop an `.md` file in their host skills directory; the
registry picks it up; per agent R4 the bundle's content hash
governs trust.

## What does NOT land

- **No second LLM seam.** R2; reuses agent R2.
- **No second push channel.** R3; reuses flow slot writes.
- **No second token model.** R5; theme-builder targets the 38 tokens
  the theme editor already owns.
- **No second UI wire format.** R1; reuses SDUI's `ComponentTree`.
  This is the headline change from the prior `ai-ui` prototype and
  from the initial ai-builder draft (which invented `UiTree` /
  `BuilderEvent` payloads). Having two structured-UI formats in one
  workspace was the wrong call.
- **No upstream OpenUI dependency.** SDUI is the substrate; OpenUI
  is not in the dep graph.
- **No marketplace of generated pages or themes.** Persistence of
  prior generations is the consumer's job (or `SessionStore`'s for
  chat continuity).
- **No agent topology types.** A "planner → renderer" pipeline is a
  sequential flow of two `ai-agent`-style nodes (agent R-agent-1),
  not a `BuilderPipeline` Rust type.
- **No per-component AI overrides in v1.** The agent emits whole
  trees and `Patch` deltas on named subtrees. "Edit just this card"
  is a v2 capability and lands as a richer diff protocol if a
  consumer demands it.
- **No bespoke renderer.** ai-builder reuses SDUI's `<Renderer>`. A
  divergent renderer for AI output would re-introduce the cross-
  authoring-mode parity problem SDUI exists to solve.

## Decisions made

- **Target SDUI's IR, don't invent a wire format.** R1. Hand-
  authored, DSL-authored, and AI-authored trees converge on the
  same `ComponentTree`. The renderer is shared. The binding grammar
  is shared. The action protocol is shared. The DoS limits are
  shared.
- **ai-builder is a flow, not a parallel runtime.** Unlocks
  MCP/REST/CLI/sub-flow surfaces for free via `FlowAsTool` /
  `FlowAsService`.
- **No new provider trait.** `AiRunner` is the seam (R2).
- **No new skill format.** `SKILL.md` is the format (agent R4),
  quarantined by content hash.
- **Page-builder and theme-builder share one node kind.** A `slice:
  page | theme` config flag switches the prompt shape and the
  emitted event variants. Avoids inventing two node kinds whose
  bodies are 90% identical.
- **shadcn-only.** Starter's UI is shadcn (`starter-ui-kit`); SDUI's
  React renderer targets shadcn; ai-builder inherits that.
- **Theme-builder writes through the existing `ThemeTransport`.** R5.
  Reuses the validator, asset limits, auth gating, undo/redo store,
  and contrast badges the human editor already has.

## Open questions

### D1 — Streaming granularity inside one ComponentTree

A FullRender per token is wasteful for a 200-node tree; a Patch per
token is wasteful for a 5-node tree. Two options:

- **(a)** Node body buffers the model's token stream and emits
  `FullRender` on each complete tree, `Patch` on each complete
  sub-tree. Higher perceived latency (user waits for closing braces).
- **(b)** Node body emits a finer-grained `BuilderEvent::Append {
  parent_id, child: ComponentTree }` per completed sub-tree, letting
  the client paint nodes as they arrive. Lower latency; one more
  wire variant.

**(a) is not cheaper to implement than (b).** Both require the same
JSON-streaming parser inside the node body — you can't emit *any*
`BuilderEvent` without recognising tree boundaries in the token
stream. Once that parser exists, the question is purely UX: do we
want users to see a tree assemble incrementally (`Append`) or
appear in chunks (`Patch` / `FullRender`)?

Lean (a) for v1 — fewer wire variants, simpler React side. Revisit
if user-perceived "thinking..." time on large trees is a real
complaint; the cost of moving to (b) is one variant, not a
re-architecture.

### D2 — How the entity-graph summary lands in the prompt

The prompt needs to teach the model what kinds and slots are
queryable so it can author `{{$target/<child>.<slot>}}` bindings
that resolve. Options:

- **(a)** A short JSON summary the host generates from the
  `EntityGraph` trait at flow load.
- **(b)** A tool the agent calls (`describe_kind`, `list_children`)
  to introspect on demand.

(a) is simpler and cache-friendly. (b) scales better for large
graphs but adds per-turn round-trips. Lean (a) for v1; promote to
(b) if a real consumer's graph blows past ~50 KB of summary.

### D3 — Persistence model for AI-generated pages — committed

**Decision: v1 is ephemeral-by-default. Persistence is the
consumer's job, via an `onSave` callback. Same posture as SDUI
hand-authored pages.**

The `<AiBuilder>` `onSave` contract:

| | |
|---|---|
| Fires when | The user clicks "Save" in the surface chrome. Never on every BuilderEvent — that would be a write per token. |
| Payload | `{ tree: ComponentTree, ir_version: number, source_prompt: string, generated_at: ISO8601 }`. The tree is fully-resolved (`Patch`es applied, `Status` frames stripped). |
| Failure mode | If `onSave` throws or returns a rejected promise, the chrome shows the error inline and the tree stays in the editor (not lost). |
| If absent | No save button rendered. The session is generation-only; closing the tab loses the tree. **This is the default.** |

What this means in practice:

- **ScopePreview-style use** ("generate something quickly, look at
  it, throw it away"): drop `<AiBuilder flowId="...">` and never
  pass `onSave`. The tree is ephemeral. This is the default v1
  experience.
- **Iterative authoring** ("spend 20 minutes refining a dashboard,
  then save it"): pass `onSave={(payload) => savePageRevision(payload)}`.
  Persist however the consumer's product persists pages —
  `ui.page` nodes, a CMS, a git-backed config dir, anything. The
  agent doesn't know and doesn't need to.

Why not write into a `ui.page` node directly: the agent doesn't
know the consumer's persistence model (revisions? draft/publish?
ownership? per-user vs shared?). Picking one and hardcoding it
would make ai-builder useful for one consumer and wrong for the
next. Same reason theme-builder writes through `ThemeTransport` and
not directly to the theme store (R5).

Re-opens if a consumer needs an "auto-save on every meaningful
delta" mode. Tractable as a separate `onTreeChange` callback;
deferring until asked.

### D4 — Auth on theme-save under AI control

The human theme editor requires admin role to `PUT
/api/v1/ui/theme`. An AI assistant making the call inherits the
caller's principal, so an admin-driven AI session writes; a
viewer-driven one fails at the route. This is the right default
("adapters apply auth, not extensions", agent R-agent-4), but worth
calling out so it doesn't surprise anyone.

## Smoke tests (before merging)

In addition to the SDUI, flow SCOPE, and agent SCOPE smokes:

### "AI output validates as SDUI IR" test (R1)

A fixture agent transcript (one prompt → BuilderEvent stream) is
replayed through the prompt assembler + a stub `AiRunner` returning
the recorded tokens. Every `BuilderEvent::FullRender` and
`BuilderEvent::Patch` payload round-trips through `starter-ui-ir`'s
JSON Schema validator without error. If a payload validates as
ai-builder-specific but fails SDUI validation, R1 has slipped —
ai-builder is emitting a parallel format.

### "Page-builder over MCP" test

A binary registers a `com.acme.page-builder` flow. A Claude Desktop
client connects to the host's MCP endpoint, calls the flow with a
prompt, and observes `BuilderEvent`s streaming as
`notifications/progress`. The final tree round-trips through SDUI's
`<Renderer>` and renders to expected component types. If any LLM
call bypasses `AiRunner`, R2 has slipped.

### "Theme-builder reuses the editor's transport" test (R5)

`<AiThemeBuilder>` is mounted with an `inMemoryThemeTransport`. The
agent emits a `TokenPatch`. The transport's `save()` is called
exactly once with the patched theme; the undo store contains the
prior state; the contrast badge updates to the new ratio. If the
theme-builder writes to storage through any path other than the
provided transport, R5 has slipped.

### "Push without chat" test (R3)

A second node in the flow writes a `BuilderEvent::Patch` to the
ai-builder node's output slot (no model call in the loop). The
React surface receives the event over SSE and the SDUI renderer
applies the patch. If this requires a parallel push channel beyond
slot writes, R3 has slipped.

### "Prompt is content-hashed" test

Two runs with identical SDUI IR schema + entity summary + skills
produce byte-identical assembled system prompts. Editing one byte
in any skill body changes the hash and re-quarantines the skill
(per agent R4).

### "Hardcoded entity ids are rejected at the parser" test (R7)

Two parts, both required:

**Part A — structural.** A stub `AiRunner` returns a payload with
one field value that matches the host's `EntityId` regex literally
(not wrapped in `{{ ... }}`). The node body's post-parse pass
rejects the event with `BuilderEventError::HardcodedEntityId`; the
event never reaches the output slot; the agent receives a
structured tool error naming the path. Inverting it: the same
payload with the value wrapped in `{{ ... }}` parses and emits
successfully.

**Part B — observation.** A fixture entity graph has three sibling
targets of the same kind. A reference skill
(`starter.ai-builder.dashboards`) is selected. The agent's emitted
page renders correctly against all three targets without
re-prompting. Part B alone tests luck; Part A tests structure.
Together they pin both the rule and the outcome.

This is the test that makes the binding-grammar dependency load-
bearing — without R4 of SDUI and R7 here, ai-builder degrades to
per-entity prompting.

### "No provider SDK in dep tree" test

`cargo tree -p starter-flow-node-ai-builder --edges normal` snapshot
test in CI fails if any provider SDK appears. Same enforcement as
agent SCOPE Part B.

## Phasing

Ships as a follow-on to the SDUI port + the flow + agent
capabilities. Each phase is one session.

| # | Phase | Size | Output |
|---|---|---|---|
| 1 | `starter-ai-builder-prompt` | S | Prompt assembler reading SDUI IR JSON Schema + theme token schema + entity-graph summary + skills. Unit-tested in isolation. **Block precision:** depends on the *JSON Schema artifact* `starter-ui-ir` emits — not on the compiled crate. If the schema file is generable from the type definitions before SDUI Phase 1 ships (e.g. frozen as a fixture), this phase can start in parallel. Otherwise, blocked on [SDUI Phase 1](../sdui/SCOPE.md#phasing). |
| 2 | `starter-flow-node-ai-builder` + reference flows | M | Node kind reading prompt + calling `AiRunner` + parsing token stream into `BuilderEvent`s on the output slot. Includes the post-parse hardcoded-id rejection pass (R7) and the JSON-streaming parser (D1). Reference page-builder + theme-builder flows. Smoke: "no provider SDK in dep tree" passes; "AI output validates as SDUI IR" passes; "Hardcoded entity ids rejected at parser" Part A passes. **Block precision:** depends on the *compiled* `starter-ui-ir` crate (for type-checked event payloads) and on the `EntityGraph` trait shape from [SDUI Phase 2](../sdui/SCOPE.md#phasing). Full end-to-end demo (resolve + render + action round-trip) additionally needs [SDUI Phase 5](../sdui/SCOPE.md#phasing). |
| 3 | `@nube/starter-ai-builder-react` | M | `<AiBuilder>` component wrapping `<Renderer>` from `@nube/starter-sdui-react`. SSE wiring feeds `BuilderEvent`s into the renderer's `applyPatch` helpers. Prompt input chrome. Smoke: "AI binds against `$target`" passes against fixture graph. |
| 4 | `@nube/starter-ai-builder-react-theme` | S | `<AiThemeBuilder>` wired to the existing `ThemeTransport`; reuses theme editor's preview, undo store, contrast badges. Smoke: "theme-builder reuses the editor's transport" passes. |
| 5 | Reference skills + end-to-end smokes | S | `skills/starter.ai-builder.dashboards/SKILL.md` and `skills/starter.ai-builder.themes/SKILL.md` ship and load through `starter-skills::SkillRegistry::load_dir(...)` (approved by default per SKILLS.md R-skills-3); all smokes above passing in CI. **Blocked on [SKILLS.md](../../agent/SKILLS.md) Phases 1–4 landing.** |
| 6 | First consumer adopts | M | A starter-based product depends on the published crates + packages; gaps fed back as issues. |

Phases 1–4 are the MVP. Phase 5 closes safety. Phase 6 is adoption.

## Pointers

- **SDUI substrate** (the wire format, renderer, binding grammar,
  action protocol — ai-builder targets all of this):
  [DOCS/frontend/sdui/SCOPE.md](../sdui/SCOPE.md)
- Prior internal prototype that informed this design:
  `/home/user/code/rust/ai-ui/SCOPE.md`
- Flow engine: [DOCS/flow/scope/SCOPE.md](../../flow/scope/SCOPE.md)
- AI agent: [DOCS/agent/SCOPE.md](../../agent/SCOPE.md)
- Skills crate: [DOCS/agent/SKILLS.md](../../agent/SKILLS.md)
- Theme editor (theme-builder's transport):
  [DOCS/frontend/theme/README.md](../theme/README.md)

## Bottom line

**ai-builder is the AI authoring mode for starter's SDUI.** It does
not invent a wire format; it emits `ComponentTree` (and
`TokenPatch`) deltas in SDUI's existing format, against SDUI's
existing binding grammar, through SDUI's existing renderer. The
LLM seam is `AiRunner`. The push channel is a slot write. The
skill format is `SKILL.md`. The theme-builder writes through the
same `ThemeTransport` the human editor uses, inheriting its
validator, asset limits, and auth gating. Hand-authored,
DSL-authored, and AI-authored pages render through one renderer —
there is no second world.
