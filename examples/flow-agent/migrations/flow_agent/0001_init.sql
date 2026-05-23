-- flow-agent schema — Postgres dialect. Three tables: flows, agents,
-- runs. Conversations live in client state for MVP.
--
-- Lands in `_sqlx_migrations_flow_agent` via the
-- starter-store-postgres namespaced migration runner — keeps version
-- numbers separate from any starter-owned tables a future consumer
-- might add.
--
-- Dialect notes (ADR-001 translation rules):
--   * TEXT timestamps  → TIMESTAMPTZ NOT NULL DEFAULT NOW()
--   * JSON-as-TEXT     → JSONB
--   * ?N placeholders  → $N
--   * ON CONFLICT targets are explicit

CREATE TABLE IF NOT EXISTS flows (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    graph_json  JSONB NOT NULL,
    version     BIGINT NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS flows_updated_at_idx ON flows (updated_at DESC);

CREATE TABLE IF NOT EXISTS agents (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    provider      TEXT NOT NULL,
    model         TEXT NOT NULL,
    system_prompt TEXT,
    tools_json    JSONB NOT NULL DEFAULT '[]',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS agents_updated_at_idx ON agents (updated_at DESC);

CREATE TABLE IF NOT EXISTS runs (
    id          TEXT PRIMARY KEY,
    flow_id     TEXT NOT NULL REFERENCES flows(id) ON DELETE CASCADE,
    status      TEXT NOT NULL,
    started_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at TIMESTAMPTZ,
    trace_json  JSONB
);

CREATE INDEX IF NOT EXISTS runs_flow_id_idx    ON runs (flow_id, started_at DESC);
CREATE INDEX IF NOT EXISTS runs_started_at_idx ON runs (started_at DESC);
