-- 0001_connections.sql — see rubix/docs/design/mobile/local-db.md
-- Saved connections to remote rubix-agent servers. The whole reason the
-- mobile app is a multi-instance client.

CREATE TABLE IF NOT EXISTS connection (
  id              TEXT PRIMARY KEY,
  label           TEXT NOT NULL,
  base_url        TEXT NOT NULL,
  colour          TEXT NOT NULL DEFAULT '',
  created_at      INTEGER NOT NULL,
  last_seen_at    INTEGER,
  agent_version   TEXT,
  notes           TEXT NOT NULL DEFAULT ''
);
-- No UNIQUE on base_url on purpose — operators legitimately add the same
-- URL twice with different labels (tunnels, dev/prod toggle, etc.). The
-- `connection/create.ts` verb warns at the app layer, never rejects.
