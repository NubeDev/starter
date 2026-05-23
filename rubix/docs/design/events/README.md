# EVENTS — the typed agent SSE event taxonomy

> Cites: SCOPE [R13](../../SCOPE.md#r13).

## The taxonomy

| Event | Emitted by | Carries |
|---|---|---|
| `agent.turn.start` | `ai-agent` node | turn id, **skill id** (for observability) |
| `agent.thinking` | `ai-agent` node | partial token stream |
| `agent.tool.start` | `ai-agent` node | tool id, args |
| `agent.tool.complete` | `ai-agent` node | tool id, result, duration |
| `agent.tool.error` | `ai-agent` node | tool id, MessageKey (incl. `SkillForbidden`) |
| `flow.step` | engine | node id, slot writes |
| `slot.write` | engine | path, before, after |
| `skill.match` | `SkillSelector` | skill id, score |
| `progress` | long-running tool | percent (optional) + message_key |

The Rust shape lives in
[rubix-spi/src/events.rs](../../crates/rubix-spi/src/events.rs)
until the upstream taxonomy lands in `starter-flow` (see
[STARTER-CHANGES.md](./STARTER-CHANGES.md)). When the upstream
shape lands, `rubix-spi::events` becomes a re-export.

## Strings are `MessageKey`, not raw text

A tool emitting `"Disk full"` is a bug. The right shape is
`MessageKey::new("rubix.system.disk_full")` + params; the transport
resolves the locale via `starter-i18n`. Anti-i18n-rot.

## Progress cadence

Long-running tools (>2s) emit `progress` at least once every 5
seconds. Otherwise the user sees a hang.

The agent's *between-tool* thinking phase is just as bad as a tool
hang from the user's perspective — `agent.thinking` carries the
token stream and the transport maps it to client motion.

## MCP transport mapping (pinned, R13)

The MCP transport in `rubix-agent` maps:

- `agent.thinking` → `notifications/progress` with
  `progress.message` carrying the partial text.
- `agent.tool.start` → `notifications/progress` with
  `progress.message = "calling <tool>"`.
- `agent.tool.complete` / `agent.tool.error` → progress updates +
  the tool result on the response.
- `progress` events from long tools → `notifications/progress`
  with their percent + message verbatim.

## Cancellation

A `Cancel` token fire produces:
1. `agent.tool.error` with `reason: canceled`.
2. The tool result is a localized
   `MessageKey::new("rubix.flow.canceled")`.
3. No further events for that turn.

Stack traces in user-facing cancellation fail review.

## Skill observability

`agent.turn.start.skill` is what makes "why did the agent do that?"
answerable. Grep traces by turn id → see the skill → read the
skill body. Without this field, every agent action is opaque.
