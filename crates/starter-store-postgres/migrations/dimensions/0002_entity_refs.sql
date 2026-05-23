-- Warehouse SCOPE W6: refs are the source of truth in Postgres.
-- Both directions FK CASCADE so an entity delete prunes all incident
-- refs. The composite PK doubles as the natural uniqueness constraint.

CREATE TABLE IF NOT EXISTS entity_refs (
    from_id  TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    rel      TEXT NOT NULL,
    to_id    TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    PRIMARY KEY (from_id, rel, to_id)
);

CREATE INDEX IF NOT EXISTS entity_refs_to
    ON entity_refs (to_id, rel);
