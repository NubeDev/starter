-- com.acme.devices.device_detail — a SINGLE-device read, pinned by `device_id`.
--
-- This backs a per-user "My devices" dashboard panel, which is bound to ONE
-- specific device (the one that user registered). Unlike `devices_list` (the
-- team-scoped list of "all my devices"), this returns exactly the row named by
-- the `$device_id` param, so the panel shows that one device to ANYONE
-- authorised to open the page — the owner sees their device, and an admin
-- opening the same page sees that same single device (not the whole fleet).
--
-- Still tenant-scoped by the un-spoofable `$caller_tenant_id` host token, so a
-- caller can never read a device id from another tenant. Access to the PAGE is
-- gated separately by the dashboard + nav grants (the owner's team + admins);
-- this query just renders the page's pinned device.
SELECT
    "device_id",
    "barcode",
    "location",
    "owner",
    "team",
    "created_at"
FROM "com_acme_devices__devices"
WHERE "tenant_id" = $caller_tenant_id
  AND "device_id" = $device_id;
