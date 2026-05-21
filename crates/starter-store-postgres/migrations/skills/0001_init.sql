-- Skill bundle approval store (Phase 5, R-skills-7).
--
-- Postgres twin of the SQLite migration. See the SQLite file for
-- the column rationale. The only schema difference is the
-- `approved_at` column type: Postgres has a real timestamp type
-- (`TIMESTAMPTZ`), so we use it. Conversion to/from the Rust-side
-- `approved_at_unix_ms: u64` happens in the store impl using
-- `to_timestamp(...)` / `EXTRACT(EPOCH FROM ...)` so this crate
-- does not need a `chrono`/`time` dependency.
CREATE TABLE IF NOT EXISTS skill_approvals (
    skill_id    TEXT        NOT NULL,
    hash        TEXT        NOT NULL,
    approved_at TIMESTAMPTZ NOT NULL,
    approved_by TEXT        NOT NULL,
    PRIMARY KEY (skill_id, hash)
);
