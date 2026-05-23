-- Phase 7d.2 — adapter surface label on decision audit rows.
-- SCOPE-EXT.md §5.

ALTER TABLE starter_authz_decisions
    ADD COLUMN IF NOT EXISTS surface TEXT;
