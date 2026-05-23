# FLOWS — how the bundled flows compose tools and the `ai-agent`

> **Authoritative flow YAML format:** starter's flow SCOPE
> (`DOCS/flow/scope/SCOPE.md`). This doc covers rubix bundling
> patterns only.
>
> Cites: SCOPE [R7](../../SCOPE.md#r7).

## Where bundled flows live

[rubix-flows/flows/](../../crates/rubix-flows/flows/) — one YAML per
goal, embedded via `include_dir!`. The rubix-agent binary feeds them
into the host's `FlowRegistry` at boot alongside operator-dropped and
extension-contributed flows.

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

Multi-node flows are allowed (review/refine pipelines, parallel
fan-out) but the Phase 1 / Phase 3 / Phase 4 deliverables stay
single-node until a real need surfaces.

## Automatic MCP exposure

Per SCOPE R7 and starter's agent SCOPE, every flow in the
`FlowRegistry` auto-surfaces as a callable MCP tool via
`FlowAsTool`. **Rubix writes zero MCP-per-flow code.** A
contributor adding a seventh flow only adds the YAML; the MCP
endpoint picks it up on rubix-agent restart.

## Triggers

| Trigger | Used by | Notes |
|---|---|---|
| `explicit` | Phase 1–3 flows | Called as MCP / REST / gRPC / CLI tool |
| `schedule(cron = "...")` | Goal 5 + 6 (Phase 4+) | Needs `cron-schedule` node kind upstream — see [STARTER-CHANGES.md](./STARTER-CHANGES.md) |
| `event(slot)` | Future | If a goal needs slot-change reactivity |
