-- 0003_per_connection_state.sql — see rubix/docs/scope/mobile/LOCAL-DB.md
-- Per-connection resume state: where the operator was when they last
-- left this server. Cascade-deleted when the connection is removed.

CREATE TABLE IF NOT EXISTS connection_state (
  connection_id        TEXT PRIMARY KEY
                       REFERENCES connection(id) ON DELETE CASCADE,
  last_opened_page_ref TEXT,
  last_synced_at       INTEGER
);
