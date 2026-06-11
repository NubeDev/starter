-- com.acme.devices.site_checkout — the post-provision verify page query-kind
-- (DOCS identity-scoped-pages §5). Scoped by `$caller_team_ids` (the P3a host
-- token, bound from Principal.teams, un-spoofable). On a real deployment this
-- joins `meters`/`meter_latest` filtered by `site_team = ANY($caller_team_ids)`;
-- with `tables: []` (robust on a fresh DB) it echoes the caller's team scope so
-- the page proves the team token reaches the query path.
SELECT
    $caller_team_ids                         AS site_teams,
    $caller_tenant_id                        AS owner_tenant,
    'reporting'                              AS status,
    NOW()                                    AS checked_at;
