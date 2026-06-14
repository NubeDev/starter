# `com.acme.devices.device_create`

Provisions a device from a scanned `barcode`. The **natural dedup key is the
barcode** (DOCS §8c): calling it twice with the same barcode returns the same
`device_id` and creates no second device — which is exactly what makes
**resume-from-failure** safe. If the run halts after this step and is resumed,
the device is not re-created.

Identity (`caller_user_id` / `caller_team_ids` / `caller_tenant_id`) is read
from the **server-seeded trusted slots** (DOCS §9), never from form input — so
an installer cannot spoof which site/owner the device is tagged to.
