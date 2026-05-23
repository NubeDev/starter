# flow-agent

A self-contained example that wires the starter workspace's flow
engine and AI runner into a single product: a visual flow editor and
an AI chat surface, where the agent can invoke flows as tools.

> **No auth.** This example binds to `127.0.0.1` and trusts the
> OS-level boundary. Do not expose it to a public interface.

## What's in here

- **Backend** — a single Cargo binary (`flow-agent`) built on
  [`starter-server`](../../crates/starter-server),
  [`starter-flow`](../../crates/starter-flow), and
  [`starter-ai`](../../crates/starter-ai). REST + SSE only, Postgres
  storage (see [ADR-001](../../DOCS/storage/ADR-001-flow-agent-postgres-only.md)
  for why this example dropped SQLite).
- **Frontend** — a Vite + React SPA built against
  [`@nube/starter-ui-flow`](../../packages/starter-ui-flow),
  [`@nube/starter-ui-chat`](../../packages/starter-ui-chat), and
  [`@nube/starter-ui-kit`](../../packages/starter-ui-kit).

See [`SCOPE.md`](./SCOPE.md) for the surface design and the F6 visual
checklist.

## Run it locally

### 1. Backend

Start a local Postgres first (see [Database](#database) below), then:

```bash
export DATABASE_URL=postgres://flow_agent:flow_agent@localhost:5432/flow_agent
cargo run -p flow-agent
```

The server binds to `http://127.0.0.1:8080` by default. Migrations
run automatically on boot.

In release mode the binary serves the prebuilt SPA from
`frontend/dist`. In development you want the Vite dev server instead:

### 2. Frontend (dev)

```bash
pnpm install                                # once at the workspace root
pnpm --filter flow-agent-frontend dev
```

The Vite dev server runs on `http://127.0.0.1:5173` and proxies
`/api/*` to the backend.

### 3. Build the SPA for release

```bash
pnpm --filter flow-agent-frontend build
```

Then re-run the backend in release mode (`cargo run -p flow-agent
--release`) — it will serve the built assets.

## Database

flow-agent is **Postgres-only**. The minimal local setup:

```bash
docker run -d --name flow-agent-pg \
  -e POSTGRES_USER=flow_agent \
  -e POSTGRES_PASSWORD=flow_agent \
  -e POSTGRES_DB=flow_agent \
  -p 5432:5432 \
  postgres:16-alpine
```

Configuration:

- `DATABASE_URL` — full Postgres connection string. Required. No
  hidden default — boot fails fast if unset.

**Running on a Raspberry Pi.** Postgres works fine on a Pi 4/5 with
4 GB+ of RAM, but **use SSD/USB3 storage, not an SD card**. PG's WAL
fsync pattern wears SD cards and is dramatically slower on them.
Tune `shared_buffers=128MB`, `work_mem=4MB`, `max_connections=20`
for a small footprint.

See [ADR-001](../../DOCS/storage/ADR-001-flow-agent-postgres-only.md)
for the reasoning behind dropping SQLite from this example.

## Providers

The agent chat needs at least one provider:

- **Claude CLI** — install [the Claude CLI](https://docs.anthropic.com/)
  and sign in. The example detects an active session automatically.
- **`ANTHROPIC_API_KEY`** — export the key before running the server
  to use Anthropic's HTTP API directly.
- **`OPENAI_API_KEY`** — export the key to use OpenAI's HTTP API.

The Settings page surfaces which providers were detected so you can
see why an agent failed to start.

## Demo flow

1. Open `http://127.0.0.1:5173`.
2. Create a flow, drop a `trigger.explicit → log` graph, save, hit
   **Run** — watch the SSE overlay drive the canvas.
3. Create an agent (provider `anthropic.claude`), open it, send "say
   hi". Token stream lands live.
4. Wire the same flow as a tool on the agent (set `tools` to
   `["flow:<flow_id>"]`), ask the agent to "run my flow with input
   'hello'". The agent's tool call fires the flow; the run appears in
   the editor's recent runs panel.

## Layout

```
examples/flow-agent/
  Cargo.toml
  README.md            # this file
  SCOPE.md             # design + acceptance criteria
  migrations/          # sqlx migrations
  src/                 # Rust backend
  frontend/            # Vite + React SPA
  tests/               # integration tests
```

## CI checks

```bash
pnpm typecheck
cargo build -p flow-agent
cargo clippy -p flow-agent -- -D warnings
cargo test -p flow-agent
```
