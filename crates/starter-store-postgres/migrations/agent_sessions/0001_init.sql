-- DOCS/agent/MEMORY.md Phase M-B — Postgres twin of the
-- agent-session schema. Shape mirrors the SQLite migration
-- under `crates/starter-store-sqlite/migrations/agent_sessions/`
-- one-for-one; only the column types differ (Postgres-native
-- `timestamptz` + `jsonb`).

CREATE TABLE IF NOT EXISTS agent_sessions (
    id            TEXT PRIMARY KEY,                              -- UUIDv7, time-sorted
    kind          TEXT NOT NULL,
    owner         TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    metadata_json JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE IF NOT EXISTS agent_session_turns (
    session_id     TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    seq            INTEGER NOT NULL,
    role           TEXT NOT NULL,
    content_json   JSONB NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1,
    content_bytes  INTEGER NOT NULL,
    tokens_in      INTEGER,
    tokens_out     INTEGER,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (session_id, seq)
);

CREATE INDEX IF NOT EXISTS agent_session_turns_by_seq
    ON agent_session_turns (session_id, seq DESC);

-- Same compound-FK shape as the SQLite migration so the artifact
-- row's back-pointer to its producing turn cascades cleanly when
-- the parent session is dropped. `ON DELETE SET NULL` is fine on
-- Postgres because the FK targets the turns table — when a turn
-- is deleted directly (e.g. by the M-E retention sweeper), the
-- impl pre-nulls `produced_by_seq` to avoid the NOT NULL on
-- `session_id` (same mitigation as the SQLite impl).
CREATE TABLE IF NOT EXISTS agent_session_artifacts (
    session_id      TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    key             TEXT NOT NULL,
    version         INTEGER NOT NULL,
    parent_version  INTEGER,
    value_json      JSONB NOT NULL,
    value_bytes     INTEGER NOT NULL,
    produced_by_seq INTEGER,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (session_id, key, version),
    FOREIGN KEY (session_id, produced_by_seq)
        REFERENCES agent_session_turns(session_id, seq)
        ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS agent_session_artifacts_latest
    ON agent_session_artifacts (session_id, key, version DESC);
