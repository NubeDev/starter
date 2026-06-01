# `com.nubeio.rubixos.bc_provision`

The provisioning orchestrator. Takes a scanned identity plus a
site / location / page placement and trend/alarm toggles, then
materialises the device, its points, page widgets and alarm rules
in one pass, writing an audit trail to `bc_provision_log`.
Idempotent on `device_id`: re-scanning an existing device repairs
its rows rather than duplicating them.
