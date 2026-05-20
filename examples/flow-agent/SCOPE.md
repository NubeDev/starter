# flow-agent — Scope

A self-contained example in the `starter` workspace that wires a **flow
editor** and an **AI agent chat** together against the real
`starter-flow` engine and a real `starter-ai` runner. No login, no
CLI, no gRPC. SSE for live state, REST for everything else.

Lives at [`examples/flow-agent/`](.) alongside [`examples/notes/`](../notes/)
and [`examples/minimal/`](../minimal/) and consumes the same starter
crates and TS packages a real product would.

> Parent rules: see the workspace [`SCOPE.md`](../../SCOPE.md). Hard
> rules R1–R10 apply here too — this example does not get to invent
> new patterns; it just demonstrates the existing ones end-to-end.

---

## One-line summary

`flow-agent` is the reference demo that shows how to compose
[`starter-flow`](../../crates/starter-flow), [`starter-flow-nodes`](../../crates/starter-flow-nodes),
[`starter-ai`](../../crates/starter-ai), [`starter-server`](../../crates/starter-server),
[`@nube/starter-ui-flow`](../../packages/starter-ui-flow),
[`@nube/starter-ui-chat`](../../packages/starter-ui-chat),
[`@nube/starter-ui-kit`](../../packages/starter-ui-kit), and
[`@nube/starter-client-ts`](../../packages/starter-client-ts) into one
small product that:

1. Lets the user **build a flow visually** in the browser
   (drag/connect typed nodes from the palette, including an
   `ai-agent` node).
2. Lets the user **chat with an AI agent** that can itself **invoke
   flows as tools** (the agent reads the flow registry and calls them
   over the same REST surface).
3. Streams **live run state** (node status, edge activity, agent
   tokens, tool calls) back to the UI over **SSE**.

Everything else (auth, CLI, persistence beyond SQLite, multi-user,
RBAC) is explicitly out of scope.

---

## Why this exists

`examples/notes` already demonstrates [`flow_demo`](../notes/src/flow_demo.rs)
end-to-end against a fixed three-node topology, but:

- The flow is **hard-coded** in Rust — you can't author one in the UI.
- There's **no chat surface** — `starter-ui-chat` has no example.
- There's **no nested sidebar** — `starter-ui-kit` ships the
  primitives but no realistic consumer pattern.

`flow-agent` is the missing demo that proves the pieces compose into a
visually-edited, agent-driven product without forking starter or
reinventing wiring per project.

---

## Hard rules (load-bearing)

### F1 — Single example, single binary

One Cargo binary (`flow-agent`), one Vite app (`frontend/`). No
sub-crates, no nested workspaces. Matches the
[notes](../notes/Cargo.toml) layout.

### F2 — Reuse starter packages verbatim

No copy-paste of `starter-ui-flow` / `starter-ui-chat` /
`starter-ui-kit` source into the example. The example imports them
via `workspace:*` exactly like a downstream consumer would. If
something is missing in those packages, **fix it upstream and
re-export**; do not patch in the example.

### F3 — SSE + REST only

- **REST** for CRUD on flows, nodes, edges, agents, and conversations
  (OpenAPI-emitted from `starter-server`).
- **SSE** for `GET /api/flows/:id/events` (live run overlay) and
  `POST /api/agents/:id/run` (chat stream — same shape
  `starter-ui-chat`'s `createSseAdapter` already speaks).
- No WebSocket, no gRPC, no MCP transport for this example. (MCP can
  be added later as a separate example without changing this one.)

### F4 — No login, no auth

No `starter-auth-token`, no `starter-auth-users`, no cookies, no
bearer headers, no `Authenticator` wired into the router. The
example binds to `127.0.0.1` by default and trusts the OS-level
boundary. The README must say this in big letters.

### F5 — No CLI

The product is the web UI. No `clap` binary, no `flow-agent run …`
subcommands. The server has exactly one entry: `cargo run -p
flow-agent` boots the HTTP server and serves the SPA from
`frontend/dist` in release builds (Vite dev server in dev).

### F6 — Apple-modern shadcn aesthetic

Visual bar to clear:

- shadcn + Tailwind v4 tokens from [`@nube/starter-ui-kit`](../../packages/starter-ui-kit).
- Generous whitespace, **system font stack** (`-apple-system, BlinkMacSystemFont, "SF Pro", "Inter", …`).
- Soft shadows (`shadow-sm` everywhere, never `shadow-lg`).
- Rounded corners (`rounded-xl` on cards, `rounded-lg` on inputs).
- Subtle borders (`border-border/60`) — no heavy 1px black lines.
- Light/dark/system switch via `starter-ui-kit`'s theme provider.
- Frosted-glass top bar (`backdrop-blur`, `bg-background/70`,
  sticky).
- Motion: 150ms ease-out on hover, 200ms on panel expand. No bounce.

A "looks fine in screenshots, looks great in person" target. See
[F6 acceptance checklist](#f6-acceptance-checklist) at the bottom.

### F7 — Nested sidebar

The left sidebar is the navigation spine. It must:

- Use [`@nube/starter-ui-kit`'s `Sidebar` primitives](../../packages/starter-ui-kit/src/components/ui/sidebar.tsx).
- Render a **nested, expandable tree** for flows and agents — the
  same shape as
  [`rubix-ui-core/src/lib/sidebar`](/home/user/code/rubix-workspace/rubix-ui-core/src/lib/sidebar)
  (read `SidebarNavTree.tsx`, `useNodeSidebarItems.ts`,
  `useAutoExpandActiveRoute.ts`).
- Persist expanded paths + open/collapsed state across refresh per
  [SIDEBAR.md §1–§4](/home/user/code/rubix-workspace/rubix-agent/docs/design/frontend/SIDEBAR.md)
  (cookie for sidebar open/collapsed; localStorage `"fa-ui"` for
  expanded paths and active section).
- Live-update the tree as flows are created/renamed/deleted — driven
  by an SSE subscription to `GET /api/flows/events` (not polling).

Structure:

```
▾ Flows
  ▾ Customer onboarding
    • welcome-email
    • verify-account
  ▸ Daily report
▾ Agents
  • Assistant
  • Notes-bot
▾ Settings
  • Providers
  • Appearance
```

---

## Surfaces

### Backend (Rust)

```
examples/flow-agent/
  Cargo.toml
  README.md
  SCOPE.md                # this file
  migrations/             # sqlx migrations for flows, agents, runs
  src/
    main.rs               # tokio entry, builds AppState, mounts router
    server.rs             # axum app builder; serves SPA + /api
    domain.rs             # FlowService, AgentService — no HTTP, no SQL
    rest.rs               # REST handlers (thin: extract → domain → DTO)
    sse.rs                # SSE handlers for run + agent + sidebar feeds
    store.rs              # sqlx queries against starter-store-sqlite
    flow_engine.rs        # starter-flow wiring; node kind registry
    ai_registry.rs        # starter-ai providers + tool bridge to flows
    migrations.rs         # embed_migrations! against ./migrations
```

Crate deps (additive only — no new starter crates):

```toml
starter-spi          = { path = "../../crates/starter-spi" }
starter-config       = { path = "../../crates/starter-config" }
starter-observability= { path = "../../crates/starter-observability" }
starter-server       = { path = "../../crates/starter-server" }
starter-store-sqlite = { path = "../../crates/starter-store-sqlite" }
starter-flow         = { path = "../../crates/starter-flow" }
starter-flow-nodes   = { path = "../../crates/starter-flow-nodes" }
starter-flow-spi     = { path = "../../crates/starter-flow-spi" }
starter-ai           = { path = "../../crates/starter-ai", features = ["provider-claude", "provider-openai"] }
```

### REST endpoints

| Method | Path                              | Body / Notes                                    |
|--------|-----------------------------------|-------------------------------------------------|
| GET    | `/api/flows`                      | List `FlowSummary[]`                            |
| POST   | `/api/flows`                      | `{ name, description? }` → `FlowSummary`        |
| GET    | `/api/flows/:id`                  | Full `FlowGraph` (nodes + edges + metadata)     |
| PUT    | `/api/flows/:id`                  | Replace `FlowGraph` (optimistic version bump)   |
| DELETE | `/api/flows/:id`                  |                                                  |
| POST   | `/api/flows/:id/fire`             | `{ trigger, payload }` → `{ runId }`            |
| GET    | `/api/flows/:id/runs`             | Recent runs, paged                              |
| GET    | `/api/flows/:id/runs/:runId`      | Full run record (final state)                   |
| GET    | `/api/agents`                     | List `AgentSummary[]`                           |
| POST   | `/api/agents`                     | `{ name, provider, model, systemPrompt, tools }`|
| GET    | `/api/agents/:id`                 | Full agent config                               |
| PUT    | `/api/agents/:id`                 |                                                 |
| DELETE | `/api/agents/:id`                 |                                                 |
| GET    | `/api/agents/:id/conversations`   | Recent conversations                            |
| POST   | `/api/agents/:id/conversations`   | New conversation                                |

OpenAPI is emitted by `starter-server`'s utoipa scaffolding — see
the notes example's [`rest.rs`](../notes/src/rest.rs) for the pattern.
The TS client is regenerated via the workspace
[`pnpm gen:client`](../../packages/starter-client-ts) script.

### SSE endpoints

| Path                                      | Stream payload                                                                                                           |
|-------------------------------------------|--------------------------------------------------------------------------------------------------------------------------|
| `GET /api/flows/events`                   | `flow-created`, `flow-renamed`, `flow-deleted` — drives the sidebar tree.                                                |
| `GET /api/flows/:id/events`               | `node-status`, `edge-active`, `run-started`, `run-finished` — drives `<FlowCanvas overlay={…}>`.                          |
| `POST /api/agents/:id/run` *(SSE response)* | `data: {"type":"text","text":"…"}`, `data: {"type":"tool-call",…}`, `data: {"type":"status",…}`, `data: [DONE]`. Matches `createSseAdapter`'s default parser in [`starter-ui-chat`](../../packages/starter-ui-chat). |

All SSE handlers use [`starter-server`](../../crates/starter-server)'s
`sse::keepalive` helper (15 s heartbeat) and `Cache-Control:
no-store`.

### Frontend (Vite + React)

```
examples/flow-agent/frontend/
  index.html
  package.json
  vite.config.ts
  tsconfig.json
  src/
    main.tsx                 # React root, theme provider, query client
    app.tsx                  # router + Shell
    layout/
      Shell.tsx              # Sidebar + topbar + outlet
      Topbar.tsx             # frosted-glass header, theme toggle
      Sidebar.tsx            # nested FlowsTree + AgentsTree (see F7)
      sidebar/
        FlowsTree.tsx
        AgentsTree.tsx
        useExpandedPaths.ts  # mirrors SIDEBAR.md §2
    pages/
      FlowsList.tsx
      FlowEditor.tsx         # <FlowCanvas /> + node inspector + run panel
      AgentsList.tsx
      AgentChat.tsx          # <Chat /> with createSseAdapter
      Settings.tsx           # provider keys, appearance
    state/
      ui-store.ts            # zustand persisted ("fa-ui")
      flows-store.ts         # react-query over starter-client-ts
      agents-store.ts        # react-query over starter-client-ts
      run-overlay.ts         # SSE subscription → RunOverlay state
    lib/
      sse.ts                 # tiny EventSource helper with reconnect
      api.ts                 # thin wrapper around @nube/starter-client-ts
```

TS deps:

```jsonc
{
  "dependencies": {
    "@nube/starter-client-ts": "workspace:*",
    "@nube/starter-ui-kit":    "workspace:*",
    "@nube/starter-ui-flow":   "workspace:*",
    "@nube/starter-ui-chat":   "workspace:*",
    "@nube/starter-ui-core":   "workspace:*",
    "@tanstack/react-query":   "^5",
    "@xyflow/react":           "^12",
    "react":                   "^18",
    "react-dom":               "^18",
    "react-router-dom":        "^6",
    "zustand":                 "^4"
  }
}
```

---

## The data model

Three tables in SQLite (one set of `_sqlx_migrations` namespaced to
`flow-agent`):

```sql
CREATE TABLE flows (
  id          TEXT PRIMARY KEY,           -- "flow_01H…"
  name        TEXT NOT NULL,
  description TEXT,
  graph_json  TEXT NOT NULL,              -- FlowGraph serialized
  version     INTEGER NOT NULL DEFAULT 1, -- optimistic lock
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL
);

CREATE TABLE agents (
  id            TEXT PRIMARY KEY,
  name          TEXT NOT NULL,
  provider      TEXT NOT NULL,            -- "anthropic.claude", "openai", …
  model         TEXT NOT NULL,
  system_prompt TEXT,
  tools_json    TEXT NOT NULL DEFAULT '[]', -- ["flow:flow_…", …]
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL
);

CREATE TABLE runs (
  id          TEXT PRIMARY KEY,
  flow_id     TEXT NOT NULL REFERENCES flows(id) ON DELETE CASCADE,
  status      TEXT NOT NULL,              -- queued|running|ok|error|cancelled
  started_at  INTEGER NOT NULL,
  finished_at INTEGER,
  trace_json  TEXT                        -- per-node status, errors
);
```

Conversations are **not persisted** in MVP — they live in
react-query cache and reset on reload. (Adding a `conversations`
table is a one-migration follow-up if needed.)

---

## The agent-as-tool bridge

This is the load-bearing trick of the example: an `Agent` can invoke
any `Flow` as a tool.

1. On boot, [`ai_registry.rs`](#) reads all `flows` from the store
   and registers each as an `AiTool`:
   ```rust
   AiTool {
     name:        format!("flow:{}", flow.id),
     description: flow.description.unwrap_or(flow.name),
     input_schema: schema_for_flow_inputs(&flow.graph),
   }
   ```
2. When the agent calls a tool, the registry looks up the flow,
   fires its trigger over the same `POST /api/flows/:id/fire`
   pathway the UI uses, awaits the run, and returns the output node's
   value as the tool result.
3. Flow runs triggered by the agent emit on the same SSE channel —
   so opening that flow in the UI shows the agent's invocation live.

This is the closest a small example can get to demonstrating R3
(transport never contains domain logic) — the agent path and the
UI path collapse onto the same domain function.

---

## Phases

| Phase | Deliverable                                                                                  |
|-------|----------------------------------------------------------------------------------------------|
| 1     | Cargo binary boots, serves empty SPA, REST `/api/flows` CRUD against SQLite.                 |
| 2     | `FlowEditor` renders `<FlowCanvas>` with the built-in node kinds; save round-trips JSON.     |
| 3     | `/fire` runs the flow through `starter-flow`; SSE overlay drives node + edge animation.      |
| 4     | Agents CRUD + `AgentChat` working against `starter-ai` (Claude CLI runner) over SSE.         |
| 5     | Agent-as-tool bridge: flows appear as callable tools in the agent's prompt.                  |
| 6     | Nested sidebar + persistence per [F7](#f7--nested-sidebar) and [SIDEBAR.md].                 |
| 7     | F6 visual pass — fonts, spacing, motion, dark mode.                                          |

Each phase ends with a `cargo test -p flow-agent` green + a manual
smoke note in [`README.md`](#).

---

## Non-goals

- ❌ Authentication or multi-tenant isolation.
- ❌ CLI.
- ❌ Postgres (SQLite only — the pattern transfers).
- ❌ gRPC, WebSocket, MCP transport in this example.
- ❌ Versioned/forked flows, undo history beyond in-session.
- ❌ Custom node-kind authoring UI (kinds are registered in Rust).
- ❌ Cloud deploy story — this is a localhost demo.
- ❌ i18n (English only; `starter-i18n` integration is a separate
  example).

---

## F6 acceptance checklist

A reviewer should be able to tick every box without writing CSS:

- [ ] Top bar is sticky, frosted-glass, 56 px tall.
- [ ] Sidebar collapses to icon-rail at ≤ 1024 px wide.
- [ ] Tree caret rotates 90° with a 150 ms ease-out transition.
- [ ] Selected nav item uses `bg-accent/60` — never solid `bg-primary`.
- [ ] Cards have `rounded-xl`, `shadow-sm`, `border-border/60`.
- [ ] No raw hex colours in any TSX — only Tailwind tokens.
- [ ] Light / dark / system toggle in the top bar; choice persists.
- [ ] `<FlowCanvas>` background uses the kit's `--background`.
- [ ] Chat message bubbles align like macOS Messages (user right,
      assistant left, 70 % max width).
- [ ] Empty states have a centred illustration slot + one-line
      helper + primary action button — no walls of text.

---

## File-size budget

Per workspace rule R1 (≤ 400 lines per file). Expected counts:

| File              | Target |
|-------------------|--------|
| `src/main.rs`     | < 60   |
| `src/server.rs`   | < 200  |
| `src/rest.rs`     | < 300  |
| `src/sse.rs`      | < 250  |
| `src/domain.rs`   | < 350  |
| `src/store.rs`    | < 300  |
| `src/flow_engine.rs` | < 250 |
| `src/ai_registry.rs` | < 250 |
| Any TSX page      | < 300  |

If any file approaches the limit, split by concept (not by `utils`).
