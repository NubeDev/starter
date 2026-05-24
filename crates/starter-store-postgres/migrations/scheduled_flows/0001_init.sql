-- starter Phase A.2 — scheduled_flows dimension table.
--
-- Source-of-truth store for durable per-flow cron schedules. Rows
-- are written by `FlowAsService::register_schedule` (Phase B) and
-- claimed by `FlowAsService::tick` via `SELECT … FOR UPDATE SKIP
-- LOCKED` so multiple rubix-agent instances can share the table
-- without double-firing a schedule.
--
-- Column notes:
--
-- - `id TEXT` — ULID (time-sortable) so `ORDER BY id DESC` is a
--   valid newest-first proxy. Stored as TEXT to stay portable
--   with the sqlite twin.
-- - `tenant_id UUID` — populated from the principal that
--   registered the schedule; bundled flows land under the all-zero
--   sentinel (mirrors `flows_definitions`).
-- - `flow_id TEXT` — reverse-DNS flow id (e.g.
--   `com.rubix.weekly-report`). Validated upstream by
--   `FlowId::new` before any insert reaches PG.
-- - `cron_expr TEXT` — the raw cron expression. Validated by
--   `starter_cron::next_fire` before insert; stored verbatim so
--   round-trips are byte-stable.
-- - `next_run_at TIMESTAMPTZ` — the next wall-clock time the tick
--   loop should fire this schedule. Recomputed via
--   `starter_cron::next_fire` after each fire.
-- - `last_run_at TIMESTAMPTZ NULL` — wall-clock timestamp of the
--   most recent fire (NULL until the first fire).
-- - `last_run_status TEXT NULL` — one of `succeeded`, `failed`,
--   `cancelled`. Enforced at the application layer; CHECK keeps
--   the column honest at the DB level.
-- - `last_run_message TEXT NULL` — failure summary if any;
--   capped at 4 KB by the application layer (truncate-on-write).
-- - `enabled BOOL` — soft delete / pause switch. The tick loop
--   filters on `enabled = TRUE`.
-- - `created_by UUID` — actor UUID; bundled rows use the all-zero
--   sentinel ("system seeded").
--
-- The UNIQUE constraint guarantees one schedule per
-- `(tenant_id, flow_id)`. The seeder relies on it for its
-- `INSERT … ON CONFLICT DO NOTHING` idempotency probe.

CREATE TABLE IF NOT EXISTS starter_scheduled_flows (
    id                TEXT        PRIMARY KEY,
    tenant_id         UUID        NOT NULL,
    flow_id           TEXT        NOT NULL,
    cron_expr         TEXT        NOT NULL,
    next_run_at       TIMESTAMPTZ NOT NULL,
    last_run_at       TIMESTAMPTZ NULL,
    last_run_status   TEXT        NULL,
    last_run_message  TEXT        NULL,
    enabled           BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by        UUID        NOT NULL,
    CONSTRAINT starter_scheduled_flows_unique
        UNIQUE (tenant_id, flow_id),
    CONSTRAINT starter_scheduled_flows_status_check
        CHECK (last_run_status IS NULL
               OR last_run_status IN ('succeeded', 'failed', 'cancelled'))
);

-- Hot path: the tick loop scans the live, due set every 60s. The
-- partial index over `enabled` rows ordered by `next_run_at`
-- keeps that scan O(due) regardless of how many disabled rows
-- accumulate.
CREATE INDEX IF NOT EXISTS idx_starter_scheduled_flows_due
    ON starter_scheduled_flows (next_run_at)
    WHERE enabled = TRUE;

-- Cross-instance reload channel. Every INSERT or UPDATE of the
-- claim-relevant columns (`next_run_at`, `enabled`) pushes a JSON
-- payload onto `starter_scheduled_flows`; the listener in the
-- rubix-agent boot path consumes it and nudges the in-process
-- tick loop so a schedule registered on instance A is picked up
-- by instances B/C without waiting for the next 60s tick.
CREATE OR REPLACE FUNCTION starter_scheduled_flows_notify() RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify(
        'starter_scheduled_flows',
        json_build_object(
            'op',          TG_OP,
            'id',          NEW.id,
            'tenant_id',   NEW.tenant_id,
            'flow_id',     NEW.flow_id,
            'next_run_at', NEW.next_run_at,
            'enabled',     NEW.enabled
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS starter_scheduled_flows_notify_trg
    ON starter_scheduled_flows;
CREATE TRIGGER starter_scheduled_flows_notify_trg
    AFTER INSERT ON starter_scheduled_flows
    FOR EACH ROW EXECUTE FUNCTION starter_scheduled_flows_notify();

-- Separate UPDATE trigger so we can scope it to the two columns
-- that affect the claim plan; bookkeeping-only updates (e.g.
-- `last_run_at`) do not need to wake every listener.
DROP TRIGGER IF EXISTS starter_scheduled_flows_notify_upd_trg
    ON starter_scheduled_flows;
CREATE TRIGGER starter_scheduled_flows_notify_upd_trg
    AFTER UPDATE OF next_run_at, enabled ON starter_scheduled_flows
    FOR EACH ROW EXECUTE FUNCTION starter_scheduled_flows_notify();
