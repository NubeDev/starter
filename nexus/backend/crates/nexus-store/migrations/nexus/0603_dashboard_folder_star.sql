-- Place a dashboard in a folder and mark it starred (WS-05). Both add-only with
-- safe defaults so existing rows stay valid: folder_id NULL means "filed at the
-- root", starred false means "not a favourite". folder_id references a folder in
-- the same tenant (RLS scopes the candidate rows); ON DELETE SET NULL so
-- deleting a folder re-roots its dashboards rather than destroying them.
ALTER TABLE nexus_dashboards
    ADD COLUMN folder_id uuid REFERENCES nexus_folders(id) ON DELETE SET NULL,
    ADD COLUMN starred   boolean NOT NULL DEFAULT false;

CREATE INDEX nexus_dashboards_folder_idx ON nexus_dashboards (folder_id);
