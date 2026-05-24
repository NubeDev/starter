-- starter-ext-store-pg: enable/disable persistence.
--
-- One row per extension id. Default for a freshly-loaded extension is
-- `enabled`; the row only gets written when an operator explicitly
-- toggles state (or the host autostart routine reconciles).
CREATE TABLE IF NOT EXISTS extensions_enablement (
    extension_id TEXT        PRIMARY KEY,
    state        TEXT        NOT NULL CHECK (state IN ('enabled', 'disabled')),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by   TEXT        NOT NULL
);
