-- starter source placeholder. See the sqlite twin for rationale.
CREATE TABLE IF NOT EXISTS starter_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
INSERT INTO starter_meta (key, value) VALUES ('schema', 'starter-v1')
    ON CONFLICT (key) DO NOTHING;
