-- Setup / Automation Builder catalog + run index (DOCS §5) — Postgres
-- twin of the SQLite migration.
--
-- Dialect translation:
--   * JSON-as-TEXT        -> JSONB
--   * TEXT timestamps     -> TIMESTAMPTZ DEFAULT NOW()
--   * INTEGER bool        -> BOOLEAN
--   * CURRENT_TIMESTAMP   -> NOW()

CREATE TABLE IF NOT EXISTS setup_templates (
    -- Tenant is part of IDENTITY (DOCS §5); '__global__' namespaces
    -- extension-provided templates all tenants inherit.
    tenant_id     TEXT NOT NULL DEFAULT '__global__',
    id            TEXT NOT NULL,
    version       TEXT NOT NULL,
    display_name  TEXT NOT NULL,
    description   TEXT NOT NULL DEFAULT '',
    icon          TEXT,
    category      TEXT NOT NULL DEFAULT '',
    input_schema  JSONB NOT NULL,
    flow_body     JSONB NOT NULL,
    bindings      JSONB NOT NULL,
    access        JSONB NOT NULL,
    source        JSONB NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
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
    status        TEXT NOT NULL,
    progress_json JSONB NOT NULL,
    failed_node   TEXT,                 -- DOCS §8b resume cursor
    resumable     BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at   TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS setup_runs_by_owner  ON setup_runs(owner, created_at);
CREATE INDEX IF NOT EXISTS setup_runs_by_tenant ON setup_runs(tenant_id, created_at);
CREATE INDEX IF NOT EXISTS setup_runs_open
    ON setup_runs(status) WHERE status IN ('Pending', 'Running', 'Failed');
