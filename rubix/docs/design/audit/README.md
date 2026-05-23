# AUDIT — changelog, audit, and agent-log wiring

## The three layers

Rubix consumes three starter crates that together form the audit
trail. Each layer has a single, narrow responsibility:

| Layer | Crate | Role |
|---|---|---|
| Write side | `starter-changelog` | Append-only envelope. Every write tool emits one row per call. |
| Read projection — humans | `starter-audit` | User-facing audit log. Filters and renders changelog rows for operators. |
| Read projection — agents | `starter-agent-log` | Agent-turn projection. Every `agent.turn.start` SSE event produces one row, carrying the active skill id and the resolved principal. |

The split matters because the questions differ. "Who disabled
user X?" is a `starter-audit` query. "Which skill was steering
the agent when it decided to disable user X?" is a
`starter-agent-log` query. Both projections derive from the same
changelog source so they cannot drift.

## What writes to the changelog

Every rubix tool that mutates state writes one changelog row.
The row carries:

- The actor principal (resolved from the request context).
- The tool id and the verb (e.g. `rubix.user.disable`).
- A structured before/after delta, suitable for `starter-undo`
  to reverse (see [docs/design/undo/](../undo/README.md) once
  that lands).
- The active skill id when the call originated from an agent
  turn (empty when the call came directly via REST/CLI/MCP).

Read tools (everything in `rubix.system.*` and `rubix.analytics.*`)
do **not** write to the changelog. The audit log is not a
request log; for that, see the structured access log emitted by
`starter-server`.

## The agent-log row

Every `agent.turn.start` SSE event (per R13) implicitly produces
one `starter-agent-log` row. The row carries:

- Turn id.
- Flow id and skill id.
- Resolved principal.
- The user-facing prompt that triggered the turn.

When the turn ends, a matching `agent.turn.end` row records the
tools invoked and the outcome. The two rows share the turn id so
a single agent turn can be reconstructed end-to-end.

## Storage

All three layers persist to Postgres (per ADR
[0001-postgres-only](../../adr/0001-postgres-only.md)) via
`starter-changelog-postgres`. The audit and agent-log projections
materialise on read; no separate persistence.

## Migration ordering

Per [docs/design/migrations/](../migrations/README.md):

1. `starter-changelog-postgres` migrations run.
2. `starter-audit` and `starter-agent-log` register their
   read-side views.
3. Rubix's own migrations may then reference the changelog
   tables by name (read-only joins; never an FK).

## Retention

`starter-changelog` is append-only and does not delete. Pruning
policy is operator config, not a rubix decision — see the
`starter-changelog` README. Rubix's only stance: a default
deployment keeps every row indefinitely. Operators with
compliance constraints (GDPR right-to-erasure) configure
redaction at the changelog layer, not in rubix.

## What rubix does NOT build

- **No second audit table.** Every audit trail flows through
  `starter-changelog`. Tool authors do not write to ad-hoc log
  tables.
- **No structured logging into the audit log.** `slog` /
  `tracing` is for operator debugging; the audit log is for
  attribution. They are separate sinks.
- **No "soft delete" pattern.** State changes write a changelog
  row that captures the before-state; undo replays from the
  changelog.

## Cross-references

- [docs/design/auth/](../auth/README.md) — principal resolution
  feeds the actor field.
- [docs/design/migrations/](../migrations/README.md) — boot order
  guarantees the changelog tables exist before rubix tables that
  reference them.
- [docs/design/undo/](../undo/README.md) (when it lands) — undo
  reverses changelog rows; the contract lives there.
