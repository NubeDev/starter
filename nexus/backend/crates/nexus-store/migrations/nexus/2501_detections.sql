-- Detections (WS-15): a saved insight run on a schedule to emit findings.
--
-- This is the alert-rule table's analytic sibling. Where an alert reduces a
-- query to one scalar and compares it, a detection runs the tenant's stored
-- Rhai *insight* (RW-06) over the query's result frame and records one finding
-- per flagged row. The shared scaffolding — tenant RLS, the `next_eval_at`
-- cadence column, the SECURITY DEFINER cross-tenant claim — is a deliberate
-- near-copy of `0004_alerting.sql`, so the detection scheduler can mirror the
-- proven alert scheduler rather than fork it.
--
-- A detection *references* an insight (one insight, many detections with
-- different params); the insight FK is RESTRICT because a detection with no rule
-- is meaningless (unlike a panel, which still renders its raw SQL — there the FK
-- is SET NULL). The datasource FK stays SET NULL: a detection whose datasource
-- was deleted falls back to the dev pool exactly like an alert rule.
CREATE TABLE nexus_detections (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id      text NOT NULL,
    name           text NOT NULL,
    -- The Rhai rule. RESTRICT: an insight a detection uses cannot be deleted.
    insight_id     uuid NOT NULL REFERENCES nexus_insights(id) ON DELETE RESTRICT,
    datasource_id  uuid REFERENCES nexus_datasources(id) ON DELETE SET NULL,
    -- The query whose result frame the insight runs over.
    sql            text NOT NULL,
    -- Params passed to the insight (thresholds, window, z, …).
    params         jsonb NOT NULL DEFAULT '{}'::jsonb,
    -- The insight output column whose truthy value means "this row is flagged".
    flag_column    text NOT NULL,
    -- The columns that identify the finding's target (site_id, meter_id, …).
    -- Their values form the dedup key, so a re-flagged target updates one
    -- finding instead of spawning a new one each interval.
    target_columns text[] NOT NULL DEFAULT '{}',
    -- The numeric column carried onto the finding as its `value` (optional).
    value_column   text,
    -- Optional dwell, mirroring alert `for_secs`. Default 0: most analytic
    -- findings are point-in-time. (Wired forward; v1 runner treats >0 as 0.)
    for_secs       integer NOT NULL DEFAULT 0,
    interval_secs  integer NOT NULL DEFAULT 300,
    enabled        boolean NOT NULL DEFAULT true,
    -- When the scheduler should next run this detection. Claimed with
    -- FOR UPDATE SKIP LOCKED so exactly one runner takes each.
    next_eval_at   timestamptz NOT NULL DEFAULT now(),
    created_at     timestamptz NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);
ALTER TABLE nexus_detections ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_detections FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_detections_tenant_isolation ON nexus_detections
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_detections TO nexus_runtime;
CREATE INDEX nexus_detections_due_idx ON nexus_detections (next_eval_at) WHERE enabled;

-- The cross-tenant claim, mirroring `nexus_claim_due_alert_rules`. SECURITY
-- DEFINER (owned by the migration role) exposes exactly one controlled
-- cross-tenant read — the ids and tenants of due, enabled detections — and
-- advances `next_eval_at` in the same statement so a claimed detection is not
-- re-claimed before its next interval. FOR UPDATE SKIP LOCKED keeps the claim
-- exactly-once even under a future multi-node deploy. The runner then loads and
-- runs each detection under its own tenant's RLS context.
CREATE FUNCTION nexus_claim_due_detections(batch integer)
RETURNS TABLE (id uuid, tenant_id text)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public
AS $$
    WITH due AS (
        SELECT d.id
        FROM nexus_detections d
        WHERE d.enabled AND d.next_eval_at <= now()
        ORDER BY d.next_eval_at
        LIMIT batch
        FOR UPDATE SKIP LOCKED
    )
    UPDATE nexus_detections d
    SET next_eval_at = now() + make_interval(secs => d.interval_secs)
    FROM due
    WHERE d.id = due.id
    RETURNING d.id, d.tenant_id;
$$;
REVOKE ALL ON FUNCTION nexus_claim_due_detections(integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION nexus_claim_due_detections(integer) TO nexus_runtime;
