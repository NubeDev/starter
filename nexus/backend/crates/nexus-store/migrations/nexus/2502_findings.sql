-- Findings (WS-15): a persistent "spark" — one per flagged row per detection.
--
-- This generalises `nexus_alert_events` (0004). An alert event is one row per
-- rule transition; a finding is one row per *target* (which meter, which site),
-- with an identifying `target` jsonb, the flagged `value`, the rest of the
-- flagged row as `context`, and a workflow lifecycle (open → acknowledged →
-- resolved). Twenty anomalous meters in one run = twenty findings, each
-- browsable and ackable — the core difference from an alert.
--
-- Dedup is the single most important design decision in the WS. `dedup_key` is
-- the hash of the target column values; a partial unique index over only the
-- non-resolved rows means a target that stays flagged for N consecutive
-- intervals is ONE open finding (its `value`/`at` updated), not N. Once a
-- finding resolves it leaves the unique set, so the *next* time that target
-- flags it opens a fresh finding — the resolved one stays as history.
CREATE TABLE nexus_findings (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    text NOT NULL,
    detection_id uuid NOT NULL REFERENCES nexus_detections(id) ON DELETE CASCADE,
    -- Event time of the flagged row (from the detection's time column, or now()).
    at           timestamptz NOT NULL DEFAULT now(),
    -- The identifying column values, e.g. {"site":"s1","meter":"m7"}.
    target       jsonb NOT NULL DEFAULT '{}'::jsonb,
    -- The flagged row's numeric value (from the detection's value_column).
    value        double precision,
    -- The flagged row's other derived columns (zscore, baseline, …) for "why".
    context      jsonb NOT NULL DEFAULT '{}'::jsonb,
    -- Lifecycle: open | acknowledged | resolved.
    status       text NOT NULL DEFAULT 'open',
    acked_by     text,
    acked_at     timestamptz,
    resolved_at  timestamptz,
    note         text,
    -- Hash of the target values; unique among non-resolved rows (see index).
    dedup_key    text NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
ALTER TABLE nexus_findings ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_findings FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_findings_tenant_isolation ON nexus_findings
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_findings TO nexus_runtime;

-- Dedup: at most one non-resolved finding per (detection, dedup_key). The
-- runner's upsert targets this constraint; a resolved finding is excluded, so
-- the same target can recur as a new finding after resolution.
CREATE UNIQUE INDEX nexus_findings_open_dedup_idx
    ON nexus_findings (detection_id, dedup_key)
    WHERE status <> 'resolved';

-- Browse/trend access paths: by detection+status (the detection's open list)
-- and by status+time (the tenant-wide findings feed, newest first).
CREATE INDEX nexus_findings_detection_idx ON nexus_findings (tenant_id, detection_id, status);
CREATE INDEX nexus_findings_feed_idx ON nexus_findings (tenant_id, status, at DESC);
