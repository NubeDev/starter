# FLOWS — how the bundled flows compose tools and the `ai-agent`

> **Authoritative flow YAML format:** starter's flow SCOPE
> (`DOCS/flow/scope/SCOPE.md`). This doc covers rubix bundling
> patterns only.
>
> Cites: SCOPE [R7](../../SCOPE.md#r7).

## Where bundled flows live

[rubix-flows/flows/](../../../crates/rubix-flows/flows/) — one YAML per
goal, embedded via `include_dir!`. The `rubix-flows` crate also owns
the loader that converts each YAML into the typed
[`starter_flow::definition::body::FlowBody`](../../../../crates/starter-flow/src/definition/body.rs)
the host's `FlowRegistry` accepts.

## The contract: `rubix_flows::load_all()`

The single entry point the agent binary calls at boot:

```rust
let triples: Vec<(FlowId, FlowRevisionId, FlowBody)> =
    rubix_flows::load_all()?;
for (flow_id, revision, body) in triples {
    flow_registry.register(spec_for(flow_id, revision, body), &kinds).await?;
}
```

`load_all` walks every `*.yaml` under `flows/`, deserialises each
into the surface [`RubixFlowYaml`](../../../crates/rubix-flows/src/load.rs)
shape, and converts it into the typed body. No hand-rolled flow
bodies live in `rubix-agent` — the YAML is the source of truth.

## The default shape

Every bundled rubix flow is a single-node flow rooted at `ai-agent`:

```yaml
id: com.rubix.<goal>
description: |
  One-paragraph description (shows up in MCP tool catalogue).
trigger: explicit              # cron added Phase 4 for goals 5+6
nodes:
  - id: agent
    kind: ai-agent
    config:
      session_policy: continue # or 'fresh' for system-check
      skill_hint: com.rubix.<goal>
      cost_cap: 0.50_usd
links: []
```

The loader rewrites `kind: ai-agent` to the registered reverse-DNS
id (`com.rubix.ai-agent` today; replaced by the
`starter-flow-node-loop::KIND_ID` constant once Block B lands) and
prefixes short node ids (`agent`, `check`, …) with `com.rubix.` so
they satisfy the [`NodeId`](../../../../crates/starter-flow-spi/src/node.rs)
reverse-DNS shape.

Multi-node flows are allowed (review/refine pipelines, parallel
fan-out) but every bundled flow stays single-node until a real
need surfaces.

## Automatic MCP exposure

Per SCOPE R7 and starter's agent SCOPE, every flow registered on
the `FlowRegistry` auto-surfaces as a callable MCP tool via
`FlowAsTool::from_registry`. **Rubix writes zero MCP-per-flow
code.** A contributor adding a seventh flow drops one YAML under
`flows/`; the boot log goes from `mcp_tools=6` to `mcp_tools=7` on
the next agent restart.

## The six bundled flows

| Flow id                            | Goal                            |
|------------------------------------|---------------------------------|
| `com.rubix.scheduled-system-check` | Goal 5 — host health watchdog   |
| `com.rubix.weekly-report`          | Goal 6 — periodic analytics     |
| `com.rubix.dashboard-assistant`    | Goal 1 — dashboard authoring    |
| `com.rubix.flow-programmer`        | Goal 2 — flow editing assistant |
| `com.rubix.clickhouse-ruler`       | Goal 3 — ad-hoc analytics       |
| `com.rubix.user-admin`             | Goal 4 — user / tenant admin    |

All six register at boot; all six surface as MCP tools.

## Triggers

| Trigger | Used by | Notes |
|---|---|---|
| `explicit` | Phase 1–3 flows | Called as MCP / REST / gRPC / CLI tool |
| `schedule(cron = "...")` | Goal 5 + 6 (Phase 4+) | Needs `cron-schedule` node kind upstream — see [STARTER-CHANGES.md](./STARTER-CHANGES.md) |
| `event(slot)` | Future | If a goal needs slot-change reactivity |

## Where invocation behaviour comes from

The loader owns YAML → body conversion and boot-time registration
only. The `ai-agent`
[`NodeBehavior`](../../../../crates/starter-flow-spi/src/node.rs)
itself is supplied by the upstream `starter-flow-node-loop` crate
(`AiAgentNode`), which the host wires to a Claude runner. The host
binds that behaviour to the registered `com.rubix.ai-agent` kind so
every bundled flow becomes both a registered MCP tool *and*
invocable end-to-end through the same registry.
