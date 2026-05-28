-- rubix-owned `rubix_teams` table backing
-- `PgTeamAdminStore`. Mirrors `rubix_spi::team::TeamRow`
-- byte-exact:
--
-- - `team_id TEXT PRIMARY KEY` \u{2014} operator-visible stable
--   id, TEXT (not UUID) so bundled / typed ids stay readable.
-- - `name TEXT NOT NULL UNIQUE` \u{2014} the create / rename
--   uniqueness key. Pg enforces; the in-memory fake does the
--   same via a `list().any(...)` scan.
-- - `description TEXT NULL` \u{2014} mirrors `Option<String>`.
-- - `members JSONB NOT NULL DEFAULT '{}'::jsonb` \u{2014}
--   `user_id -> assigned_at_ms` map. Held on the team row
--   (NOT a separate join table) because:
--     1. `TeamReversible` is patch-shaped against the full
--        row; a join table would lose the single-row snapshot
--        property the Reversible relies on and force every
--        `member.assign` / `member.unassign` to write the
--        change AND mutate a sibling table in the same
--        transaction \u{2014} more moving parts, no
--        operator-visible upside.
--     2. Members never exceed Pg's TOAST page in any
--        realistic team (\u{2264} a few thousand entries).
--     3. The verb surface is per-team
--        (`member.assign` / `member.unassign`); we never
--        query "all teams a user belongs to" from a write
--        path, only from the future user-list-by-team report
--        which can scan via `members ? user_id` jsonb
--        operator.
--   NOT NULL + default so `create` can omit the column
--   entirely without surfacing a null-vs-empty-map
--   distinction that would diverge from the in-memory fake
--   (which always inserts `BTreeMap::new()`).
--
-- No FK from `members` keys into `rubix_users.user_id`
-- because:
--   a) JSONB keys can't carry FKs.
--   b) The `rubix.user.delete` verb (when it lands) will
--      walk teams and unassign before deleting; the join
--      table alternative would just push the same logic
--      into a deferred FK that crashes a transaction far
--      from the originating verb.

CREATE TABLE IF NOT EXISTS rubix_teams (
    team_id     TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    members     JSONB NOT NULL DEFAULT '{}'::jsonb
);

-- GIN index on `members` so the future "list teams containing
-- user X" report (when the user-detail page lands) stays
-- sub-millisecond. The cost of the GIN index on inserts is
-- negligible relative to the verb cadence (single-actor
-- operator workflow), and the only alternative \u{2014} a
-- separate join table \u{2014} blows up the snapshot model
-- as discussed above.
CREATE INDEX IF NOT EXISTS rubix_teams_members_gin
    ON rubix_teams USING gin (members);
