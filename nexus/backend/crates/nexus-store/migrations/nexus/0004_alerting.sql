-- Alerting: rules, their evaluation state, the event history, notification
-- channels, and silences. All tenant-scoped and RLS-isolated like the rest of
-- the control plane; grants and refs key on the immutable rule id. The
-- evaluator reads/writes these inside the tenant transaction, so a rule's state
-- never races across tenants and a restart resumes from the persisted state.

-- A rule is a saved query + a comparison against a threshold, evaluated on its
-- own cadence. `for_secs` is the pending dwell: the rule must breach this long
-- before it fires, absorbing a transient spike. `channel_ids` lists the
-- notification targets a transition fans out to.
CREATE TABLE nexus_alert_rules (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     text NOT NULL,
    name          text NOT NULL,
    datasource_id uuid REFERENCES nexus_datasources(id) ON DELETE SET NULL,
    -- The query whose first row/first column is the evaluated value.
    query         text NOT NULL,
    -- Comparison operator: gt|gte|lt|lte|eq|ne.
    op            text NOT NULL,
    threshold     double precision NOT NULL,
    -- Pending dwell before firing, and the evaluation cadence.
    for_secs      integer NOT NULL DEFAULT 0,
    interval_secs integer NOT NULL DEFAULT 60,
    enabled       boolean NOT NULL DEFAULT true,
    channel_ids   uuid[] NOT NULL DEFAULT '{}',
    -- When the scheduler should next evaluate this rule. Past-due rules are
    -- claimed with FOR UPDATE SKIP LOCKED so exactly one evaluator takes each.
    next_eval_at  timestamptz NOT NULL DEFAULT now(),
    created_at    timestamptz NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);
ALTER TABLE nexus_alert_rules ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_alert_rules FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_alert_rules_tenant_isolation ON nexus_alert_rules
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_alert_rules TO nexus_runtime;
CREATE INDEX nexus_alert_rules_due_idx ON nexus_alert_rules (next_eval_at) WHERE enabled;

-- The scheduler is a system actor, not a tenant request: it must discover due
-- rules across every tenant, which RLS (correctly) forbids the runtime role from
-- doing directly. Rather than give the runtime role BYPASSRLS (a blanket hole),
-- this SECURITY DEFINER function — owned by the migration role — exposes exactly
-- one controlled cross-tenant read: the ids and tenants of due, enabled rules.
-- It also advances next_eval_at in the same statement so a claimed rule is not
-- re-claimed before its next interval, and uses FOR UPDATE SKIP LOCKED so even a
-- second evaluator (a future multi-node deploy) takes each rule exactly once.
-- The function returns only (id, tenant_id); the evaluator then loads and
-- evaluates each rule under its own tenant's RLS context.
CREATE FUNCTION nexus_claim_due_alert_rules(batch integer)
RETURNS TABLE (id uuid, tenant_id text)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public
AS $$
    WITH due AS (
        SELECT r.id
        FROM nexus_alert_rules r
        WHERE r.enabled AND r.next_eval_at <= now()
        ORDER BY r.next_eval_at
        LIMIT batch
        FOR UPDATE SKIP LOCKED
    )
    UPDATE nexus_alert_rules r
    SET next_eval_at = now() + make_interval(secs => r.interval_secs)
    FROM due
    WHERE r.id = due.id
    RETURNING r.id, r.tenant_id;
$$;
-- Only the runtime role may invoke it; the function's definer rights are scoped
-- to the single read it performs, not handed to the caller generally.
REVOKE ALL ON FUNCTION nexus_claim_due_alert_rules(integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION nexus_claim_due_alert_rules(integer) TO nexus_runtime;

-- One state row per rule: the state machine's memory. `state` is one of
-- ok|pending|firing|resolved; `since` is when it entered that state (drives the
-- `for_secs` dwell); `last_value` is the most recent evaluated value.
CREATE TABLE nexus_alert_rule_state (
    rule_id      uuid PRIMARY KEY REFERENCES nexus_alert_rules(id) ON DELETE CASCADE,
    tenant_id    text NOT NULL,
    state        text NOT NULL DEFAULT 'ok',
    since        timestamptz NOT NULL DEFAULT now(),
    last_eval_at timestamptz,
    last_value   double precision
);
ALTER TABLE nexus_alert_rule_state ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_alert_rule_state FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_alert_rule_state_tenant_isolation ON nexus_alert_rule_state
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_alert_rule_state TO nexus_runtime;

-- Append-only history of transitions. Written on every firing/resolved
-- transition, including silenced ones (the history stays honest even when no one
-- was paged).
CREATE TABLE nexus_alert_events (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id  text NOT NULL,
    rule_id    uuid NOT NULL REFERENCES nexus_alert_rules(id) ON DELETE CASCADE,
    at         timestamptz NOT NULL DEFAULT now(),
    -- The transition: firing | resolved.
    transition text NOT NULL,
    value      double precision,
    -- Whether an active silence suppressed notification, and whether the channel
    -- delivery actually succeeded.
    silenced   boolean NOT NULL DEFAULT false,
    notified   boolean NOT NULL DEFAULT false,
    detail     text
);
ALTER TABLE nexus_alert_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_alert_events FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_alert_events_tenant_isolation ON nexus_alert_events
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_alert_events TO nexus_runtime;
CREATE INDEX nexus_alert_events_rule_idx ON nexus_alert_events (rule_id, at DESC);

-- A delivery target. `kind` selects the Notifier impl (webhook in v1); `config`
-- is the kind-specific settings (e.g. the webhook url).
CREATE TABLE nexus_alert_channels (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id  text NOT NULL,
    name       text NOT NULL,
    kind       text NOT NULL,
    config     jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);
ALTER TABLE nexus_alert_channels ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_alert_channels FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_alert_channels_tenant_isolation ON nexus_alert_channels
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_alert_channels TO nexus_runtime;

-- A maintenance window that suppresses notification (not evaluation) for a rule
-- — or, when rule_id is null, for the whole tenant — between starts_at/ends_at.
CREATE TABLE nexus_alert_silences (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id  text NOT NULL,
    rule_id    uuid REFERENCES nexus_alert_rules(id) ON DELETE CASCADE,
    starts_at  timestamptz NOT NULL,
    ends_at    timestamptz NOT NULL,
    reason     text,
    created_by text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
ALTER TABLE nexus_alert_silences ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_alert_silences FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_alert_silences_tenant_isolation ON nexus_alert_silences
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_alert_silences TO nexus_runtime;
CREATE INDEX nexus_alert_silences_active_idx ON nexus_alert_silences (tenant_id, ends_at);
