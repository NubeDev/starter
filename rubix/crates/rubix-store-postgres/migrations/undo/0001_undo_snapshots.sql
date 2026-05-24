-- rubix Phase A.2 — undo_snapshots dimension table.
--
-- Backs every `Reversible` rubix tool: before a destructive
-- write, the tool snapshots prior state into this table; the
-- `rubix.undo.last` verb reads the most recent live snapshot
-- for `(tenant_id, resource_kind, resource_id)` and replays the
-- inverse op. Retention is bounded by the boot-time +
-- 24h-tick sweep in `rubix-agent::boot::undo_sweep` which keeps
-- the smaller of N most-recent rows (default 50, configurable
-- via `[undo] max_rows_per_resource`) or rows newer than X days
-- (default 90, configurable via `[undo] max_age_days`).
--
-- Column notes:
--
-- - `id ULID` — stored as `TEXT` to stay portable across the
--   sqlite twin a future stage may add; ULIDs are time-sortable
--   so `ORDER BY id DESC` is a valid proxy for newest-first.
-- - `tenant_id UUID` — populated from the principal on the
--   request context; system-scope rows (e.g. ClickHouse
--   retention on a system table) use the all-zero sentinel per
--   `rubix/docs/sessions/2026-05-24-smoke-test-pr30.md` Step 5.
-- - `actor_id` — UUID of the principal that performed the
--   write. Stored even when equal to `tenant_id` so the audit
--   trail survives tenant-admin handovers.
-- - `resource_kind` — one of the seven values listed in SCOPE
--   Phase A; held as `TEXT` + `CHECK` rather than a Postgres
--   `ENUM` so adding new kinds in later phases is an additive
--   migration, not an `ALTER TYPE`.
-- - `snapshot_jsonb JSONB` — verb-specific shape; documented
--   per goal in `docs/design/<goal>/README.md`. NULL is invalid
--   (see `NOT NULL`); an empty `{}` is the canonical "resource
--   did not exist before" marker.
-- - `created_at TIMESTAMPTZ` — wall clock at snapshot time;
--   `idx_age` uses this for the X-day retention prune.
-- - `superseded_at TIMESTAMPTZ NULL` — set when `rubix.undo.last`
--   consumes the row so a subsequent undo skips it. The
--   `idx_live` partial index keeps the live-snapshot lookup
--   cheap regardless of how many superseded rows accumulate
--   between sweep ticks.

CREATE TABLE IF NOT EXISTS undo_snapshots (
    id             TEXT        PRIMARY KEY,
    tenant_id      UUID        NOT NULL,
    actor_id       UUID        NOT NULL,
    resource_kind  TEXT        NOT NULL,
    resource_id    TEXT        NOT NULL,
    snapshot_jsonb JSONB       NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    superseded_at  TIMESTAMPTZ NULL,
    CONSTRAINT undo_snapshots_resource_kind_check CHECK (
        resource_kind IN (
            'user',
            'team',
            'tenant',
            'clickhouse_rule',
            'clickhouse_mart',
            'clickhouse_retention',
            'flow_def'
        )
    )
);

-- Newest-first scan per resource — the hot path for both
-- `rubix.undo.last` (LIMIT 1 WHERE superseded_at IS NULL) and
-- for the retention sweep's N-row-keep computation.
CREATE INDEX IF NOT EXISTS idx_undo_snapshots_resource
    ON undo_snapshots (tenant_id, resource_kind, resource_id, created_at DESC);

-- Age-based prune index; the sweep runs `DELETE WHERE created_at
-- < NOW() - $1 days` and benefits from a plain b-tree on the
-- timestamp.
CREATE INDEX IF NOT EXISTS idx_undo_snapshots_created_at
    ON undo_snapshots (created_at);

-- Partial index over live snapshots only — the `rubix.undo.last`
-- lookup never wants superseded rows, and the partial index
-- keeps that probe O(1) even when superseded history is large.
CREATE INDEX IF NOT EXISTS idx_undo_snapshots_live
    ON undo_snapshots (tenant_id, resource_kind, resource_id, created_at DESC)
    WHERE superseded_at IS NULL;
