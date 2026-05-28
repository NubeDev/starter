-- Drop `tenant` from the undo_snapshots.resource_kind CHECK constraint.
--
-- The `tenant` kind was reserved during the initial undo design but
-- never received a `Reversible` impl. Tenant create / delete is a
-- rare, operator-driven, high-blast-radius op: recovery from an
-- accidental tenant deletion needs restore-from-backup plus
-- audit-log replay, not a `rubix.undo.last` walk. Carrying the kind
-- in the CHECK keeps the surface ambiguous (the catalogue claims
-- coverage that does not exist) and tempts future contributors to
-- add an impl whose correctness story does not survive 89-day
-- retention plus downstream warehouse pruning.
--
-- The drop is safe even on a populated table: no production code
-- path ever inserted `resource_kind = 'tenant'`, so the new CHECK
-- accepts every existing row. The migration is additive in shape
-- (DROP + ADD with the same name) to keep the constraint name
-- stable for any operator query that joins on it.

ALTER TABLE undo_snapshots
    DROP CONSTRAINT IF EXISTS undo_snapshots_resource_kind_check;

ALTER TABLE undo_snapshots
    ADD CONSTRAINT undo_snapshots_resource_kind_check CHECK (
        resource_kind IN (
            'user',
            'team',
            'clickhouse_rule',
            'clickhouse_mart',
            'clickhouse_retention',
            'flow_def',
            'rubix.dashboard.page'
        )
    );
