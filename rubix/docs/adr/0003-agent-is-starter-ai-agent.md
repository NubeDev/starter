# ADR 0003 — The agent is starter's `ai-agent` node kind

**Status:** accepted, 2026-05-23
**Cites:** [SCOPE R8](../../SCOPE.md#r8),
[starter DOCS/agent/SCOPE.md](../../../DOCS/agent/SCOPE.md)

## Decision

Rubix does **not** build an agent runtime. Every rubix agent is a
**flow rooted at starter's `ai-agent` node kind**. Multi-agent
topology is flow topology. The LLM seam is `AiRunner`.

## Context

- Starter's `DOCS/agent/SCOPE.md` already specifies the `ai-agent`
  node kind as the first-class agent capability: turn-based LLM
  loop with tool dispatch, session continuity, skill binding,
  streaming, and cancellation.
- Codeless already wrote one agent loop (`Runner` trait); rubix
  must not write a third.
- Sequential / parallel / loop / graph agents are *flow shapes*,
  not Rust types. Building these as types would duplicate flow
  engine work with worse ergonomics.

## Consequences

- Rubix ships **tools + skills + flows + bundled MCP catalogue**,
  not a runtime.
- The "make starter better" deliverable (R2) is what justifies
  this ADR — every starter consumer benefits from the shared
  agent runtime.
- Phase 1's first deliverable (`com.rubix.scheduled-system-check`)
  is a one-node flow rooted at `ai-agent`; the binary's job is
  to wire registries and serve transports.

## Alternatives considered

- **A bespoke rubix agent runtime.** Codeless + rubix-old both
  show this drifts toward duplication. Rejected.
- **Wrap `adk-rust` directly.** Starter's D1 leaves room for a
  `starter-flow-node-adk` sibling; rubix takes whatever starter
  picks (rubix has no preference beyond "the simpler one").
