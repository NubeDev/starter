-- starter-prefs Phase 1 schema — Postgres dialect.
--
-- Direct rewrite of migrations/0001_starter_prefs.sql (the SQLite
-- migration) for Postgres.  The schema is identical in shape; the
-- only dialect differences are:
--
--   • updated_at uses BIGINT (64-bit) instead of INTEGER.
--     SQLite's INTEGER affinity is already 64-bit so there is no
--     semantic change — we are merely making the type explicit in
--     Postgres, as the SQLite migration comment already called out
--     ("the Postgres mirror will use BIGINT").
--
--   • All pref columns stay TEXT / TEXT NULL — same semantics as
--     SQLite's TEXT affinity.  NULL on disk ↔ None in Rust per the
--     three-layer resolver contract (R3).
--
-- No SQLite-specific tricks (rowid, INTEGER affinity quirks) are
-- present in the original schema, so there is nothing extra to
-- handle here.

CREATE TABLE IF NOT EXISTS starter_prefs_org (
    workspace_id     TEXT PRIMARY KEY,
    timezone         TEXT,
    locale           TEXT,
    language         TEXT,
    unit_system      TEXT,
    temperature_unit TEXT,
    pressure_unit    TEXT,
    speed_unit       TEXT,
    length_unit      TEXT,
    mass_unit        TEXT,
    date_format      TEXT,
    time_format      TEXT,
    week_start       TEXT,
    number_format    TEXT,
    currency         TEXT,
    updated_at       BIGINT
);

CREATE TABLE IF NOT EXISTS starter_prefs_user (
    user_id          TEXT        NOT NULL,
    workspace_id     TEXT        NOT NULL,
    timezone         TEXT,
    locale           TEXT,
    language         TEXT,
    unit_system      TEXT,
    temperature_unit TEXT,
    pressure_unit    TEXT,
    speed_unit       TEXT,
    length_unit      TEXT,
    mass_unit        TEXT,
    date_format      TEXT,
    time_format      TEXT,
    week_start       TEXT,
    number_format    TEXT,
    currency         TEXT,
    theme            TEXT,
    updated_at       BIGINT,
    PRIMARY KEY (user_id, workspace_id)
);
