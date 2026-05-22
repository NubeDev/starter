-- Insights Phase 3 schema: derivation cache (tier 3 materialisation).
--
-- A derivation rule whose schema declares `persist: true` writes its
-- Dataset output here, keyed on (rule_namespace, rule_name,
-- rule_major, window_start_ms). Frontends and downstream pipelines
-- read from this table rather than re-deriving.
--
-- Invalidation seams (see crate::cache::DerivationCache):
-- - `invalidate(rule_id)` — admin `cache.invalidate` call.
-- - `invalidate_rule_version(ns, name)` — RuleId major bump.
-- "Nothing auto-rewarms" — the next scheduled tick repopulates.

CREATE TABLE IF NOT EXISTS derivation_cache (
    rule_namespace  TEXT    NOT NULL,
    rule_name       TEXT    NOT NULL,
    rule_major      INTEGER NOT NULL,
    window_start_ms INTEGER NOT NULL,
    window_end_ms   INTEGER NOT NULL,
    payload_json    TEXT    NOT NULL,
    written_ms      INTEGER NOT NULL,
    PRIMARY KEY (rule_namespace, rule_name, rule_major, window_start_ms)
);

CREATE INDEX IF NOT EXISTS idx_derivation_cache_rule
    ON derivation_cache (rule_namespace, rule_name, rule_major);
