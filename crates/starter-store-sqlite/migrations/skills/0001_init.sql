-- Skill bundle approval store (Phase 5, R-skills-7).
--
-- One row per `(skill_id, bundle_hash)` pair. Append-mostly:
-- `record()` inserts (or refreshes metadata on the same key),
-- `revoke()` deletes the row, and registry drift never touches
-- the table. Schema mirrors `ApprovalRow` in `starter-skills`:
--
--   * skill_id    — reverse-DNS id, free-form TEXT (validation
--                   lives in `SkillId::new`, not the column).
--   * hash        — content hash from `hash_bundle` (hex, 64
--                   chars today; left untyped so a future digest
--                   change does not need a column migration).
--   * approved_at — Unix milliseconds at the time of approval
--                   (INTEGER per stage spec; SQLite has no
--                   timestamp type).
--   * approved_by — free-form principal identifier for audit.
--
-- The primary key is the `(skill_id, hash)` pair the registry's
-- trust matrix keys off; no extra index needed.
CREATE TABLE IF NOT EXISTS skill_approvals (
    skill_id    TEXT    NOT NULL,
    hash        TEXT    NOT NULL,
    approved_at INTEGER NOT NULL,
    approved_by TEXT    NOT NULL,
    PRIMARY KEY (skill_id, hash)
);
