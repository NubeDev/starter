-- Tags SCOPE T6 (BI-4): prefix registry.
--
-- A pack owning `energy.*` registers a single row here. `prefix` is
-- the PRIMARY KEY so a second pack claiming the same prefix fails
-- the install transaction with a unique-violation — exactly the
-- "two pack inserts claiming the same prefix" guard tested in
-- `tests/dimensions_prefix.rs`.

CREATE TABLE IF NOT EXISTS tag_prefix_registry (
    prefix       TEXT PRIMARY KEY,
    owner_pack   TEXT NOT NULL,
    description  TEXT,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT tag_prefix_registry_prefix_shape
        CHECK (prefix ~ '^[a-z][a-z0-9_]*\.\*$')
);
