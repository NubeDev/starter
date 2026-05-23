# MCP-UX — prompts + resources + descriptor authoring contract

> Cites: SCOPE [R12](../../SCOPE.md#r12).

## The three surfaces

Per R12, rubix's MCP server exposes **all three** MCP surfaces, not
just tools:

| Surface | Where it comes from | Why |
|---|---|---|
| **Tools** | every flow in `FlowRegistry` via `FlowAsTool` | what the LLM can *do* |
| **Prompts** | one per bundled flow | discoverable starting points — a user picks a prompt, never types a tool name |
| **Resources** | at least one per goal (`rubix://<goal>/<resource>`) | cheap grounding without burning a tool round-trip |

The starter-mcp work to support prompts + resources is upstream
(see [STARTER-CHANGES.md](./STARTER-CHANGES.md)). Until that lands,
rubix ships tools only and prompts/resources are deferred.

## Resource URI scheme (locked)

`rubix://<goal>/<resource>`. Extensions use their `block.yaml` id:
`rubix://com.acme.foo/<resource>`. Examples:

```
rubix://system/last-check
rubix://dashboard/pages
rubix://flow/registry
rubix://clickhouse/marts
rubix://analytics/last-report
```

Fragmenting the namespace (e.g. `mycorp://`) fails review.

## Tool descriptor — the five-field contract (R12)

Every rubix tool ships a `ToolDescriptor`:

1. **purpose** — one sentence, plain.
2. **when_to_use** — concrete trigger conditions.
3. **when_not_to_use** — the most common misuse.
4. **example** — one realistic input + output, ≤10 lines.
5. **siblings** — near-neighbour tools + the phrase explaining
   when *this* one wins.

### Worked example

```rust
ToolDescriptor {
    purpose: "Return current disk usage for the rubix host.",
    when_to_use: "User asks 'is the disk full?', 'what's our \
        storage like?', or anything about free space.",
    when_not_to_use: "User is asking about a specific database's \
        size — that's rubix.system.db.",
    example: r#"
        in:  {}
        out: { total_gb: 500, used_gb: 380, percent: 76 }
    "#,
    siblings: &[
        SiblingTool { id: "rubix.system.db",
            wins_when: "DB-specific size, not host-level disk." },
        SiblingTool { id: "rubix.system.flow_errors",
            wins_when: "Recent flow failures, not capacity." },
    ],
}
```

### Anti-pattern

```rust
// FAILS REVIEW: every field is trivial.
ToolDescriptor {
    purpose: "Get disk usage.",
    when_to_use: "When you want disk usage.",
    when_not_to_use: "When you don't.",
    example: "n/a",
    siblings: &[],
}
```

## Descriptor calibration test (Phase 1 exit gate)

For each goal, two reviewers are given:
1. A realistic user prompt.
2. The descriptors of every tool in that goal (only — no SCOPE,
   no flow YAML, no other context).

Each picks the tool they think the agent should call.
**Reviewers must agree on at least 80% of prompts.** Disagreement
points at descriptors that fail to disambiguate. Fix them, re-run.

This makes "good descriptor" measurable, not present/absent.

## Descriptors steer both surfaces

Per R12, the same descriptors steer:
- The **bundled rubix agent** (internal flows calling tools).
- **External MCP clients** (Claude Desktop, other rubix instances,
  any MCP client).

The bar is the same. A descriptor good enough for Claude Desktop
is good enough for the internal agent, and vice-versa.

## Cancellation UX

Per R13: a canceled flow returns a localized
`MessageKey::new("rubix.flow.canceled")`, **not** a stack trace.
The MCP client sees a normal tool response carrying that message,
plus an `agent.tool.error` event with `reason: canceled`.
