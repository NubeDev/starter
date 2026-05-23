# TOOLS — how to add a rubix tool

> Cites: SCOPE [R1](../../SCOPE.md#r1), [R5](../../SCOPE.md#r5),
> [R6](../../SCOPE.md#r6), [R12](../../SCOPE.md#r12).

## What a "rubix tool" is

A type implementing `starter_spi::Tool`, registered into the host's
shared `ToolRegistry` so the `ai-agent` node can dispatch it. Per
R7, rubix tools, extension-contributed tools, and (rarely) operator-
contributed tools all flow into the **same** registry.

## File layout

Per R1, one tool per file under the matching goal:

```
rubix-tools/src/
├── system/
│   ├── mod.rs              re-exports only
│   ├── disk.rs             ← one tool
│   ├── db.rs
│   └── flow_errors.rs
├── user/...
└── ...
```

Per R5, the tool file contains **dispatch logic only**. Its REST
DTO, proto message, and `ToolDescriptor` live in `rubix-spi`
(`rubix-spi::dto::<goal>::...` for the wire types,
`rubix-spi::descriptor::ToolDescriptor` for the metadata).

## The five-field descriptor (R12)

Every tool ships a `ToolDescriptor` with:

1. `purpose` — one sentence, plain English.
2. `when_to_use` — concrete trigger conditions.
3. `when_not_to_use` — the most common misuse.
4. `example` — one realistic input + output, ≤10 lines.
5. `siblings` — list of near-neighbour tool ids + the phrase
   explaining when *this* tool wins.

A descriptor with empty or one-line fields fails review. See the
worked-example bar and the calibration test in [MCP-UX.md](./MCP-UX.md).

## Upstream-first

Per R2, before adding a tool, ask: *would any other starter
consumer want this?* If yes, the tool ships as `starter-tool-*`
(matching the existing `starter-tool-github`, `starter-tool-slack`
pattern). The `starter-tool-sysdiag`, `starter-tool-flow-ops`,
`starter-tool-sdui`, and `starter-tool-clickhouse` items in
[STARTER-CHANGES.md](./STARTER-CHANGES.md) are exactly this.

Genuinely rubix-only tools (e.g. `rubix.user.create` because it
consumes rubix's tenant model) live in `rubix-tools`.

## Testing (R10)

Each tool ships:

1. A unit test inline (`#[cfg(test)] mod tests`) for pure logic.
2. An integration test under `tests/<tool_name>_test.rs` that
   round-trips through the `ai-agent` node loop using the
   recorded-LLM-response harness (no live LLM in CI — see
   [STARTER-CHANGES.md](./STARTER-CHANGES.md)).
