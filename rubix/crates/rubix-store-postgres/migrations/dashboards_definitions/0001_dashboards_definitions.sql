-- rubix Phase A.1 (Goal 1) — dashboards_definitions dimension table.
--
-- Source-of-truth store for SDUI dashboard page bodies. Mirrors
-- `flows_definitions` (sibling migration source under this crate):
-- bundled pages are seeded under the all-zero `(tenant_id,
-- created_by)` sentinel on first boot; subsequent edits land as new
-- revisions and supersede the previous head via `superseded_at`.
-- Per `rubix/docs/scope/dashboards/01-storage.md` every write is
-- insert-only; "latest" = `superseded_at IS NULL`.
--
-- Column notes:
--
-- - `page_id`  — the SDUI page id (e.g. `"dashboard.disk-overview"`).
--   Stable across revisions.
-- - `revision_id` — UUID generated server-side; round-trips back to
--   the in-memory `DashboardRevision::revision_id`.
-- - `body_json` — a fully-typed `starter_ui_ir::ComponentTree`
--   serialised as JSON. Validation happens at the rubix-tools write
--   path (see `04-tools.md` in the scope); the column stores
--   whatever the writer produced byte-stable.
-- - `tenant_id` — populated from the authoring principal; bundled
--   rows use the all-zero sentinel ("system seeded"). Tenant
--   filtering happens in `list_active`.
-- - `owner_principal` — the principal who can `edit` / `delete`;
--   `"system"` for bundled rows.
-- - `tags[]` — powers "show me dashboards tagged X" via the
--   `starter-tags` substrate. Indexed via GIN over the live set.
-- - `superseded_at` — set when a newer revision replaces this one;
--   the active-set query filters on `superseded_at IS NULL`.

CREATE TABLE IF NOT EXISTS dashboards_definitions (
    page_id          TEXT        NOT NULL,
    revision_id      UUID        NOT NULL DEFAULT gen_random_uuid(),
    body_json        JSONB       NOT NULL,
    tenant_id        TEXT        NOT NULL,
    owner_principal  TEXT        NOT NULL,
    title            TEXT        NOT NULL,
    tags             TEXT[]      NOT NULL DEFAULT '{}',
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by       TEXT        NOT NULL,
    superseded_at    TIMESTAMPTZ NULL,
    PRIMARY KEY (page_id, revision_id)
);

-- Hot path: the page-resolve query reads the one live row per
-- `(tenant_id, page_id)`; the listing query scans by tenant.
CREATE INDEX IF NOT EXISTS dashboards_definitions_active_idx
    ON dashboards_definitions (tenant_id, page_id)
    WHERE superseded_at IS NULL;

CREATE INDEX IF NOT EXISTS dashboards_definitions_tags_idx
    ON dashboards_definitions USING GIN (tags)
    WHERE superseded_at IS NULL;

-- Cross-instance reload channel. Every INSERT pushes a JSON payload
-- onto `rubix_dashboards_definitions`; the listener in
-- `rubix-agent::boot::dashboards_notify` (Phase A.2 — host glue)
-- invalidates the in-process `PageProvider` cache so a page edited
-- on instance A is picked up by instances B/C without a redeploy.
CREATE OR REPLACE FUNCTION dashboards_definitions_notify() RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify(
        'rubix_dashboards_definitions',
        json_build_object(
            'op',            TG_OP,
            'page_id',       NEW.page_id,
            'revision_id',   NEW.revision_id,
            'tenant_id',     NEW.tenant_id,
            'superseded_at', NEW.superseded_at
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS dashboards_definitions_notify_trg ON dashboards_definitions;
CREATE TRIGGER dashboards_definitions_notify_trg
    AFTER INSERT OR UPDATE ON dashboards_definitions
    FOR EACH ROW EXECUTE FUNCTION dashboards_definitions_notify();
