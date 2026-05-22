-- Insights Phase 1 schema: verdict log + tag index.
--
-- Per DOCS/Insights/SCOPE.md Phase 1: verdict log + tag index only,
-- no rollups, no derivation cache. Those tables land in Phase 2+.
--
-- Tag index implements the R-ins-8 frontend filter contract:
-- `verdict_tag(verdict_id, key, value)` with a composite index.
-- A bare-flag tag is stored with `value = NULL`; a `key:value` tag
-- carries its value.

CREATE TABLE IF NOT EXISTS verdict_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_namespace  TEXT    NOT NULL,
    rule_name       TEXT    NOT NULL,
    rule_major      INTEGER NOT NULL,
    at_ms           INTEGER NOT NULL,
    severity        TEXT    NOT NULL,
    summary         TEXT    NOT NULL,
    body_json       TEXT    NOT NULL
);

-- `(rule_id, at)` indexed per the SCOPE materialisation contract.
CREATE INDEX IF NOT EXISTS idx_verdict_log_rule_at
    ON verdict_log (rule_namespace, rule_name, rule_major, at_ms);

CREATE TABLE IF NOT EXISTS verdict_tag (
    verdict_id  INTEGER NOT NULL,
    key         TEXT    NOT NULL,
    value       TEXT,
    FOREIGN KEY (verdict_id) REFERENCES verdict_log (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_verdict_tag_key_value
    ON verdict_tag (key, value);

CREATE INDEX IF NOT EXISTS idx_verdict_tag_verdict
    ON verdict_tag (verdict_id);
