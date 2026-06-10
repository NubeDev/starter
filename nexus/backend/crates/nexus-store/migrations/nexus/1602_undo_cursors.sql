-- Per-actor redo cursor (WS-12) — backs `POST /v1/undo|redo`.
--
-- Ports the `starter-undo` Postgres schema
-- (crates/starter-undo/migrations/postgres/0001_init.sql) verbatim so the reused
-- `starter_undo::cursor_postgres::PgUndoCursor` finds its table, and adds the
-- GRANT the nexus non-owner runtime role needs (the starter migration assumes the
-- connecting role owns the table; nexus migrates as an owner and runs as
-- `nexus_runtime`).
--
-- The redo stack targets the authenticated principal (`actor_key`), not a tenant:
-- undo is per-actor by design (a crate non-goal is cross-actor "global undo").
-- The table is therefore NOT RLS-bound — `PgUndoCursor` runs its CAS-on-epoch
-- writes directly on the pool, like the WS-11 prefs store. A user belongs to one
-- tenant, so `actor_key` (user:subject) does not collide across tenants in
-- practice; full per-tenant cursor partitioning is a fast-follow if needed.

CREATE TABLE IF NOT EXISTS starter_undo_cursors (
    actor_key  text        PRIMARY KEY,
    redo_stack jsonb       NOT NULL DEFAULT '[]'::jsonb,
    epoch      bigint      NOT NULL DEFAULT 0,
    updated_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_starter_undo_cursors_updated_at
    ON starter_undo_cursors (updated_at);

GRANT SELECT, INSERT, UPDATE, DELETE ON starter_undo_cursors TO nexus_runtime;
