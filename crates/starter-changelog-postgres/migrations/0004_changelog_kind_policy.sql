-- Per-kind retention policy for `starter_changes`.
--
-- Until this migration, `starter_changes` retention was
-- *implicit-unbounded*: no rubix-side sweep, only the operator-driven
-- `Prune` trait. That leaves a footgun — an operator who decides to
-- trim the table for storage reasons can accidentally drop the
-- audit row for "alice demoted bob to reader last year." Once the
-- row is gone, `GET /v1/audit` cannot return it.
--
-- This table makes the intent explicit:
--
-- - A kind with NO row here  → no policy, no automatic prune (today's
--                                behaviour; nothing surprises an
--                                operator who hasn't opted in).
-- - A row with `max_age_days = NULL` → explicit "keep forever." Used
--                                       for security-relevant kinds
--                                       (`user`, `team`) so the audit
--                                       floor is recorded in SQL, not
--                                       in tribal memory.
-- - A row with `max_age_days = N`     → operator opted into bounded
--                                       retention for that kind.
--
-- The sweep helper `starter_changelog_postgres::policy::apply_policy`
-- reads this table and only deletes from kinds whose policy row
-- specifies a finite `max_age_days`. Kinds with no row are skipped
-- entirely — opting in is always explicit.
--
-- See `rubix/docs/proposal/audit-log.md` for the full rationale and
-- the consumer plan (rubix-agent boot sweep, seed migration for
-- `user`/`team`).

CREATE TABLE IF NOT EXISTS changelog_kind_policy (
    resource_kind TEXT        PRIMARY KEY,
    max_age_days  INT,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
