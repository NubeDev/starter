-- Example consumer table. Demonstrates that consumer migrations
-- coexist with starter-owned migrations via the namespaced runner.
CREATE TABLE IF NOT EXISTS app_kv (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
