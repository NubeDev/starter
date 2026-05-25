-- 0002_active_connection.sql — see rubix/docs/design/mobile/local-db.md
-- One-row-per-key bag for app-wide singletons. Today it carries exactly
-- one key: 'active_connection_id'. If it grows past five keys, promote
-- each to its own table per LOCAL-DB.md.

CREATE TABLE IF NOT EXISTS app_state (
  k TEXT PRIMARY KEY,
  v TEXT NOT NULL
);

INSERT OR IGNORE INTO app_state (k, v) VALUES ('active_connection_id', '');
