-- com.acme.devices.devices_list — the read API over the extension-owned
-- `com_acme_devices__devices` table (WS-17 §4.1.3). The owned table is the
-- storage; this query-kind is the read API.
--
-- Tenant- and team-scoped by the un-spoofable host tokens: `$caller_tenant_id`
-- (bound from the caller's tenant) clamps to the tenant's rows, and
-- `$caller_team_ids` (P3a, bound from Principal.teams) narrows to the caller's
-- teams. A row whose `team` is NULL/'' is visible to every member of the tenant
-- (an unassigned device); a row with a team is visible only to that team — so a
-- `hvac-ops` installer sees their site's devices, not another team's.
SELECT
    "device_id",
    "barcode",
    "location",
    "owner",
    "team",
    "created_at"
FROM "com_acme_devices__devices"
WHERE "tenant_id" = $caller_tenant_id
  AND (
        "team" IS NULL
     OR "team" = ''
     OR "team" = ANY($caller_team_ids)
      )
ORDER BY "created_at" DESC, "device_id" ASC;
