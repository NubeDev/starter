-- starter-flow Phase 3 persistence schema (D-F3.3, D-F3.8, D-F3.9, D-F3.12).
--
-- Forward-only convention: never destructively rewrite shipped rows.
-- Backfills land as a NEW migration that adds a column (or sibling
-- table) and reads from the prior shape; existing rows stay
-- byte-stable so a partial-rollout supervisor that's still on the
-- older binary keeps reading what it wrote.
--
-- All JSON payloads are SPI types serialized with `serde_json` —
-- the store treats them as opaque blobs (R6) and only the engine
-- crate is allowed to peek inside.

CREATE TABLE IF NOT EXISTS flow_revisions (
    flow_id     TEXT NOT NULL,
    revision_id TEXT NOT NULL,
    body_json   TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (flow_id, revision_id)
);

CREATE TABLE IF NOT EXISTS flow_heads (
    flow_id     TEXT PRIMARY KEY,
    revision_id TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Per-run row. `status` is the coarse RunState (running/paused/...);
-- `outcome_json` is populated by `finish()` and stays NULL while
-- the run is open. The (service_name, dedup_key) pair is written
-- atomically by `start()` when the caller supplies an idempotency
-- key; D-F3.12 race-safety leans on the UNIQUE partial index below.
CREATE TABLE IF NOT EXISTS runs (
    run_id           TEXT PRIMARY KEY,
    flow_revision_id TEXT NOT NULL,
    principal_json   TEXT NOT NULL,
    run_opts_json    TEXT NOT NULL,
    status           TEXT NOT NULL,
    dedup_key        TEXT,
    service_name     TEXT,
    created_at       TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at      TEXT,
    outcome_json     TEXT
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
    seq               INTEGER NOT NULL,
    run_state_json    TEXT NOT NULL,
    slot_writes_json  TEXT NOT NULL,
    created_at        TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (run_id, seq)
);

CREATE TABLE IF NOT EXISTS sessions (
    session_id     TEXT PRIMARY KEY,
    principal_json TEXT NOT NULL,
    body_json      TEXT NOT NULL,
    created_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
