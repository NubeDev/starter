-- Dashboard appearance: a lucide icon name and an accent colour, both used by
-- the UI for the sidebar entry and page chrome. Added add-only with defaults so
-- existing rows stay valid; the accent is stored as an HSL triple string
-- ("152 76% 44%") to match how the UI applies it (`hsl(var-or-literal)`), and
-- the icon as a lucide component name.
ALTER TABLE nexus_dashboards
    ADD COLUMN icon   text NOT NULL DEFAULT 'Activity',
    ADD COLUMN accent text NOT NULL DEFAULT '152 76% 44%';
