-- Hot-reload HR3 audit: record the publish source on every flow
-- revision so the revisions table is itself the audit trail
-- (`api | cli | file:<path> | extension:<id>`). Defaulted on
-- existing rows so the forward-only migration policy holds —
-- pre-HR3 revisions report as `"api"` which is the historical
-- truth (REST/programmatic publishes were the only path).

ALTER TABLE flow_revisions
    ADD COLUMN source TEXT NOT NULL DEFAULT 'api';

-- Quick lookup of "every revision authored from a given source"
-- for the operator-facing audit query. Cheap to maintain — the
-- table grows append-only.
CREATE INDEX IF NOT EXISTS flow_revisions_source_idx
    ON flow_revisions (source);
