-- starter-agent MEMORY.md M2 schema — agent sessions, turns, and
-- versioned artifacts.
--
-- Distinct from the existing `sessions` table (which is the opaque
-- key/value `SessionStore` from starter-flow-spi::flow). This is the
-- richer "store everything, replay selectively" substrate that the
-- ai-agent loop and the page builder use.
--
-- Forward-only: same convention as `0001_init.sql`. No destructive
-- rewrites of shipped rows; future shape changes land as a new
-- migration that reads from the prior shape.

CREATE TABLE IF NOT EXISTS agent_sessions (
    id           TEXT PRIMARY KEY,                            -- UUIDv7, time-sorted
    kind         TEXT NOT NULL,                               -- "page-builder", "chat", ...
    owner        TEXT NOT NULL,                               -- principal subject; "system" for unowned
    created_at   TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

-- Append-only audit log of every turn (M5). The store assigns `seq`
-- transactionally; callers never compute it client-side.
--
-- `content_bytes` is the serialised JSON length of `content`,
-- recorded at write time so analytics and the cap-enforcement layer
-- don't need to re-serialise on read. `schema_version` is the
-- payload-shape version (M2); readers handle every version they
-- know about, writers always write current.
CREATE TABLE IF NOT EXISTS agent_session_turns (
    session_id     TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    seq            INTEGER NOT NULL,                          -- monotonic per session
    role           TEXT NOT NULL,                             -- "user" | "assistant" | "tool"
    content_json   TEXT NOT NULL,                             -- normalised turn payload
    schema_version INTEGER NOT NULL DEFAULT 1,
    content_bytes  INTEGER NOT NULL,                          -- length of content_json
    tokens_in      INTEGER,                                   -- nullable; CLI runners often don't report
    tokens_out     INTEGER,
    created_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (session_id, seq)
);

CREATE INDEX IF NOT EXISTS agent_session_turns_by_seq
    ON agent_session_turns (session_id, seq DESC);

-- Versioned snapshot log of named state (M2 / M5). One row per
-- `(session, key, version)`; "latest" is `ORDER BY version DESC`.
-- `parent_version` captures undo/branching lineage; the store records
-- the graph, the surface decides what it means.
--
-- `produced_by_seq` ties this version back to the turn that wrote it,
-- when written through `append_turn_with_artifacts`. Surface-initiated
-- writes via `put_artifact_direct` leave it NULL.
--
-- The FK references the turns table's compound key so a turn that
-- gets pruned via session DELETE also cascades through these rows.
CREATE TABLE IF NOT EXISTS agent_session_artifacts (
    session_id      TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    key             TEXT NOT NULL,
    version         INTEGER NOT NULL,                         -- monotonic per (session, key)
    parent_version  INTEGER,                                  -- lineage; NULL for v1
    value_json      TEXT NOT NULL,
    value_bytes     INTEGER NOT NULL,
    produced_by_seq INTEGER,                                  -- turn seq that produced this; NULL for direct writes
    updated_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (session_id, key, version),
    FOREIGN KEY (session_id, produced_by_seq)
        REFERENCES agent_session_turns(session_id, seq)
        ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS agent_session_artifacts_latest
    ON agent_session_artifacts (session_id, key, version DESC);
