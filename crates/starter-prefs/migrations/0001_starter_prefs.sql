-- starter-prefs Phase 1 schema.
--
-- Mirrors the SCOPE.md "Preferences model" block verbatim: two
-- sibling tables, one per layer, with all-NULLABLE pref columns on
-- the user-layer so the resolver can apply the R3 "first non-null
-- layer wins" rule per column. Timestamps are UTC epoch
-- milliseconds (INTEGER) per the SCOPE.md "Where time columns store
-- milliseconds" block — SQLite's INTEGER affinity is 64-bit, so no
-- 2038 risk; the Postgres mirror (deferred per Phase 1 decision
-- lock) will use BIGINT.

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
    updated_at       INTEGER
);

CREATE TABLE IF NOT EXISTS starter_prefs_user (
    user_id          TEXT NOT NULL,
    workspace_id     TEXT NOT NULL,
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
    updated_at       INTEGER,
    PRIMARY KEY (user_id, workspace_id)
);
