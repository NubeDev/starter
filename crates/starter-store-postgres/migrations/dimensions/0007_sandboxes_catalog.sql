-- Warehouse SCOPE `sandbox.define` (RF-4): analyst L1.5 sandbox catalog.
--
-- `columns_revision` tracks redefines (`sandbox.redefine` bumps the
-- counter; a cleaner promotion freezes it via `frozen_at_revision`).
-- A promoted cleaner that points at a sandbox MUST have its
-- `frozen_at_revision` match the sandbox's current `columns_revision`
-- at promote time — drift between the two is the indicator that the
-- analyst kept iterating after promote.

CREATE TABLE IF NOT EXISTS sandboxes (
    name                  TEXT PRIMARY KEY,
    description           TEXT,
    owner                 TEXT NOT NULL,
    columns               JSONB NOT NULL,
    columns_revision      BIGINT NOT NULL DEFAULT 1,
    frozen_at_revision    BIGINT,
    ttl_days              INT NOT NULL DEFAULT 30
        CHECK (ttl_days BETWEEN 1 AND 365),
    promoted_to_cleaner   TEXT,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status                TEXT NOT NULL,
    CONSTRAINT sandboxes_status_valid
        CHECK (status IN ('pending', 'live', 'promoted', 'failed')),
    CONSTRAINT sandboxes_owner_valid CHECK (
        owner LIKE 'user:%' OR owner LIKE 'agent:%'
    ),
    CONSTRAINT sandboxes_name_shape
        CHECK (name ~ '^sandbox_[a-z0-9_]+$')
);
