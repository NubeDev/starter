-- Detection notifications: the "alert-type" detection.
--
-- This folds the one capability the deleted alert subsystem had that detections
-- lacked — delivering a notification when something fires — onto the detection /
-- findings model. Rather than a parallel ok→pending→firing state machine, a
-- detection notifies off its findings' existing lifecycle: when a finding opens
-- it is a "firing" signal, when it auto-resolves it is a "resolved" signal. The
-- finding IS the state; we only add the delivery layer on top.
--
-- Three pieces, ported from `0004_alerting.sql` and re-homed under detections:
--   * channels  — webhook|slack|email delivery targets (was nexus_alert_channels)
--   * silences  — maintenance windows that suppress delivery (was nexus_alert_silences)
--   * a per-detection `channel_ids` list + a notify-event log
-- All tenant-scoped and RLS-isolated like the rest of the control plane.

-- A delivery target. `kind` selects the notifier impl (webhook|slack|email);
-- `config` is the kind-specific settings (the webhook url, the slack url, the
-- smtp host/from/to/password). Secrets in `config` are redacted on read.
CREATE TABLE nexus_notify_channels (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id  text NOT NULL,
    name       text NOT NULL,
    kind       text NOT NULL,
    config     jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);
ALTER TABLE nexus_notify_channels ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_notify_channels FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_notify_channels_tenant_isolation ON nexus_notify_channels
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_notify_channels TO nexus_runtime;

-- A maintenance window that suppresses notification (not detection runs) for one
-- detection — or, when detection_id is null, for the whole tenant — between
-- starts_at/ends_at. The runner still records findings while silenced; it just
-- doesn't deliver.
CREATE TABLE nexus_notify_silences (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    text NOT NULL,
    detection_id uuid REFERENCES nexus_detections(id) ON DELETE CASCADE,
    starts_at    timestamptz NOT NULL,
    ends_at      timestamptz NOT NULL,
    reason       text,
    created_by   text NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now()
);
ALTER TABLE nexus_notify_silences ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_notify_silences FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_notify_silences_tenant_isolation ON nexus_notify_silences
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_notify_silences TO nexus_runtime;
CREATE INDEX nexus_notify_silences_active_idx ON nexus_notify_silences (tenant_id, ends_at);

-- The channels a detection fans its finding transitions out to. Empty (the
-- default) means the detection records findings but notifies no one — the pure
-- analytic detection; a non-empty list makes it an "alert-type" detection.
ALTER TABLE nexus_detections
    ADD COLUMN channel_ids uuid[] NOT NULL DEFAULT '{}',
    -- Optional override of the default notification message; NULL uses the
    -- built-in template. Mirrors the old per-rule message_template.
    ADD COLUMN message_template text;

-- Append-only history of notifications a detection delivered. One row per
-- finding transition (opened|resolved) the runner tried to notify on, including
-- silenced ones so the history stays honest even when no one was paged.
-- `finding_id` ties the notification back to the spark that triggered it.
CREATE TABLE nexus_notify_events (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    text NOT NULL,
    detection_id uuid NOT NULL REFERENCES nexus_detections(id) ON DELETE CASCADE,
    finding_id   uuid REFERENCES nexus_findings(id) ON DELETE SET NULL,
    at           timestamptz NOT NULL DEFAULT now(),
    -- The finding transition that triggered this: opened | resolved.
    transition   text NOT NULL,
    value        double precision,
    -- Whether an active silence suppressed delivery, and whether at least one
    -- channel delivery succeeded.
    silenced     boolean NOT NULL DEFAULT false,
    notified     boolean NOT NULL DEFAULT false,
    detail       text
);
ALTER TABLE nexus_notify_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_notify_events FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_notify_events_tenant_isolation ON nexus_notify_events
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_notify_events TO nexus_runtime;
CREATE INDEX nexus_notify_events_detection_idx ON nexus_notify_events (detection_id, at DESC);
