-- Phase 7d.2 — adapter surface label on decision audit rows.
-- SCOPE-EXT.md §5: dashboards must be able to distinguish where a
-- deny originated (REST vs MCP vs gRPC). Nullable so rows produced
-- by direct in-process engine calls (background jobs, tests) stay
-- valid and pre-7d.2 rows survive the migration unchanged.

ALTER TABLE starter_authz_decisions
    ADD COLUMN surface TEXT;
