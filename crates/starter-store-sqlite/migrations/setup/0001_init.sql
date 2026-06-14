-- Setup / Automation Builder catalog + run index (DOCS §5).
--
-- Execution state (checkpoints, resume) reuses the flow engine's
-- `runs` / `run_checkpoints` tables — this source adds ONLY the
-- template catalog and a thin run index.
--
-- SQLite dialect: JSON-as-TEXT, TEXT timestamps (CURRENT_TIMESTAMP).

CREATE TABLE IF NOT EXISTS setup_templates (
    -- Tenant is part of IDENTITY (DOCS §5): two tenants installing the
    -- same extension template id@version must not collide. The reserved
    -- sentinel '__global__' namespaces extension-provided templates that
    -- all tenants inherit (and may override with a same-(id,version) row
    -- under their own tenant_id — the read path prefers tenant rows).
    tenant_id     TEXT NOT NULL DEFAULT '__global__',
    id            TEXT NOT NULL,
    version       TEXT NOT NULL,
    display_name  TEXT NOT NULL,
    description   TEXT NOT NULL DEFAULT '',
    icon          TEXT,
    category      TEXT NOT NULL DEFAULT '',
    input_schema  TEXT NOT NULL,        -- JSON
    flow_body     TEXT NOT NULL,        -- JSON (FlowBody)
    bindings      TEXT NOT NULL,        -- JSON ({ input, output } bindings)
    access        TEXT NOT NULL,        -- JSON (teams, run_role)
    source        TEXT NOT NULL,        -- JSON
    created_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (tenant_id, id, version)
);

CREATE INDEX IF NOT EXISTS setup_templates_by_category
    ON setup_templates(tenant_id, category);

CREATE TABLE IF NOT EXISTS setup_runs (
    run_id        TEXT PRIMARY KEY,     -- FK -> runs.run_id (flow source)
    template_id   TEXT NOT NULL,
    template_ver  TEXT NOT NULL,
    owner         TEXT NOT NULL,
    tenant_id     TEXT,
    team          TEXT,
    status        TEXT NOT NULL,        -- Pending|Running|Failed|Completed|Cancelled
    progress_json TEXT NOT NULL,        -- { done, total, current_step }
    failed_node   TEXT,                 -- DOCS §8b resume cursor
    resumable     INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at   TEXT
);

CREATE INDEX IF NOT EXISTS setup_runs_by_owner  ON setup_runs(owner, created_at);
CREATE INDEX IF NOT EXISTS setup_runs_by_tenant ON setup_runs(tenant_id, created_at);
CREATE INDEX IF NOT EXISTS setup_runs_open
    ON setup_runs(status) WHERE status IN ('Pending', 'Running', 'Failed');
