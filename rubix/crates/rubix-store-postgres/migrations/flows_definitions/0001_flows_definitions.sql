-- rubix Phase D.1 — flows_definitions dimension table.
--
-- Source-of-truth store for rubix flow definitions. The bundled
-- YAML files in `rubix-flows/flows/` are seeded into this table
-- on first boot per `(tenant_id, flow_id)`; from that point on
-- the table is the authoritative source the `FlowRegistry` loads
-- from. Edits made through the goal-3 `flow-programmer` verbs
-- write a *new* revision and supersede the previous head via
-- `superseded_at`; revisions are immutable per starter's
-- SCOPE ("revisions are immutable; `head_seq` pointer per flow
-- tracks the current revision").
--
-- Column notes:
--
-- - `id TEXT` — ULID (time-sortable) so `ORDER BY id DESC` is a
--   valid newest-first proxy. Stored as TEXT to stay portable with
--   any future sqlite twin.
-- - `tenant_id UUID` — populated from the principal that authored
--   the revision; bundled YAMLs land under the all-zero sentinel
--   per `rubix/docs/sessions/2026-05-24-smoke-test-pr30.md`.
-- - `flow_id TEXT` — the reverse-DNS flow id (e.g.
--   `com.rubix.flow-programmer`). Validated upstream by
--   `FlowId::new` before any insert reaches PG.
-- - `revision_id TEXT` — UUID rendered as text; the same value the
--   in-memory `FlowRevisionId` carries so the `FlowRegistry`'s
--   `(flow_id, revision_id)` key round-trips losslessly across
--   the seed-load boundary.
-- - `body_yaml TEXT` — the raw YAML text. Stored verbatim so the
--   flow-programmer round-trip is byte-stable; parsing happens at
--   load time via `rubix_flows::parse_yaml`.
-- - `created_by UUID` — actor UUID; bundled rows use the all-zero
--   sentinel ("system seeded").
-- - `superseded_at TIMESTAMPTZ NULL` — set when a newer revision
--   replaces this one; the `FlowRegistry` load query filters on
--   `superseded_at IS NULL`.
--
-- The UNIQUE constraint guarantees that the seeder's
-- "INSERT … ON CONFLICT DO NOTHING" idempotency probe on
-- `(tenant_id, flow_id, revision_id)` is enforced at the
-- database level — a second boot cannot accidentally duplicate
-- a row even under a race between two rubix-agent replicas.

CREATE TABLE IF NOT EXISTS flows_definitions (
    id            TEXT        PRIMARY KEY,
    tenant_id     UUID        NOT NULL,
    flow_id       TEXT        NOT NULL,
    revision_id   TEXT        NOT NULL,
    body_yaml     TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by    UUID        NOT NULL,
    superseded_at TIMESTAMPTZ NULL,
    CONSTRAINT flows_definitions_unique_revision
        UNIQUE (tenant_id, flow_id, revision_id)
);

-- Hot path: the boot-time loader reads every live revision for
-- one tenant in one scan; the partial index over the live set
-- keeps the lookup cheap even when superseded history is large.
CREATE INDEX IF NOT EXISTS idx_flows_definitions_live
    ON flows_definitions (tenant_id, flow_id, created_at DESC)
    WHERE superseded_at IS NULL;

-- Cross-instance reload channel. Every INSERT or UPDATE pushes a
-- JSON payload onto `rubix_flows_definitions`; the listener in
-- `rubix-agent::boot::flow_notify` consumes it and calls into the
-- in-process `FlowRegistry` reload helper so a flow edited on
-- instance A is picked up by instances B/C without a redeploy.
CREATE OR REPLACE FUNCTION flows_definitions_notify() RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify(
        'rubix_flows_definitions',
        json_build_object(
            'op',            TG_OP,
            'id',            NEW.id,
            'tenant_id',     NEW.tenant_id,
            'flow_id',       NEW.flow_id,
            'revision_id',   NEW.revision_id,
            'superseded_at', NEW.superseded_at
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS flows_definitions_notify_trg ON flows_definitions;
CREATE TRIGGER flows_definitions_notify_trg
    AFTER INSERT OR UPDATE ON flows_definitions
    FOR EACH ROW EXECUTE FUNCTION flows_definitions_notify();
