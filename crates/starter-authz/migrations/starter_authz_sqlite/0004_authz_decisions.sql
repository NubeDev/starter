-- Phase 7c — decision audit log. SCOPE-EXT.md R14.
-- Best-effort append-only table; bounded-channel writer task in
-- `starter_authz::audit::db::DbDecisionSink` is the only intended
-- producer. Reads come from the `/v1/authz/decisions` admin route.

CREATE TABLE IF NOT EXISTS starter_authz_decisions (
    id              TEXT PRIMARY KEY,
    at              TEXT NOT NULL,            -- ISO8601 UTC
    tenant_id       TEXT,
    subject         TEXT NOT NULL,
    principal_role  TEXT NOT NULL,
    action          TEXT NOT NULL,
    kind            TEXT NOT NULL,
    resource_id     TEXT,
    effect          TEXT NOT NULL,            -- allow | deny
    rule_id         TEXT,                     -- Some only when a rule matched
    reason          TEXT                      -- Some only for engine-driven decisions
);

CREATE INDEX IF NOT EXISTS idx_authz_decisions_tenant_at
    ON starter_authz_decisions (tenant_id, at);
CREATE INDEX IF NOT EXISTS idx_authz_decisions_subject_at
    ON starter_authz_decisions (subject, at);
CREATE INDEX IF NOT EXISTS idx_authz_decisions_effect_at
    ON starter_authz_decisions (effect, at);    -- "find recent denies"
CREATE INDEX IF NOT EXISTS idx_authz_decisions_rule_at
    ON starter_authz_decisions (rule_id, at);   -- "which rule fires most"
