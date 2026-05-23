# AGENT — how the six goals map onto starter's `ai-agent` node

> **Authoritative spec:** starter's [DOCS/agent/SCOPE.md](../../../DOCS/agent/SCOPE.md).
> Read it first. This doc is the rubix-specific overlay.

## The model

In rubix there is no "agent runtime." Per the SCOPE one-line summary
and R8, the *agent* in rubix is starter's **`ai-agent` node kind**.
Multi-agent orchestration is flow topology. Every rubix goal is a
flow whose root node is an `ai-agent` dispatching tools from the
host's shared `ToolRegistry`, steered by a skill from the host's
shared `SkillRegistry`.

## The six bundled flows

| Goal | Flow id | Skill id | Trigger |
|---|---|---|---|
| 1 — dashboards | `com.rubix.dashboard-assistant` | `com.rubix.dashboard-builder` | explicit |
| 2 — users | `com.rubix.user-admin` | `com.rubix.user-admin` | explicit |
| 3 — flows | `com.rubix.flow-programmer` | `com.rubix.flow-programmer` | explicit |
| 4 — clickhouse | `com.rubix.clickhouse-ruler` | `com.rubix.clickhouse-ruler` | explicit |
| 5 — system | `com.rubix.scheduled-system-check` | `com.rubix.system-checker` | explicit → cron (Phase 4) |
| 6 — analytics | `com.rubix.weekly-report` | `com.rubix.analytics-reporter` | explicit → cron (Phase 4) |

Each lives in [rubix-flows/flows/](../../crates/rubix-flows/flows/) as
a YAML file embedded via `include_dir!`. The matching skill lives
in [rubix-skills/skills/](../../crates/rubix-skills/skills/).

## How the parts compose at boot

The `rubix-agent` binary composes:

```text
NodeKindRegistry  ← starter built-ins (includes ai-agent) + extensions
ToolRegistry      ← rubix-tools + extension-contributed
SkillRegistry     ← rubix-skills (approved) + operator dir + extensions (quarantined)
FlowRegistry      ← rubix-flows + operator dir + extensions

then:
  starter-flow::Engine::builder()
    .with_runner(starter-ai Claude CLI)
    .with_tools(tools)
    .with_skills(skills)
    .with_node_kinds(kinds)
    .with_flows(flows)
    ...
  starter-mcp::mcp_router(engine.as_tool_registry(), ...)  // ← every flow is an MCP tool
  starter-server::ServerBuilder::new(state).merge_router(mcp_router)...
```

Every bundled flow auto-surfaces as an MCP tool via
`FlowAsTool` — see SCOPE R7. No per-flow MCP code.

## What rubix never builds

- A second LLM seam (SCOPE R8 — `AiRunner` only).
- A second tool / skill / flow registry (R7).
- An extension host (`starter-ext-flow` does that — see
  [STARTER-CHANGES.md](./STARTER-CHANGES.md)).
- A scheduler (cron is a flow trigger upstream in `starter-flow`).
