-- starter-flow Phase 3 persistence schema (D-F3.3, D-F3.8, D-F3.9,
-- D-F3.12) — Postgres twin of
-- `crates/starter-store-sqlite/migrations/flow/0001_init.sql`
-- + `0002_revision_source.sql` consolidated into the single
-- baseline migration this crate ships as 0001.
--
-- Dialect translation rules applied uniformly:
--
--   * `TEXT` timestamps   -> `TIMESTAMPTZ NOT NULL DEFAULT NOW()`
--   * JSON-as-`TEXT`      -> `JSONB`
--   * `INTEGER` seq fields used as i64 -> `BIGINT`
--   * `CURRENT_TIMESTAMP` -> `NOW()`
--   * `ON CONFLICT (...) DO UPDATE` keeps explicit conflict targets
--
-- UUIDs and reverse-DNS ids stay as `TEXT` so the Rust binding shape
-- (`Uuid::to_string()`) mirrors the SQLite store one-for-one.

CREATE TABLE IF NOT EXISTS flow_revisions (
    flow_id     TEXT NOT NULL,
    revision_id TEXT NOT NULL,
    body_json   JSONB NOT NULL,
    source      TEXT NOT NULL DEFAULT 'api',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (flow_id, revision_id)
);

CREATE INDEX IF NOT EXISTS flow_revisions_source_idx
    ON flow_revisions (source);

CREATE TABLE IF NOT EXISTS flow_heads (
    flow_id     TEXT PRIMARY KEY,
    revision_id TEXT NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Per-run row. `status` is the coarse RunState (running/paused/...);
-- `outcome_json` is populated by `finish()` and stays NULL while
-- the run is open. The (service_name, dedup_key) pair is written
-- atomically by `start()` when the caller supplies an idempotency
-- key; D-F3.12 race-safety leans on the UNIQUE partial index below.
CREATE TABLE IF NOT EXISTS runs (
    run_id           TEXT PRIMARY KEY,
    flow_revision_id TEXT NOT NULL,
    principal_json   JSONB NOT NULL,
    run_opts_json    JSONB NOT NULL,
    status           TEXT NOT NULL,
    dedup_key        TEXT,
    service_name     TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at      TIMESTAMPTZ,
    outcome_json     JSONB
);

-- Partial UNIQUE so the column is nullable for runs without a
-- dedup key but two runs sharing (service_name, dedup_key) collide.
CREATE UNIQUE INDEX IF NOT EXISTS runs_dedup_uniq
    ON runs (service_name, dedup_key)
    WHERE service_name IS NOT NULL AND dedup_key IS NOT NULL;

CREATE INDEX IF NOT EXISTS runs_open_idx
    ON runs (status)
    WHERE finished_at IS NULL;

-- Append-only checkpoint log; the engine numbers seq monotonically
-- per run (D-F3.9). Pruning per CheckpointRetention happens inside
-- the same transaction as the insert so a crash mid-checkpoint
-- leaves either (a) the prior checkpoint or (b) the new one — never
-- a half-pruned state.
CREATE TABLE IF NOT EXISTS run_checkpoints (
    run_id            TEXT NOT NULL,
    seq               BIGINT NOT NULL,
    run_state_json    JSONB NOT NULL,
    slot_writes_json  JSONB NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (run_id, seq)
);

CREATE TABLE IF NOT EXISTS sessions (
    session_id     TEXT PRIMARY KEY,
    principal_json JSONB NOT NULL,
    body_json      JSONB NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
