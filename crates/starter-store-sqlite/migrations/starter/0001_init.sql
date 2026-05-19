-- starter source placeholder. Real starter-owned tables live in the
-- crate that owns them (starter-auth-token, starter-auth-users); this
-- migration only exists so the namespaced runner has something to
-- apply for the `starter` source on first boot.
CREATE TABLE IF NOT EXISTS starter_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
INSERT OR IGNORE INTO starter_meta (key, value) VALUES ('schema', 'starter-v1');
