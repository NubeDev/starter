# `com.nubeio.ce.device_*`

CRUD over the `ce_devices` catalog — the registered control engines.

- `device_create` — register an engine (IP, port, username, password).
- `device_update` — set columns on one engine row (keyed by `device_id`).
- `device_delete` — remove an engine row.

`tenant_id` is bound host-side and must never be supplied by the
caller. The connection password is held server-side so the browser
never sees it.
