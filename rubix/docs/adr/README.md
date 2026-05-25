# Architecture Decision Records

One file per decision. Numbered, monotonic, never deleted —
superseded ADRs are marked `Status: superseded by ADR-NNNN`.

| # | Decision |
|---|---|
| [0001](./0001-postgres-only.md) | Postgres only — no SQLite |
| [0002](./0002-backend-only.md) | Backend only — no frontend in this tree *(superseded by 0004)* |
| [0003](./0003-agent-is-starter-ai-agent.md) | The agent is starter's `ai-agent` node kind |
| [0004](./0004-react-native-mobile-app.md) | React Native mobile app reuses the chassis at the kit seam |

## Format

Each ADR has:
- **Status** — proposed / accepted / superseded.
- **Cites** — SCOPE rules or sibling ADRs.
- **Decision** — the actual choice, one paragraph.
- **Context** — what made the decision necessary.
- **Consequences** — what it costs to live with.
- **Alternatives considered** — what was rejected and why.

ADRs are immutable except for status changes. To revisit, write a
new ADR that supersedes the old one.
